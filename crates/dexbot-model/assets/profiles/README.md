# Canonical robot profiles

These files are the source of truth for built-in robot composition. They were
seeded from the Python configurations of the former `dexbot_utils` package,
captured in the Phase 0 fixtures; that package has since been removed.
Runtime code must resolve these files through `dexbot-model`; it must not add
hard-coded variant builders.

## Composition

Every built-in profile is a thin `extends` composition over the plain
fragments in `common/`:

- `common/vega_upper_body.yaml` — arms, head, estop, heartbeat, head/wrist
  cameras, runtime defaults, querables, intent profiles, shared robot
  identity fields.
- `common/vega_mobile_base.yaml` — torso, chassis, battery.
- `common/vega_1_sensors.yaml` — Vega-1 platform sensors (chassis IMU, head
  IMU, 2D front lidar, ultrasonic).
- `common/vega_1p_sensors.yaml` — Vega-1P platform sensors (head IMU,
  front/back 3D lidars).
- `common/vega_hands_f5d6.yaml` / `common/vega_hands_gripper.yaml` — the two
  hand options.

Each profile file then contributes only `schema_version`, its `extends`
list, and the `robot.model`/`robot.urdf` identity. Fragments are plain by
design: they cannot extend further (composition is one level deep), and they
are embedded in the crate alongside the profiles. Profiles loaded from disk
may extend fragments by paths relative to the profile file, falling back to
the embedded catalog for `common/...` names.

## Joints

Component joint lists use explicit ordered `names`. This is deliberate: the
former Python configurations fixed the joint order, and that
order is curated, not derivable from the URDF (for example the F5D6 hand
order `th_j1, ff_j1, mf_j1, rf_j1, lf_j1, th_j0` differs from URDF document
order, and the chassis interleaves steer/drive pairs). Name-prefix or
document-order matching cannot reproduce it, so the schema uses explicit
group→name-list tables instead.

A profile may alternatively declare `joints: {source: urdf, group: <name>}`
and define the ordered list once in a top-level `joint_groups` table;
resolution materializes the names from the table. Either way, every
referenced joint must exist in the robot URDF, be movable and not a `<mimic>`
follower, and carry finite `lower`/`upper` limits (only `continuous` joints
may omit them). A joint belongs to exactly one enabled commanding
component (a read-only `joint_state` observer may alias it). Resolution
exports per-joint limit metadata (`joint_metadata`) in the resolved
configuration, and every `pose_pool` target must lie within those limits
(the first joint of a torso-referenced pose is exempt: it is offset by the
measured torso pitch at run time). The URDF referenced by `robot.urdf` resolves against the
vendored assets in `assets/urdf/` (see its README for override order).

## Endpoints and metadata

Endpoint values and `robot.namespace` are plain transport keys: no leading,
trailing or doubled `/`, no whitespace, and none of `* $ ? #`. Two enabled
components cannot share a `publisher_topic`.

`metadata` is an open map, but the keys the runtime reads are validated when
present: `state_max_age_ms` (positive milliseconds, or `false` to opt out of
the freshness check — `null` is rejected because a half-finished edit parses
to it), `state_idle_timeout_ms` (whole milliseconds, at least 100, as is
`runtime.state_idle_timeout_ms`), `default_control_hz` (> 0),
`max_linear_vel` (≤ 5 m/s), `max_angular_vel` (≤ 10 rad/s), `wheels_dist` and
`center_to_wheel_axis_dist` (≤ 5 m), `max_steering_angle` (≤ π rad), and
`grasp_torque` ([0, 1]). A key that is a near miss of one of these
(`state_max_age_m`, `max_linear_velocity`) is rejected as a typo.

## Safety flags

Top-level `safety` accepts `estop_failure_action`,
`heartbeat_failure_action`, and `estop_unreadable_action`. Each takes one of
five actions: `stop_motion`, `shutdown_robot`, `activate_software_estop`,
`request_process_termination`, or `none` (report only). The shipped profiles
set `heartbeat_failure_action: request_process_termination`.
`estop_unreadable_action` covers the dead-stream case — a stream that stops
delivering can never produce a button edge, so the client escalates to this
action once the E-stop channel is provably dead; absent, it follows
`estop_failure_action`.

Per-component safety maps are scoped to the role that consumes them. A
component holds a role through its standard driver or the matching
capability; a flag on any other component is rejected, because it would
validate and then be ignored.

| Role (driver / capability) | Flags |
| --- | --- |
| `standard.estop` / `emergency_stop` | `monitoring` (bool), `estop_query_name` (string), `timeout_seconds` (poll interval, 0.001–1 s), `state_max_age_seconds` (0.1–60 s), `estop_state_periodic` (bool) |
| `standard.heartbeat` / `heartbeat` | `monitoring` (bool), `heartbeat_topic` (string), `timeout_seconds` (dead-man timeout, 0.05–10 s) |
| `standard.battery` / `battery` | `monitoring` (bool), `low_battery_percentage` (percent in (0, 95], default 20), `battery_hysteresis_percentage` (percent in [1, 50], default 5) |

`estop_query_name` and `heartbeat_topic` must agree with the endpoint of the
same role. The low-battery warning raises at `low_battery_percentage` and
clears once the charge recovers by `battery_hysteresis_percentage` above it;
their sum must stay below 100. The defaults are exported as
`DEFAULT_LOW_BATTERY_PERCENTAGE` and `DEFAULT_BATTERY_HYSTERESIS_PERCENTAGE`.
The heartbeat component mirrors `safety.timeout_seconds` into
`metadata.timeout_seconds` for status reporting; when both are present they
must be equal, so an overlay changes both. The legacy redundant
`monitoring_enabled` spelling was migrated to `monitoring` and is rejected.

`timeout_seconds` and `state_max_age_seconds` answer different questions.
`timeout_seconds` is how often supervision looks (E-stop poll interval) or
how long a heartbeat may be silent. `state_max_age_seconds` is how old a
decodable E-stop sample may be before the state counts as *unreadable*
rather than "not engaged" — a frozen sample reports "no button pressed"
forever, so silence would read as safety. Delivery on real robots is bursty
(measured on a Vega-1p: samples 101 ms apart, then gaps up to ~2.9 s, against
a configured 100 ms period), so this threshold is deliberately far looser
than the publish rate; the client default is 5 s. Raise it on deployments
with slower or burstier links, where a tighter value produces continuous
false "unreadable" alarms on a healthy robot.

`estop_state_periodic` says whether `state_max_age_seconds` means anything at
all, and it is **false** unless a profile says otherwise. Measured on a
Vega-1p, the server publishes `state/estop` *only while the button is
engaged*: twenty seconds subscribed with the button released yielded zero
samples, and the state appeared the instant it was pressed. E-stop state is
therefore an event stream whose last value is latched, and on a healthy robot
its resting state is silence — so the client treats absence and age as
carrying no verdict, and escalates only on payloads that fail to decode.
Liveness of that path is covered by the heartbeat (20 ms, same server
process) and by the component-status report, not by E-stop sample age. Set
`estop_state_periodic: true` only for firmware that genuinely publishes on a
period; it restores age-based supervision, and with it the
`estop_unreadable_action` escalation. No shipped profile sets it.
