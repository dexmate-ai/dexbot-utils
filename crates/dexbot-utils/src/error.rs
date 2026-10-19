use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("unknown robot profile: {0}")]
    UnknownProfile(String),
    #[error(
        "robot name {name:?} does not select a built-in profile: {reason}; \
         pass a profile name explicitly if this robot is supported"
    )]
    UnknownRobotName { name: String, reason: String },
    #[error("unknown profile fragment {fragment:?} extended by {profile:?}")]
    UnknownFragment { profile: String, fragment: String },
    #[error("profile {profile:?} cannot extend nested fragment {fragment:?}")]
    NestedExtends { profile: String, fragment: String },
    #[error("unsupported schema_version {found}; this build supports schema_version {expected}")]
    UnsupportedSchemaVersion { found: String, expected: u32 },
    #[error("profile parse failed in {source_name}: {message}")]
    Parse {
        source_name: String,
        message: String,
    },
    #[error("profile validation failed: {0}")]
    Validation(String),
    #[error(
        "discovery layer {incoming:?} contradicts {existing:?} at {path:?}; \
         discovery adds facts and never silently overrides explicit user configuration"
    )]
    DiscoveryConflict {
        path: String,
        existing: String,
        incoming: String,
    },
    #[error("Sensor {sensor:?} is not configured for robot model {model:?} (profile {profile:?}). Configured sensors: {available}. Select a configured sensor, or use a robot configuration that declares the requested sensor.")]
    UnknownSensor {
        sensor: String,
        model: String,
        profile: String,
        available: String,
    },
    #[error("overlay deletion directive cannot replace the document root (path {0:?})")]
    InvalidDeletion(String),
    #[error(
        "malformed overlay deletion at {0}: the only deletion directive is {{\"$delete\": true}}"
    )]
    MalformedDeletion(String),
    #[error("I/O failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("configuration serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("URDF parse failed: {0}")]
    Urdf(String),
    #[error("resource URI is unsupported or unresolved: {0}")]
    UnresolvedResource(String),
}

pub type Result<T> = std::result::Result<T, ModelError>;
