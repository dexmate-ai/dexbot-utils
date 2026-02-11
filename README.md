<div align="center">
  <h1>Dexmate Robot Configuration Utilities</h1>
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
print(robot.robot_version)   # "1"

# Component access
print(robot.has_left_arm)    # True
print(robot.get_component_list())
# ['left_arm', 'right_arm', 'torso', 'chassis', 'head', ...]

# Get component details
joints = robot.get_component_joints("left_arm")
# ['L_arm_j1', 'L_arm_j2', ..., 'L_arm_j7']

dof = robot.get_component_dof("left_arm")  # 7

# Access component config directly
arm_config = robot.get_component_config("left_arm")
print(arm_config.side)       # "left"
print(arm_config.pv_mode)    # False

# URDF queries (if URDF is loaded)
# URDF queries (if URDF is loaded)
if robot.has_urdf:
    joint_limits = robot.get_joint_limits()
    pos_limits = robot.get_joint_pos_limits()
    vel_limits = robot.get_joint_vel_limits()
    link_names = robot.get_link_names()
```

### Environment Variables

Load configuration from environment:

```bash
export ROBOT_CONFIG=vega_1
# or derive from robot name
export ROBOT_NAME=dm/vgabcd123456-1  # -> vega_1
```

```python
robot = RobotInfo()  # auto-loads from env
```

### Direct Config Access

For direct access to configuration dataclasses:

```python
from dexbot_utils.configs import get_robot_config, get_available_variants

# List available variants
variants = get_available_variants()
# ['vega_1', 'vega_1_gripper', ...]

# Get config directly
config = get_robot_config("vega_1")
left_arm = config.components["left_arm"]
print(left_arm.joints)
```

## CLI

```bash
# List available configurations
# List available configurations
dexbot cfg list

# Show configuration details
dexbot cfg show vega_1
```

## Advance Usage: Adding New Configs

### 1. Create Component Configs

Define components in `configs/components/your_robot/`:

```python
# configs/components/my_robot/arm.py
from dataclasses import dataclass
from ..base import BaseJointComponentConfig

@dataclass
class MyArmConfig(BaseJointComponentConfig):
    side: str = "left"
    pv_mode: bool = False

    @property
    def joints(self) -> list[str]:
        prefix = "L" if self.side == "left" else "R"
        return [f"{prefix}_arm_j{i}" for i in range(1, 8)]
```

### 2. Create Robot Config

Define the robot in `configs/robots/` with `@register_variant`:

```python
# configs/robots/my_robot.py
from dataclasses import dataclass, field
from ..registry import register_variant
from ..components.my_robot import MyArmConfig
from .base import BaseRobotConfig, BaseComponentConfig

@register_variant("my_robot")
@dataclass
class MyRobotConfig(BaseRobotConfig):
    robot_model: str = "my_robot"
    abbr: str = "mr"
    urdf_path: str = "robots/my_robot/robot.urdf"

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": MyArmConfig(side="left"),
            "right_arm": MyArmConfig(side="right"),
        }
    )
```

### 3. Register

Import in `configs/robots/__init__.py`:

```python
from .my_robot import MyRobotConfig
```

Now use it:

```python
robot = RobotInfo("my_robot")
```

## Development

### Setup

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Setup pre-commit hooks
prek install
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
