"""Head component configuration."""

from dataclasses import dataclass, field

from ..base import BaseJointComponentConfig


@dataclass
class Vega1HeadConfig(BaseJointComponentConfig):
    """Configuration for Vega robot head component.

    Attributes:
        pv_mode: Position-velocity control mode flag
        joints: List of joint names for the head
        state_sub_topic: Topic for head state feedback
        control_pub_topic: Topic for head control commands
        set_mode_query: Service name for setting head control mode
        pose_pool: Dictionary of predefined head poses with joint positions
    """

    pv_mode: bool = True

    state_sub_topic: str = "state/head"
    control_pub_topic: str = "control/head"
    set_mode_query: str = "mode/head"
    pose_pool: dict[str, list[float]] = field(
        default_factory=lambda: {
            "home": [0.0, 0.0, 0.0],
            "tucked": [0.0, 0.0, -1.37],
        }
    )

    @property
    def joints(self) -> list[str]:
        return ["head_j1", "head_j2", "head_j3"]
