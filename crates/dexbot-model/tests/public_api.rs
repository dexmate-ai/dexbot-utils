use dexbot_model::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn profile(body: &str) -> String {
    format!("schema_version: 1\nrobot:\n  model: test\ncomponents:\n  part:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/test\n{body}")
}

/// Applies `overlay` as detected-hardware facts from `source`.
fn discover(
    config: &ResolvedRobotConfig,
    source: &str,
    overlay: RobotOverlay,
) -> dexbot_model::Result<ResolvedRobotConfig> {
    apply_discovery(
        config,
        DiscoveryFacts {
            source: source.into(),
            overlay,
        },
    )
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("dexbot-model-{name}-{stamp}"));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn profile_sources_files_overlays_and_info_work() {
    let path = temp_file("profile.yaml", &profile(""));
    let overlay = temp_file("overlay.yaml", "robot:\n  namespace: demo\n");
    let resolved = RobotConfig::from_file(&path)
        .unwrap()
        .with_overlay_file(&overlay)
        .unwrap()
        .resolve()
        .unwrap();
    let info = RobotInfo::new(resolved.clone());
    assert_eq!(info.robot_model(), "test");
    assert!(info.has_component("part"));
    assert!(!info.has_sensor("missing"));
    assert_eq!(info.component_names().collect::<Vec<_>>(), vec!["part"]);
    assert!(info.sensor_names().next().is_none());
    assert!(info.config().normalized_json().unwrap().ends_with('\n'));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(overlay).unwrap();
}

#[test]
fn source_and_overlay_failures_are_structured() {
    assert!(matches!(
        RobotConfig::from_profile("missing"),
        Err(ModelError::UnknownProfile(_))
    ));
    assert!(matches!(
        RobotConfig::from_yaml("bad", "["),
        Err(ModelError::Parse { .. })
    ));
    assert!(matches!(
        RobotConfig::from_yaml("root", "[]").unwrap_err(),
        ModelError::Parse { .. }
    ));
    assert!(matches!(
        RobotConfig::from_file("/definitely/missing"),
        Err(ModelError::Io { .. })
    ));
    let cfg = RobotConfig::from_yaml("ok", &profile("")).unwrap();
    assert!(matches!(
        cfg.with_overlay_yaml("bad", "extends: [vega_1]\n"),
        Err(ModelError::Validation(_))
    ));
    let cfg = RobotConfig::from_yaml("ok", &profile("")).unwrap();
    assert!(matches!(
        cfg.with_overlay_file("/definitely/missing"),
        Err(ModelError::Io { .. })
    ));
}

#[test]
fn validation_rejects_schema_identity_component_and_endpoint_errors() {
    for text in [
        profile("").replace("model: test", "model: ''"),
        profile("").replace("driver: vendor.test", "driver: ''"),
        profile("").replace(
            "driver: vendor.test",
            "driver: vendor.test\n    required: true\n    enabled: false",
        ),
        profile("").replace(
            "    endpoints:\n      state:\n        kind: topic\n        value: state/test\n",
            "",
        ),
        profile("").replace("kind: topic", "kind: ''"),
    ] {
        assert!(matches!(
            RobotConfig::from_yaml("invalid", &text).unwrap().resolve(),
            Err(ModelError::Validation(_))
        ));
    }
}

#[test]
fn unsupported_schema_version_is_reported_before_strict_field_checks() {
    // A future schema may carry fields unknown to this build; the version
    // verdict must win over any unknown-field parse error.
    let future = "schema_version: 2\nrobot:\n  model: test\nnew_v2_section:\n  x: 1\n";
    let error = RobotConfig::from_yaml("future", future)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(
        matches!(&error, ModelError::UnsupportedSchemaVersion { found, expected: 1 } if found == "2")
    );
    let error = RobotConfig::from_yaml("typed", "schema_version: two\nrobot:\n  model: test\n")
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(matches!(error, ModelError::UnsupportedSchemaVersion { .. }));
    let error = RobotConfig::from_yaml("missing", "robot:\n  model: test\n")
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(matches!(error, ModelError::Validation(_)));
}

#[test]
fn validation_rejects_bad_capabilities_joints_and_dependencies() {
    let cases = [
        "    capabilities: [bad]\n",
        "    capabilities: [joint_state, joint_state]\n",
        "    joints:\n      names: [a, a]\n",
        "    joints:\n      names: []\n",
        "    joints:\n      names: [a]\n      source: dynamixel\n",
        "    joints:\n      names: [a]\n      group: arm\n",
        "    joints:\n      source: urdf\n",
        "    dependencies: [missing]\n",
    ];
    for extra in cases {
        assert!(matches!(
            RobotConfig::from_yaml("invalid", &profile(extra))
                .unwrap()
                .resolve(),
            Err(ModelError::Validation(_))
        ));
    }
    let cycle=profile("    dependencies: [other]\n")+"  other:\n    driver: vendor.test\n    dependencies: [part]\n    endpoints:\n      state: {kind: topic, value: state/other}\n";
    assert!(matches!(
        RobotConfig::from_yaml("cycle", &cycle).unwrap().resolve(),
        Err(ModelError::Validation(_))
    ));
}

#[test]
fn optional_extension_capability_and_local_component_are_valid() {
    let text =
        profile("    capabilities:\n      - id: vendor.product.diag\n        required: false\n")
            + "  local:\n    driver: vendor.local\n    local: true\n    dependencies: [part]\n";
    let value = RobotConfig::from_yaml("extensions", &text)
        .unwrap()
        .resolve()
        .unwrap();
    assert!(!value.component("part").unwrap().capabilities[0].required);
    assert!(value.component("local").unwrap().local);
}

#[test]
fn discovery_and_overlay_stage_rules_are_enforced() {
    let static_config = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .resolve()
        .unwrap();
    let operational = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("robot:\n  namespace: found\n").unwrap(),
    )
    .unwrap();
    assert_eq!(operational.resolution_stage, ResolutionStage::Operational);
    assert!(matches!(
        discover(&operational, "again", RobotOverlay(serde_json::json!({}))),
        Err(ModelError::Validation(_))
    ));
    assert!(RobotOverlay::from_yaml("[").is_err());
}

#[test]
fn resource_resolution_and_urdf_error_paths_work() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::write(temporary.path().join("a.urdf"), "").unwrap();
    let resolver = PackageResolver::new().with_package("robot", temporary.path());
    assert_eq!(
        resolver.resolve("relative.urdf").unwrap(),
        PathBuf::from("relative.urdf")
    );
    assert_eq!(
        resolver.resolve("file:///tmp/a.urdf").unwrap(),
        PathBuf::from("/tmp/a.urdf")
    );
    assert_eq!(
        resolver.resolve("package://robot/a.urdf").unwrap(),
        temporary.path().join("a.urdf").canonicalize().unwrap()
    );
    assert!(resolver.resolve("http://x").is_err());
    assert!(resolver.resolve("package://missing/a").is_err());
    assert!(UrdfModel::parse("<robot>").is_err());
    assert!(UrdfModel::parse("<robot name='x'><joint type='fixed'/></robot>").is_err());
    assert!(UrdfModel::parse("<robot name='x'><joint name='j'/></robot>").is_err());
    assert!(UrdfModel::parse(
        "<robot name='x'><joint name='j' type='revolute'><limit velocity='x'/></joint></robot>"
    )
    .is_err());
    let path = temp_file("sample.urdf", "<robot name='x'><link name='a'/></robot>");
    assert_eq!(UrdfModel::from_file(&path).unwrap().links, vec!["a"]);
    std::fs::remove_file(path).unwrap();
    assert!(matches!(
        UrdfModel::from_file("/definitely/missing"),
        Err(ModelError::Io { .. })
    ));
}

#[test]
fn discovery_preserves_static_provenance() {
    let static_config = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .with_overlay_yaml("user", "robot:\n  namespace: user-set\n")
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(
        static_config
            .provenance
            .get("robot.namespace")
            .map(String::as_str),
        Some("overlay:user")
    );

    let operational = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 9000\n").unwrap(),
    )
    .unwrap();

    // Untouched values keep their original layer attribution.
    assert_eq!(
        operational
            .provenance
            .get("robot.namespace")
            .map(String::as_str),
        Some("overlay:user")
    );
    assert_eq!(
        operational
            .provenance
            .get("robot.model")
            .map(String::as_str),
        Some("profile:test")
    );
    // Discovery-set values are attributed to the discovery layer.
    assert_eq!(
        operational
            .provenance
            .get("runtime.state_idle_timeout_ms")
            .map(String::as_str),
        Some("discovery:probe")
    );
    // No synthetic "static-resolved" layer appears anywhere.
    assert!(operational
        .provenance
        .values()
        .all(|layer| !layer.contains("static-resolved")));
}

#[test]
fn discovery_conflicts_with_user_layers_fail_with_both_provenances() {
    let static_config = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .with_overlay_yaml("user", "runtime:\n  state_idle_timeout_ms: 1234\n")
        .unwrap()
        .resolve()
        .unwrap();
    // A contradictory discovered fact fails resolution, naming both layers.
    let error = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 9999\n").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        &error,
        ModelError::DiscoveryConflict { path, existing, incoming }
            if path == "runtime.state_idle_timeout_ms"
                && existing == "overlay:user"
                && incoming == "discovery:probe"
    ));
    let message = error.to_string();
    assert!(message.contains("overlay:user") && message.contains("discovery:probe"));
    // An equal discovered fact is a no-op that keeps the user provenance.
    let operational = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 1234\n").unwrap(),
    )
    .unwrap();
    assert_eq!(operational.runtime.state_idle_timeout_ms, 1234);
    assert_eq!(
        operational
            .provenance
            .get("runtime.state_idle_timeout_ms")
            .map(String::as_str),
        Some("overlay:user")
    );
    // Values from base profiles and `extends` fragments stay discoverable.
    let profile_default = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .resolve()
        .unwrap();
    let operational = discover(
        &profile_default,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 9999\n").unwrap(),
    )
    .unwrap();
    assert_eq!(operational.runtime.state_idle_timeout_ms, 9999);
    assert_eq!(
        operational
            .provenance
            .get("runtime.state_idle_timeout_ms")
            .map(String::as_str),
        Some("discovery:probe")
    );
}

#[test]
fn discovery_conflicts_cover_api_layers() {
    let static_config = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_sensor_enabled("head_camera")
        .resolve()
        .unwrap();
    let error = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("sensors:\n  head_camera:\n    enabled: false\n").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        &error,
        ModelError::DiscoveryConflict { path, existing, incoming }
            if path == "sensors.head_camera.enabled"
                && existing == "api:enable_sensor:head_camera"
                && incoming == "discovery:probe"
    ));
    let operational = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("sensors:\n  head_camera:\n    enabled: true\n").unwrap(),
    )
    .unwrap();
    assert!(operational.sensor("head_camera").unwrap().enabled);
}

#[test]
fn apply_discovery_reuses_the_static_asset_root() {
    // The custom root shadows an embedded asset path with a URDF whose
    // joints and limits differ from the embedded file.
    let root = std::env::temp_dir().join(format!("dexbot-discovery-root-{}", std::process::id()));
    let directory = root.join("robots/humanoid/vega_1");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("vega_1.urdf"),
        r#"<robot name="custom">
  <link name="base"/><link name="tip"/>
  <joint name="custom_j1" type="revolute"><parent link="base"/><child link="tip"/>
    <limit lower="-9" upper="9" effort="5" velocity="3"/></joint>
</robot>"#,
    )
    .unwrap();
    let text = profile("    joints:\n      names: [custom_j1]\n").replace(
        "model: test",
        "model: test\n  urdf: package://dexmate_urdf/robots/humanoid/vega_1/vega_1.urdf",
    );
    let static_config = RobotConfig::from_yaml("custom", &text)
        .unwrap()
        .with_asset_root(&root)
        .resolve()
        .unwrap();
    assert_eq!(
        static_config.joint_metadata("part").unwrap()[0].lower,
        Some(-9.0)
    );
    // No machine-specific path leaks into the serialized output.
    assert!(!static_config
        .normalized_json()
        .unwrap()
        .contains(root.to_str().unwrap()));
    // Operational re-resolution validates against the same custom URDF; the
    // embedded vega_1.urdf has no `custom_j1` and would fail here.
    let operational = discover(
        &static_config,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 1000\n").unwrap(),
    )
    .unwrap();
    assert_eq!(
        operational.joint_metadata("part").unwrap()[0].lower,
        Some(-9.0)
    );
    // A config deserialized from JSON loses the in-memory asset root and
    // falls back to the env/embedded URDF, which lacks `custom_j1`.
    let deserialized: ResolvedRobotConfig =
        serde_json::from_str(&static_config.normalized_json().unwrap()).unwrap();
    assert_eq!(deserialized.asset_root, None);
    assert!(discover(
        &deserialized,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 1000\n").unwrap(),
    )
    .is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn content_hash_is_portable_and_verifiable() {
    // Identical content resolved from differently-named directories hashes
    // and serializes identically: overlay layers are named by basename.
    let overlay = "runtime:\n  state_idle_timeout_ms: 750\n";
    let mut outputs = Vec::new();
    for label in ["first", "second"] {
        let directory =
            std::env::temp_dir().join(format!("dexbot-hash-{label}-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("site.yaml"), overlay).unwrap();
        let resolved = RobotConfig::from_yaml("test", &profile(""))
            .unwrap()
            .with_overlay_file(directory.join("site.yaml"))
            .unwrap()
            .resolve()
            .unwrap();
        assert_eq!(
            resolved
                .provenance
                .get("runtime.state_idle_timeout_ms")
                .map(String::as_str),
            Some("overlay:site.yaml")
        );
        outputs.push((
            resolved.content_hash.clone(),
            resolved.normalized_json().unwrap(),
        ));
        std::fs::remove_dir_all(&directory).unwrap();
    }
    assert_eq!(outputs[0], outputs[1]);
    // Provenance is diagnostic, not semantic: a differently-named overlay
    // layer with the same content produces the same hash.
    let renamed = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .with_overlay_yaml("elsewhere", overlay)
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(renamed.content_hash, outputs[0].0);
    // Round-trip verification and tamper detection.
    let resolved = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .resolve()
        .unwrap();
    assert!(resolved.verify_content_hash().unwrap());
    let mut round: ResolvedRobotConfig =
        serde_json::from_str(&resolved.normalized_json().unwrap()).unwrap();
    assert!(round.verify_content_hash().unwrap());
    round
        .provenance
        .insert("diagnostic.note".into(), "overlay:elsewhere".into());
    assert!(round.verify_content_hash().unwrap());
    round.runtime.state_idle_timeout_ms += 1;
    assert!(!round.verify_content_hash().unwrap());
}

#[test]
fn endpoint_kinds_are_validated_against_the_vocabulary() {
    for kind in [
        "topic",
        "subscriber_topic",
        "publisher_topic",
        "service",
        "channel",
    ] {
        let text = profile("").replace("kind: topic", &format!("kind: {kind}"));
        assert!(
            RobotConfig::from_yaml("ok", &text)
                .unwrap()
                .resolve()
                .is_ok(),
            "kind {kind} must be accepted"
        );
    }
    let text = profile("").replace("kind: topic", "kind: topics");
    let error = RobotConfig::from_yaml("typo", &text)
        .unwrap()
        .resolve()
        .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("unknown kind \"topics\"") && message.contains("subscriber_topic"));
}

#[test]
fn runtime_readiness_policy_must_be_known() {
    let text = profile("") + "runtime:\n  readiness: required_components\n";
    assert!(RobotConfig::from_yaml("ok", &text)
        .unwrap()
        .resolve()
        .is_ok());
    let text = profile("") + "runtime:\n  readiness: all_of_it\n";
    let error = RobotConfig::from_yaml("typo", &text)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("runtime.readiness"));
}

#[test]
fn enabling_an_unknown_sensor_is_a_dedicated_resolve_error() {
    let error = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_sensor_enabled("head_camera_typo")
        .resolve()
        .unwrap_err();
    assert!(
        matches!(&error, ModelError::UnknownSensor { sensor, model, profile, available } if sensor == "head_camera_typo" && model == "vega_1" && profile == "vega_1" && available.contains("head_camera"))
    );
    // Components are not sensors; the API only enables declared sensors.
    let error = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_sensor_enabled("left_arm")
        .resolve()
        .unwrap_err();
    assert!(matches!(error, ModelError::UnknownSensor { .. }));
    // A declared sensor still resolves.
    assert!(RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_sensor_enabled("head_camera")
        .resolve()
        .is_ok());
}

#[test]
fn grasp_torque_metadata_is_range_checked() {
    for value in ["0", "0.2", "1"] {
        let text = profile(&format!("    metadata:\n      grasp_torque: {value}\n"));
        assert!(
            RobotConfig::from_yaml("ok", &text)
                .unwrap()
                .resolve()
                .is_ok(),
            "grasp_torque {value} must be accepted"
        );
    }
    for value in ["-0.1", "1.5", "\"high\""] {
        let text = profile(&format!("    metadata:\n      grasp_torque: {value}\n"));
        let error = RobotConfig::from_yaml("bad", &text)
            .unwrap()
            .resolve()
            .unwrap_err();
        assert!(
            error.to_string().contains("grasp_torque"),
            "grasp_torque {value} must be rejected, got {error}"
        );
    }
}

#[test]
fn repeated_resolves_reuse_the_embedded_urdf_cache_transparently() {
    let first = RobotConfig::from_profile("vega_1u_f5d6")
        .unwrap()
        .resolve()
        .unwrap();
    let second = RobotConfig::from_profile("vega_1u_f5d6")
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.joint_metadata("left_arm").unwrap().len(), 7);
}

#[test]
fn expanded_capability_entries_reject_unknown_fields() {
    for extra in [
        "    capabilities:\n      - id: vendor.product.diag\n        required: false\n        typo: 1\n",
        "    capabilities:\n      - required: false\n",
        "    capabilities:\n      - id: 7\n",
    ] {
        assert!(matches!(
            RobotConfig::from_yaml("invalid", &profile(extra))
                .unwrap()
                .resolve(),
            Err(ModelError::Parse { .. })
        ));
    }
}

#[test]
fn dependency_chains_beyond_the_supported_depth_are_rejected() {
    let mut text = profile("    dependencies: [chain_0]\n");
    for index in 0..70 {
        text.push_str(&format!(
            "  chain_{index}:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/chain{index}\n"
        ));
        if index < 69 {
            text.push_str(&format!("    dependencies: [chain_{}]\n", index + 1));
        }
    }
    let error = RobotConfig::from_yaml("deep", &text)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("supported depth"));
}

#[test]
fn local_component_without_dependencies_is_rejected() {
    let text = profile("") + "  virtual:\n    driver: vendor.local\n    local: true\n";
    let error = RobotConfig::from_yaml("virtual", &text)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("must declare"));
}

#[test]
fn safety_flags_are_validated_top_level_and_per_component() {
    // Safety flags are scoped to the role that consumes them; these are all
    // E-stop flags, so the test component holds the E-stop capability.
    let estop = |body: &str| profile(&format!("    capabilities: [emergency_stop]\n{body}"));
    // Accepted vocabulary resolves.
    let text = estop("    safety:\n      monitoring: true\n")
        + "safety:\n  estop_failure_action: stop_motion\n  heartbeat_failure_action: shutdown_robot\n  estop_unreadable_action: shutdown_robot\n";
    assert!(RobotConfig::from_yaml("ok", &text)
        .unwrap()
        .resolve()
        .is_ok());
    // The documented E-stop freshness knob is part of the vocabulary: a
    // profile that used it was rejected outright, so the robot could not
    // connect at all and the tuning knob was unusable.
    let tuned = estop("    safety:\n      state_max_age_seconds: 12.5\n");
    let resolved = RobotConfig::from_yaml("tuned", &tuned)
        .unwrap()
        .resolve()
        .expect("state_max_age_seconds is an accepted per-component safety flag");
    assert_eq!(
        resolved
            .component("part")
            .unwrap()
            .safety
            .get("state_max_age_seconds")
            .and_then(serde_json::Value::as_f64),
        Some(12.5)
    );
    // The E-stop delivery model is opt-in vocabulary too: the client defaults
    // to event-driven (latched) semantics because measured firmware publishes
    // `state/estop` only while the button is engaged, and a deployment whose
    // server publishes periodically must be able to say so in its profile
    // rather than being stuck with a default that does not match its robot.
    let periodic = estop("    safety:\n      estop_state_periodic: true\n");
    let resolved = RobotConfig::from_yaml("periodic", &periodic)
        .unwrap()
        .resolve()
        .expect("estop_state_periodic is an accepted per-component safety flag");
    assert_eq!(
        resolved
            .component("part")
            .unwrap()
            .safety
            .get("estop_state_periodic")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    // Rejections: legacy spelling, unknown flags, bad types, bad actions.
    for (name, text) in [
        (
            "estop-periodic-type",
            estop("    safety:\n      estop_state_periodic: 1\n"),
        ),
        (
            "legacy",
            estop("    safety:\n      monitoring_enabled: true\n"),
        ),
        ("unknown", estop("    safety:\n      watchdog: true\n")),
        ("type", estop("    safety:\n      monitoring: 3\n")),
        ("timeout", estop("    safety:\n      timeout_seconds: 0\n")),
        (
            "state-max-age",
            estop("    safety:\n      state_max_age_seconds: 0\n"),
        ),
        (
            "top-unknown",
            profile("") + "safety:\n  self_destruct: true\n",
        ),
        (
            "top-action",
            profile("") + "safety:\n  estop_failure_action: explode\n",
        ),
        (
            "top-unreadable-action",
            profile("") + "safety:\n  estop_unreadable_action: explode\n",
        ),
    ] {
        assert!(
            matches!(
                RobotConfig::from_yaml(name, &text).unwrap().resolve(),
                Err(ModelError::Validation(_))
            ),
            "case {name} must be rejected"
        );
    }
    // A safety value duplicating an endpoint must agree with it.
    let contradictory = estop("    safety:\n      estop_query_name: other/estop\n").replace(
        "      state:\n        kind: topic\n        value: state/test\n",
        "      estop_query_name:\n        kind: service\n        value: system/estop\n",
    );
    let error = RobotConfig::from_yaml("contradiction", &contradictory)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("contradicts"));
}

const GROUPED_URDF: &str = r#"<robot name="grouped">
  <link name="base"/><link name="l1"/><link name="l2"/><link name="l3"/><link name="floating_link"/>
  <joint name="j1" type="revolute"><parent link="base"/><child link="l1"/>
    <limit lower="-1.5" upper="1.5" effort="10" velocity="2"/></joint>
  <joint name="j2" type="prismatic"><parent link="l1"/><child link="l2"/>
    <limit lower="0" upper="0.2" effort="50" velocity="0.5"/></joint>
  <joint name="anchor" type="fixed"><parent link="l2"/><child link="l3"/></joint>
  <joint name="floating" type="floating"><parent link="base"/><child link="floating_link"/></joint>
</robot>"#;

fn grouped_profile(urdf_path: &std::path::Path, joints: &str, groups: &str) -> String {
    format!(
        "schema_version: 1\nrobot:\n  model: grouped\n  urdf: file://{}\n{groups}components:\n  part:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/test\n{joints}",
        urdf_path.display()
    )
}

#[test]
fn urdf_joint_groups_resolve_ordered_names_and_limits() {
    let urdf = temp_file("grouped.urdf", GROUPED_URDF);
    // Group order is authoritative and deliberately not document order.
    let text = grouped_profile(
        &urdf,
        "    joints:\n      source: urdf\n      group: arm\n",
        "joint_groups:\n  arm: [j2, j1]\n",
    );
    let resolved = RobotConfig::from_yaml("grouped", &text)
        .unwrap()
        .with_asset_root(urdf.parent().unwrap())
        .resolve()
        .unwrap();
    let joints = resolved.component("part").unwrap().joints.as_ref().unwrap();
    assert_eq!(joints.names, vec!["j2", "j1"]);
    assert_eq!(resolved.joint_names("part").unwrap(), vec!["j2", "j1"]);
    let metadata = resolved.joint_metadata("part").unwrap();
    assert_eq!(metadata[0].name, "j2");
    assert_eq!(metadata[0].joint_type, "prismatic");
    assert_eq!(metadata[0].upper, Some(0.2));
    assert_eq!(metadata[0].velocity, Some(0.5));
    assert_eq!(metadata[1].name, "j1");
    assert_eq!(metadata[1].lower, Some(-1.5));
    assert_eq!(metadata[1].effort, Some(10.0));
    // Discovery re-validation stays consistent with materialized names.
    let operational = discover(
        &resolved,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 1000\n").unwrap(),
    )
    .unwrap();
    assert_eq!(operational.joint_names("part").unwrap(), vec!["j2", "j1"]);
    let info = RobotInfo::new(operational);
    assert_eq!(info.joint_names("part").unwrap(), vec!["j2", "j1"]);
    assert_eq!(info.joint_metadata("part").unwrap().len(), 2);
    std::fs::remove_file(urdf).unwrap();
}

#[test]
fn urdf_joint_resolution_failures_are_validation_errors() {
    let urdf = temp_file("failures.urdf", GROUPED_URDF);
    let cases = [
        // Unknown group.
        (
            "does not exist in joint_groups",
            grouped_profile(
                &urdf,
                "    joints:\n      source: urdf\n      group: leg\n",
                "joint_groups:\n  arm: [j1]\n",
            ),
        ),
        // Explicit names disagree with the group table.
        (
            "disagree with",
            grouped_profile(
                &urdf,
                "    joints:\n      names: [j1, j2]\n      source: urdf\n      group: arm\n",
                "joint_groups:\n  arm: [j2, j1]\n",
            ),
        ),
        // Explicit name missing from the URDF.
        (
            "does not exist in URDF",
            grouped_profile(&urdf, "    joints:\n      names: [ghost]\n", ""),
        ),
        // Fixed joints cannot be commanded.
        (
            "fixed joint",
            grouped_profile(&urdf, "    joints:\n      names: [anchor]\n", ""),
        ),
        // Multi-DOF joints are not valid scalar command channels.
        (
            "unsupported non-scalar joint",
            grouped_profile(&urdf, "    joints:\n      names: [floating]\n", ""),
        ),
        // Empty or duplicate group tables.
        (
            "lists no joints",
            grouped_profile(
                &urdf,
                "    joints:\n      source: urdf\n      group: arm\n",
                "joint_groups:\n  arm: []\n",
            ),
        ),
    ];
    for (needle, text) in cases {
        let error = RobotConfig::from_yaml("failure", &text)
            .unwrap()
            .with_asset_root(urdf.parent().unwrap())
            .resolve()
            .unwrap_err();
        assert!(
            matches!(&error, ModelError::Validation(message) if message.contains(needle)),
            "expected {needle:?} in {error}"
        );
    }
    std::fs::remove_file(urdf).unwrap();

    // URDF-sourced joints without any robot.urdf are rejected.
    let text = profile("    joints:\n      source: urdf\n      group: arm\n")
        + "joint_groups:\n  arm: [j1]\n";
    let error = RobotConfig::from_yaml("no-urdf", &text)
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("no robot.urdf"));
}

#[test]
fn package_uris_resolve_against_embedded_assets_and_explicit_roots() {
    // Built-in profiles resolve their URDF from the embedded assets and
    // expose per-joint limits for every jointed component.
    let resolved = RobotConfig::from_profile("vega_1_f5d6")
        .unwrap()
        .resolve()
        .unwrap();
    let arm = resolved.joint_metadata("left_arm").unwrap();
    assert_eq!(arm.len(), 7);
    assert_eq!(arm[0].name, "L_arm_j1");
    assert!(arm[0].lower.is_some() && arm[0].velocity.is_some());
    // Legacy curated hand order is preserved exactly.
    assert_eq!(
        resolved.joint_names("left_hand").unwrap(),
        vec!["L_th_j1", "L_ff_j1", "L_mf_j1", "L_rf_j1", "L_lf_j1", "L_th_j0"]
    );
    // No metadata for objects without joints.
    assert!(resolved.joint_metadata("battery").is_none());

    // An explicit asset root overrides the embedded assets.
    let root = std::env::temp_dir().join(format!("dexbot-assets-{}", std::process::id()));
    let directory = root.join("robots/humanoid/vega_1");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("vega_1.urdf"), GROUPED_URDF).unwrap();
    let error = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_asset_root(&root)
        .resolve()
        .unwrap_err();
    // The replacement URDF lacks the Vega joints, proving the root was used.
    assert!(error.to_string().contains("does not exist in URDF"));
    std::fs::remove_dir_all(&root).unwrap();

    // Unknown packages and unsupported schemes fail resolution.
    for uri in ["package://other_pkg/robot.urdf", "http://example/x.urdf"] {
        let text = profile("    joints:\n      names: [j1]\n")
            .replace("model: test", &format!("model: test\n  urdf: {uri}"));
        assert!(matches!(
            RobotConfig::from_yaml("unresolved", &text)
                .unwrap()
                .resolve(),
            Err(ModelError::UnresolvedResource(_))
        ));
    }
}

#[test]
fn file_based_profiles_extend_fragments_by_relative_path() {
    let directory = std::env::temp_dir().join(format!("dexbot-fragments-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("shared")).unwrap();
    std::fs::write(
        directory.join("shared/base.yaml"),
        "runtime:\n  state_idle_timeout_ms: 750\ncomponents:\n  part:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/test\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("robot.yaml"),
        "schema_version: 1\nextends:\n- shared/base.yaml\nrobot:\n  model: filetest\n",
    )
    .unwrap();
    let resolved = RobotConfig::from_file(directory.join("robot.yaml"))
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(resolved.runtime.state_idle_timeout_ms, 750);
    assert!(resolved.component("part").is_some());
    assert_eq!(
        resolved.provenance.get("runtime.state_idle_timeout_ms"),
        Some(&"extends:file:shared/base.yaml".to_string())
    );

    // A file-based profile can also reach the embedded catalog fragments.
    std::fs::write(
        directory.join("vega_like.yaml"),
        "schema_version: 1\nextends:\n- common/vega_upper_body.yaml\nrobot:\n  model: vega_custom\n",
    )
    .unwrap();
    let resolved = RobotConfig::from_file(directory.join("vega_like.yaml"))
        .unwrap()
        .resolve()
        .unwrap();
    assert!(resolved.component("left_arm").is_some());
    assert_eq!(
        resolved.provenance.get("components.left_arm.driver"),
        Some(&"extends:common/vega_upper_body.yaml".to_string())
    );
    // An on-disk file shadowing an embedded fragment name is told apart
    // from the embedded one in provenance.
    std::fs::create_dir_all(directory.join("common")).unwrap();
    std::fs::write(
        directory.join("common/vega_upper_body.yaml"),
        std::fs::read_to_string(directory.join("shared/base.yaml")).unwrap(),
    )
    .unwrap();
    let shadowed = RobotConfig::from_file(directory.join("vega_like.yaml"))
        .unwrap()
        .resolve()
        .unwrap();
    assert!(shadowed.component("left_arm").is_none());
    assert_eq!(
        shadowed.provenance.get("components.part.driver"),
        Some(&"extends:file:common/vega_upper_body.yaml".to_string())
    );

    // Unknown fragments and nested extends are rejected.
    std::fs::write(
        directory.join("missing.yaml"),
        "schema_version: 1\nextends:\n- shared/ghost.yaml\nrobot:\n  model: x\n",
    )
    .unwrap();
    assert!(matches!(
        RobotConfig::from_file(directory.join("missing.yaml")),
        Err(ModelError::UnknownFragment { .. })
    ));
    std::fs::write(
        directory.join("shared/nested.yaml"),
        "extends:\n- shared/base.yaml\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("nested.yaml"),
        "schema_version: 1\nextends:\n- shared/nested.yaml\nrobot:\n  model: x\n",
    )
    .unwrap();
    assert!(matches!(
        RobotConfig::from_file(directory.join("nested.yaml")),
        Err(ModelError::NestedExtends { .. })
    ));
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn built_in_profiles_are_fragment_composed_yet_stay_deeply_merged() {
    // The nine catalog profiles now share fragments; spot-check that a hands
    // variant carries upper body, mobility, sensors, and hands together.
    let resolved = RobotConfig::from_profile("vega_1p_gripper")
        .unwrap()
        .resolve()
        .unwrap();
    for component in ["left_arm", "torso", "chassis", "left_hand", "estop"] {
        assert!(
            resolved.component(component).is_some(),
            "missing {component}"
        );
    }
    assert!(resolved.sensor("lidar_3d_front").is_some());
    assert_eq!(
        resolved.component("left_hand").unwrap().driver,
        "standard.dex_gripper"
    );
    // The estop safety map uses only the canonical monitoring spelling.
    let estop_safety = &resolved.component("estop").unwrap().safety;
    assert_eq!(
        estop_safety.get("monitoring"),
        Some(&serde_json::Value::Bool(true))
    );
    assert!(!estop_safety.contains_key("monitoring_enabled"));
}

#[test]
fn extended_safety_actions_validate_in_profiles() {
    let text = profile("")
        + "safety:\n  heartbeat_failure_action: request_process_termination\n  estop_failure_action: activate_software_estop\n";
    let resolved = RobotConfig::from_yaml("actions", &text)
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(
        resolved
            .safety
            .get("heartbeat_failure_action")
            .and_then(|v| v.as_str()),
        Some("request_process_termination")
    );
    assert!(matches!(
        RobotConfig::from_yaml(
            "bad",
            &(profile("") + "safety:\n  heartbeat_failure_action: reboot_moon\n")
        )
        .unwrap()
        .resolve(),
        Err(ModelError::Validation(_))
    ));
}

/// One component per safety role, as the shipped profiles declare them.
fn safety_profile(estop: &str, heartbeat: &str, battery: &str) -> String {
    let component = |name: &str, capability: &str, extra: &str| {
        format!(
            "  {name}:\n    driver: standard.{name}\n    capabilities: [{capability}]\n    endpoints:\n      state_sub_topic:\n        kind: subscriber_topic\n        value: state/{name}\n{extra}"
        )
    };
    profile("")
        + &component("estop", "emergency_stop", estop)
        + &component("heartbeat", "heartbeat", heartbeat)
        + &component("battery", "battery", battery)
}

fn validation_error(name: &str, text: &str) -> String {
    match RobotConfig::from_yaml(name, text).unwrap().resolve() {
        Err(ModelError::Validation(message)) => message,
        other => panic!("case {name} must fail validation, got {other:?}"),
    }
}

#[test]
fn safety_durations_are_bounded() {
    let heartbeat = |seconds: &str| {
        safety_profile(
            "",
            &format!("    safety:\n      timeout_seconds: {seconds}\n"),
            "",
        )
    };
    for seconds in ["0.05", "1.0", "10"] {
        RobotConfig::from_yaml("ok", &heartbeat(seconds))
            .unwrap()
            .resolve()
            .unwrap_or_else(|error| panic!("{seconds}: {error}"));
    }
    // 1e30 used to validate and then panic `Duration::from_secs_f64`
    // downstream; 86400 silently disabled the dead-man.
    for seconds in [
        "1e30", "86400", "10.5", "0.01", "0", "-1", ".inf", ".nan", "soon",
    ] {
        let message = validation_error(seconds, &heartbeat(seconds));
        assert!(message.contains("[0.05, 10]"), "{seconds}: {message}");
    }
    // On an E-stop component the same flag is the poll interval.
    let estop = |flag: &str, seconds: &str| {
        safety_profile(&format!("    safety:\n      {flag}: {seconds}\n"), "", "")
    };
    RobotConfig::from_yaml("ok", &estop("timeout_seconds", "0.02"))
        .unwrap()
        .resolve()
        .unwrap();
    for seconds in ["1e30", "5", "0.0001"] {
        let message = validation_error(seconds, &estop("timeout_seconds", seconds));
        assert!(message.contains("[0.001, 1]"), "{seconds}: {message}");
    }
    for seconds in ["0.1", "12.5", "60"] {
        RobotConfig::from_yaml("ok", &estop("state_max_age_seconds", seconds))
            .unwrap()
            .resolve()
            .unwrap();
    }
    for seconds in ["1e30", "61", "0.05"] {
        let message = validation_error(seconds, &estop("state_max_age_seconds", seconds));
        assert!(message.contains("[0.1, 60]"), "{seconds}: {message}");
    }
}

#[test]
fn startup_verification_window_has_a_floor() {
    for (millis, valid) in [("0", false), ("99", false), ("100", true), ("5000", true)] {
        let text = profile("") + &format!("runtime:\n  state_idle_timeout_ms: {millis}\n");
        let result = RobotConfig::from_yaml("runtime", &text).unwrap().resolve();
        assert_eq!(result.is_ok(), valid, "{millis}: {result:?}");
    }
}

#[test]
fn battery_thresholds_are_bounded_and_defaults_are_exported() {
    assert_eq!(dexbot_model::DEFAULT_LOW_BATTERY_PERCENTAGE, 20.0);
    assert_eq!(dexbot_model::DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE, 5.0);
    // The shipped profile spells out the exported defaults.
    let shipped = RobotConfig::from_profile("vega_1")
        .unwrap()
        .resolve()
        .unwrap();
    let safety = &shipped.component("battery").unwrap().safety;
    assert_eq!(
        safety["low_battery_percentage"].as_f64(),
        Some(dexbot_model::DEFAULT_LOW_BATTERY_PERCENTAGE)
    );
    assert_eq!(
        safety["battery_hysteresis_percentage"].as_f64(),
        Some(dexbot_model::DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE)
    );
    let battery = |low: &str, hysteresis: &str| {
        safety_profile(
            "",
            "",
            &format!(
                "    safety:\n      low_battery_percentage: {low}\n      battery_hysteresis_percentage: {hysteresis}\n"
            ),
        )
    };
    for (low, hysteresis) in [("20", "5"), ("0.5", "1"), ("95", "4"), ("30", "50")] {
        RobotConfig::from_yaml("ok", &battery(low, hysteresis))
            .unwrap()
            .resolve()
            .unwrap_or_else(|error| panic!("{low}/{hysteresis}: {error}"));
    }
    for (low, hysteresis, expected) in [
        // A zero threshold never warns; a zero hysteresis chatters.
        ("0", "5", "(0, 95]"),
        ("20", "0", "[1, 50]"),
        ("96", "1", "(0, 95]"),
        ("-1", "5", "(0, 95]"),
        ("20", "51", "[1, 50]"),
        ("20", "0.5", "[1, 50]"),
        ("low", "5", "(0, 95]"),
        ("95", "5", "must be below 100"),
    ] {
        let message = validation_error("battery", &battery(low, hysteresis));
        assert!(message.contains(expected), "{low}/{hysteresis}: {message}");
    }
    // One key alone is checked against the other's default.
    let message = validation_error(
        "battery",
        &safety_profile("", "", "    safety:\n      low_battery_percentage: 95\n"),
    );
    assert!(message.contains("must be below 100"), "{message}");
}

#[test]
fn safety_flags_are_scoped_to_the_role_that_consumes_them() {
    // Each role accepts its own vocabulary...
    RobotConfig::from_yaml(
        "ok",
        &safety_profile(
            "    safety:\n      monitoring: true\n      estop_query_name: system/estop\n      estop_state_periodic: false\n",
            "    safety:\n      monitoring: true\n      heartbeat_topic: heartbeat\n",
            "    safety:\n      monitoring: false\n",
        ),
    )
    .unwrap()
    .resolve()
    .unwrap();
    // ...and a standard driver name holds the role without the capability.
    let by_driver = safety_profile("", "", "").replace("    capabilities: [battery]\n", "")
        + "    safety:\n      low_battery_percentage: 25\n";
    RobotConfig::from_yaml("driver", &by_driver)
        .unwrap()
        .resolve()
        .unwrap();
    // A flag on a component that does not consume it used to validate and
    // then be ignored.
    for (name, text, flag) in [
        (
            "battery-on-estop",
            safety_profile("    safety:\n      low_battery_percentage: 20\n", "", ""),
            "low_battery_percentage",
        ),
        (
            "estop-on-heartbeat",
            safety_profile("", "    safety:\n      state_max_age_seconds: 5\n", ""),
            "state_max_age_seconds",
        ),
        (
            "heartbeat-on-battery",
            safety_profile("", "", "    safety:\n      timeout_seconds: 1\n"),
            "timeout_seconds",
        ),
        (
            "estop-on-battery",
            safety_profile("", "", "    safety:\n      estop_state_periodic: true\n"),
            "estop_state_periodic",
        ),
        (
            "monitoring-on-plain-component",
            profile("    safety:\n      monitoring: true\n"),
            "monitoring",
        ),
    ] {
        let message = validation_error(name, &text);
        assert!(message.contains(flag), "{name}: {message}");
        assert!(message.contains("would be ignored"), "{name}: {message}");
    }
}

#[test]
fn heartbeat_timeout_copies_must_agree() {
    let heartbeat = |safety: &str, metadata: &str| {
        safety_profile(
            "",
            &format!(
                "    safety:\n      timeout_seconds: {safety}\n    metadata:\n      timeout_seconds: {metadata}\n"
            ),
            "",
        )
    };
    RobotConfig::from_yaml("ok", &heartbeat("2", "2.0"))
        .unwrap()
        .resolve()
        .unwrap();
    let message = validation_error("stale", &heartbeat("2", "1.0"));
    assert!(message.contains("must be equal"), "{message}");
    // The shipped profiles keep the timeout in the safety map only, so an
    // overlay that retunes it has nothing to fall out of step with.
    let tuned = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml(
            "wifi",
            "components:\n  heartbeat:\n    safety:\n      timeout_seconds: 3\n",
        )
        .unwrap()
        .resolve()
        .unwrap();
    let heartbeat = &tuned.components["heartbeat"];
    assert_eq!(heartbeat.safety["timeout_seconds"], 3);
    assert!(!heartbeat.metadata.contains_key("timeout_seconds"));
}

#[test]
fn behaviour_relevant_metadata_is_validated_without_closing_the_map() {
    let metadata = |entry: &str| profile(&format!("    metadata:\n      {entry}\n"));
    for entry in [
        "state_max_age_ms: 250",
        "state_max_age_ms: 0.5",
        // `false` is the explicit opt-out of the freshness check.
        "state_max_age_ms: false",
        "state_idle_timeout_ms: 100",
        "default_control_hz: 100",
        "max_linear_vel: 0.8",
        "max_linear_vel: 5",
        "max_angular_vel: 10",
        "wheels_dist: 0.45",
        "center_to_wheel_axis_dist: 0.219",
        "max_steering_angle: 2.35",
        // Metadata stays open: unrelated keys of any shape pass through.
        "vendor_note: {anything: [1, two, null]}",
        "camera_serial: null",
        "hz: 5",
    ] {
        RobotConfig::from_yaml("ok", &metadata(entry))
            .unwrap()
            .resolve()
            .unwrap_or_else(|error| panic!("{entry}: {error}"));
    }
    for entry in [
        // A half-finished edit parses as null and would switch the check off.
        "state_max_age_ms:",
        "state_max_age_ms: null",
        "state_max_age_ms: true",
        "state_max_age_ms: 0",
        "state_max_age_ms: -5",
        "state_max_age_ms: .inf",
        "state_max_age_ms: soon",
        "state_idle_timeout_ms: 0",
        "state_idle_timeout_ms: 99",
        "state_idle_timeout_ms: 150.5",
        "state_idle_timeout_ms: null",
        "default_control_hz: 0",
        "default_control_hz: fast",
        "max_linear_vel: 0",
        "max_linear_vel: 5.5",
        "max_linear_vel: .nan",
        "max_angular_vel: 11",
        "wheels_dist: -0.45",
        "wheels_dist: 45",
        "center_to_wheel_axis_dist: null",
        "max_steering_angle: 3.2",
    ] {
        let message = validation_error(entry, &metadata(entry));
        let key = entry.split(':').next().unwrap();
        assert!(message.contains(key), "{entry}: {message}");
    }
    // Near misses of a validated key -- a couple of edits, or a dropped or
    // spelled-out unit suffix -- are typos, not vendor keys.
    for (typo, intended) in [
        ("state_max_age_m", "state_max_age_ms"),
        ("state_max_age", "state_max_age_ms"),
        ("state_idle_timout_ms", "state_idle_timeout_ms"),
        ("max_linear_velocity", "max_linear_vel"),
        ("max_angular_vel_", "max_angular_vel"),
        ("wheel_dist", "wheels_dist"),
        ("default_control_Hz", "default_control_hz"),
    ] {
        let result = RobotConfig::from_yaml("typo", &metadata(&format!("{typo}: 1")))
            .unwrap()
            .resolve();
        let message = result.unwrap_err().to_string();
        assert!(message.contains(typo), "{message}");
        assert!(
            message.contains(&format!("did you mean {intended:?}")),
            "{message}"
        );
    }
}

#[test]
fn a_joint_belongs_to_one_enabled_component() {
    // right_arm listing the left arm's joints used to validate and inherit
    // the left arm's limits.
    let stolen = "components:\n  right_arm:\n    joints:\n      names: [L_arm_j1, L_arm_j2, L_arm_j3, L_arm_j4, L_arm_j5, L_arm_j6, L_arm_j7]\n    metadata:\n      pose_pool:\n        $delete: true\n";
    let error = RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml("swap", stolen)
        .unwrap()
        .resolve()
        .unwrap_err()
        .to_string();
    assert!(error.contains("\"L_arm_j1\""), "{error}");
    assert!(
        error.contains("\"left_arm\"") && error.contains("\"right_arm\""),
        "{error}"
    );
    // A disabled object does not command anything and may overlap.
    let disabled = format!("{stolen}    required: false\n    enabled: false\n");
    RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml("swap", &disabled)
        .unwrap()
        .resolve()
        .unwrap();
    // Neither does a read-only observer of the same joints.
    let observer = format!("{stolen}    capabilities: [joint_state, temperature]\n");
    RobotConfig::from_profile("vega_1")
        .unwrap()
        .with_overlay_yaml("swap", &observer)
        .unwrap()
        .resolve()
        .unwrap();
    // URDF-sourced groups are checked too, once their names are final.
    let urdf = temp_file("owners.urdf", GROUPED_URDF);
    let text = grouped_profile(
        &urdf,
        "    joints:\n      source: urdf\n      group: arm\n  twin:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/twin\n    joints:\n      names: [j1]\n",
        "joint_groups:\n  arm: [j2, j1]\n",
    );
    let error = RobotConfig::from_yaml("owners", &text)
        .unwrap()
        .with_asset_root(urdf.parent().unwrap())
        .resolve()
        .unwrap_err()
        .to_string();
    assert!(error.contains("\"j1\" is claimed by both"), "{error}");
    std::fs::remove_file(urdf).unwrap();
}

#[test]
fn endpoints_and_namespace_must_be_plain_keys() {
    let endpoint =
        |value: &str| profile("").replace("value: state/test", &format!("value: {value:?}"));
    for value in [
        "/state/test",
        "state/test/",
        "state//test",
        "state/ test",
        " state/test",
        "state/*",
        "state/**",
        "state/$*",
        "state/test?x=1",
        "state/test#frag",
    ] {
        let message = validation_error(value, &endpoint(value));
        assert!(message.contains("endpoint \"state\""), "{value}: {message}");
    }
    for namespace in [
        "/robot", "robot/", "a//b", "my robot", "robot/*", "robot$", "r?", "r#1",
    ] {
        let text = profile("").replace(
            "  model: test\n",
            &format!("  model: test\n  namespace: {namespace:?}\n"),
        );
        let message = validation_error(namespace, &text);
        assert!(
            message.contains("robot.namespace"),
            "{namespace}: {message}"
        );
    }
    let text = profile("").replace(
        "  model: test\n",
        "  model: test\n  namespace: dm/vg1p-0001\n",
    );
    RobotConfig::from_yaml("ns", &text)
        .unwrap()
        .resolve()
        .unwrap();
}

#[test]
fn enabled_components_cannot_share_a_publisher_topic() {
    let twin = |extra: &str| {
        let part = "    endpoints:\n      command:\n        kind: publisher_topic\n        value: control/arm\n";
        format!(
            "schema_version: 1\nrobot:\n  model: test\ncomponents:\n  left:\n    driver: vendor.test\n{part}  right:\n    driver: vendor.test\n{part}{extra}"
        )
    };
    let message = validation_error("twin", &twin(""));
    assert!(message.contains("\"control/arm\""), "{message}");
    assert!(
        message.contains("\"left\"") && message.contains("\"right\""),
        "{message}"
    );
    // Disabled objects publish nothing; shared subscriptions are normal.
    RobotConfig::from_yaml("off", &twin("    required: false\n    enabled: false\n"))
        .unwrap()
        .resolve()
        .unwrap();
    RobotConfig::from_yaml(
        "sub",
        &twin("").replace("publisher_topic", "subscriber_topic"),
    )
    .unwrap()
    .resolve()
    .unwrap();
}

/// `apply_overlay` used to be `apply_discovery` under another name: the
/// layer was recorded as `discovery:<name>`, so it could not override an
/// earlier overlay and a later discovery could overwrite it.
#[test]
fn post_resolve_overlays_are_user_layers_not_discovery() {
    let static_config = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .with_overlay_yaml("site", "runtime:\n  state_idle_timeout_ms: 1234\n")
        .unwrap()
        .resolve()
        .unwrap();
    // A later user overlay overrides an earlier one...
    let tuned = apply_overlay(
        &static_config,
        "operator",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 2500\n").unwrap(),
    )
    .unwrap();
    assert_eq!(tuned.runtime.state_idle_timeout_ms, 2500);
    assert_eq!(
        tuned
            .provenance
            .get("runtime.state_idle_timeout_ms")
            .map(String::as_str),
        Some("overlay:operator")
    );
    assert!(tuned.verify_content_hash().unwrap());
    // ...stays static, so discovery still runs afterwards...
    assert_eq!(tuned.resolution_stage, ResolutionStage::Static);
    // ...and discovery cannot silently overwrite it.
    let error = discover(
        &tuned,
        "probe",
        RobotOverlay::from_yaml("runtime:\n  state_idle_timeout_ms: 9999\n").unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        &error,
        ModelError::DiscoveryConflict { existing, incoming, .. }
            if existing == "overlay:operator" && incoming == "discovery:probe"
    ));
    // User intent precedes discovery: an operational config takes no overlay.
    let operational = discover(&tuned, "probe", RobotOverlay(serde_json::json!({}))).unwrap();
    assert!(matches!(
        apply_overlay(&operational, "late", RobotOverlay(serde_json::json!({}))),
        Err(ModelError::Validation(_))
    ));
}

#[test]
fn extends_is_rejected_in_every_layer_after_the_base_profile() {
    let static_config = RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .resolve()
        .unwrap();
    let extending =
        || RobotOverlay::from_yaml("extends: [common/vega_mobile_base.yaml]\n").unwrap();
    for error in [
        apply_overlay(&static_config, "user", extending()).unwrap_err(),
        discover(&static_config, "probe", extending()).unwrap_err(),
    ] {
        assert!(
            matches!(&error, ModelError::Validation(message) if message.contains("cannot contain extends")),
            "{error}"
        );
    }
    assert!(RobotConfig::from_yaml("test", &profile(""))
        .unwrap()
        .with_overlay_yaml("user", "extends: []\n")
        .is_err());
}

#[test]
fn yaml_merge_keys_are_applied_not_stored() {
    // `part` anchors its free-form metadata; `twin` merges it.
    let text = profile("    metadata: &limits\n      max_linear_vel: 0.5\n      vendor_note: shared\n")
        + "  twin:\n    driver: vendor.test\n    endpoints:\n      state:\n        kind: topic\n        value: state/twin\n    metadata:\n      <<: *limits\n      vendor_note: local\n";
    let resolved = RobotConfig::from_yaml("merge", &text)
        .unwrap()
        .resolve()
        .unwrap();
    let metadata = &resolved.component("twin").unwrap().metadata;
    assert!(!metadata.contains_key("<<"), "{metadata:?}");
    assert_eq!(metadata["max_linear_vel"], 0.5);
    // Explicit keys win over merged ones, as YAML specifies.
    assert_eq!(metadata["vendor_note"], "local");
    // Merged values are validated like any other.
    let error = RobotConfig::from_yaml("merge", &text.replace("0.5", "50"))
        .unwrap()
        .resolve()
        .unwrap_err();
    assert!(error.to_string().contains("max_linear_vel"), "{error}");
    // Overlays (both entry points) apply merge keys too.
    let overlay = "sensors: {}\ncomponents:\n  part:\n    metadata: &m\n      vendor_a: 1\n  twin:\n    metadata:\n      <<: *m\n";
    let resolved = RobotConfig::from_yaml("merge", &text)
        .unwrap()
        .with_overlay_yaml("user", overlay)
        .unwrap()
        .resolve()
        .unwrap();
    assert_eq!(resolved.component("twin").unwrap().metadata["vendor_a"], 1);
    let overlay = RobotOverlay::from_yaml(overlay).unwrap();
    assert!(!overlay.0.to_string().contains("<<"));
    assert_eq!(overlay.0["components"]["twin"]["metadata"]["vendor_a"], 1);
    // A merge key that does not point at a mapping is a parse error.
    assert!(matches!(
        RobotConfig::from_yaml("bad", &profile("    metadata:\n      <<: 3\n")),
        Err(ModelError::Parse { .. })
    ));
}

#[test]
fn dex_gripper_bounds_preserve_legacy_safety_contract() {
    use dexbot_model::gripper::{GRASP_TORQUE_HIGH_THRESHOLD, GRASP_TORQUE_MAX, GRASP_TORQUE_MIN};
    assert_eq!(
        (
            GRASP_TORQUE_MIN,
            GRASP_TORQUE_MAX,
            GRASP_TORQUE_HIGH_THRESHOLD
        ),
        (0.0, 1.0, 0.5)
    );
    for profile in ["vega_1_gripper", "vega_1p_gripper", "vega_1u_gripper"] {
        let config = dexbot_model::RobotConfig::from_profile(profile)
            .unwrap()
            .resolve()
            .unwrap();
        for hand in ["left_hand", "right_hand"] {
            let torque = config.components[hand].metadata["grasp_torque"]
                .as_f64()
                .unwrap();
            assert_eq!(torque, 0.2);
            assert!((GRASP_TORQUE_MIN..=GRASP_TORQUE_HIGH_THRESHOLD).contains(&torque));
        }
    }
}

#[test]
fn dependency_depth_boundary_is_independent_of_name_order() {
    for backwards in [false, true] {
        for size in [64, 65] {
            let mut text = String::from("schema_version: 1\nrobot: {model: test}\ncomponents:\n");
            for index in 0..size {
                let name = if backwards { size - 1 - index } else { index };
                text.push_str(&format!("  node_{name:03}:\n    driver: vendor.test\n    endpoints:\n      state: {{kind: topic, value: state/{name}}}\n"));
                if index + 1 < size {
                    let next = if backwards { name - 1 } else { name + 1 };
                    text.push_str(&format!("    dependencies: [node_{next:03}]\n"));
                }
            }
            let result = RobotConfig::from_yaml("boundary", &text).unwrap().resolve();
            if size == 64 {
                result.unwrap();
            } else {
                assert!(result.unwrap_err().to_string().contains("supported depth"));
            }
        }
    }
}
