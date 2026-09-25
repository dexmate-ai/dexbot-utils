//! Canonical robot profiles, configuration resolution, and URDF metadata.
//!
//! This crate has no networking, async-runtime, or Python dependency.

mod assets;
mod catalog;
mod error;
pub mod gripper;
mod joints;
mod merge;
mod migrate;
mod model;
mod types;
mod urdf;
mod validate;
pub mod vega;

pub use assets::{ASSET_ROOT_ENV, URDF_PACKAGE_NAME};
pub use error::{ModelError, Result};
pub use joints::JOINT_SOURCE_URDF;
pub use migrate::migrate_profile;
pub use model::{
    apply_discovery, apply_overlay, available_profiles, profile_for_robot_name,
    try_profile_for_robot_name, RobotConfig,
};
pub use types::*;
pub use urdf::*;
pub use validate::{DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE, DEFAULT_LOW_BATTERY_PERCENTAGE};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_embedded_profiles_resolve() {
        for name in available_profiles() {
            let config = RobotConfig::from_profile(name).unwrap().resolve().unwrap();
            assert_eq!(config.profile_name, *name);
            assert_eq!(config.resolution_stage, ResolutionStage::Static);
            assert_eq!(config.content_hash.len(), 64);
            assert!(!config.components.is_empty());
        }
    }

    #[test]
    fn api_overlay_enables_sensor_without_mutating_static_value() {
        let base = RobotConfig::from_profile("vega_1")
            .unwrap()
            .resolve()
            .unwrap();
        assert!(!base.sensor("head_camera").unwrap().enabled);
        let enabled = RobotConfig::from_profile("vega_1")
            .unwrap()
            .with_sensor_enabled("head_camera")
            .resolve()
            .unwrap();
        assert!(enabled.sensor("head_camera").unwrap().enabled);
        assert_ne!(base.content_hash, enabled.content_hash);
    }

    #[test]
    fn discovery_produces_operational_snapshot() {
        let static_config = RobotConfig::from_profile("vega_1")
            .unwrap()
            .resolve()
            .unwrap();
        let facts = DiscoveryFacts {
            source: "test-probe".into(),
            overlay: RobotOverlay::from_yaml("sensors:\n  head_camera:\n    enabled: true\n")
                .unwrap(),
        };
        let operational = apply_discovery(&static_config, facts).unwrap();
        assert_eq!(operational.resolution_stage, ResolutionStage::Operational);
        assert!(operational.sensor("head_camera").unwrap().enabled);
        assert_ne!(static_config.content_hash, operational.content_hash);
    }

    #[test]
    fn null_is_data_and_delete_is_explicit() {
        let config = RobotConfig::from_profile("vega_1")
            .unwrap()
            .with_overlay_yaml("null", "robot:\n  namespace: null\n")
            .unwrap()
            .resolve()
            .unwrap();
        assert_eq!(config.robot.namespace, None);
        let config = RobotConfig::from_profile("vega_1")
            .unwrap()
            .with_overlay_yaml("remove", "sensors:\n  ultrasonic:\n    $delete: true\n")
            .unwrap()
            .resolve()
            .unwrap();
        assert!(!config.sensors.contains_key("ultrasonic"));
    }

    #[test]
    fn profile_for_robot_name_implements_the_legacy_rule_set() {
        // Rule 1: exact profile names pass through unchanged, including the
        // hand-specific variants a robot name can never derive.
        for profile in available_profiles() {
            assert_eq!(profile_for_robot_name(profile), *profile);
        }
        // Rule 2: serial-number names select the variant by version suffix
        // (legacy RobotInfo._derive_variant_from_robot_name examples),
        // matched case-insensitively and without validating the serial.
        assert_eq!(profile_for_robot_name("dm/vg0123456789-1u"), "vega_1u");
        assert_eq!(profile_for_robot_name("dm/vg0123456789-1p"), "vega_1p");
        assert_eq!(profile_for_robot_name("DM/VGABCD123456-1U"), "vega_1u");
        assert_eq!(profile_for_robot_name("dm/xyz-1u"), "vega_1u");
        // Rule 3: base names and unknown input fall back to vega_1.
        assert_eq!(profile_for_robot_name("dm/vgabcd123456-1"), "vega_1");
        assert_eq!(profile_for_robot_name("dm/vgabcd123456-2"), "vega_1");
        assert_eq!(profile_for_robot_name("robot"), "vega_1");
        assert_eq!(profile_for_robot_name(""), "vega_1");
    }

    #[test]
    fn parses_urdf_metadata() {
        let model = UrdfModel::parse(
            r#"<robot name="sample">
          <link name="base"/><link name="tip"/>
          <joint name="axis" type="revolute"><parent link="base"/><child link="tip"/>
          <axis xyz="0 0 1"/><limit lower="-1" upper="1" effort="4" velocity="2"/></joint>
          <joint name="floating" type="floating"><parent link="base"/><child link="tip"/></joint>
          <joint name="planar" type="planar"><parent link="base"/><child link="tip"/></joint>
        </robot>"#,
        )
        .unwrap();
        assert_eq!(model.robot_name, "sample");
        assert_eq!(
            model.movable_joint_names().collect::<Vec<_>>(),
            vec!["axis"]
        );
        assert_eq!(
            model
                .joint("axis")
                .unwrap()
                .limit
                .as_ref()
                .unwrap()
                .velocity,
            Some(2.0)
        );
        assert_eq!(model.joint("axis").unwrap().axis, Some([0.0, 0.0, 1.0]));
    }
}
