# dexbot-utils

Robot model utilities for Dexmate robots. See https://github.com/dexmate-ai/dexbot-utils for installation, examples, and API documentation.

## Sensor capability lookup

A device can declare several data types. `head_camera` has `camera_stream` and
`imu` capabilities; Vega-1p LiDARs have `lidar_3d` and `imu`. The head IMU uses
`sensors/head_camera/imu` under the `imu_topic` endpoint role. There is no separate
`head_imu` entry. See the [profile contracts](assets/profiles/README.md).

```rust
use dexbot_utils::RobotConfig;

let config = RobotConfig::from_profile("vega_1p")?
    .with_sensor_enabled("head_camera")
    .resolve()?;
let camera = &config.sensors["head_camera"];
assert!(camera.capabilities.iter().any(|cap| cap.id == "imu"));
assert_eq!(camera.endpoints["imu_topic"].value, "sensors/head_camera/imu");
# Ok::<(), dexbot_utils::ModelError>(())
```

These APIs resolve declarations; sensor data is obtained through DexControl.

## Vega model selection after discovery

Use `vega::apply_discovery` when the transport has queried hand hardware and
wrist force/torque sensor presence. Supply hand injection, replacement, and
disable decisions in the discovery overlay; the selected model follows the
resulting components, including user decisions to retain configured hands.
Existing `apply_discovery` callers keep their current behavior.

```rust
use dexbot_utils::{vega, DiscoveryFacts, RobotConfig, RobotOverlay};

let config = RobotConfig::from_profile("vega_1p_gripper")?.resolve()?;
let report = vega::HardwareReport {
    reported_hands: ["left".into(), "right".into()].into(),
    wrist_ft: [("left".into(), Some(false)), ("right".into(), Some(false))].into(),
};
let facts = DiscoveryFacts {
    source: "robot-query".into(),
    overlay: RobotOverlay::from_yaml("{}")?,
};
let (operational, selection) = vega::apply_discovery(&config, facts, &report)?;
// operational uses vega_1p_no_ft_gripper.urdf.
// Display ModelSelection::Unchanged.reason when selection cannot be made.
# Ok::<(), dexbot_utils::ModelError>(())
```

The values above illustrate a complete report; applications must use actual
query results. Missing reports, unknown wrist status, asymmetric hardware, and
unsupported bodies retain the configured URDF. Configuration validation and
user-overlay conflict checks still apply. This library performs no queries.
`vega::select_model` also exposes selection without modifying a configuration.

## Command-line tool

The same crate provides the `dexbot` CLI by default (Rust 1.85 or newer):

```bash
cargo install dexbot-utils --locked
dexbot list
dexbot show vega_1p
```

No feature flags are required. Existing library imports and CLI commands are unchanged.
