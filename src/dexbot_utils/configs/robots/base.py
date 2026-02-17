from dataclasses import dataclass, field

from ..components.base import BaseComponentConfig


@dataclass
class BaseRobotConfig:
    """Base configuration for a robot.

    Attributes:
        robot_model: Robot model name (e.g., "vega_1", "vega_1u")
        abbr: Robot abbreviation (e.g., "vg")
        urdf_path: Path to URDF file (relative or absolute)
        components: Robot components (arm, hand, torso, etc.), enabled by default.
        sensors: Sensor components (cameras, IMU, lidar, etc.), disabled by
                 default and must be explicitly enabled by the user.
        querables: Dictionary of queryable service names
    """

    robot_model: str = ""
    abbr: str = ""
    urdf_path: str = ""

    components: dict[str, BaseComponentConfig] = field(default_factory=dict)
    sensors: dict[str, BaseComponentConfig] = field(default_factory=dict)
    querables: dict[str, str] = field(default_factory=dict)

    def has_sensor(self, name: str) -> bool:
        """Check if a sensor is available on this robot configuration.

        Args:
            name: The sensor name (e.g., "ultrasonic", "head_imu").

        Returns:
            True if the sensor exists in the configuration.
        """
        return name in self.sensors

    def enable_sensor(self, name: str) -> None:
        """Enable a sensor by name.

        Args:
            name: The sensor name (e.g., "ultrasonic", "head_imu").

        Raises:
            KeyError: If the sensor is not available on this robot configuration.
        """
        if name not in self.sensors:
            available = ", ".join(sorted(self.sensors.keys())) or "none"
            raise KeyError(
                f"Sensor '{name}' is not available on robot config '{type(self).__name__}'. "
                f"Available sensors: {available}"
            )
        self.sensors[name].enabled = True
