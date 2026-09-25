use crate::joints::JOINT_SOURCE_URDF;
use crate::types::ResolvedJoint;
use crate::{ComponentConfig, ModelError, ProfileDocument, Result, SOURCE_SCHEMA_VERSION};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const BUILT_INS: &[&str] = &[
    "joint_state",
    "joint_position_command",
    "joint_velocity_command",
    "managed_joint_motion",
    "trajectory_execution",
    "cartesian_velocity_2d",
    "gripper",
    "mode_control",
    "temperature",
    "force_torque",
    "camera_stream",
    "imu",
    "lidar_2d",
    "lidar_3d",
    "ultrasonic",
    "emergency_stop",
    "heartbeat",
    "battery",
];

/// Capabilities that only read joint state. An object holding nothing else
/// cannot command its joints.
const OBSERVER_CAPABILITIES: &[&str] = &["joint_state", "temperature", "force_torque"];

/// Safety-failure actions accepted for the top-level safety flags.
const SAFETY_ACTIONS: &[&str] = &[
    "stop_motion",
    "shutdown_robot",
    "activate_software_estop",
    "request_process_termination",
    "none",
];

/// Low-battery warning threshold (percent) used when a battery component
/// does not set `safety.low_battery_percentage`.
pub const DEFAULT_LOW_BATTERY_PERCENTAGE: f64 = 20.0;
/// Percentage the charge must recover above the threshold before the
/// low-battery warning clears, when `safety.battery_hysteresis_percentage`
/// is not set.
pub const DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE: f64 = 5.0;

/// Heartbeat dead-man timeout bounds in seconds. The floor keeps ordinary
/// jitter from tripping the failure action; the ceiling keeps a typo from
/// disabling the dead-man (or overflowing a `Duration` downstream).
const HEARTBEAT_TIMEOUT_SECONDS: (f64, f64) = (0.05, 10.0);
/// E-stop poll interval bounds in seconds; the range DexControl honours.
const ESTOP_POLL_SECONDS: (f64, f64) = (0.001, 1.0);
/// E-stop sample freshness bounds in seconds.
const STATE_MAX_AGE_SECONDS: (f64, f64) = (0.1, 60.0);
/// Shortest startup-verification window. Zero would make every component
/// fail (or trivially pass) its startup check.
const MIN_STATE_IDLE_TIMEOUT_MS: u64 = 100;

/// Which per-component safety flags each safety role consumes. A component
/// holds a role through its standard driver or through the capability the
/// runtime keys its monitor on. A flag outside the component's roles would
/// validate and then be ignored, which reads as protection that is not there.
struct SafetyRole {
    driver: &'static str,
    capability: &'static str,
    flags: &'static [&'static str],
}

const SAFETY_ROLES: &[SafetyRole] = &[
    SafetyRole {
        driver: "standard.estop",
        capability: "emergency_stop",
        flags: &[
            "monitoring",
            "estop_query_name",
            "timeout_seconds",
            "state_max_age_seconds",
            "estop_state_periodic",
        ],
    },
    SafetyRole {
        driver: "standard.heartbeat",
        capability: "heartbeat",
        flags: &["monitoring", "heartbeat_topic", "timeout_seconds"],
    },
    SafetyRole {
        driver: "standard.battery",
        capability: "battery",
        flags: &[
            "monitoring",
            "low_battery_percentage",
            "battery_hysteresis_percentage",
        ],
    },
];

/// Characters that are wildcards or reserved in Zenoh key expressions. An
/// endpoint or namespace containing one subscribes to, or publishes on,
/// more than the one key it appears to name.
const KEY_EXPRESSION_RESERVED: &[char] = &['*', '$', '?', '#'];

/// Endpoint kinds understood by DexControl drivers and transports. This is
/// the de-facto vocabulary used by every built-in profile.
const ENDPOINT_KINDS: &[&str] = &[
    "topic",
    "subscriber_topic",
    "publisher_topic",
    "service",
    "channel",
];

/// Readiness policies understood by the DexControl runtime.
const READINESS_POLICIES: &[&str] = &["required_components"];

/// Longest supported component dependency chain. Anything deeper is treated
/// as a configuration error before recursion can exhaust the stack.
const MAX_DEPENDENCY_DEPTH: usize = 64;

/// Slack for pose targets that sit exactly on a joint limit after decimal
/// round-tripping (rad or m).
const POSE_LIMIT_EPSILON: f64 = 1e-6;

pub fn validate(document: &ProfileDocument) -> Result<()> {
    if document.schema_version != SOURCE_SCHEMA_VERSION {
        return invalid(format!(
            "unsupported schema_version {}; expected {}",
            document.schema_version, SOURCE_SCHEMA_VERSION
        ));
    }
    if document.robot.model.trim().is_empty() {
        return invalid("robot.model cannot be empty");
    }
    if !READINESS_POLICIES.contains(&document.runtime.readiness.as_str()) {
        return invalid(format!(
            "runtime.readiness must be one of {READINESS_POLICIES:?}, got {:?}",
            document.runtime.readiness
        ));
    }
    if document.runtime.state_idle_timeout_ms < MIN_STATE_IDLE_TIMEOUT_MS {
        return invalid(format!(
            "runtime.state_idle_timeout_ms must be at least {MIN_STATE_IDLE_TIMEOUT_MS}, got {}",
            document.runtime.state_idle_timeout_ms
        ));
    }
    if let Some(namespace) = &document.robot.namespace {
        if let Some(problem) = key_expression_problem(namespace) {
            return invalid(format!("robot.namespace {namespace:?} {problem}"));
        }
    }
    validate_joint_groups(document)?;
    validate_top_level_safety(&document.safety)?;
    let mut all: BTreeMap<&str, &ComponentConfig> = BTreeMap::new();
    for (name, component) in document.components.iter().chain(document.sensors.iter()) {
        if all.insert(name, component).is_some() {
            return invalid(format!("duplicate runtime object {name:?}"));
        }
        validate_component(name, component)?;
    }
    validate_publisher_topics(&all)?;
    for (name, component) in &all {
        for dependency in &component.dependencies {
            if !all.contains_key(dependency.as_str()) {
                return invalid(format!("{name:?} depends on unknown object {dependency:?}"));
            }
        }
    }
    detect_cycles(&all)
}

fn validate_joint_groups(document: &ProfileDocument) -> Result<()> {
    for (group, names) in &document.joint_groups {
        if group.trim().is_empty() {
            return invalid("joint_groups contains an empty group name");
        }
        if names.is_empty() {
            return invalid(format!("joint group {group:?} lists no joints"));
        }
        let mut seen = BTreeSet::new();
        for name in names {
            if name.trim().is_empty() || !seen.insert(name) {
                return invalid(format!(
                    "joint group {group:?} has an empty or duplicate joint name"
                ));
            }
        }
    }
    Ok(())
}

fn validate_top_level_safety(safety: &BTreeMap<String, Value>) -> Result<()> {
    for (flag, value) in safety {
        match flag.as_str() {
            // `estop_unreadable_action` covers the *dead E-stop stream* case
            // separately from an observed engagement: a stream that stops
            // delivering can never produce a button edge, so the client
            // escalates to this action once the channel is provably dead.
            // Absent, it follows `estop_failure_action`.
            "estop_failure_action" | "heartbeat_failure_action" | "estop_unreadable_action" => {
                match value.as_str() {
                    Some(action) if SAFETY_ACTIONS.contains(&action) => {}
                    _ => {
                        return invalid(format!(
                            "safety flag {flag:?} must be one of {SAFETY_ACTIONS:?}, got {value}"
                        ))
                    }
                }
            }
            _ => return invalid(format!("unknown top-level safety flag {flag:?}")),
        }
    }
    Ok(())
}

fn validate_component(name: &str, component: &ComponentConfig) -> Result<()> {
    if name.trim().is_empty() || component.driver.trim().is_empty() {
        return invalid(format!(
            "runtime object {name:?} has an empty name or driver"
        ));
    }
    if component.required && !component.enabled {
        return invalid(format!(
            "required runtime object {name:?} cannot be disabled"
        ));
    }
    if component.enabled && component.endpoints.is_empty() {
        if !component.local {
            return invalid(format!(
                "enabled runtime object {name:?} has no endpoints and is not local"
            ));
        }
        if component.dependencies.is_empty() {
            return invalid(format!(
                "local runtime object {name:?} has no endpoints and must declare \
                 dependencies from which its driver derives state or commands"
            ));
        }
    }
    for (role, endpoint) in &component.endpoints {
        if role.trim().is_empty()
            || endpoint.kind.trim().is_empty()
            || endpoint.value.trim().is_empty()
        {
            return invalid(format!(
                "runtime object {name:?} has an invalid endpoint role {role:?}"
            ));
        }
        if let Some(problem) = key_expression_problem(&endpoint.value) {
            return invalid(format!(
                "runtime object {name:?} endpoint {role:?} value {:?} {problem}",
                endpoint.value
            ));
        }
        if !ENDPOINT_KINDS.contains(&endpoint.kind.as_str()) {
            return invalid(format!(
                "runtime object {name:?} endpoint {role:?} has unknown kind {:?}; \
                 the supported kinds are {ENDPOINT_KINDS:?}",
                endpoint.kind
            ));
        }
    }
    validate_component_safety(name, component)?;
    validate_metadata(name, component)?;
    validate_pose_pool(name, component, None)?;
    // Normalized grip-force limit; re-closes the legacy v0.5.1 range check
    // at the model layer.
    if let Some(value) = component.metadata.get("grasp_torque") {
        match value.as_f64() {
            Some(torque)
                if (crate::gripper::GRASP_TORQUE_MIN..=crate::gripper::GRASP_TORQUE_MAX)
                    .contains(&torque) => {}
            _ => {
                return invalid(format!(
                    "runtime object {name:?} metadata \"grasp_torque\" must be a \
                     number in [0, 1], got {value}"
                ));
            }
        }
    }
    let mut capabilities = BTreeSet::new();
    for capability in &component.capabilities {
        if !capabilities.insert(&capability.id) {
            return invalid(format!(
                "runtime object {name:?} repeats capability {:?}",
                capability.id
            ));
        }
        if !BUILT_INS.contains(&capability.id.as_str()) && !valid_extension_id(&capability.id) {
            return invalid(format!(
                "runtime object {name:?} has invalid capability id {:?}",
                capability.id
            ));
        }
    }
    if let Some(joints) = &component.joints {
        let mut names = BTreeSet::new();
        for joint in &joints.names {
            if joint.trim().is_empty() || !names.insert(joint) {
                return invalid(format!(
                    "runtime object {name:?} has an empty or duplicate joint name"
                ));
            }
        }
        match joints.source.as_deref() {
            None => {
                if joints.group.is_some() {
                    return invalid(format!(
                        "runtime object {name:?} sets a joint group without \
                         `source: {JOINT_SOURCE_URDF}`"
                    ));
                }
                if joints.names.is_empty() {
                    return invalid(format!(
                        "runtime object {name:?} joint config needs names or a URDF group"
                    ));
                }
            }
            Some(JOINT_SOURCE_URDF) => {
                if joints.group.as_deref().is_none_or(str::is_empty) {
                    return invalid(format!(
                        "runtime object {name:?} declares `source: {JOINT_SOURCE_URDF}` \
                         without a joint group"
                    ));
                }
            }
            Some(other) => {
                return invalid(format!(
                    "runtime object {name:?} has unsupported joint source {other:?}; \
                     the only supported source is {JOINT_SOURCE_URDF:?}"
                ));
            }
        }
    }
    Ok(())
}

/// Per-component safety maps accept a small canonical vocabulary. The
/// duplicated `estop_query_name`/`heartbeat_topic` values must agree with the
/// endpoint of the same role, and the legacy `monitoring_enabled` spelling is
/// rejected in favor of `monitoring`.
/// Checks pose objects (joint_pos + frame), or legacy raw joint arrays,
/// one per joint. The joint count is known at document time only when the
/// names are listed inline; `require_length` turns the length check on for
/// the post-resolution pass, after URDF joint groups have filled the names.
fn validate_pose_pool(
    name: &str,
    component: &ComponentConfig,
    resolved: Option<&[ResolvedJoint]>,
) -> Result<()> {
    let require_length = resolved.is_some();
    if component.metadata.contains_key("pose_frames") {
        return invalid(format!(
            "runtime object {name:?}: pose_frames is no longer supported; put frame next to joint_pos inside each pose_pool entry"
        ));
    }
    let Some(pool) = component.metadata.get("pose_pool") else {
        return Ok(());
    };
    let Some(poses) = pool.as_object() else {
        return invalid(format!(
            "runtime object {name:?} metadata pose_pool must be an object of named poses"
        ));
    };
    let joint_count = component
        .joints
        .as_ref()
        .map(|joints| joints.names.len())
        .filter(|count| *count > 0);
    if joint_count.is_none() && component.joints.is_none() {
        return invalid(format!(
            "runtime object {name:?} declares a pose_pool but has no joints"
        ));
    }
    for (pose, entry) in poses {
        let frame = entry
            .as_object()
            .and_then(|fields| fields.get("frame"))
            .and_then(Value::as_str)
            .unwrap_or("joint");
        let targets = if let Some(fields) = entry.as_object() {
            if fields
                .keys()
                .any(|key| !matches!(key.as_str(), "joint_pos" | "frame"))
            {
                return invalid(format!(
                    "runtime object {name:?} pose {pose:?} accepts only joint_pos and frame"
                ));
            }
            if !matches!(
                fields.get("frame").and_then(serde_json::Value::as_str),
                Some("joint" | "torso_horizontal" | "torso_upright")
            ) {
                return invalid(format!("runtime object {name:?} pose {pose:?} requires frame: joint, torso_horizontal or torso_upright"));
            }
            let Some(targets) = fields.get("joint_pos") else {
                return invalid(format!(
                    "runtime object {name:?} pose {pose:?} requires joint_pos"
                ));
            };
            targets
        } else {
            // Existing raw array poses retain joint-frame semantics.
            entry
        };
        let Some(targets) = targets.as_array() else {
            return invalid(format!(
                "runtime object {name:?} pose {pose:?} must be an array of joint \
                 targets, got {targets}"
            ));
        };
        if let Some((index, value)) = targets
            .iter()
            .enumerate()
            .find(|(_, value)| !value.as_f64().is_some_and(f64::is_finite))
        {
            return invalid(format!(
                "runtime object {name:?} pose {pose:?}[{index}] must be a finite \
                 number, got {value}"
            ));
        }
        match joint_count {
            Some(count) if targets.len() != count => {
                return invalid(format!(
                    "runtime object {name:?} pose {pose:?} has {} targets for {count} joints",
                    targets.len()
                ));
            }
            None if require_length => {
                return invalid(format!(
                    "runtime object {name:?} pose {pose:?} cannot be checked: the \
                     component resolved to no joints"
                ));
            }
            _ => {}
        }
        // Torso-referenced frames shift the first joint by the measured
        // torso pitch at run time, so only a joint-frame value of that joint
        // is final here.
        let skip = usize::from(frame != "joint");
        for (index, (target, joint)) in targets
            .iter()
            .zip(resolved.unwrap_or_default())
            .enumerate()
            .skip(skip)
        {
            let target = target.as_f64().unwrap_or_default();
            let below = joint
                .lower
                .is_some_and(|lower| target < lower - POSE_LIMIT_EPSILON);
            let above = joint
                .upper
                .is_some_and(|upper| target > upper + POSE_LIMIT_EPSILON);
            if below || above {
                return invalid(format!(
                    "runtime object {name:?} pose {pose:?}[{index}] = {target} is outside \
                     the limits of joint {:?} [{}, {}]",
                    joint.name,
                    joint.lower.unwrap_or(f64::NEG_INFINITY),
                    joint.upper.unwrap_or(f64::INFINITY),
                ));
            }
        }
    }
    Ok(())
}

/// Checks that need final joint names and URDF limits (URDF-sourced joint
/// groups are only filled in during resolution): pose lengths, pose targets
/// against their joint limits, and single ownership of every joint.
pub(crate) fn validate_resolved(
    document: &ProfileDocument,
    joint_metadata: &BTreeMap<String, Vec<ResolvedJoint>>,
) -> Result<()> {
    let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, component) in document.components.iter().chain(document.sensors.iter()) {
        let resolved = joint_metadata.get(name).map(Vec::as_slice);
        validate_pose_pool(name, component, Some(resolved.unwrap_or_default()))?;
        // Two enabled objects commanding one joint fight over it, and the
        // second silently inherits limits meant for the first. A read-only
        // observer of the same joints commands nothing and may alias them.
        let observer = !component.capabilities.is_empty()
            && component
                .capabilities
                .iter()
                .all(|capability| OBSERVER_CAPABILITIES.contains(&capability.id.as_str()));
        let names = component.joints.iter().flat_map(|joints| &joints.names);
        for joint in names.filter(|_| component.enabled && !observer) {
            if let Some(owner) = owners.insert(joint, name) {
                return invalid(format!(
                    "joint {joint:?} is claimed by both {owner:?} and {name:?}; a joint \
                     belongs to exactly one enabled commanding runtime object"
                ));
            }
        }
    }
    Ok(())
}

fn validate_component_safety(name: &str, component: &ComponentConfig) -> Result<()> {
    let roles: Vec<&SafetyRole> = SAFETY_ROLES
        .iter()
        .filter(|role| {
            component.driver == role.driver
                || component
                    .capabilities
                    .iter()
                    .any(|capability| capability.id == role.capability)
        })
        .collect();
    let has_role = |capability: &str| roles.iter().any(|role| role.capability == capability);
    for (flag, value) in &component.safety {
        if flag == "monitoring_enabled" {
            return invalid(format!(
                "runtime object {name:?} uses legacy safety flag \"monitoring_enabled\"; \
                 the canonical spelling is \"monitoring\""
            ));
        }
        if !SAFETY_ROLES
            .iter()
            .any(|role| role.flags.contains(&flag.as_str()))
        {
            return invalid(format!(
                "runtime object {name:?} has unknown safety flag {flag:?}"
            ));
        }
        if !roles.iter().any(|role| role.flags.contains(&flag.as_str())) {
            let consumers: Vec<&str> = SAFETY_ROLES
                .iter()
                .filter(|role| role.flags.contains(&flag.as_str()))
                .map(|role| role.driver)
                .collect();
            return invalid(format!(
                "runtime object {name:?} (driver {:?}) sets safety flag {flag:?}, which only \
                 {consumers:?} components consume; it would be ignored here",
                component.driver
            ));
        }
        match flag.as_str() {
            "monitoring" => {
                if !value.is_boolean() {
                    return invalid(format!(
                        "runtime object {name:?} safety flag \"monitoring\" must be a boolean"
                    ));
                }
            }
            // A zero threshold never warns, and a zero hysteresis makes the
            // warning chatter on every sample around the threshold.
            "low_battery_percentage" => {
                if !matches!(value.as_f64(), Some(v) if v > 0.0 && v <= 95.0) {
                    return invalid(format!(
                        "runtime object {name:?} safety flag {flag:?} must be a percentage \
                         in (0, 95], got {value}"
                    ));
                }
            }
            "battery_hysteresis_percentage" => {
                if !matches!(value.as_f64(), Some(v) if (1.0..=50.0).contains(&v)) {
                    return invalid(format!(
                        "runtime object {name:?} safety flag {flag:?} must be a percentage \
                         in [1, 50], got {value}"
                    ));
                }
            }
            "estop_query_name" | "heartbeat_topic" => {
                let Some(text) = value.as_str().filter(|text| !text.trim().is_empty()) else {
                    return invalid(format!(
                        "runtime object {name:?} safety flag {flag:?} must be a non-empty string"
                    ));
                };
                if let Some(endpoint) = component.endpoints.get(flag) {
                    if endpoint.value != text {
                        return invalid(format!(
                            "runtime object {name:?} safety flag {flag:?} ({text:?}) \
                             contradicts endpoint {flag:?} ({:?})",
                            endpoint.value
                        ));
                    }
                }
            }
            // `timeout_seconds` is the heartbeat dead-man timeout on a
            // heartbeat component and the poll interval on an E-stop one.
            "timeout_seconds" => {
                let mut bounds = Vec::new();
                if has_role("heartbeat") {
                    bounds.push(HEARTBEAT_TIMEOUT_SECONDS);
                }
                if has_role("emergency_stop") {
                    bounds.push(ESTOP_POLL_SECONDS);
                }
                for (min, max) in bounds {
                    check_seconds(name, flag, value, min, max)?;
                }
            }
            // How old a decodable safety sample may be before the state
            // counts as unreadable (E-stop supervision). Separate from
            // `timeout_seconds`, which is the poll interval: delivery on real
            // robots is bursty, so the freshness threshold is deliberately far
            // looser than the polling rate and must be tunable per deployment.
            "state_max_age_seconds" => {
                let (min, max) = STATE_MAX_AGE_SECONDS;
                check_seconds(name, flag, value, min, max)?;
            }
            // Whether this deployment's server publishes E-stop state on a
            // period rather than only when it changes. Measured firmware
            // (Vega-1p) publishes `state/estop` *only while the button is
            // engaged*, so the client defaults to event-driven/latched
            // semantics: silence is a released E-stop, not a dead stream, and
            // `state_max_age_seconds` is inert. A deployment whose server
            // really does publish periodically sets this to `true` and gets
            // age-based supervision (and the escalation it drives) back.
            "estop_state_periodic" => {
                if !value.is_boolean() {
                    return invalid(format!(
                        "runtime object {name:?} safety flag \"estop_state_periodic\" \
                         must be a boolean"
                    ));
                }
            }
            _ => unreachable!("every flag in SAFETY_ROLES is handled above"),
        }
    }
    if component.safety.contains_key("low_battery_percentage")
        || component
            .safety
            .contains_key("battery_hysteresis_percentage")
    {
        let low = component
            .safety
            .get("low_battery_percentage")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_LOW_BATTERY_PERCENTAGE);
        let hysteresis = component
            .safety
            .get("battery_hysteresis_percentage")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE);
        if low + hysteresis >= 100.0 {
            return invalid(format!("runtime object {name:?}: low_battery_percentage plus battery_hysteresis_percentage must be below 100"));
        }
    }
    // The heartbeat timeout is mirrored into metadata for status reporting.
    // Different consumers read each copy, so an overlay that changes one
    // must change both.
    if let (Some(safety), Some(metadata)) = (
        component.safety.get("timeout_seconds"),
        component.metadata.get("timeout_seconds"),
    ) {
        if safety.as_f64() != metadata.as_f64() {
            return invalid(format!(
                "runtime object {name:?} safety.timeout_seconds ({safety}) and \
                 metadata.timeout_seconds ({metadata}) must be equal"
            ));
        }
    }
    Ok(())
}

fn check_seconds(name: &str, flag: &str, value: &Value, min: f64, max: f64) -> Result<()> {
    match value.as_f64() {
        Some(seconds) if (min..=max).contains(&seconds) => Ok(()),
        _ => invalid(format!(
            "runtime object {name:?} safety flag {flag:?} must be a number of seconds \
             in [{min}, {max}], got {value}"
        )),
    }
}

/// Metadata keys whose values change robot behaviour, with their accepted
/// range. `metadata` stays an open map -- drivers and applications keep
/// their own keys there -- but these are read by the runtime, where a bad
/// value is either ignored or turns a check off.
const NUMERIC_METADATA: &[(&str, f64, f64)] = &[
    // Hz.
    ("default_control_hz", f64::MIN_POSITIVE, f64::MAX),
    // Chassis velocity limits: m/s and rad/s.
    ("max_linear_vel", f64::MIN_POSITIVE, 5.0),
    ("max_angular_vel", f64::MIN_POSITIVE, 10.0),
    // Chassis geometry: m, m, rad.
    ("wheels_dist", f64::MIN_POSITIVE, 5.0),
    ("center_to_wheel_axis_dist", f64::MIN_POSITIVE, 5.0),
    (
        "max_steering_angle",
        f64::MIN_POSITIVE,
        std::f64::consts::PI,
    ),
];

/// Every metadata key the built-in profiles or the runtime use. A key close
/// to one of [`CHECKED_METADATA`] that is not in this list is a typo.
const KNOWN_METADATA: &[&str] = &[
    "drive_joints",
    "enable_depth",
    "enable_ee_pass_through",
    "enable_rgb",
    "grasp_torque",
    "name",
    "pose_pool",
    "pv_mode",
    "side",
    "steer_joints",
    "timeout_seconds",
];

/// Keys validated by [`validate_metadata`]; the typo check guards these.
const CHECKED_METADATA: &[&str] = &[
    "state_max_age_ms",
    "state_idle_timeout_ms",
    "default_control_hz",
    "max_linear_vel",
    "max_angular_vel",
    "wheels_dist",
    "center_to_wheel_axis_dist",
    "max_steering_angle",
];

fn validate_metadata(name: &str, component: &ComponentConfig) -> Result<()> {
    for (key, value) in &component.metadata {
        let key = key.as_str();
        if !CHECKED_METADATA.contains(&key) && !KNOWN_METADATA.contains(&key) {
            if let Some(intended) = CHECKED_METADATA.iter().find(|known| near_miss(key, known)) {
                return invalid(format!(
                    "runtime object {name:?} metadata key {key:?} is not recognised; \
                     did you mean {intended:?}?"
                ));
            }
            continue;
        }
        match key {
            // `false` is the explicit opt-out. `null` is rejected: it is what
            // a half-finished edit (`state_max_age_ms:`) parses to, and it
            // would silently turn the freshness check off.
            "state_max_age_ms" => match value {
                Value::Bool(false) => {}
                _ if value.as_f64().is_some_and(|ms| ms > 0.0) => {}
                _ => {
                    return invalid(format!(
                        "runtime object {name:?} metadata \"state_max_age_ms\" must be a \
                         positive number of milliseconds, or false to disable the \
                         freshness check; got {value}"
                    ));
                }
            },
            "state_idle_timeout_ms" => {
                if value
                    .as_u64()
                    .is_none_or(|ms| ms < MIN_STATE_IDLE_TIMEOUT_MS)
                {
                    return invalid(format!(
                        "runtime object {name:?} metadata \"state_idle_timeout_ms\" must be \
                         a whole number of milliseconds, at least \
                         {MIN_STATE_IDLE_TIMEOUT_MS}; got {value}"
                    ));
                }
            }
            _ => {
                if let Some((_, min, max)) = NUMERIC_METADATA.iter().find(|entry| entry.0 == key) {
                    if !value.as_f64().is_some_and(|v| (*min..=*max).contains(&v)) {
                        let bound = if *max == f64::MAX {
                            "a positive number".to_owned()
                        } else {
                            format!("a positive number no greater than {max}")
                        };
                        return invalid(format!(
                            "runtime object {name:?} metadata {key:?} must be {bound}, \
                             got {value}"
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// A key within two edits of `known`, or one that drops or spells out its
/// unit suffix (`state_max_age`, `max_linear_velocity`).
fn near_miss(key: &str, known: &str) -> bool {
    let (short, long) = if key.len() < known.len() {
        (key, known)
    } else {
        (known, key)
    };
    edit_distance(key, known) <= 2 || (short.len() >= 10 && long.starts_with(short))
}

/// Levenshtein distance over bytes; metadata keys are ASCII identifiers.
fn edit_distance(left: &str, right: &str) -> usize {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, a) in left.iter().enumerate() {
        let mut current = vec![row + 1];
        for (column, b) in right.iter().enumerate() {
            let substitute = previous[column] + usize::from(a != b);
            current.push(
                substitute
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

/// Why `text` cannot be a single transport key, if it cannot.
fn key_expression_problem(text: &str) -> Option<&'static str> {
    if text.chars().any(char::is_whitespace) {
        Some("contains whitespace")
    } else if text.starts_with('/') || text.ends_with('/') {
        Some("starts or ends with '/'")
    } else if text.contains("//") {
        Some("contains an empty '//' segment")
    } else if text.contains(KEY_EXPRESSION_RESERVED) {
        Some("contains a wildcard or reserved character (one of * $ ? #)")
    } else {
        None
    }
}

/// Two enabled objects publishing on one topic interleave commands to the
/// same actuator; nothing downstream can tell the streams apart.
fn validate_publisher_topics(all: &BTreeMap<&str, &ComponentConfig>) -> Result<()> {
    let mut publishers: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
    for (name, component) in all {
        if !component.enabled {
            continue;
        }
        for (role, endpoint) in &component.endpoints {
            if endpoint.kind != "publisher_topic" {
                continue;
            }
            if let Some((owner, owner_role)) =
                publishers.insert(&endpoint.value, (name, role.as_str()))
            {
                if owner != *name {
                    return invalid(format!(
                        "publisher topic {:?} is used by both {owner:?} ({owner_role:?}) \
                         and {name:?} ({role:?})",
                        endpoint.value
                    ));
                }
            }
        }
    }
    Ok(())
}

fn valid_extension_id(id: &str) -> bool {
    let segments: Vec<_> = id.split('.').collect();
    segments.len() >= 3
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

fn detect_cycles(all: &BTreeMap<&str, &ComponentConfig>) -> Result<()> {
    fn visit<'a>(
        name: &'a str,
        all: &BTreeMap<&'a str, &'a ComponentConfig>,
        visiting: &mut BTreeSet<&'a str>,
        done: &mut BTreeMap<&'a str, usize>,
    ) -> Result<usize> {
        if visiting.len() + done.get(name).copied().unwrap_or(1) > MAX_DEPENDENCY_DEPTH {
            return invalid(format!(
                "component dependency chain exceeds the supported depth of \
                 {MAX_DEPENDENCY_DEPTH} at {name:?}"
            ));
        }
        if let Some(depth) = done.get(name) {
            return Ok(*depth);
        }
        if !visiting.insert(name) {
            return invalid(format!("component dependency cycle includes {name:?}"));
        }
        let mut depth = 1;
        for dependency in &all[name].dependencies {
            depth = depth.max(1 + visit(dependency, all, visiting, done)?);
        }
        visiting.remove(name);
        done.insert(name, depth);
        Ok(depth)
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeMap::new();
    for name in all.keys() {
        visit(name, all, &mut visiting, &mut done)?;
    }
    Ok(())
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(ModelError::Validation(message.into()))
}
