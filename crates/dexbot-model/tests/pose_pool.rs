//! `metadata.pose_pool` is validated at the model layer: every pose is an
//! array of finite numbers with one target per joint, so a typo is caught by
//! `dexbot validate` instead of by the first robot that asks for the pose.

use dexbot_model::{available_profiles, RobotConfig};

fn overlay(pose_pool: &str) -> Result<dexbot_model::ResolvedRobotConfig, dexbot_model::ModelError> {
    RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml(
            "test",
            &format!("components:\n  left_arm:\n    metadata:\n      pose_pool:\n{pose_pool}\n"),
        )
        .unwrap()
        .resolve()
}

#[test]
fn every_shipped_profile_still_validates() {
    for profile in available_profiles() {
        RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap_or_else(|error| panic!("{profile}: {error}"));
    }
}

#[test]
fn a_pose_with_the_wrong_length_is_refused() {
    let error = overlay("        short: [0.0, 0.0, 0.0]")
        .unwrap_err()
        .to_string();
    assert!(error.contains("left_arm"), "{error}");
    assert!(
        error.contains("\"short\" has 3 targets for 7 joints"),
        "{error}"
    );
}

#[test]
fn a_non_numeric_entry_is_refused() {
    let error = overlay("        typo: [0.0, 0.0, 0.0, \"0.1\", 0.0, 0.0, 0.0]")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("\"typo\"[3] must be a finite number"),
        "{error}"
    );
}

#[test]
fn a_pose_that_is_not_an_array_is_refused() {
    let error = overlay("        scalar: 0.5").unwrap_err().to_string();
    assert!(error.contains("\"scalar\" must be an array"), "{error}");
}

#[test]
fn a_pose_pool_that_is_not_an_object_is_refused() {
    let error = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml(
            "test",
            "components:\n  left_arm:\n    metadata:\n      pose_pool: [1, 2, 3]\n",
        )
        .unwrap()
        .resolve()
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("must be an object of named poses"),
        "{error}"
    );
}

#[test]
fn a_well_formed_extra_pose_is_accepted() {
    let resolved = overlay("        extra: [0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]").unwrap();
    let pool = &resolved.components["left_arm"].metadata["pose_pool"];
    assert!(pool.get("extra").is_some());
}

#[test]
fn pose_frames_are_explicit_and_validated() {
    for invalid in ["[]", "{folded: world}", "{missing_pose: joint}"] {
        let result = RobotConfig::from_profile("vega_1")
            .unwrap()
            .with_overlay_yaml(
                "bad-frame",
                &format!("components:\n  left_arm:\n    metadata:\n      pose_frames: {invalid}\n"),
            );
        assert!(result.and_then(|c| c.resolve()).is_err(), "{invalid}");
    }
    let c = RobotConfig::from_profile("vega_1p")
        .unwrap()
        .resolve()
        .unwrap();
    for side in ["left_arm", "right_arm"] {
        let metadata = &c.components[side].metadata;
        assert_eq!(metadata["pose_pool"]["folded"]["frame"], "joint");
        assert!(metadata["pose_pool"].get("joint_zero").is_none());
        assert!(!metadata.contains_key("pose_frames"));
        assert_eq!(metadata["pose_pool"]["zero"]["frame"], "torso_upright");
        assert_eq!(
            metadata["pose_pool"]["zero"]["joint_pos"],
            serde_json::json!([0, 0, 0, 0, 0, 0, 0])
        );
    }
}

#[test]
fn structured_pose_fields_are_required_and_checked() {
    for entry in [
        "{joint_pos: [0, 0, 0, 0, 0, 0, 0]}",
        "{frame: joint}",
        "{joint_pos: [0, 0, 0, 0, 0, 0, 0], frame: world}",
        "{joint_pos: [0, 0, 0, 0, 0, 0, 0], frame: joint, typo: true}",
        "{joint_pos: [0, 0], frame: joint}",
        "{joint_pos: [0, 0, 0, bad, 0, 0, 0], frame: joint}",
    ] {
        assert!(
            overlay(&format!("        custom: {entry}")).is_err(),
            "{entry}"
        );
    }
    for frame in ["joint", "torso_horizontal", "torso_upright"] {
        assert!(overlay(&format!(
            "        custom: {{joint_pos: [0, 0, 0, 0, 0, 0, 0], frame: {frame}}}"
        ))
        .is_ok());
    }
}

/// The shipped poses are pinned inside their limits by the contract test; a
/// user overlay gets the same check at resolve time instead of at the first
/// `get_predefined_pose` on a robot.
#[test]
fn a_pose_outside_its_joint_limits_is_refused() {
    let limits = RobotConfig::from_profile("vega_1")
        .unwrap()
        .resolve()
        .unwrap()
        .joint_metadata("left_arm")
        .unwrap()
        .to_vec();
    let (name, upper) = (limits[3].name.clone(), limits[3].upper.unwrap());
    let pose = |target: f64| format!("        reach: [0.0, 0.0, 0.0, {target}, 0.0, 0.0, 0.0]");
    let error = overlay(&pose(upper + 0.01)).unwrap_err().to_string();
    assert!(error.contains("\"reach\"[3]"), "{error}");
    assert!(error.contains(&name), "{error}");
    assert!(error.contains("outside the limits"), "{error}");
    // Sitting exactly on a limit is intentional and stays valid.
    overlay(&pose(upper)).unwrap();
    overlay(&pose(limits[3].lower.unwrap())).unwrap();
    // The same check applies to pose objects in the joint frame.
    let lower = limits[0].lower.unwrap();
    let framed = |frame: &str| {
        overlay(&format!(
            "        reach:\n          frame: {frame}\n          joint_pos: [{}, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]",
            lower - 0.5
        ))
    };
    let error = framed("joint").unwrap_err().to_string();
    assert!(error.contains("\"reach\"[0]"), "{error}");
    // Torso-referenced frames offset the first joint by the measured torso
    // pitch at run time, so its stored value is not final and is not judged.
    framed("torso_horizontal").unwrap();
    let error = overlay(&format!(
        "        reach:\n          frame: torso_upright\n          joint_pos: [0.0, 0.0, 0.0, {}, 0.0, 0.0, 0.0]",
        upper + 0.01
    ))
    .unwrap_err()
    .to_string();
    assert!(error.contains("\"reach\"[3]"), "{error}");
}

#[test]
fn yaml_non_finite_pose_targets_are_rejected() {
    for number in [".nan", ".inf", "-.inf"] {
        let error = overlay(&format!("        invalid: [{number}, 0, 0, 0, 0, 0, 0]"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("finite") || error.contains("invalid"),
            "{number}: {error}"
        );
    }
}
