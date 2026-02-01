"""Robot configuration system using dataclasses."""

from .components import BaseComponentConfig
from .registry import (
    ConfigRegistry,
    get_available_variants,
    get_robot_config,
    register_variant,
)
from .robots.base import BaseRobotConfig
from .robots.vega_1 import Vega1Config, Vega1DGripperConfig, Vega1F5D6Config
from .robots.vega_1p import Vega1pConfig, Vega1pDGripperConfig, Vega1pF5D6Config
from .robots.vega_1u import Vega1UConfig, Vega1UDGripperConfig, Vega1UF5D6Config

__all__ = [
    "ConfigRegistry",
    "register_variant",
    "get_robot_config",
    "get_available_variants",
    "BaseRobotConfig",
    "BaseComponentConfig",
    "Vega1Config",
    "Vega1DGripperConfig",
    "Vega1F5D6Config",
    "Vega1UConfig",
    "Vega1UDGripperConfig",
    "Vega1UF5D6Config",
    "Vega1pConfig",
    "Vega1pDGripperConfig",
    "Vega1pF5D6Config",
]
