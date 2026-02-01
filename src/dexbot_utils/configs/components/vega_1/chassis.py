"""Chassis component configuration."""

from dataclasses import dataclass, field

from ..base import BaseJointComponentConfig


@dataclass
class Vega1ChassisConfig(BaseJointComponentConfig):
    """Configuration for Vega robot chassis/mobile base component.

    Attributes:
        pv_mode: Position-velocity control mode flag
        max_linear_vel: Maximum linear velocity (m/s)
        max_steering_angle: Maximum steering angle (rad)
        center_to_wheel_axis_dist: Distance from base center to wheel axis (m)
        wheels_dist: Distance between two wheels (m)
        steer_joints: List of steering joint names
        drive_joints: List of drive joint names
        steer_control_pub_topic: Topic for steering control commands
        steer_state_sub_topic: Topic for steering state feedback
        drive_control_pub_topic: Topic for drive control commands
        drive_state_sub_topic: Topic for drive state feedback
        joints: Property returning all chassis joints (steer + drive)
    """

    pv_mode: bool = False
    max_linear_vel: float = 0.8
    max_steering_angle: float = 2.35
    center_to_wheel_axis_dist: float = 0.219
    wheels_dist: float = 0.45
    steer_joints: list[str] = field(
        default_factory=lambda: ["L_wheel_j1", "R_wheel_j1"]
    )
    drive_joints: list[str] = field(
        default_factory=lambda: ["L_wheel_j2", "R_wheel_j2"]
    )

    steer_control_pub_topic: str = "control/chassis/steer"
    steer_state_sub_topic: str = "state/chassis/steer"
    drive_control_pub_topic: str = "control/chassis/drive"
    drive_state_sub_topic: str = "state/chassis/drive"

    @property
    def joints(self) -> list[str]:
        """Return all chassis joints (steer + drive)."""
        return self.steer_joints + self.drive_joints
