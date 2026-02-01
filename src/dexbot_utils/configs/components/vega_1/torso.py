"""Torso component configuration."""

from dataclasses import dataclass, field

from ..base import BaseJointComponentConfig


@dataclass
class Vega1TorsoConfig(BaseJointComponentConfig):
    """Configuration for Vega robot torso component.

    Attributes:
        pv_mode: Position-velocity control mode flag
        joints: List of joint names for the torso
        state_sub_topic: Topic for torso state feedback
        control_pub_topic: Topic for torso control commands
        pose_pool: Dictionary of predefined torso poses with joint positions
    """

    pv_mode: bool = True
    joints: list[str] = field(
        default_factory=lambda: ["torso_j1", "torso_j2", "torso_j3"]
    )
    state_sub_topic: str = "state/torso"
    control_pub_topic: str = "control/torso"

    pose_pool: dict[str, list[float]] = field(
        default_factory=lambda: {
            "home": [0.0, 0.0, 0.0],
            "folded": [0.0, 0.0, -1.5708],
            "crouch20_low": [0.0, 0.0, -0.35],
            "crouch20_medium": [0.52, 1.05, 0.18],
            "crouch20_high": [0.78, 1.57, 0.44],
            "crouch45_low": [0.0, 0.0, -0.79],
            "crouch45_medium": [0.52, 1.05, -0.26],
            "crouch45_high": [0.78, 1.57, 0],
            "crouch90_low": [0.0, 0.0, -1.57],
            "crouch90_medium": [0.52, 1.05, -1.04],
            "crouch90_high": [0.78, 1.57, -0.78],
        }
    )
