"""Hand component configuration."""

from dataclasses import dataclass, field

from ..base import BaseJointComponentConfig


@dataclass
class F5D6HandV1Config(BaseJointComponentConfig):
    """Configuration for F5D6 V1 hand component.

    Attributes:
        pv_mode: Position-velocity control mode flag
        side: Hand side ("left" or "right")
        pose_pool: Dictionary of predefined hand poses with joint positions
        state_sub_topic: Property returning topic for hand state feedback
        control_pub_topic: Property returning topic for hand control commands
        joints: Property returning list of joint names for the hand
    """

    pv_mode: bool = False
    side: str = "left"
    pose_pool: dict[str, list[float]] = field(
        default_factory=lambda: {
            "open": [0.1834, 0.2891, 0.2801, 0.284, 0.2811, -0.0158],
            "close": [-0.1, -1.0946, -1.0844, -1.0154, -1.0118, 0.84],
        }
    )

    @property
    def state_sub_topic(self) -> str:
        return f"state/hand/{self.side}"

    @property
    def control_pub_topic(self) -> str:
        return f"control/hand/{self.side}"

    @property
    def joints(self) -> list[str]:
        joint_prefix = "L" if self.side == "left" else "R"
        joint_suffixes = [
            "th_j1",
            "ff_j1",
            "mf_j1",
            "rf_j1",
            "lf_j1",
            "th_j0",
        ]
        return [f"{joint_prefix}_{suffix}" for suffix in joint_suffixes]


@dataclass
class F5D6HandV2Config(F5D6HandV1Config):
    """Configuration for F5D6 V2 hand component with touch sensors.

    Inherits all attributes from F5D6HandV1Config and adds touch sensor support.

    Attributes:
        touch_sensor_sub_topic: Property returning topic for touch sensor feedback
    """

    @property
    def touch_sensor_sub_topic(self) -> str:
        # Only for V2 hand
        return f"state/hand/{self.side}/touch"


@dataclass
class DexSGripperConfig(BaseJointComponentConfig):
    """Configuration for DexS single gripper component.

    Attributes:
        pv_mode: Position-velocity control mode flag
        side: Gripper side ("left" or "right")
        pose_pool: Dictionary of predefined gripper poses with joint positions
        state_sub_topic: Property returning topic for gripper state feedback
        control_pub_topic: Property returning topic for gripper control commands
        joints: Property returning list of joint names for the gripper
    """

    pv_mode: bool = False
    side: str = "left"
    pose_pool: dict[str, list[float]] = field(
        default_factory=lambda: {
            "open": [0.7854],
            "close": [0.0],
        }
    )

    # TODO: when gripper auto detection is available, we should use the hand namespace instead of gripper namespace
    @property
    def state_sub_topic(self) -> str:
        return f"state/gripper/{self.side}"

    @property
    def control_pub_topic(self) -> str:
        return f"control/gripper/{self.side}"

    @property
    def joints(self) -> list[str]:
        joint_prefix = "L" if self.side == "left" else "R"
        return [f"{joint_prefix}_gripper_j1"]


@dataclass
class DexDGripperConfig(DexSGripperConfig):
    """Configuration for DexD double gripper component.

    Inherits all attributes and behavior from DexSGripperConfig.
    Double and single grippers currently share the same interface.
    """

    pass
