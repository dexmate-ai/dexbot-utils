"""
dexbot-utils: Common robot utility classes and functions.

This package provides utilities for:
- Robot identification and configuration
- Component validation
- URDF parsing for joint information
"""

__version__ = "0.1.0"

# Main interface
from .hand import HandType
from .robot_info import RobotInfo

# URDF utilities
from .urdf_utils import (
    get_joint_names,
    get_movable_joint_names,
    parse_urdf,
)

# Component validation
from .validators import (
    ComponentValidationError,
    has_all_components,
    has_component,
    validate_component,
    validate_components,
)

__all__ = [
    # Version
    "__version__",
    # Main interface
    "RobotInfo",
    # Validators
    "ComponentValidationError",
    "validate_component",
    "validate_components",
    "has_component",
    "has_all_components",
    # Hand
    "HandType",
    # URDF utilities
    "parse_urdf",
    "get_joint_names",
    "get_movable_joint_names",
]
