# Changelog

All notable changes to this project will be documented in this file.

## [0.5.1] - 2026-07-19

### Added
- `DexSGripperConfig.grasp_torque`: normalized grip-force limit in `[0, 1]` (default `0.2`) applied on every gripper position command while the gripper runs in `"pvt"` (position-velocity-torque) mode. Lets grip force be tuned per robot/gripper instead of being hardcoded on the client side. The value is validated to the `[0, 1]` range at construction (`DexDGripperConfig` inherits the check).

## [0.5.0] - 2026-06-01

### Added
- Temperature subscription topics for Vega-1 components: `temperature_sub_topic` on arm, head, and torso configs, plus `steer_temperature_sub_topic` and `drive_temperature_sub_topic` on the chassis config.
- `idle_mode_query` topic to the torso config for the torso auto-idle service.

### Fixed
- 2D LiDAR and gripper configuration bugs, including the front 2D LiDAR scan topic and wrist camera setup.
- Sensor configuration bugs across Vega-1, Vega-1P, and Vega-1U robot variants.

## [0.4.4] - 2026-03-05

### Added
- Monitoring field to `EStopConfig` for e-stop state monitoring.

## [0.4.3] - 2026-02-16

### Added
- `RobotInfo(configs=...)` parameter to initialize from a pre-built `BaseRobotConfig` instance.
- `RobotInfo.get_default_config()` static method to retrieve and modify registry configs.
- `RobotInfo._resolve_variant_from_env()` refactored to `@staticmethod`.
- `BaseRobotConfig.has_sensor()` method to check sensor availability.
- `BaseRobotConfig.enable_sensor()` method to enable sensors by name with validation and error reporting.

### Changed
- `enable_ee_pass_through` default changed from `False` to `True` in `Vega1ArmConfig`.
- Runtime configuration modifier now auto-detects hand type and injects the appropriate hand config when not explicitly provided.

### Fixed
- Warning message for hand detection failure now correctly refers to end-effector detection.
- Removed `chassis_imu` and `ultrasonic` sensors from `Vega1pConfig` and `Vega1pDGripperConfig` — these sensors are not present on Vega-1P hardware.

## [0.4.2] - 2026-02-15

### Added
- Force torque sensor mode query name (`force_torque_sensor_query`) to `Vega1ArmConfig`.

## [0.4.1] - 2026-02-06

### Added
- Arm PID configuration query name (`pid_query`) to `Vega1ArmConfig`.
- Arm brake control query name (`brake_query`) to `Vega1ArmConfig`.
- End-effector baud rate query name (`ee_baud_rate_query`) to `Vega1ArmConfig`.
- End-effector pass-through state subscription topic (`ee_pass_through_state_sub_topic`) to `Vega1ArmConfig`.
- Gripper mode query name (`set_mode_query`) to `DexSGripperConfig`.

## [0.4.0] - 2026-01-20

### Added
- Unified support for Vega-1, Vega-1P (Pro), and Vega-1U (Upper body) robot variants.
- Dex-gripper (single and double) end effector configurations.
- Component-level topic and query name properties for all actuated components (arm, hand, head, torso, chassis).
- Sensor configurations for cameras (ZedX, ZedXOne, USB), IMU, LiDAR, and ultrasonic sensors.
- Robot variant registry with decorator-based auto-registration.
- `RobotInfo` high-level API with lazy URDF loading and component access methods.
- CLI tool (`dexbot`) for listing and inspecting robot configurations.
- Component validators for runtime configuration checks.
- Configuration modifier utilities.
- Arm wrench and wrist button subscription topics.
- End-effector pass-through control topic.
- Arm pose pools with side-aware mirroring (folded, L_shape, lift_up, zero).

### Changed
- Consolidated robot configs from per-component files into 3 robot variant files (vega_1, vega_1p, vega_1u).
- Aligned with `dexmate-urdf` package reorganization.
- Bumped version to 0.4.0 for consistency with dexcontrol release.

### Fixed
- Close hand pose values.
- Gripper naming convention (renamed from dgripper).

### Dependencies
- Requires `dexmate-urdf` for URDF models.
- Requires `numpy`, `loguru`, `typer`.
