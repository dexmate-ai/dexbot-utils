from dataclasses import dataclass, field

from ..components.base import BaseComponentConfig


@dataclass
class BaseRobotConfig:
    """Base configuration for a robot.

    Attributes:
        robot_model: Robot model name (e.g., "vega_1", "vega_1u")
        abbr: Robot abbreviation (e.g., "vg")
        urdf_path: Path to URDF file (relative or absolute)
        components: Dictionary of component configurations
        querables: Dictionary of queryable service names
    """

    robot_model: str = ""
    abbr: str = ""
    urdf_path: str = ""

    components: dict[str, BaseComponentConfig] = field(default_factory=dict)
    sensors: dict[str, BaseComponentConfig] = field(default_factory=dict)
    querables: dict[str, str] = field(default_factory=dict)
