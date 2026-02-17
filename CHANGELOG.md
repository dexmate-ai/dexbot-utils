# Changelog

All notable changes to this project will be documented in this file.

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
