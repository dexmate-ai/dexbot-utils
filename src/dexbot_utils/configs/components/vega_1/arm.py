"""Arm component configuration."""

from dataclasses import dataclass

from ..base import BaseJointComponentConfig


@dataclass
class Vega1ArmConfig(BaseJointComponentConfig):
    """Configuration for Vega robot arm component.

    Attributes:
        side: Arm side ("left" or "right")
        pv_mode: Position-velocity control mode flag
        default_control_hz: Default control frequency in Hz
        enable_ee_pass_through: Enable end-effector pass-through mode by default
        joints: Property returning list of joint names for the arm
        pose_pool: Property returning dictionary of predefined arm poses
        state_sub_topic: Property returning topic for arm state feedback
        wrench_sub_topic: Property returning topic for wrench feedback
        wrist_button_sub_topic: Property returning topic for wrist button state
        control_pub_topic: Property returning topic for arm control commands
        set_mode_query: Property returning service name for setting arm control mode
        pid_query: Property returning service name for PID configuration
        brake_query: Property returning service name for brake control
        ee_baud_rate_query: Property returning service name for end-effector baud rate configuration
        force_torque_sensor_query: Property returning service name for force torque sensor mode control
        ee_pass_through_pub_topic: Property returning topic for end-effector pass-through commands
        ee_pass_through_state_sub_topic: Property returning topic for end-effector pass-through state feedback
    """

    side: str = "left"
    pv_mode: bool = False
    default_control_hz: int = 100
    enable_ee_pass_through: bool = True

    @property
    def joints(self) -> list[str]:
        joint_prefix = "L" if self.side == "left" else "R"
        joint_suffixes = [
            "arm_j1",
            "arm_j2",
            "arm_j3",
            "arm_j4",
            "arm_j5",
            "arm_j6",
            "arm_j7",
        ]
        return [f"{joint_prefix}_{suffix}" for suffix in joint_suffixes]

    @property
    def pose_pool(self) -> dict[str, list[float]]:
        pool = {
            "folded": [1.57079, 0.0, 0, -3.1, 0, 0, -0.69813],
            "folded_closed_hand": [1.57079, 0.0, 0, -3.1, 0, 0, -0.9],
            "L_shape": [0.064, 0.3, 0.0, -1.556, 1.271, 0.0, 0.0],
            "lift_up": [0.064, 0.3, 0.0, -2.756, 1.271, 0.0, 0.0],
            "zero": [-1.57079, 0.0, 0, 0.0, 0, 0, 0.0],
        }
        if self.side == "right":
            for k, v in pool.items():
                pool[k] = [-v[0], -v[1], -v[2], v[3], -v[4], -v[5], -v[6]]
        return pool

    @property
    def state_sub_topic(self) -> str:
        return f"state/arm/{self.side}"

    @property
    def wrench_sub_topic(self) -> str:
        return f"state/wrench/{self.side}"

    @property
    def wrist_button_sub_topic(self) -> str:
        return f"state/wrist_button/{self.side}"

    @property
    def control_pub_topic(self) -> str:
        return f"control/arm/{self.side}"

    @property
    def set_mode_query(self) -> str:
        return f"mode/arm/{self.side}"

    @property
    def pid_query(self) -> str:
        return f"system/arm_pid/{self.side}"

    @property
    def brake_query(self) -> str:
        return f"system/arm_brake/{self.side}"

    @property
    def ee_baud_rate_query(self) -> str:
        return f"system/ee_baud_rate/{self.side}"

    @property
    def force_torque_sensor_query(self) -> str:
        return f"mode/force_torque_sensor/{self.side}"

    @property
    def ee_pass_through_pub_topic(self) -> str:
        return f"control/ee_pass_through/{self.side}"

    @property
    def ee_pass_through_state_sub_topic(self) -> str:
        return f"state/ee_pass_through/{self.side}"
