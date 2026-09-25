//! Schema-version migration framework.
//!
//! Schema version 1 is the only version that exists, so the step table is
//! empty and a v1 -> v1 migration is the identity. When schema version 2
//! arrives, its transformation slots into `MIGRATION_STEPS` without changing
//! the public entry point.

use crate::{ModelError, Result, SOURCE_SCHEMA_VERSION};
use serde_yaml::Value;

/// One single-step schema transformation, applied to the raw profile
/// document before any validation.
struct MigrationStep {
    from: u32,
    to: u32,
    apply: fn(&mut Value) -> Result<()>,
}

const MIGRATION_STEPS: &[MigrationStep] = &[];

/// Migrates a raw profile document between schema versions and returns the
/// migrated YAML. A same-version migration returns the source unchanged.
pub fn migrate_profile(source: &str, target_version: u32) -> Result<String> {
    run_migrations(source, target_version, MIGRATION_STEPS)
}

/// The engine behind [`migrate_profile`], with the step table as a
/// parameter.
///
/// Separated so the machinery can be exercised before the first real step
/// exists: with an empty `MIGRATION_STEPS` the stepping loop is unreachable
/// through the public entry point, which would mean the first time it ever
/// ran was in production, on the schema-v2 rollout.
fn run_migrations(source: &str, target_version: u32, steps: &[MigrationStep]) -> Result<String> {
    let mut document: Value = serde_yaml::from_str(source).map_err(|error| ModelError::Parse {
        source_name: "migration input".into(),
        message: error.to_string(),
    })?;
    let current = schema_version_of(&document)?;
    if current == target_version {
        return Ok(source.to_string());
    }
    let mut version = current;
    while version != target_version {
        let Some(step) = steps
            .iter()
            .find(|step| step.from == version && step.to <= target_version)
        else {
            return Err(ModelError::Validation(format!(
                "no migration path from schema_version {current} to {target_version}"
            )));
        };
        // A step that does not advance the version would loop forever; reject
        // the step table rather than hang whoever loads the profile.
        if step.to <= step.from {
            return Err(ModelError::Validation(format!(
                "invalid migration step {} -> {}: steps must increase the version",
                step.from, step.to
            )));
        }
        (step.apply)(&mut document)?;
        set_schema_version(&mut document, step.to)?;
        version = step.to;
    }
    serde_yaml::to_string(&document).map_err(|error| ModelError::Parse {
        source_name: "migration output".into(),
        message: error.to_string(),
    })
}

fn schema_version_of(document: &Value) -> Result<u32> {
    let Some(mapping) = document.as_mapping() else {
        return Err(ModelError::Parse {
            source_name: "migration input".into(),
            message: "document root must be an object".into(),
        });
    };
    match mapping.get(Value::from("schema_version")) {
        None => Err(ModelError::Validation("schema_version is required".into())),
        Some(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| ModelError::UnsupportedSchemaVersion {
                found: serde_yaml::to_string(value)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                expected: SOURCE_SCHEMA_VERSION,
            }),
    }
}

fn set_schema_version(document: &mut Value, version: u32) -> Result<()> {
    let Some(mapping) = document.as_mapping_mut() else {
        return Err(ModelError::Parse {
            source_name: "migration input".into(),
            message: "document root must be an object".into(),
        });
    };
    mapping.insert(Value::from("schema_version"), Value::from(version));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_version_migration_is_byte_identity() {
        let source = "schema_version: 1\nrobot:\n  model: test\n# comment survives\n";
        assert_eq!(migrate_profile(source, 1).unwrap(), source);
    }

    #[test]
    fn unreachable_target_version_is_an_error() {
        let error = migrate_profile("schema_version: 1\n", 2).unwrap_err();
        assert!(matches!(error, ModelError::Validation(_)));
        assert!(error.to_string().contains("no migration path"));
    }

    fn bump(document: &mut Value) -> Result<()> {
        let mapping = document.as_mapping_mut().expect("mapping in test");
        mapping.insert(Value::from("migrated"), Value::from(true));
        Ok(())
    }

    #[test]
    fn steps_chain_until_the_target_and_rewrite_the_version() {
        // The engine that will run the first real schema migration: two
        // steps chain 1 -> 2 -> 3, each transformation is applied, and the
        // final document carries the target version.
        let steps = [
            MigrationStep {
                from: 1,
                to: 2,
                apply: bump,
            },
            MigrationStep {
                from: 2,
                to: 3,
                apply: bump,
            },
        ];
        let output = run_migrations("schema_version: 1\nrobot: {}\n", 3, &steps).unwrap();
        let value: Value = serde_yaml::from_str(&output).unwrap();
        assert_eq!(value["schema_version"], Value::from(3));
        assert_eq!(value["migrated"], Value::from(true));
    }

    #[test]
    fn a_step_beyond_the_target_is_not_taken() {
        // Migrating to 2 must stop at 2 even when a 2 -> 3 step exists.
        let steps = [
            MigrationStep {
                from: 1,
                to: 2,
                apply: bump,
            },
            MigrationStep {
                from: 2,
                to: 3,
                apply: bump,
            },
        ];
        let output = run_migrations("schema_version: 1\n", 2, &steps).unwrap();
        let value: Value = serde_yaml::from_str(&output).unwrap();
        assert_eq!(value["schema_version"], Value::from(2));
    }

    #[test]
    fn a_failing_step_propagates_its_error() {
        fn explode(_: &mut Value) -> Result<()> {
            Err(ModelError::Validation("step failed".into()))
        }
        let steps = [MigrationStep {
            from: 1,
            to: 2,
            apply: explode,
        }];
        let error = run_migrations("schema_version: 1\n", 2, &steps).unwrap_err();
        assert!(error.to_string().contains("step failed"));
    }

    #[test]
    fn a_non_advancing_step_is_rejected_instead_of_looping_forever() {
        // A step table with from == to would otherwise spin the engine
        // indefinitely inside whoever loads the profile.
        let steps = [MigrationStep {
            from: 1,
            to: 1,
            apply: bump,
        }];
        let error = run_migrations("schema_version: 1\n", 2, &steps).unwrap_err();
        assert!(error.to_string().contains("must increase"), "{error}");
    }

    #[test]
    fn missing_or_malformed_schema_version_is_rejected() {
        assert!(matches!(
            migrate_profile("robot: {model: x}\n", 1),
            Err(ModelError::Validation(_))
        ));
        assert!(matches!(
            migrate_profile("schema_version: nope\n", 1),
            Err(ModelError::UnsupportedSchemaVersion { .. })
        ));
        assert!(matches!(
            migrate_profile("- not-an-object\n", 1),
            Err(ModelError::Parse { .. })
        ));
    }
}
