use dexbot_model::{
    vega::{self, EndEffector, HardwareReport, ModelSelection},
    DiscoveryFacts, RobotConfig, RobotOverlay,
};
use std::collections::BTreeMap;

fn both<T: Clone>(value: T) -> BTreeMap<String, T> {
    [("left".into(), value.clone()), ("right".into(), value)].into()
}
fn report(fitted: bool) -> HardwareReport {
    HardwareReport {
        reported_hands: ["left".into(), "right".into()].into(),
        wrist_ft: both(Some(fitted)),
    }
}
fn facts(yaml: &str) -> DiscoveryFacts {
    DiscoveryFacts {
        source: "hardware-test".into(),
        overlay: RobotOverlay::from_yaml(yaml).unwrap(),
    }
}

#[test]
fn every_supported_combination_resolves_through_discovery() {
    for body in ["vega_1", "vega_1p", "vega_1u"] {
        for (suffix, hand) in [
            ("", EndEffector::Bare),
            ("_gripper", EndEffector::Gripper),
            ("_f5d6", EndEffector::F5d6),
        ] {
            let config = RobotConfig::from_profile(&format!("{body}{suffix}"))
                .unwrap()
                .resolve()
                .unwrap();
            for fitted in [true, false] {
                let (resolved, selection) =
                    vega::apply_discovery(&config, facts("{}"), &report(fitted)).unwrap();
                if body == "vega_1" && !fitted {
                    assert!(matches!(selection, ModelSelection::Unchanged { .. }));
                    assert_eq!(resolved.robot.urdf, config.robot.urdf);
                } else {
                    assert_eq!(
                        selection,
                        vega::select_model(body, &both(hand), &both(Some(fitted)))
                    );
                    let wrist = if fitted { "" } else { "_no_ft" };
                    assert!(resolved
                        .robot
                        .urdf
                        .as_ref()
                        .unwrap()
                        .ends_with(&format!("/{body}{wrist}{suffix}.urdf")));
                    assert_eq!(resolved.joint_names("left_arm").unwrap().len(), 7);
                    assert!(resolved.verify_content_hash().unwrap());
                }
            }
        }
    }
}

#[test]
fn ambiguous_reports_do_not_guess_a_model() {
    let config = RobotConfig::from_profile("vega_1p_gripper")
        .unwrap()
        .resolve()
        .unwrap();
    let mut cases = vec![HardwareReport::default()];
    let mut unknown = report(true);
    unknown.wrist_ft.insert("right".into(), None);
    cases.push(unknown);
    let mut missing = report(true);
    missing.reported_hands.remove("right");
    cases.push(missing);
    let mut extra = report(true);
    extra.reported_hands.insert("other".into());
    cases.push(extra);
    let mut different = report(true);
    different.wrist_ft.insert("right".into(), Some(false));
    cases.push(different);
    for report in cases {
        let (resolved, selection) = vega::apply_discovery(&config, facts("{}"), &report).unwrap();
        assert!(matches!(selection, ModelSelection::Unchanged { .. }));
        assert_eq!(resolved.robot.urdf, config.robot.urdf);
    }
    let mut hands = both(EndEffector::Gripper);
    hands.insert("right".into(), EndEffector::F5d6);
    assert!(matches!(
        vega::select_model("vega_1p", &hands, &both(Some(true))),
        ModelSelection::Unchanged { .. }
    ));
    assert!(matches!(
        vega::select_model("vega_9", &both(EndEffector::Bare), &both(Some(true))),
        ModelSelection::Unchanged { .. }
    ));
}

#[test]
fn effective_hands_and_user_urdf_policy_are_preserved() {
    // Retaining configured F5D6 hands must select F5D6 even if transport saw
    // other hand hardware but hand-type overrides were disabled by the user.
    let config = RobotConfig::from_profile("vega_1p_f5d6")
        .unwrap()
        .resolve()
        .unwrap();
    let (retained, _) = vega::apply_discovery(&config, facts("{}"), &report(false)).unwrap();
    assert!(retained.robot.urdf.unwrap().ends_with("_no_ft_f5d6.urdf"));
    let disable = "components:\n  left_hand: {enabled: false, required: false}\n  right_hand: {enabled: false, required: false}\n";
    let (bare, _) = vega::apply_discovery(&config, facts(disable), &report(false)).unwrap();
    assert!(bare.robot.urdf.unwrap().ends_with("/vega_1p_no_ft.urdf"));
    let explicit = RobotConfig::from_profile("vega_1p")
        .unwrap()
        .with_overlay_yaml(
            "user",
            "robot:\n  urdf: package://dexmate_urdf/robots/humanoid/vega_1p/vega_1p.urdf\n",
        )
        .unwrap()
        .resolve()
        .unwrap();
    assert!(matches!(
        vega::apply_discovery(&explicit, facts("{}"), &report(false)),
        Err(dexbot_model::ModelError::DiscoveryConflict { .. })
    ));
}

#[test]
fn unavailable_external_variant_retains_the_configured_model() {
    let root = tempfile::tempdir().unwrap();
    let relative = "robots/humanoid/vega_1p/vega_1p.urdf";
    let path = root.path().join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/urdf")
            .join(relative),
        path,
    )
    .unwrap();
    let config = RobotConfig::from_profile("vega_1p")
        .unwrap()
        .with_asset_root(root.path())
        .resolve()
        .unwrap();
    let (resolved, selection) =
        vega::apply_discovery(&config, facts("{}"), &report(false)).unwrap();
    assert_eq!(resolved.robot.urdf, config.robot.urdf);
    assert!(
        matches!(selection, ModelSelection::Unchanged { reason } if reason.contains("missing"))
    );
}

#[test]
fn injected_hands_select_the_matching_model_before_joint_validation() {
    let config = RobotConfig::from_profile("vega_1p")
        .unwrap()
        .resolve()
        .unwrap();
    let template = RobotConfig::from_profile("vega_1p_gripper")
        .unwrap()
        .resolve()
        .unwrap();
    let facts = DiscoveryFacts {
        source: "detected-hands".into(),
        overlay: RobotOverlay(serde_json::json!({"components": {
            "left_hand": template.component("left_hand").unwrap(),
            "right_hand": template.component("right_hand").unwrap(),
        }})),
    };
    let (resolved, _) = vega::apply_discovery(&config, facts, &report(false)).unwrap();
    assert!(resolved
        .robot
        .urdf
        .as_ref()
        .unwrap()
        .ends_with("vega_1p_no_ft_gripper.urdf"));
    assert_eq!(resolved.joint_names("left_hand").unwrap(), ["L_gripper_j1"]);
}
