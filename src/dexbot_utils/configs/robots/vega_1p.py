"""Vega-1p robot configurations."""

from dataclasses import dataclass, field

from dexmate_urdf import robots

from ..components.sensors.cameras import ZedXCameraConfig, ZedXOneCameraConfig
from ..components.sensors.imu import ChassisIMUConfig, ZedIMUConfig
from ..components.sensors.lidar import Lidar3DConfig
from ..components.sensors.ultrasonic import UltraSonicConfig
from ..components.vega_1 import (
    BatteryConfig,
    DexDGripperConfig,
    EStopConfig,
    F5D6HandV2Config,
    HeartbeatConfig,
    Vega1ArmConfig,
    Vega1ChassisConfig,
    Vega1HeadConfig,
    Vega1TorsoConfig,
)
from ..registry import register_variant
from .base import BaseComponentConfig, BaseRobotConfig


@register_variant("vega_1p")
@dataclass
class Vega1pConfig(BaseRobotConfig):
    """Configuration for Vega-1p robot base (no hands)."""

    robot_model: str = "vega_1p"
    abbr: str = "vg"
    urdf_path: str = str(robots.humanoid.vega_1p.vega_1p.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "torso": Vega1TorsoConfig(),
            "chassis": Vega1ChassisConfig(),
            "head": Vega1HeadConfig(),
            "battery": BatteryConfig(),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )

    sensors: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "head_camera": ZedXCameraConfig(name="head_camera"),
            "chassis_imu": ChassisIMUConfig(name="chassis_imu"),
            "head_imu": ZedIMUConfig(name="head_imu"),
            "front_lidar_3d": Lidar3DConfig(name="lidar_3d_front"),
            "ultrasonic": UltraSonicConfig(name="ultrasonic"),
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


@register_variant("vega_1p_f5d6")
@dataclass
class Vega1pF5D6Config(Vega1pConfig):
    """Configuration for Vega-1p robot with F5D6 hands."""

    urdf_path: str = str(robots.humanoid.vega_1p.vega_1p_f5d6.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "torso": Vega1TorsoConfig(),
            "chassis": Vega1ChassisConfig(),
            "head": Vega1HeadConfig(),
            "left_hand": F5D6HandV2Config(side="left"),
            "right_hand": F5D6HandV2Config(side="right"),
            "battery": BatteryConfig(),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )


@register_variant("vega_1p_gripper")
@dataclass
class Vega1pDGripperConfig(Vega1pConfig):
    """Configuration for Vega-1p robot with D-gripper hands and wrist cameras."""

    urdf_path: str = str(robots.humanoid.vega_1p.vega_1p_gripper.urdf)

    components: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "left_arm": Vega1ArmConfig(side="left"),
            "right_arm": Vega1ArmConfig(side="right"),
            "torso": Vega1TorsoConfig(),
            "chassis": Vega1ChassisConfig(),
            "head": Vega1HeadConfig(),
            "left_hand": DexDGripperConfig(side="left"),
            "right_hand": DexDGripperConfig(side="right"),
            "battery": BatteryConfig(),
            "estop": EStopConfig(),
            "heartbeat": HeartbeatConfig(),
        }
    )

    sensors: dict[str, BaseComponentConfig] = field(
        default_factory=lambda: {
            "head_camera": ZedXCameraConfig(name="head_camera"),
            "chassis_imu": ChassisIMUConfig(name="chassis_imu"),
            "head_imu": ZedIMUConfig(name="head_imu"),
            "front_lidar_3d": Lidar3DConfig(name="lidar_3d_front"),
            "ultrasonic": UltraSonicConfig(name="ultrasonic"),
            "left_wrist_camera": ZedXOneCameraConfig(side="left"),
            "right_wrist_camera": ZedXOneCameraConfig(side="right"),
        }
    )
