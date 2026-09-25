# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.2.3] - 2026-09-24

- Vendor all nine FT URDFs from dexmate-urdf 0.9.0 (main at `d5636b5`),
  matching the no-FT variants. The 0.8.0-based files made hardware selection
  move the tool frame by ~613 mm on Vega-1u (0.9.0 adds the `Lift` and
  `torso_flip` joints and re-roots the arms) and by 25.7 mm instead of 30.2 mm
  on grippers (0.9.0 gripper geometry). A new test requires each FT/no-FT pair
  to differ only by the 30.2 mm wrist flange.

- Fix C++ `Value` move ownership so moved-from views cannot retain dangling
  pointers; reserve collection storage and avoid repeated FFI size queries.
- Reject duplicate URDF attributes and empty mimic targets.
- Resolve discovery through one shared serialization and merge path, preserving
  user-intent conflict checks and model-specific selection.
- Consolidate release documentation and make local verification independent of
  the working directory, using locked dependencies and tooling regression tests.

- Merge PR #27 into the Rust SDK: add Vega hardware-aware model selection,
  preserving effective hand choices and requiring complete, agreeing wrist
  reports. Bundle the six upstream no-FT variants and expose diagnostic fallback
  reasons without restoring the removed Python package.

- Backport Yuzhe's upstream back-lidar and left-middle-finger inertial data
  across the affected Vega variants. Match his gripper limits and fully open
  pose at 0.96 rad for both hands; regenerate the resolved contracts.

- Reject profile resource traversal and symlink escapes; filesystem URDFs now
  require an explicit trusted asset root. Reject malformed URDF roots and names.
- Enforce dependency depth independently of component ordering and tighten
  safety schemas and regression coverage.
- Preserve fixtures on generation failure and build the generator only once.
- Include the upstream URDF license in source, SDK, and CLI packages.

- Preserve the legacy gripper torque bounds (0–1) and high-torque warning
  threshold (0.5) as public Rust constants when merging the Python 0.5.2 change.

- Audit public source snapshots and installed SDK file lists before export;
  reject unexpected files, private dependencies, symlinks and credential patterns.
- Pin release jobs to one source commit, guard concurrent staging updates, and
  require complete checksum coverage before uploading release assets.

- Add a C ABI and C++17 model SDK backed by the Rust model library, with typed
  components, joint limits, stored poses, overlays, metadata and JSON access.
- Install a relocatable CMake package exposing `dexbot::model`; test every
  built-in profile against the standalone Rust CLI after relocation.
- Build Linux/macOS SDK and CLI release archives with SHA-256 checksums; publish
  Rust model and CLI crates in dependency order after a public release is approved.
- Validate workspace, dependency, CMake and tag versions; require the public
  workflow to be installed before the existing source sync creates release tags.

## [0.2.2] - 2026-09-23

- Authenticate the private remote-tag check before attaching attested SDK builds.

## [0.2.1] - 2026-09-23

- Build and attest native SDKs in private CI and publish inspected Rust crates
  only from the private repository.
- Upload verified SDK downloads to the public preview branch release.

## [0.2.0] - 2026-09-22

Minor bump: stricter validation rejects profiles and overlays that 0.1
accepted, `apply_discovery` is renamed, and stored `content_hash` values
change. Consumers pin `dexbot-model = "0.2"`.

The 0.2.x entries describe the Rust SDK; the older 0.5.x entries below
describe the retired Python package and use an independent version series.

### Changed

- **Validation is stricter; profiles and overlays that used to load may now
  be rejected.** Heartbeat `safety.timeout_seconds` must be in [0.05, 10] s,
  E-stop `timeout_seconds` in [0.001, 1] s, `state_max_age_seconds` in
  [0.1, 60] s, `runtime.state_idle_timeout_ms` at least 100,
  `low_battery_percentage` in (0, 95] and `battery_hysteresis_percentage` in
  [1, 50]. Per-component safety flags are scoped to the estop, heartbeat or
  battery role that consumes them. `safety.timeout_seconds` and
  `metadata.timeout_seconds` must agree. The metadata keys the runtime reads
  (`state_max_age_ms`, `state_idle_timeout_ms`, `default_control_hz`, the
  chassis limits and geometry) are range-checked, `state_max_age_ms: null`
  is rejected (`false` is the opt-out), and near-miss spellings of those keys
  are rejected as typos. Pose targets must lie within their joint limits, a
  joint belongs to one enabled component, enabled components cannot share a
  publisher topic, and endpoint values and `robot.namespace` cannot contain
  wildcards, whitespace, or leading/trailing/doubled `/`.
- URDF: revolute and prismatic joints need finite `lower` and `upper` limits
  (only `continuous` joints may omit them); `nan`/`inf` attributes, duplicate
  joint or link names, and mimic joints listed as component joints are
  rejected. `<joint>`/`<link>` elements nested in `<transmission>` or
  `<gazebo>` are no longer read as robot joints, and `<mimic>` is parsed
  (`UrdfJoint::mimic`) and excluded from `movable_joint_names()`.
- `apply_overlay` records an `overlay:<name>` layer and keeps the
  configuration static; it used to be `apply_discovery` under another name.
  `extends` is rejected in every overlay and discovery layer, even when empty.
- `content_hash` no longer covers `profile_name`, so a profile copied to
  another filename hashes identically. All stored hashes change.
- On-disk `extends` fragments are recorded in provenance as
  `extends:file:<name>`; embedded fragments keep `extends:<name>`.
- YAML merge keys (`<<: *anchor`) are applied instead of being stored as a
  literal `<<` key in free-form maps.
- `dexbot profile-for` exits 1 on a name that selects no built-in profile.
- `dexbot migrate` validates its output before printing it.

### Added

- `try_profile_for_robot_name`, which fails with
  `ModelError::UnknownRobotName` on typos, unknown model tokens or versions,
  and token/suffix disagreement instead of selecting `vega_1`. It trims and
  matches profile names case-insensitively. `profile_for_robot_name` remains
  as the legacy total form.
- `DEFAULT_LOW_BATTERY_PERCENTAGE` and `DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE`.
- `RobotConfig::from_yaml_in` for in-memory documents whose fragments live
  in a directory.
- `dexbot --version`, and `dexbot diff --exit-code`.
- `tools/regenerate-fixtures.sh`.
- `pose_pool` validation: every pose must be an array of finite numbers with
  one entry per joint; violations are reported by the CLI `validate` command
  and at resolve time instead of on the first `predefined_pose` call.

### Removed

- Python seed artifacts from `common/vega_upper_body.yaml`: the head
  camera's `depth_rtc_channel` endpoint (depth is Zenoh-only; the value
  pointed at an RGB channel) and the `CameraConfig(...)` repr strings in
  camera metadata.
- **The `dexbot-utils` Python package** (`dexbot_utils`, the `dexbot-python`
  PyO3 crate, `pyproject.toml`, and the Python test suite). Runtime consumers
  get model data through dexcontrol, which embeds `dexbot-model`:
  `dexcontrol.robot_config()` / `dexcontrol.available_profiles()` in Python,
  `dex_resolved_config_json` / `dex_available_profiles_json` /
  `dex_profile_for_robot_name` over the C ABI, and the crate itself in Rust.
  Offline profile work (`list`, `show`, `validate`, `diff`, `migrate`, `urdf`,
  `profile-for`) is the `dexbot` CLI binary from `crates/dexbot-cli`. The
  pure-Python layer (`RobotInfo`, dataclass configs, `urdf_utils` over the
  `dexmate-urdf` pip package, validators, the config modifier) had drifted
  from the embedded model: it read URDF limits from whatever pip version was
  installed rather than the 0.8.0 the robot runs against. Known consumer to
  migrate: omniteleop's `RobotInfo` use (`has_torso`, `has_base`,
  `has_component`, joint names), all available from `dexcontrol.robot_config`.

### Fixed

- Identity-v2 robot names (`dm/vg1p-<serial>` and `dm-vg1p-<serial>`, including
  case variants) now select the same profiles in the model library, CLI,
  and DexControl.
- The README describes supported custom-profile composition from plain
  fragments and shows overlays for customizing a complete built-in profile;
  complete profiles cannot be nested through `extends`.

## [0.5.2] - 2026-08-05

### Added

- Grasp-torque bounds as class attributes on `DexSGripperConfig`: `GRASP_TORQUE_MIN`, `GRASP_TORQUE_MAX`, and `GRASP_TORQUE_HIGH_THRESHOLD` (`DexDGripperConfig` inherits them). They are `ClassVar`s, so they describe the gripper without becoming constructor arguments, and `__post_init__` validates `grasp_torque` against them. Clients read the safe range off the gripper config they already hold instead of duplicating the numbers or importing separate module constants.

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
