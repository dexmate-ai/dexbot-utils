//! Capability parsing, joint-name lookup, and the content-hash contract.
//!
//! The content hash is what lets two machines agree they resolved the same
//! robot, and the capability list is what gates every driver feature -- both
//! places where a silently-wrong answer costs a debugging day, so their edge
//! cases are pinned here rather than assumed.

use dexbot_model::{CapabilitySpec, RobotConfig};

fn resolved() -> dexbot_model::ResolvedRobotConfig {
    RobotConfig::from_profile("vega_1")
        .unwrap()
        .resolve()
        .unwrap()
}

// ------------------------------------------------------------- capabilities

#[test]
fn capabilities_parse_in_both_documented_spellings() {
    // Short form: a bare id, required by default.
    let short: CapabilitySpec = serde_yaml::from_str("joint_state").unwrap();
    assert_eq!((short.id.as_str(), short.required), ("joint_state", true));

    // Full form, with and without the optional flag.
    let full: CapabilitySpec = serde_yaml::from_str("{id: imu, required: false}").unwrap();
    assert_eq!((full.id.as_str(), full.required), ("imu", false));
    let defaulted: CapabilitySpec = serde_yaml::from_str("{id: imu}").unwrap();
    assert!(
        defaulted.required,
        "required defaults to true in both forms"
    );
}

#[test]
fn malformed_capabilities_are_rejected_with_the_reason() {
    // Each of these once deserialized would silently gate features on or
    // off; the deserializer names the exact problem instead.
    for (source, needle) in [
        ("\"\"", "cannot be empty"),
        ("{id: \"\"}", "non-empty string"),
        ("{id: 7}", "non-empty string"),
        ("{required: true}", "needs an id"),
        ("{id: imu, required: maybe}", "must be a boolean"),
        ("{id: imu, requierd: true}", "unknown field"),
    ] {
        let error = serde_yaml::from_str::<CapabilitySpec>(source).unwrap_err();
        assert!(error.to_string().contains(needle), "{source}: {error}");
    }
}

// -------------------------------------------------------------- joint names

#[test]
fn joint_names_prefer_urdf_metadata_and_fall_back_to_the_declaration() {
    let mut config = resolved();

    // The arm resolves through the URDF, so metadata is the source of truth
    // and the two views must agree in content and order.
    let from_lookup = config.joint_names("left_arm").unwrap();
    let from_metadata: Vec<String> = config
        .joint_metadata("left_arm")
        .unwrap()
        .iter()
        .map(|joint| joint.name.clone())
        .collect();
    assert_eq!(from_lookup, from_metadata);
    assert_eq!(from_lookup.len(), 7);

    config.joint_metadata.remove("left_arm");
    assert_eq!(config.joint_names("left_arm").unwrap(), from_metadata);

    // An unknown object is None, not an empty list: "no such component" and
    // "component with no joints" are different answers.
    assert!(config.joint_names("no_such_object").is_none());
    // The battery exists but declares no joints.
    assert!(config.joint_names("battery").is_none());
}

// ------------------------------------------------------------- content hash

#[test]
fn the_stored_hash_matches_the_recomputed_one() {
    let config = resolved();
    assert!(config.verify_content_hash().unwrap());
    assert_eq!(config.content_hash, config.compute_content_hash().unwrap());
}

#[test]
fn the_hash_covers_semantics_and_ignores_provenance() {
    let mut config = resolved();

    // Provenance records where values came from on this machine; two
    // machines resolving identical content must hash identically, so it is
    // excluded.
    config.provenance.clear();
    assert!(config.verify_content_hash().unwrap());

    // A semantic change, however small, must break the stored hash.
    config.robot.namespace = Some("dm/other".into());
    assert!(!config.verify_content_hash().unwrap());
    // And recomputing after the change yields a different digest.
    assert_ne!(config.compute_content_hash().unwrap(), config.content_hash);
}

/// The profile name of a file-based profile is its file stem. Hashing it
/// made a byte-identical copy under another filename a "different" config.
#[test]
fn the_hash_ignores_the_profile_name() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().to_path_buf();
    std::fs::create_dir_all(&directory).unwrap();
    let source =
        "schema_version: 1\nextends: [common/vega_upper_body.yaml]\nrobot:\n  model: vega_1u\n";
    let mut hashes = Vec::new();
    for stem in ["site_a", "backup-of-site_a"] {
        let path = directory.join(format!("{stem}.yaml"));
        std::fs::write(&path, source).unwrap();
        let config = dexbot_model::RobotConfig::from_file(&path)
            .unwrap()
            .resolve()
            .unwrap();
        assert_eq!(config.profile_name, stem);
        assert!(config.verify_content_hash().unwrap());
        hashes.push(config.content_hash);
    }

    assert_eq!(hashes[0], hashes[1]);

    let mut config = resolved();
    config.profile_name = "renamed".into();
    assert!(config.verify_content_hash().unwrap());
}

#[test]
fn normalized_json_is_stable_and_newline_terminated() {
    let config = resolved();
    let first = config.normalized_json().unwrap();
    assert!(
        first.ends_with('\n'),
        "tools diff these files; no-EOF-newline churn"
    );
    assert_eq!(first, config.normalized_json().unwrap(), "deterministic");
    // Round-trips to an equal config.
    let reparsed: dexbot_model::ResolvedRobotConfig = serde_json::from_str(&first).unwrap();
    assert_eq!(reparsed, config);
}
