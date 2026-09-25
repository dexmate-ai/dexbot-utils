//! URDF joint-limit validation through the full public resolve path.
//!
//! These are the checks standing between a hand-edited URDF and a robot
//! commanded to move somewhere impossible: inconsistent bounds, negative
//! rate limits, and joint types the runtime cannot command. Each is driven
//! through `with_asset_root`, the same override a deployment uses, with the
//! real vega_1u URDF doctored one defect at a time -- so the test proves the
//! defect is caught in a realistic document, not a synthetic two-line one.

use dexbot_model::{ModelError, RobotConfig};
use std::path::PathBuf;

const RELATIVE: &str = "robots/humanoid/vega_1u/vega_1u.urdf";

/// The vega_1u URDF with only the `L_arm_j1` element rewritten, so a defect
/// lands on a joint a component commands rather than an unused one.
fn doctored_arm_joint(doctor: impl Fn(&str) -> String) -> String {
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/urdf")
            .join(RELATIVE),
    )
    .unwrap();
    let start = source.find("<joint name=\"L_arm_j1\"").unwrap();
    let end = start + source[start..].find("</joint>").unwrap() + "</joint>".len();
    let doctored = format!(
        "{}{}{}",
        &source[..start],
        doctor(&source[start..end]),
        &source[end..]
    );
    assert_ne!(
        doctored, source,
        "the doctoring must have changed something"
    );
    doctored
}

fn resolve_with_doctored_urdf(tag: &str, doctor: impl Fn(&str) -> String) -> ModelError {
    let doctored = doctored_arm_joint(doctor);

    let temporary = tempfile::Builder::new().prefix(tag).tempdir().unwrap();
    let root = temporary.path();
    let path = root.join(RELATIVE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, doctored).unwrap();

    RobotConfig::from_profile("vega_1u")
        .unwrap()
        .with_asset_root(root)
        .resolve()
        .unwrap_err()
}

#[test]
fn inverted_limits_name_the_joint_and_both_bounds() {
    let error = resolve_with_doctored_urdf("inverted", |source| {
        // Push one negative lower bound far above any upper bound.
        source.replacen("lower=\"-", "lower=\"99", 1)
    });
    match error {
        ModelError::Validation(message) => {
            assert!(message.contains("inconsistent limits"), "{message}");
            assert!(message.contains("exceeds"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn negative_velocity_limits_are_rejected() {
    let error = resolve_with_doctored_urdf("velocity", |source| {
        source.replacen("velocity=\"", "velocity=\"-", 1)
    });
    match error {
        ModelError::Validation(message) => {
            assert!(message.contains("negative velocity limit"), "{message}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn negative_effort_limits_are_rejected() {
    let error = resolve_with_doctored_urdf("effort", |source| {
        source.replacen("effort=\"", "effort=\"-", 1)
    });
    match error {
        ModelError::Validation(message) => {
            assert!(message.contains("negative effort limit"), "{message}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn non_scalar_joint_types_are_rejected() {
    // A planar joint has no single scalar position; commanding it through a
    // one-value-per-joint interface would be silently wrong.
    let error = resolve_with_doctored_urdf("planar", |source| {
        source.replacen("type=\"revolute\"", "type=\"planar\"", 1)
    });
    match error {
        ModelError::Validation(message) => {
            assert!(message.contains("non-scalar joint"), "{message}")
        }
        other => panic!("{other:?}"),
    }
}

/// Removes the attribute `key="..."` from the first `<limit>` element.
fn without_limit_attribute(source: &str, key: &str) -> String {
    let start = source.find("<limit").unwrap();
    let end = start + source[start..].find("/>").unwrap();
    let attribute_start = start + source[start..end].find(&format!("{key}=\"")).unwrap();
    let value_start = attribute_start + key.len() + 2;
    let attribute_end = value_start + source[value_start..].find('"').unwrap() + 1;
    format!("{}{}", &source[..attribute_start], &source[attribute_end..])
}

fn validation_message(error: ModelError) -> String {
    match error {
        ModelError::Validation(message) => message,
        other => panic!("{other:?}"),
    }
}

/// The consumer reads an absent bound as ±infinity. For a revolute joint
/// that is never the intent: it is a `<limit>` lost in a hand edit.
#[test]
fn a_revolute_joint_without_a_limit_element_is_rejected() {
    let error = resolve_with_doctored_urdf("nolimit", |source| {
        let start = source.find("<limit").unwrap();
        let end = start + source[start..].find("/>").unwrap() + 2;
        format!("{}{}", &source[..start], &source[end..])
    });
    let message = validation_message(error);
    assert!(message.contains("needs a <limit>"), "{message}");
    assert!(message.contains("revolute"), "{message}");
}

#[test]
fn a_limit_with_only_effort_and_velocity_is_rejected() {
    for (tag, key) in [("nolower", "lower"), ("noupper", "upper")] {
        let error = resolve_with_doctored_urdf(tag, |source| without_limit_attribute(source, key));
        let message = validation_message(error);
        assert!(message.contains("both lower and upper"), "{message}");
    }
}

#[test]
fn prismatic_joints_need_position_limits_too() {
    let error = resolve_with_doctored_urdf("prismatic", |source| {
        without_limit_attribute(
            &source.replacen("type=\"revolute\"", "type=\"prismatic\"", 1),
            "upper",
        )
    });
    let message = validation_message(error);
    assert!(message.contains("prismatic joint"), "{message}");
}

/// `continuous` is the one joint type that legitimately has no position
/// limits; it must keep resolving, with `lower`/`upper` absent.
#[test]
fn continuous_joints_resolve_without_position_limits() {
    let doctored = doctored_arm_joint(|source| {
        without_limit_attribute(
            &without_limit_attribute(
                &source.replacen("type=\"revolute\"", "type=\"continuous\"", 1),
                "lower",
            ),
            "upper",
        )
    });
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_path_buf();
    let path = root.join(RELATIVE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, doctored).unwrap();
    let resolved = RobotConfig::from_profile("vega_1u")
        .unwrap()
        .with_asset_root(&root)
        .resolve();

    let resolved = resolved.unwrap();
    let continuous: Vec<_> = resolved
        .joint_metadata
        .values()
        .flatten()
        .filter(|joint| joint.joint_type == "continuous")
        .collect();
    assert_eq!(continuous.len(), 1);
    assert_eq!(continuous[0].name, "L_arm_j1");
    assert_eq!((continuous[0].lower, continuous[0].upper), (None, None));
    assert!(continuous[0].velocity.is_some());
}

#[test]
fn non_finite_limits_are_rejected_by_the_parser() {
    for (tag, value) in [("nan", "nan"), ("inf", "inf"), ("neginf", "-infinity")] {
        let error = resolve_with_doctored_urdf(tag, |source| {
            let start = source.find("upper=\"").unwrap() + "upper=\"".len();
            let end = start + source[start..].find('"').unwrap();
            format!("{}{value}{}", &source[..start], &source[end..])
        });
        match error {
            ModelError::Urdf(message) => {
                assert!(message.contains("must be finite"), "{message}");
                assert!(message.contains("upper"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }
}

/// A mimic joint follows another joint; listing it in a component would
/// hand the caller a command slot the hardware ignores.
#[test]
fn mimic_joints_cannot_be_component_joints() {
    let error = resolve_with_doctored_urdf("mimic", |source| {
        source.replacen("<limit", "<mimic joint=\"leader\"/><limit", 1)
    });
    let message = validation_message(error);
    assert!(message.contains("mimics \"leader\""), "{message}");
}

#[test]
fn every_embedded_urdf_resolves_with_complete_limits() {
    for profile in dexbot_model::available_profiles() {
        let resolved = RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        assert!(!resolved.joint_metadata.is_empty(), "{profile}");
        for joint in resolved.joint_metadata.values().flatten() {
            if joint.joint_type != "continuous" {
                let (lower, upper) = (joint.lower.unwrap(), joint.upper.unwrap());
                assert!(lower.is_finite() && upper.is_finite() && lower <= upper);
            }
        }
    }
}
