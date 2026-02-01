"""Vega-1u (Upper Body) robot configurations."""

from dataclasses import dataclass, field

from dexmate_urdf import robots

from ..components.sensors.cameras import ZedXCameraConfig, ZedXOneCameraConfig
from ..components.vega_1 import (
    DexDGripperConfig,
    EStopConfig,
    F5D6HandV2Config,
    HeartbeatConfig,
    Vega1ArmConfig,
    Vega1HeadConfig,
)
from ..registry import register_variant
from .base import BaseComponentConfig, BaseRobotConfig


@register_variant("vega_1u")
@dataclass
class Vega1UConfig(BaseRobotConfig):
    """Configuration for Vega-1 Upper Body robot base (no hands)."""

    robot_model: str = "vega_1u"
    abbr: str = "vg"
    urdf_path: str = str(robots.humanoid.vega_1u.vega_1u.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "head": Vega1HeadConfig(),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )
    querables: dict[str, str] = field(
        default_factory=lambda: {
            # Querables
            "version_info": "info/versions",
            "status_info": "info/status",
            "hand_info": "info/hand_type",
            "reboot": "system/reboot",
            "clear_error": "system/clear_error",
            "soc_ntp": "time/soc",
            "chassis_led": "system/led",
        }
    )
    sensors: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "head_camera": ZedXCameraConfig(name="head_camera"),
        }
    )


@register_variant("vega_1u_f5d6")
@dataclass
class Vega1UF5D6Config(Vega1UConfig):
    """Configuration for Vega-1 Upper Body robot with F5D6 hands."""

    urdf_path: str = str(robots.humanoid.vega_1u.vega_1u_f5d6.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "head": Vega1HeadConfig(),
            "left_hand": F5D6HandV2Config(side="left"),
            "right_hand": F5D6HandV2Config(side="right"),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )


@register_variant("vega_1u_gripper")
@dataclass
class Vega1UDGripperConfig(Vega1UConfig):
    """Configuration for Vega-1 Upper Body robot with D-gripper hands and wrist cameras."""

    urdf_path: str = str(robots.humanoid.vega_1u.vega_1u_gripper.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "head": Vega1HeadConfig(),
            "left_hand": DexDGripperConfig(side="left"),
            "right_hand": DexDGripperConfig(side="right"),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )

    sensors: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "head_camera": ZedXCameraConfig(name="head_camera"),
            "left_wrist_camera": ZedXOneCameraConfig(side="left"),
            "right_wrist_camera": ZedXOneCameraConfig(side="right"),
        }
    )
