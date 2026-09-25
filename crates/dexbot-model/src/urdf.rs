use crate::{ModelError, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JointLimit {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub effort: Option<f64>,
    pub velocity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrdfJoint {
    pub name: String,
    pub joint_type: String,
    pub parent: Option<String>,
    pub child: Option<String>,
    pub limit: Option<JointLimit>,
    pub axis: Option<[f64; 3]>,
    /// Present when the joint follows another joint
    /// (`position = multiplier * other + offset`) and is not commandable.
    #[serde(default)]
    pub mimic: Option<JointMimic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JointMimic {
    pub joint: String,
    pub multiplier: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrdfModel {
    pub robot_name: String,
    pub links: Vec<String>,
    pub joints: Vec<UrdfJoint>,
}

impl UrdfModel {
    pub fn parse(source: &str) -> Result<Self> {
        let mut reader = Reader::from_str(source);
        reader.trim_text(true);
        let mut buffer = Vec::new();
        let mut state = ParseState::default();
        let mut depth = 0usize;
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(event)) => {
                    depth += 1;
                    state.open(&event, depth)?;
                }
                Ok(Event::Empty(event)) => {
                    state.open(&event, depth + 1)?;
                    state.close(depth + 1);
                }
                Ok(Event::End(_)) => {
                    state.close(depth);
                    depth = depth.saturating_sub(1);
                }
                Ok(Event::Eof) => {
                    // A truncated file -- an interrupted copy into an asset
                    // root, a partial write -- otherwise parses as a robot
                    // with fewer links and joints, and nothing downstream can
                    // tell that anything is missing.
                    if depth != 0 {
                        return Err(ModelError::Urdf(format!(
                            "unexpected end of file: {depth} unclosed element(s)"
                        )));
                    }
                    break;
                }
                Ok(Event::Text(text)) if depth == 0 && !text.as_ref().is_empty() => {
                    return Err(ModelError::Urdf("text outside robot root element".into()));
                }
                Ok(Event::CData(_)) if depth == 0 => {
                    return Err(ModelError::Urdf("CDATA outside robot root element".into()));
                }
                Ok(_) => {}
                Err(error) => return Err(ModelError::Urdf(error.to_string())),
            }
            buffer.clear();
        }
        let robot_name = state
            .robot_name
            .ok_or_else(|| ModelError::Urdf("robot name is missing".into()))?;
        Ok(Self {
            robot_name,
            links: state.links.into_iter().collect(),
            joints: state.joints,
        })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|source| ModelError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&source)
    }

    /// Independently movable scalar joints, in document order. Mimic joints
    /// are excluded: they follow another joint and cannot be commanded.
    pub fn movable_joint_names(&self) -> impl Iterator<Item = &str> {
        self.joints
            .iter()
            .filter(|joint| {
                joint.mimic.is_none()
                    && matches!(
                        joint.joint_type.as_str(),
                        "revolute" | "continuous" | "prismatic"
                    )
            })
            .map(|joint| joint.name.as_str())
    }

    pub fn joint(&self, name: &str) -> Option<&UrdfJoint> {
        self.joints.iter().find(|joint| joint.name == name)
    }
}

/// Depth of `<robot>`'s direct children. `<joint>` and `<link>` elements
/// elsewhere -- `<transmission><joint name=.../>`, Gazebo extensions -- only
/// reference the robot's joints and links.
const ROBOT_CHILD_DEPTH: usize = 2;

#[derive(Default)]
struct ParseState {
    robot_name: Option<String>,
    links: BTreeSet<String>,
    joints: Vec<UrdfJoint>,
    current: Option<UrdfJoint>,
}

impl ParseState {
    fn open(&mut self, event: &BytesStart<'_>, depth: usize) -> Result<()> {
        let name = event.name();
        match (depth, name.as_ref()) {
            (1, b"robot") if self.robot_name.is_none() => {
                self.robot_name = Some(required_attribute(event, b"name", "robot name")?);
            }
            (1, _) => {
                return Err(ModelError::Urdf(
                    "expected exactly one robot root element".into(),
                ))
            }
            (ROBOT_CHILD_DEPTH, b"link") => {
                let name = required_attribute(event, b"name", "link name")?;
                if !self.links.insert(name.clone()) {
                    return Err(ModelError::Urdf(format!("duplicate link name {name:?}")));
                }
            }
            (ROBOT_CHILD_DEPTH, b"joint") => {
                let joint = joint(event)?;
                if self.joints.iter().any(|known| known.name == joint.name) {
                    return Err(ModelError::Urdf(format!(
                        "duplicate joint name {:?}",
                        joint.name
                    )));
                }
                self.current = Some(joint);
            }
            (depth, element) if depth == ROBOT_CHILD_DEPTH + 1 => {
                let Some(joint) = self.current.as_mut() else {
                    return Ok(());
                };
                match element {
                    b"parent" => joint.parent = attribute(event, b"link")?,
                    b"child" => joint.child = attribute(event, b"link")?,
                    b"limit" => joint.limit = Some(limit(event)?),
                    b"axis" => joint.axis = axis(event)?,
                    b"mimic" => joint.mimic = Some(mimic(event)?),
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn close(&mut self, depth: usize) {
        if depth == ROBOT_CHILD_DEPTH {
            if let Some(joint) = self.current.take() {
                self.joints.push(joint);
            }
        }
    }
}

/// Loads the URDF referenced by a profile's `robot.urdf` URI.
///
/// `file://` URIs and plain paths require an explicit asset root and stay within
/// it (including symlink targets). Direct trusted paths can instead be read with
/// [`UrdfModel::from_file`]. For
/// `package://dexmate_urdf/<relative>` URIs the search order is the explicit
/// `asset_root`, then the directory named by `DEXBOT_ASSET_ROOT`, then the
/// URDF sources embedded in this crate. Other `package://` names are not
/// bundled and fail resolution.
pub fn load_profile_urdf(uri: &str, asset_root: Option<&Path>) -> Result<UrdfModel> {
    if let Some(path) = uri.strip_prefix("file://") {
        return load_local_urdf(path, asset_root);
    }
    if let Some(rest) = uri.strip_prefix("package://") {
        let (package, relative) = rest
            .split_once('/')
            .ok_or_else(|| ModelError::UnresolvedResource(uri.into()))?;
        if package != crate::assets::URDF_PACKAGE_NAME {
            return Err(ModelError::UnresolvedResource(uri.into()));
        }
        if let Some(root) = asset_root {
            check_relative_path(relative)?;
            return UrdfModel::from_file(contained_path(root, &root.join(relative))?);
        }
        if let Some(root) =
            std::env::var_os(crate::assets::ASSET_ROOT_ENV).filter(|value| !value.is_empty())
        {
            check_relative_path(relative)?;
            let root = PathBuf::from(root);
            return UrdfModel::from_file(contained_path(&root, &root.join(relative))?);
        }
        let (path, source) = crate::assets::urdf_entry(relative)
            .ok_or_else(|| ModelError::UnresolvedResource(uri.into()))?;
        return embedded_urdf(path, source);
    }
    if uri.contains("://") {
        return Err(ModelError::UnresolvedResource(uri.into()));
    }
    load_local_urdf(uri, asset_root)
}

/// Parses an embedded URDF asset through a process-wide cache keyed by the
/// `'static` asset path. Embedded sources are immutable for the lifetime of
/// the process, so repeated resolves (static plus discovery, or several
/// robots) reuse one parse; file-based URDFs stay uncached because the file
/// can change between resolves.
fn embedded_urdf(path: &'static str, source: &'static str) -> Result<UrdfModel> {
    static CACHE: OnceLock<Mutex<BTreeMap<&'static str, UrdfModel>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut parsed = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(model) = parsed.get(path) {
        return Ok(model.clone());
    }
    let model = UrdfModel::parse(source)?;
    parsed.insert(path, model.clone());
    Ok(model)
}

pub trait ResourceResolver: Send + Sync {
    fn resolve(&self, uri: &str) -> Result<PathBuf>;
}

#[derive(Debug, Clone, Default)]
pub struct PackageResolver {
    packages: BTreeMap<String, PathBuf>,
}

impl PackageResolver {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_package(mut self, name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        self.packages.insert(name.into(), root.into());
        self
    }
}

impl ResourceResolver for PackageResolver {
    fn resolve(&self, uri: &str) -> Result<PathBuf> {
        if let Some(path) = uri.strip_prefix("file://") {
            return Ok(PathBuf::from(path));
        }
        if !uri.contains("://") {
            return Ok(PathBuf::from(uri));
        }
        let rest = uri
            .strip_prefix("package://")
            .ok_or_else(|| ModelError::UnresolvedResource(uri.into()))?;
        let (package, relative) = rest
            .split_once('/')
            .ok_or_else(|| ModelError::UnresolvedResource(uri.into()))?;
        check_relative_path(relative)?;
        let root = self
            .packages
            .get(package)
            .ok_or_else(|| ModelError::UnresolvedResource(uri.into()))?;
        contained_path(root, &root.join(relative))
    }
}

fn joint(event: &BytesStart<'_>) -> Result<UrdfJoint> {
    Ok(UrdfJoint {
        name: required_attribute(event, b"name", "joint name")?,
        joint_type: required_attribute(event, b"type", "joint type")?,
        parent: None,
        child: None,
        limit: None,
        axis: None,
        mimic: None,
    })
}

fn mimic(event: &BytesStart<'_>) -> Result<JointMimic> {
    Ok(JointMimic {
        joint: required_attribute(event, b"joint", "mimic joint")?,
        multiplier: number_attribute(event, b"multiplier")?.unwrap_or(1.0),
        offset: number_attribute(event, b"offset")?.unwrap_or(0.0),
    })
}

fn limit(event: &BytesStart<'_>) -> Result<JointLimit> {
    Ok(JointLimit {
        lower: number_attribute(event, b"lower")?,
        upper: number_attribute(event, b"upper")?,
        effort: number_attribute(event, b"effort")?,
        velocity: number_attribute(event, b"velocity")?,
    })
}

fn axis(event: &BytesStart<'_>) -> Result<Option<[f64; 3]>> {
    let Some(value) = attribute(event, b"xyz")? else {
        return Ok(Some([1.0, 0.0, 0.0]));
    };
    let values = value
        .split_whitespace()
        .map(|item| {
            item.parse::<f64>()
                .map_err(|error| ModelError::Urdf(format!("invalid joint axis {value:?}: {error}")))
        })
        .collect::<Result<Vec<_>>>()?;
    if values.len() != 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::Urdf(format!(
            "joint axis must contain exactly three finite values, got {value:?}"
        )));
    }
    Ok(Some([values[0], values[1], values[2]]))
}

/// Rust's float parser accepts `nan`, `inf` and `infinity`; none of them is
/// a usable limit, and downstream would read them as "no limit".
fn number_attribute(event: &BytesStart<'_>, key: &[u8]) -> Result<Option<f64>> {
    attribute(event, key)?
        .map(|value| match value.trim().parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(number),
            Ok(_) => Err(ModelError::Urdf(format!(
                "numeric attribute {:?} must be finite, got {value:?}",
                String::from_utf8_lossy(key)
            ))),
            Err(error) => Err(ModelError::Urdf(format!(
                "invalid numeric attribute {value:?}: {error}"
            ))),
        })
        .transpose()
}

fn attribute(event: &BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    let mut value = None;
    // Consume the complete attribute list: returning at the first match would
    // skip duplicate or malformed attributes later in the same element.
    for item in event.attributes() {
        let item = item.map_err(|error| ModelError::Urdf(error.to_string()))?;
        if item.key.as_ref() == key {
            value = Some(
                item.unescape_value()
                    .map_err(|error| ModelError::Urdf(error.to_string()))?
                    .into_owned(),
            );
        }
    }
    Ok(value)
}

fn required_attribute(event: &BytesStart<'_>, key: &[u8], label: &str) -> Result<String> {
    attribute(event, key)?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ModelError::Urdf(format!("{label} is missing or empty")))
}

pub(crate) fn check_relative_path(path: &str) -> Result<()> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(ModelError::Validation(format!(
            "resource path must be relative without traversal: {path:?}"
        )));
    }
    Ok(())
}

/// Canonicalization checks symlinks as well as lexical traversal. Callers must
/// keep the asset directory trusted against concurrent filesystem mutation.
pub(crate) fn contained_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let canonical = |path: &Path| {
        path.canonicalize().map_err(|source| ModelError::Io {
            path: path.to_path_buf(),
            source,
        })
    };
    let root = canonical(root)?;
    let path = canonical(path)?;
    if !path.starts_with(&root) {
        return Err(ModelError::Validation(
            "resource path escapes its configured root".into(),
        ));
    }
    Ok(path)
}

fn load_local_urdf(path: &str, root: Option<&Path>) -> Result<UrdfModel> {
    let root = root.ok_or_else(|| ModelError::Validation("profile filesystem URDF requires with_asset_root; use UrdfModel::from_file for a trusted direct path".into()))?;
    UrdfModel::from_file(contained_path(root, &root.join(path))?)
}
