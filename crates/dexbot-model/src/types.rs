use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const SOURCE_SCHEMA_VERSION: u32 = 1;
pub const RESOLVED_CONFIG_VERSION: u32 = 0;
pub const CAPABILITY_VOCABULARY_VERSION: u32 = 1;

fn default_true() -> bool {
    true
}
fn default_profile_version() -> String {
    "0.1.0".into()
}
fn default_asset_version() -> String {
    "legacy-dexmate-urdf".into()
}
fn default_readiness() -> String {
    "required_components".into()
}
fn default_subscription_policy() -> String {
    "auto".into()
}
fn default_idle_timeout() -> u64 {
    5_000
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RobotIdentity {
    pub model: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub urdf: Option<String>,
    #[serde(default = "default_profile_version")]
    pub profile_version: String,
    #[serde(default = "default_asset_version")]
    pub asset_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSettings {
    #[serde(default = "default_readiness")]
    pub readiness: String,
    #[serde(default = "default_subscription_policy")]
    pub default_subscription_policy: String,
    #[serde(default = "default_idle_timeout")]
    pub state_idle_timeout_ms: u64,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            readiness: default_readiness(),
            default_subscription_policy: default_subscription_policy(),
            state_idle_timeout_ms: default_idle_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilitySpec {
    pub id: String,
    pub required: bool,
}

impl<'de> Deserialize<'de> for CapabilitySpec {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            Short(String),
            Full(BTreeMap<String, Value>),
        }
        match Input::deserialize(deserializer)? {
            Input::Short(id) => {
                if id.is_empty() {
                    return Err(de::Error::custom("capability id cannot be empty"));
                }
                Ok(Self { id, required: true })
            }
            Input::Full(mut entry) => {
                let id = match entry.remove("id") {
                    Some(Value::String(id)) if !id.is_empty() => id,
                    Some(_) => {
                        return Err(de::Error::custom(
                            "capability id must be a non-empty string",
                        ))
                    }
                    None => return Err(de::Error::custom("expanded capability entry needs an id")),
                };
                let required = match entry.remove("required") {
                    Some(Value::Bool(required)) => required,
                    Some(_) => {
                        return Err(de::Error::custom("capability required must be a boolean"))
                    }
                    None => true,
                };
                if let Some(unknown) = entry.keys().next() {
                    return Err(de::Error::custom(format!(
                        "expanded capability entry has unknown field {unknown:?}"
                    )));
                }
                Ok(Self { id, required })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EndpointConfig {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JointConfig {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
}

/// URDF-derived metadata for one joint of a resolved runtime object.
///
/// Angular values are radians, translations are meters, efforts are N·m or N,
/// and velocities are rad/s or m/s, following URDF conventions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedJoint {
    pub name: String,
    pub joint_type: String,
    #[serde(default)]
    pub lower: Option<f64>,
    #[serde(default)]
    pub upper: Option<f64>,
    #[serde(default)]
    pub effort: Option<f64>,
    #[serde(default)]
    pub velocity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentConfig {
    pub driver: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub local: bool,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<CapabilitySpec>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub joints: Option<JointConfig>,
    #[serde(default)]
    pub endpoints: BTreeMap<String, EndpointConfig>,
    #[serde(default)]
    pub safety: BTreeMap<String, Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProfileDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub extends: Vec<String>,
    pub robot: RobotIdentity,
    #[serde(default)]
    pub runtime: RuntimeSettings,
    #[serde(default)]
    pub components: BTreeMap<String, ComponentConfig>,
    #[serde(default)]
    pub sensors: BTreeMap<String, ComponentConfig>,
    #[serde(default)]
    pub joint_groups: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub safety: BTreeMap<String, Value>,
    #[serde(default)]
    pub querables: BTreeMap<String, String>,
    #[serde(default)]
    pub intent_profiles: BTreeMap<String, (String, String)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStage {
    Static,
    Operational,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedRobotConfig {
    pub resolved_config_version: u32,
    pub capability_vocabulary_version: u32,
    pub resolution_stage: ResolutionStage,
    pub profile_name: String,
    pub schema_version: u32,
    pub content_hash: String,
    pub robot: RobotIdentity,
    pub runtime: RuntimeSettings,
    pub components: BTreeMap<String, ComponentConfig>,
    pub sensors: BTreeMap<String, ComponentConfig>,
    /// Named joint-group tables consumed by `joints.source: urdf` components.
    #[serde(default)]
    pub joint_groups: BTreeMap<String, Vec<String>>,
    /// URDF-derived joint metadata per runtime object, in command order.
    #[serde(default)]
    pub joint_metadata: BTreeMap<String, Vec<ResolvedJoint>>,
    pub safety: BTreeMap<String, Value>,
    pub querables: BTreeMap<String, String>,
    pub intent_profiles: BTreeMap<String, (String, String)>,
    pub provenance: BTreeMap<String, String>,
    /// URDF asset root this configuration was resolved against, carried in
    /// memory only so operational re-resolution (`apply_discovery`) validates
    /// against the same URDF. Never serialized: machine-specific paths must
    /// not enter normalized output or the content hash, so a configuration
    /// deserialized from JSON falls back to the `DEXBOT_ASSET_ROOT`
    /// environment variable and then the embedded assets.
    #[serde(skip)]
    pub asset_root: Option<PathBuf>,
}

impl ResolvedRobotConfig {
    pub fn component(&self, name: &str) -> Option<&ComponentConfig> {
        self.components.get(name)
    }
    pub fn sensor(&self, name: &str) -> Option<&ComponentConfig> {
        self.sensors.get(name)
    }
    /// URDF-derived joint metadata for one runtime object, in command order.
    pub fn joint_metadata(&self, name: &str) -> Option<&[ResolvedJoint]> {
        self.joint_metadata.get(name).map(Vec::as_slice)
    }
    /// Ordered joint names for one runtime object, preferring URDF-resolved
    /// metadata and falling back to the declared joint configuration.
    pub fn joint_names(&self, name: &str) -> Option<Vec<&str>> {
        if let Some(joints) = self.joint_metadata.get(name) {
            return Some(joints.iter().map(|joint| joint.name.as_str()).collect());
        }
        let object = self
            .components
            .get(name)
            .or_else(|| self.sensors.get(name))?;
        let joints = object.joints.as_ref()?;
        Some(joints.names.iter().map(String::as_str).collect())
    }
    pub fn normalized_json(&self) -> crate::Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }
    /// Recomputes the content hash: SHA-256 over the compact JSON form of
    /// this configuration without `content_hash` itself, the diagnostic
    /// `provenance` map, and `profile_name`. The name is a label -- a file
    /// stem for file-based profiles -- so a byte-identical profile copied to
    /// another filename, or resolved under another layer spelling or on
    /// another machine, hashes identically. Everything else, including
    /// `resolution_stage` and the URDF-derived `joint_metadata`, is hashed.
    pub fn compute_content_hash(&self) -> crate::Result<String> {
        let mut value = serde_json::to_value(self)?;
        if let Some(map) = value.as_object_mut() {
            map.remove("content_hash");
            map.remove("provenance");
            map.remove("profile_name");
        }
        let digest = Sha256::digest(serde_json::to_vec(&value)?);
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }
    /// True when the stored `content_hash` matches the recomputed hash of
    /// the semantic payload.
    pub fn verify_content_hash(&self) -> crate::Result<bool> {
        Ok(self.compute_content_hash()? == self.content_hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct RobotOverlay(pub Value);

impl RobotOverlay {
    pub fn from_yaml(yaml: &str) -> crate::Result<Self> {
        crate::model::parse_yaml_value("overlay", yaml).map(Self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryFacts {
    pub source: String,
    pub overlay: RobotOverlay,
}

#[derive(Debug, Clone)]
pub struct RobotInfo {
    config: ResolvedRobotConfig,
}

impl RobotInfo {
    pub fn new(config: ResolvedRobotConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &ResolvedRobotConfig {
        &self.config
    }
    pub fn robot_model(&self) -> &str {
        &self.config.robot.model
    }
    pub fn component_names(&self) -> impl Iterator<Item = &str> {
        self.config.components.keys().map(String::as_str)
    }
    pub fn sensor_names(&self) -> impl Iterator<Item = &str> {
        self.config.sensors.keys().map(String::as_str)
    }
    pub fn has_component(&self, name: &str) -> bool {
        self.config.components.contains_key(name)
    }
    pub fn has_sensor(&self, name: &str) -> bool {
        self.config.sensors.contains_key(name)
    }
    pub fn joint_names(&self, name: &str) -> Option<Vec<&str>> {
        self.config.joint_names(name)
    }
    pub fn joint_metadata(&self, name: &str) -> Option<&[ResolvedJoint]> {
        self.config.joint_metadata(name)
    }
}
