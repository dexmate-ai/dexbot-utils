# dexbot-model

Robot model utilities for Dexmate robots. See https://github.com/dexmate-ai/dexbot-utils for installation, examples, and API documentation.

## Vega model selection after discovery

Use `vega::apply_discovery` when the transport has queried hand hardware and
wrist force/torque sensor presence. Supply hand injection, replacement, and
disable decisions in the discovery overlay; the selected model follows the
resulting components, including user decisions to retain configured hands.
Existing `apply_discovery` callers keep their current behavior.

```rust
use dexbot_model::{vega, DiscoveryFacts, RobotConfig, RobotOverlay};

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
# Ok::<(), dexbot_model::ModelError>(())
```

The values above illustrate a complete report; applications must use actual
query results. Missing reports, unknown wrist status, asymmetric hardware, and
unsupported bodies retain the configured URDF. Configuration validation and
user-overlay conflict checks still apply. This library performs no queries.
`vega::select_model` also exposes selection without modifying a configuration.
