use crate::catalog;
use crate::merge::merge_layer;
use crate::types::{
    DiscoveryFacts, ProfileDocument, ResolutionStage, ResolvedRobotConfig, RobotOverlay,
    CAPABILITY_VOCABULARY_VERSION, RESOLVED_CONFIG_VERSION, SOURCE_SCHEMA_VERSION,
};
use crate::{ModelError, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RobotConfig {
    profile_name: String,
    layers: Vec<(String, Value)>,
    asset_root: Option<PathBuf>,
}

/// Where `extends` fragments are loaded from. Built-in profiles use the
/// embedded catalog; file-based profiles resolve relative paths against
/// their own directory first and fall back to the embedded catalog.
///
/// A file next to the profile can therefore shadow an embedded fragment of
/// the same name (`common/vega_upper_body.yaml`). Provenance keeps the two
/// apart: embedded fragments are recorded as `extends:<name>` and on-disk
/// ones as `extends:file:<name>`.
enum FragmentSource {
    Embedded,
    Directory(PathBuf),
}

impl FragmentSource {
    /// Returns the provenance layer name and the fragment source.
    fn load(&self, profile: &str, fragment: &str) -> Result<(String, String)> {
        let embedded = || {
            catalog::fragment_source(fragment)
                .map(|source| (format!("extends:{fragment}"), source.to_string()))
                .ok_or_else(|| ModelError::UnknownFragment {
                    profile: profile.into(),
                    fragment: fragment.into(),
                })
        };
        match self {
            Self::Embedded => embedded(),
            Self::Directory(directory) => {
                crate::urdf::check_relative_path(fragment)?;
                let path = directory.join(fragment);
                if path.is_file() {
                    let path = crate::urdf::contained_path(directory, &path)?;
                    std::fs::read_to_string(&path)
                        .map(|source| (format!("extends:file:{fragment}"), source))
                        .map_err(|source| ModelError::Io { path, source })
                } else {
                    embedded()
                }
            }
        }
    }
}

impl RobotConfig {
    pub fn from_profile(name: &str) -> Result<Self> {
        let source =
            catalog::source(name).ok_or_else(|| ModelError::UnknownProfile(name.into()))?;
        Self::from_source(name, source, &FragmentSource::Embedded)
    }

    pub fn from_yaml(name: impl Into<String>, source: &str) -> Result<Self> {
        let name = name.into();
        Self::from_source(&name, source, &FragmentSource::Embedded)
    }

    /// [`Self::from_yaml`] for a document that belongs to `directory`:
    /// `extends` fragments resolve relative to it first, as they do for
    /// [`Self::from_file`]. For tools that transform a profile file in
    /// memory and must check the result where the file lives.
    pub fn from_yaml_in(
        name: impl Into<String>,
        source: &str,
        directory: impl Into<PathBuf>,
    ) -> Result<Self> {
        let name = name.into();
        Self::from_source(&name, source, &FragmentSource::Directory(directory.into()))
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|source| ModelError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("external");
        let fragments = match path.parent() {
            Some(parent) => FragmentSource::Directory(parent.to_path_buf()),
            None => FragmentSource::Embedded,
        };
        Self::from_source(name, &source, &fragments)
    }

    fn from_source(name: &str, source: &str, fragments: &FragmentSource) -> Result<Self> {
        let mut profile = parse_yaml_value(name, source)?;
        let extends = take_extends(&mut profile, name)?;
        let mut layers = Vec::new();
        for fragment in extends {
            let (layer, source) = fragments.load(name, &fragment)?;
            let mut value = parse_yaml_value(&fragment, &source)?;
            let nested = take_extends(&mut value, &fragment)?;
            if !nested.is_empty() {
                return Err(ModelError::NestedExtends {
                    profile: name.into(),
                    fragment,
                });
            }
            layers.push((layer, value));
        }
        layers.push((format!("profile:{name}"), profile));
        Ok(Self {
            profile_name: name.into(),
            layers,
            asset_root: None,
        })
    }

    pub fn with_overlay_yaml(mut self, name: impl Into<String>, source: &str) -> Result<Self> {
        let name = name.into();
        let value = parse_yaml_value(&name, source)?;
        if !value.is_object() {
            return Err(ModelError::Parse {
                source_name: name,
                message: "document root must be an object".into(),
            });
        }
        // Even an empty list: the key has no meaning outside a base profile.
        if value.get("extends").is_some() {
            return Err(ModelError::Validation(
                "overlays cannot contain extends".into(),
            ));
        }
        self.layers.push((format!("overlay:{name}"), value));
        Ok(self)
    }

    pub fn with_overlay_file(self, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|source| ModelError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        // The layer is named by file basename, never by full path: absolute
        // paths would make provenance — and thus the normalized export —
        // machine-specific (§14.0 portability).
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("overlay");
        self.with_overlay_yaml(name, &source)
    }

    pub fn with_sensor_enabled(mut self, sensor: &str) -> Self {
        self.layers.push((
            format!("api:enable_sensor:{sensor}"),
            json!({"sensors": {sensor: {"enabled": true}}}),
        ));
        self
    }

    /// Sets the trusted root for filesystem URDFs and overrides `package://` resolution during
    /// static resolution. The `DEXBOT_ASSET_ROOT` environment variable and
    /// the embedded assets are used when no root is set.
    pub fn with_asset_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.asset_root = Some(root.into());
        self
    }

    pub fn resolve(self) -> Result<ResolvedRobotConfig> {
        resolve_onto(
            self.profile_name,
            json!({}),
            self.layers,
            ResolutionStage::Static,
            BTreeMap::new(),
            self.asset_root,
        )
    }
}

pub fn apply_discovery(
    static_config: &ResolvedRobotConfig,
    facts: DiscoveryFacts,
) -> Result<ResolvedRobotConfig> {
    apply_discovery_with(static_config, facts, |_| Ok((RobotOverlay(json!({})), ())))
        .map(|(config, ())| config)
}

/// Derive model-specific facts from effective components before joint validation.
/// Serialize and merge the source configuration once; derived facts pass through
/// the same provenance/conflict checks as the original discovery overlay.
pub(crate) fn apply_discovery_with<T>(
    config: &ResolvedRobotConfig,
    facts: DiscoveryFacts,
    derive: impl FnOnce(&Value) -> Result<(RobotOverlay, T)>,
) -> Result<(ResolvedRobotConfig, T)> {
    if config.resolution_stage != ResolutionStage::Static {
        return Err(ModelError::Validation(
            "discovery overlay requires a static resolved configuration".into(),
        ));
    }
    if facts.overlay.0.get("extends").is_some() {
        return Err(ModelError::Validation(
            "discovery cannot contain extends; only the base profile composes fragments".into(),
        ));
    }
    let layer = format!("discovery:{}", facts.source);
    let mut base = source_document(config)?;
    let mut provenance = config.provenance.clone();
    merge_layer(&mut base, facts.overlay.0, &layer, &mut provenance)?;
    let (derived, outcome) = derive(&base)?;
    let resolved = resolve_onto(
        config.profile_name.clone(),
        base,
        vec![(layer, derived.0)],
        ResolutionStage::Operational,
        provenance,
        config.asset_root.clone(),
    )?;
    Ok((resolved, outcome))
}

/// Applies a user overlay to an already resolved static configuration, with
/// the semantics of [`RobotConfig::with_overlay_yaml`]: the layer is recorded
/// as `overlay:<name>`, so it overrides earlier overlays and a later
/// [`apply_discovery`] cannot silently overwrite it. The result stays
/// static. Overlays express user intent and therefore precede discovery; an
/// operational configuration is rejected.
pub fn apply_overlay(
    config: &ResolvedRobotConfig,
    name: &str,
    overlay: RobotOverlay,
) -> Result<ResolvedRobotConfig> {
    if config.resolution_stage != ResolutionStage::Static {
        return Err(ModelError::Validation(
            "user overlays require a static resolved configuration; apply them before discovery"
                .into(),
        ));
    }
    apply_layer(
        config,
        format!("overlay:{name}"),
        overlay,
        ResolutionStage::Static,
    )
}

fn apply_layer(
    config: &ResolvedRobotConfig,
    layer: String,
    overlay: RobotOverlay,
    stage: ResolutionStage,
) -> Result<ResolvedRobotConfig> {
    let base = source_document(config)?;
    // Re-resolution must validate against the same URDF the static stage
    // used, so the in-memory asset root is carried through. A static config
    // deserialized from JSON has no asset root and falls back to the
    // `DEXBOT_ASSET_ROOT` environment variable and the embedded assets.
    resolve_onto(
        config.profile_name.clone(),
        base,
        vec![(layer, overlay.0)],
        stage,
        config.provenance.clone(),
        config.asset_root.clone(),
    )
}

fn source_document(config: &ResolvedRobotConfig) -> Result<Value> {
    Ok(serde_json::to_value(ProfileDocument {
        schema_version: config.schema_version,
        extends: Vec::new(),
        robot: config.robot.clone(),
        runtime: config.runtime.clone(),
        components: config.components.clone(),
        sensors: config.sensors.clone(),
        joint_groups: config.joint_groups.clone(),
        safety: config.safety.clone(),
        querables: config.querables.clone(),
        intent_profiles: config.intent_profiles.clone(),
    })?)
}

pub fn available_profiles() -> &'static [&'static str] {
    catalog::PROFILE_NAMES
}

/// Maps a `ROBOT_NAME`-style robot identifier to the built-in profile that
/// serves it, failing closed on anything it does not recognise. This is the
/// single owner of the name→profile rule set that historically lived in
/// downstream code (dexcontrol's `profile_from_environment` binding, the
/// deprecated `dexcontrol.utils.compat.resolve_robot_model`, and the removed
/// `dexbot_utils` Python package's `RobotInfo._derive_variant_from_robot_name`
/// with its `ROBOT_NAME_PATTERN`/`VERSION_SUFFIX_MAPPING` constants).
/// Surrounding whitespace is ignored and matching is case-insensitive:
///
/// 1. A string that names a built-in profile selects it, so hand-specific
///    profiles (`*_f5d6`, `*_gripper`) stay reachable. Robot names never
///    encode the hand type; those profiles are selected explicitly, never
///    derived.
/// 2. Identity-v2 names (`dm/vg1p-<serial>` or `dm-vg1p-<serial>`) select
///    their variant from the `vg1`, `vg1u`, or `vg1p` model token. A
///    trailing legacy version segment, when present, must agree with it.
/// 3. Serial-number names follow the legacy format
///    `dm/<abbr><serial>-<version><suffix>` (e.g. `dm/vg0123456789-1u`):
///    the `vg` abbreviation plus a ten-character serial, then the version
///    `1` (`vega_1`), `1u` (upper body, `vega_1u`) or `1p` (pro, `vega_1p`).
/// 4. Anything else — a typo, another separator, a model token or version
///    this build does not know — is [`ModelError::UnknownRobotName`]. The
///    wrong profile moves real hardware with the wrong limits, so an
///    unrecognised robot is never guessed.
pub fn try_profile_for_robot_name(name: &str) -> Result<&'static str> {
    let unknown = |reason: &str| ModelError::UnknownRobotName {
        name: name.into(),
        reason: reason.into(),
    };
    let lowered = name.trim().to_ascii_lowercase();
    if lowered.is_empty() {
        return Err(unknown("the name is empty"));
    }
    if let Some(profile) = catalog::PROFILE_NAMES
        .iter()
        .find(|profile| **profile == lowered)
    {
        return Ok(profile);
    }
    let Some(identity) = lowered
        .strip_prefix("dm/")
        .or_else(|| lowered.strip_prefix("dm-"))
    else {
        return Err(unknown(
            "it is neither a built-in profile name nor a dm/<model>-<serial> identity",
        ));
    };
    let Some((token, rest)) = identity.split_once('-') else {
        return Err(unknown("the identity has no serial or version segment"));
    };
    let alphanumeric =
        |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_alphanumeric());
    let version_profile = |version: &str| match version {
        "1" => Some("vega_1"),
        "1u" => Some("vega_1u"),
        "1p" => Some("vega_1p"),
        _ => None,
    };
    let token_profile = match token {
        "vg1" => Some("vega_1"),
        "vg1u" => Some("vega_1u"),
        "vg1p" => Some("vega_1p"),
        _ => None,
    };
    if let Some(profile) = token_profile {
        // Identity v2 is `<token>-<serial>[-<check>]`: the token alone selects
        // the variant, the serial may itself be dashed, and deployed names
        // end in a one-character check segment (`dm/vg1u-7k3m...-c`). Every
        // segment must still be plain alphanumeric text.
        if !rest.split('-').all(alphanumeric) {
            return Err(unknown(
                "a serial or check segment is empty or not alphanumeric",
            ));
        }
        // A trailing `1u`/`1p` cannot be a check character; it is the legacy
        // version suffix, and it must not contradict the token.
        let last = rest.rsplit('-').next().unwrap_or_default();
        if rest.contains('-')
            && matches!(last, "1u" | "1p")
            && version_profile(last) != Some(profile)
        {
            return Err(unknown(
                "the model token and the version suffix select different variants",
            ));
        }
        return Ok(profile);
    }
    // Legacy `ROBOT_NAME_PATTERN`: two-letter abbreviation plus ten-character
    // serial. Only the Vega abbreviation has built-in profiles.
    if token.len() != 12 || !alphanumeric(token) || !token.starts_with("vg") {
        return Err(unknown("the model token is not known to this build"));
    }
    version_profile(rest).ok_or_else(|| unknown("the version suffix is not known to this build"))
}

/// Legacy, total form of [`try_profile_for_robot_name`]: every name the
/// fallible form accepts maps identically, and anything it rejects falls
/// back to the historical guess — the case-insensitive `-1u`/`-1p` suffix,
/// then `"vega_1"`. It therefore never fails and also never reports a typo
/// or an unsupported robot. New code should call
/// [`try_profile_for_robot_name`] and surface the error; this function is
/// kept only for source compatibility.
pub fn profile_for_robot_name(name: &str) -> &'static str {
    try_profile_for_robot_name(name).unwrap_or_else(|_| {
        let lowered = name.to_ascii_lowercase();
        let token = lowered
            .strip_prefix("dm/")
            .or_else(|| lowered.strip_prefix("dm-"))
            .and_then(|value| value.split('-').next());
        match token {
            Some("vg1") => "vega_1",
            Some("vg1u") => "vega_1u",
            Some("vg1p") => "vega_1p",
            _ if lowered.ends_with("-1u") => "vega_1u",
            _ if lowered.ends_with("-1p") => "vega_1p",
            _ => "vega_1",
        }
    })
}

/// Rejects unsupported schema versions from the raw merged document, before
/// strict deserialization can misreport them as unknown-field errors.
fn check_schema_version(merged: &Value) -> Result<()> {
    match merged.get("schema_version") {
        None => Err(ModelError::Validation("schema_version is required".into())),
        Some(value) if value.as_u64() == Some(u64::from(SOURCE_SCHEMA_VERSION)) => Ok(()),
        Some(other) => Err(ModelError::UnsupportedSchemaVersion {
            found: other.to_string(),
            expected: SOURCE_SCHEMA_VERSION,
        }),
    }
}

/// Merges `layers` onto an existing `merged` document, extending the carried
/// `provenance` in place — the base document's own attribution is preserved
/// rather than re-recorded under a synthetic layer name.
fn resolve_onto(
    profile_name: String,
    mut merged: Value,
    layers: Vec<(String, Value)>,
    stage: ResolutionStage,
    mut provenance: BTreeMap<String, String>,
    asset_root: Option<PathBuf>,
) -> Result<ResolvedRobotConfig> {
    for (name, value) in layers {
        // Base layers had `extends` taken out while their fragments were
        // loaded. Anywhere else it would merge into the document and then be
        // ignored, leaving the author believing a fragment was pulled in.
        if value.get("extends").is_some() {
            return Err(ModelError::Validation(format!(
                "layer {name:?} cannot contain extends; only the base profile composes fragments"
            )));
        }
        if let Some(sensor) = name.strip_prefix("api:enable_sensor:") {
            let declared = merged
                .get("sensors")
                .and_then(Value::as_object)
                .is_some_and(|sensors| sensors.contains_key(sensor));
            if !declared {
                let mut available = merged
                    .get("sensors")
                    .and_then(Value::as_object)
                    .map(|sensors| sensors.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                available.sort();
                return Err(ModelError::UnknownSensor {
                    sensor: sensor.into(),
                    model: merged
                        .get("robot")
                        .and_then(|robot| robot.get("model"))
                        .and_then(Value::as_str)
                        .unwrap_or("unspecified")
                        .to_owned(),
                    profile: profile_name.clone(),
                    available: if available.is_empty() {
                        "none".into()
                    } else {
                        available.join(", ")
                    },
                });
            }
        }
        merge_layer(&mut merged, value, &name, &mut provenance)?;
    }
    check_schema_version(&merged)?;
    let mut document: ProfileDocument =
        serde_json::from_value(merged).map_err(|error| ModelError::Parse {
            source_name: profile_name.clone(),
            message: error.to_string(),
        })?;
    crate::validate::validate(&document)?;
    let joint_metadata = crate::joints::resolve_joints(&mut document, asset_root.as_deref())?;
    crate::validate::validate_resolved(&document, &joint_metadata)?;
    let mut resolved = ResolvedRobotConfig {
        resolved_config_version: RESOLVED_CONFIG_VERSION,
        capability_vocabulary_version: CAPABILITY_VOCABULARY_VERSION,
        resolution_stage: stage,
        profile_name,
        schema_version: document.schema_version,
        content_hash: String::new(),
        robot: document.robot,
        runtime: document.runtime,
        components: document.components,
        sensors: document.sensors,
        joint_groups: document.joint_groups,
        joint_metadata,
        safety: document.safety,
        querables: document.querables,
        intent_profiles: document.intent_profiles,
        provenance,
        asset_root,
    };
    resolved.content_hash = resolved.compute_content_hash()?;
    Ok(resolved)
}

/// Parses YAML into the JSON value model the merge works on. YAML merge
/// keys (`<<: *anchor`) are applied first: the strict typed sections reject
/// a literal `<<` key, but the free-form maps (`metadata`, `querables`,
/// `intent_profiles`) would keep it as data and drop the merged entries.
pub(crate) fn parse_yaml_value(name: &str, source: &str) -> Result<Value> {
    let parse_error = |error: serde_yaml::Error| ModelError::Parse {
        source_name: name.into(),
        message: error.to_string(),
    };
    let mut value: serde_yaml::Value = serde_yaml::from_str(source).map_err(parse_error)?;
    value.apply_merge().map_err(parse_error)?;
    serde_json::to_value(value).map_err(ModelError::from)
}

fn take_extends(value: &mut Value, name: &str) -> Result<Vec<String>> {
    let Some(object) = value.as_object_mut() else {
        return Err(ModelError::Parse {
            source_name: name.into(),
            message: "document root must be an object".into(),
        });
    };
    let Some(extends) = object.remove("extends") else {
        return Ok(Vec::new());
    };
    serde_json::from_value(extends).map_err(|error| ModelError::Parse {
        source_name: name.into(),
        message: format!("invalid extends: {error}"),
    })
}
