use dexbot_utils::RobotConfig;

fn load_profile(name: &str) -> dexbot_utils::Result<RobotConfig> {
    RobotConfig::from_profile(name)
}

#[test]
fn mobile_profiles_declare_optional_chassis_cameras_and_platform_imus() {
    for body in ["vega_1", "vega_1p", "vega_1u"] {
        for suffix in ["", "_gripper", "_f5d6"] {
            let name = format!("{body}{suffix}");
            let config = load_profile(&name).unwrap().resolve().unwrap();
            assert!(!config.sensors.contains_key("head_imu"));
            let head = &config.sensors["head_camera"];
            assert!(!head.enabled && !head.required);
            assert!(head
                .capabilities
                .iter()
                .any(|cap| cap.id == "camera_stream"));
            assert!(head.capabilities.iter().any(|cap| cap.id == "imu"));
            assert_eq!(head.endpoints["imu_topic"].value, "sensors/head_camera/imu");
            for side in ["front", "back", "left", "right"] {
                let sensor_name = format!("base_{side}_camera");
                if body == "vega_1u" {
                    assert!(!config.sensors.contains_key(&sensor_name));
                    continue;
                }
                let sensor = &config.sensors[&sensor_name];
                assert!(!sensor.enabled && !sensor.required);
                assert_eq!(sensor.driver, "standard.usb_camera");
                assert_eq!(
                    sensor.endpoints["rgb_topic"].value,
                    format!("sensors/{sensor_name}/rgb")
                );
                assert_eq!(
                    sensor.endpoints["rgb_rtc_channel"].value,
                    format!("sensors/{sensor_name}/rgb_rtc")
                );
                let enabled = load_profile(&name)
                    .unwrap()
                    .with_sensor_enabled(&sensor_name)
                    .resolve()
                    .unwrap();
                assert!(enabled.sensors[&sensor_name].enabled);
            }
            for side in ["front", "back"] {
                let sensor_name = format!("lidar_3d_{side}");
                assert!(!config.sensors.contains_key(&format!("{sensor_name}_imu")));
                if body != "vega_1p" {
                    assert!(!config.sensors.contains_key(&sensor_name));
                    continue;
                }
                let sensor = &config.sensors[&sensor_name];
                assert!(!sensor.enabled && !sensor.required);
                assert_eq!(sensor.driver, "standard.lidar_3d");
                assert!(sensor.capabilities.iter().any(|cap| cap.id == "imu"));
                assert!(sensor.capabilities.iter().any(|cap| cap.id == "lidar_3d"));
                assert_eq!(
                    sensor.endpoints["topic"].value,
                    format!("sensors/{sensor_name}/point_cloud")
                );
                assert_eq!(
                    sensor.endpoints["imu_topic"].value,
                    format!("sensors/{sensor_name}/imu")
                );
                assert!(
                    load_profile(&name)
                        .unwrap()
                        .with_sensor_enabled(&sensor_name)
                        .resolve()
                        .unwrap()
                        .sensors[&sensor_name]
                        .enabled
                );
            }
            assert!(config.verify_content_hash().unwrap());
        }
    }
}
