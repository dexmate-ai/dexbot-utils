use dexbot_model::{available_profiles, RobotConfig};
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../robots/contracts/resolved-config/v0")
}

#[test]
fn every_profile_matches_the_shared_v0_fixture() {
    let fixture_root = fixture_root();
    for profile in available_profiles() {
        let actual = RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        let fixture =
            std::fs::read_to_string(fixture_root.join(format!("{profile}.json"))).unwrap();
        let expected: dexbot_model::ResolvedRobotConfig = serde_json::from_str(&fixture).unwrap();
        assert_eq!(actual, expected, "resolved fixture drift for {profile}");
        assert_eq!(
            actual.normalized_json().unwrap(),
            fixture,
            "serialized fixture drift for {profile}"
        );
    }
}

#[test]
fn every_fixture_validates_against_the_v0_json_schema() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../robots/schema/resolved-config-v0.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    // Guard against a vacuous validator before trusting the fixture passes.
    assert!(!validator.is_valid(&serde_json::json!({})));
    let fixture_root = fixture_root();
    for profile in available_profiles() {
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture_root.join(format!("{profile}.json"))).unwrap(),
        )
        .unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&fixture)
            .map(|error| format!("{profile}: {} at {}", error, error.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "fixture schema violations:\n{}",
            errors.join("\n")
        );
    }
}

/// A predefined pose outside its own joint's limits is unreachable: the
/// client rejects the target before it ever goes on the wire, so the pose is
/// dead on arrival for every caller of `get_predefined_pose`.
///
/// Two shipped poses were exactly that — `folded` asked `*_arm_j4` for -3.1
/// against a -3.071 limit, and the vega_1/vega_1p torso `folded` asked for
/// -1.5708 against -1.57. Both were rounding slop, not intent.
#[test]
fn every_predefined_pose_is_within_its_joint_limits() {
    let mut checked = 0usize;
    let mut violations = Vec::new();
    for profile in available_profiles() {
        let resolved = RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&resolved.normalized_json().unwrap()).unwrap();
        let metadata = &value["joint_metadata"];
        let components = value["components"].as_object().unwrap();
        for (component, config) in components {
            let Some(pool) = config["metadata"]["pose_pool"].as_object() else {
                continue;
            };
            let limits = metadata[component]
                .as_array()
                .expect("pose component must have joint metadata");
            for (pose, targets) in pool {
                let targets = targets
                    .get("joint_pos")
                    .unwrap_or(targets)
                    .as_array()
                    .unwrap();
                assert_eq!(targets.len(), limits.len(), "{profile}: {component}.{pose}");
                for (index, target) in targets.iter().enumerate() {
                    let target = target.as_f64().expect("numeric pose target");
                    let joint = &limits[index];
                    let (lower, upper) = (
                        joint["lower"].as_f64().unwrap(),
                        joint["upper"].as_f64().unwrap(),
                    );
                    checked += 1;
                    if target < lower || target > upper {
                        violations.push(format!(
                            "{profile}: {component}.{pose}[{index}] {} = {target} outside \
                             [{lower}, {upper}]",
                            joint["name"]
                        ));
                    }
                }
            }
        }
    }
    assert!(violations.is_empty(), "{violations:#?}");
    // Guard against the walk silently finding nothing to check.
    assert!(checked > 500, "only {checked} pose targets were checked");
}

/// The profiles were seeded from Python dataclasses, and some values came
/// across as reified defaults or `repr()` strings: a `depth_rtc_channel`
/// pointing at the RGB channel (depth is Zenoh-only), and metadata such as
/// `depth: CameraConfig(enabled=False, ...)`. Nothing reads them, and a
/// consumer that ever did would get nonsense.
#[test]
fn shipped_profiles_carry_no_python_seed_artifacts() {
    for profile in available_profiles() {
        let resolved = RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        for (name, object) in resolved.components.iter().chain(resolved.sensors.iter()) {
            for (key, value) in &object.metadata {
                let text = value.as_str().unwrap_or_default();
                assert!(
                    !(text.contains('(') && text.ends_with(')')),
                    "{profile}: {name}.metadata.{key} is a Python repr: {text}"
                );
            }
        }
        let head = resolved.sensor("head_camera").unwrap();
        assert!(
            !head.endpoints.contains_key("depth_rtc_channel"),
            "{profile}"
        );
        // The depth stream itself stays, on its Zenoh topic.
        assert_eq!(head.endpoints["depth_topic"].kind, "topic");
    }
}

#[test]
fn schemas_are_well_formed_json_documents() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../robots/schema");
    for name in ["profile-v1.schema.json", "resolved-config-v0.schema.json"] {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(name)).unwrap()).unwrap();
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }
}

#[test]
fn schemas_reject_missing_models_and_unknown_safety_vocabulary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../robots/schema");
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("profile-v1.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(!validator.is_valid(&serde_json::json!({"schema_version":1,"robot":{}})));
    assert!(validator.is_valid(&serde_json::json!({"schema_version":1,"robot":{"model":"custom"}})));
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("resolved-config-v0.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let config = RobotConfig::from_profile("vega_1")
        .unwrap()
        .resolve()
        .unwrap();
    let baseline = serde_json::to_value(config).unwrap();
    assert!(validator.is_valid(&baseline));
    for (pointer, value) in [
        ("/runtime/readiness", serde_json::json!("anything")),
        ("/safety", serde_json::json!({"unknown": "none"})),
        (
            "/safety",
            serde_json::json!({"estop_failure_action": "ignore"}),
        ),
        (
            "/components/estop/safety",
            serde_json::json!({"monitoring_enabled": true}),
        ),
        (
            "/components/estop/safety",
            serde_json::json!({"monitoring": "true"}),
        ),
    ] {
        let mut value_to_check = baseline.clone();
        *value_to_check.pointer_mut(pointer).unwrap() = value;
        assert!(!validator.is_valid(&value_to_check), "{pointer}");
    }
}

#[test]
fn gripper_open_pose_reaches_the_upstream_upper_limit() {
    for profile in ["vega_1_gripper", "vega_1p_gripper", "vega_1u_gripper"] {
        let config = RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        for hand in ["left_hand", "right_hand"] {
            let component = config.component(hand).unwrap();
            let open = component.metadata["pose_pool"]["open"]["joint_pos"][0]
                .as_f64()
                .unwrap();
            let joints = config.joint_metadata(hand).unwrap();
            assert_eq!(joints.len(), 1);
            assert_eq!(open, 0.96, "{profile}: {hand}");
            assert_eq!(Some(open), joints[0].upper, "{profile}: {hand}");
            assert_eq!(component.metadata["pose_pool"]["open"]["frame"], "joint");
        }
    }
}
