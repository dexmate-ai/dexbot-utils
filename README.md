<div align="center">
  <h1>Robot Configuration and URDF Utilities</h1>
</div>

![Python](https://img.shields.io/badge/python-3.10%20|%203.11%20|%203.12%20|%203.13|%203.14-blue)

Type-safe dataclass configurations and a unified interface for accessing Dexmate robot information.

## Installation

```bash
pip install dexbot-utils
```

## Usage

### RobotInfo (Recommended)

```python
from dexbot_utils import RobotInfo

# Load by variant name
robot = RobotInfo("vega_1")

# Basic properties
print(robot.robot_model)     # "vega_1"
print(robot.robot_type)      # "vega"

# Component access
joints = robot.get_component_joints("left_arm")
dof = robot.get_component_dof("left_arm")

# URDF queries (if URDF is loaded)
if robot.has_urdf:
    joint_limits = robot.get_joint_limits()
    link_names = robot.get_link_names()
```

### Environment Variables

```bash
export ROBOT_CONFIG=vega_1
# or derive from robot name
export ROBOT_NAME=dm/vgabcd123456-1  # -> vega_1
```

```python
robot = RobotInfo()  # auto-loads from env
```

## CLI

```bash
# List available configurations
dexbot cfg list

# Show configuration details
dexbot cfg show vega_1
```

## Licensing

This project is **dual-licensed**:

### Open Source License
This software is available under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.
See the [LICENSE](./LICENSE) file for details.

### Commercial License
For businesses that want to use this software in proprietary applications without the AGPL requirements, commercial licenses are available.

---

<div align="center">
  <h3>🤝 Ready to build amazing robots?</h3>
  <p>
    <a href="mailto:contact@dexmate.ai">📧 Contact Us</a> •
  </p>
</div>
