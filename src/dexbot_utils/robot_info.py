"""High-level robot information interface.

This module provides the RobotInfo class, which combines robot configuration
and URDF data to provide a comprehensive robot information API.
"""

import os
from pathlib import Path
from typing import Any

import numpy as np
from loguru import logger

from ._helpers import ROBOT_TYPE_ABB_MAPPING
from .configs import get_robot_config
from .configs.components import BaseComponentConfig
from .configs.robots.base import BaseRobotConfig
from .constants import (
    ROBOT_CONFIG_FILE_ENV_VAR,
    ROBOT_NAME_ENV_VAR,
    ROBOT_NAME_PATTERN,
)
from .urdf_utils import URDFParser


class RobotInfo:
    """High-level interface for robot configuration and URDF information.

    Clean API that auto-loads dataclass configurations:
    - RobotInfo() -> reads from ROBOT_CONFIG env var
    - RobotInfo("vega_1") -> loads vega_1 dataclass config
    - RobotInfo(variant="vega_1") -> explicit variant specification

    URDF is automatically loaded from config's urdf_path if present.

    Examples:
        >>> # From environment (ROBOT_CONFIG env var)
        >>> robot = RobotInfo()

        >>> # From variant name
        >>> robot = RobotInfo("vega_1")

        >>> # Access components
        >>> robot.has_left_arm
        True
        >>> robot.get_component_list()
        ['left_arm', 'right_arm', 'torso', 'chassis', 'head', ...]
    """

    def __init__(
        self,
        variant: str | None = None,
    ):
        """Initialize RobotInfo with variant name.

        Args:
            variant: Robot variant name (e.g., "vega_1", "vega_1u").
                    If None, reads from ROBOT_CONFIG or ROBOT_NAME env vars.
            robot_model: Alias for variant parameter (for compatibility).

        Raises:
            ValueError: If neither parameter provided and no env vars set.
        """
        # Initialize internal state
        self._urdf_parser: URDFParser | None = None
        self._urdf_path: Path | None = None
        self._robot_name_from_env: str = os.getenv(ROBOT_NAME_ENV_VAR, "")

        # Resolve variant name
        if variant is None:
            variant = self._resolve_variant_from_env()

        # Load dataclass configuration
        self._config: BaseRobotConfig = get_robot_config(variant)

        # Auto-load URDF if specified in config
        if self._config.urdf_path:
            try:
                self.load_urdf(self._config.urdf_path)
            except FileNotFoundError:
                # URDF path specified but file doesn't exist - that's OK
                logger.warning(
                    f"URDF file not found: {self._config.urdf_path}, "
                    "continuing without URDF"
                )
        else:
            logger.debug("No URDF path specified in config")

    def _resolve_variant_from_env(self) -> str:
        """Resolve variant name from environment variables.

        Priority:
            1. ROBOT_CONFIG env var
            2. ROBOT_NAME env var (derives variant from robot name)

        Returns:
            Variant name

        Raises:
            ValueError: If no env vars set
        """
        # Try ROBOT_CONFIG first
        variant = os.getenv(ROBOT_CONFIG_FILE_ENV_VAR)
        if variant:
            # Remove .py extension if present
            return variant.replace(".py", "")

        # Fallback to ROBOT_NAME
        if self._robot_name_from_env:
            return self._derive_variant_from_robot_name(self._robot_name_from_env)

        # No env vars set
        raise ValueError(
            f"Variant not specified and neither {ROBOT_CONFIG_FILE_ENV_VAR} "
            f"nor {ROBOT_NAME_ENV_VAR} environment variables are set. "
            f"Either provide variant parameter or set one of these environment variables."
        )

    @staticmethod
    def _derive_variant_from_robot_name(robot_name: str) -> str:
        """Derive variant from ROBOT_NAME env var.

        Parses robot name to extract variant:
        - "dm/vgabcd123456-1" -> "vega_1"
        - "dm/vg0123456789-1p" -> "vega_1p"
        - "dm/vg0123456789-1u" -> "vega_1u"

        Args:
            robot_name: Robot name from ROBOT_NAME env var

        Returns:
            Variant name (e.g., "vega_1", "vega_1p", "vega_1u")

        Raises:
            ValueError: If robot name format is invalid
        """
        if not ROBOT_NAME_PATTERN.match(robot_name):
            raise ValueError(
                f"Invalid robot name format: {robot_name}. "
                f"Expected format: dm/[abbreviation][serial]-[version][suffix] "
                f"(e.g., 'dm/vgabcd123456-1', 'dm/vg0123456789-1p', 'dm/vg0123456789-1u')"
            )

        # Parse: "dm/vgabcd123456-1p" -> ["dm", "vgabcd123456", "1p"]
        parts = robot_name.replace("/", "-").split("-")
        robot_abbr = parts[1][:2]  # First 2 chars: "vg"
        version_part = parts[-1]  # Last part: "1", "1p", "1u", or "rc2"

        # Look up robot type from abbreviation
        if robot_abbr not in ROBOT_TYPE_ABB_MAPPING:
            raise ValueError(
                f"Unknown robot abbreviation: {robot_abbr}. "
                f"Valid abbreviations: {list(ROBOT_TYPE_ABB_MAPPING.keys())}"
            )

        robot_type = ROBOT_TYPE_ABB_MAPPING[robot_abbr]

        # Build variant name with underscore separator
        # "1p" -> "vega_1p", "1" -> "vega_1", "1u" -> "vega_1u", "rc2" -> "vega_rc2"
        return f"{robot_type}_{version_part}"

    # =========================================================================
    # Config Properties
    # =========================================================================

    @property
    def config(self) -> BaseRobotConfig:
        """Get the robot configuration dataclass.

        Returns:
            Robot configuration instance
        """
        return self._config

    @property
    def robot_name(self) -> str:
        """Get robot name if available.

        Returns robot name if it was loaded via ROBOT_NAME environment variable.
        Returns None if config was loaded via ROBOT_CONFIG or explicit parameter.

        Returns:
            Robot name (e.g., "dm/vgabcd123456-1") or None
        """
        return self._robot_name_from_env

    @property
    def robot_model(self) -> str:
        """Get robot model from config."""
        return self._config.robot_model

    @property
    def robot_type(self) -> str:
        """Get robot type from model name.

        Extracts type from model (e.g., "vega_1" -> "vega").
        """
        parts = self._config.robot_model.rsplit("_", 1)
        return parts[0] if len(parts) > 1 else self._config.robot_model

    @property
    def robot_version(self) -> str:
        """Get robot version from model name.

        Extracts version from model (e.g., "vega_1" -> "1").
        """
        parts = self._config.robot_model.rsplit("_", 1)
        return parts[1] if len(parts) > 1 else ""

    # =========================================================================
    # URDF Management
    # =========================================================================

    def load_urdf(self, urdf_path: str | Path) -> None:
        """Load URDF file.

        Args:
            urdf_path: Path to URDF file (relative or absolute)
        """
        urdf_path = Path(urdf_path)
        if not urdf_path.is_absolute():
            import dexmate_urdf

            dexmate_urdf_path = Path(dexmate_urdf.__path__[0])
            urdf_path = dexmate_urdf_path / urdf_path

        self._urdf_path = urdf_path
        self._urdf_parser = URDFParser(self._urdf_path)

    @property
    def has_urdf(self) -> bool:
        """Check if URDF is loaded."""
        return self._urdf_parser is not None

    @property
    def urdf_path(self) -> Path | None:
        """Get path to loaded URDF file."""
        return self._urdf_path

    # =========================================================================
    # Component Access
    # =========================================================================

    def get_component_list(self) -> list[str]:
        """Get list of component names for this robot.

        Returns:
            List of component names (e.g., ['left_arm', 'right_arm', ...])
        """
        return list(self._config.components.keys())

    def has_component(self, component: str) -> bool:
        """Check if robot has a specific component.

        Args:
            component: Component name to check

        Returns:
            True if component exists
        """
        return component in self._config.components

    def get_component_config(self, component: str) -> BaseComponentConfig:
        """Get configuration for a specific component.

        Args:
            component: Component name

        Returns:
            Component configuration dataclass

        Raises:
            KeyError: If component doesn't exist
        """
        if component not in self._config.components:
            raise KeyError(
                f"Component '{component}' not found. "
                f"Available: {list(self._config.components.keys())}"
            )
        return self._config.components[component]

    def get_component_dof(self, component: str) -> int:
        """Get degrees of freedom for a component.

        Args:
            component: Component name

        Returns:
            Number of joints in the component

        Raises:
            KeyError: If component doesn't exist
            AttributeError: If component doesn't have joints
        """
        comp_config = self.get_component_config(component)
        if not hasattr(comp_config, "joints"):
            raise AttributeError(
                f"Component '{component}' does not have joints attribute"
            )
        return len(comp_config.joints)

    def get_component_joints(self, component: str) -> list[str]:
        """Get joint names for a component.

        Args:
            component: Component name

        Returns:
            List of joint names

        Raises:
            KeyError: If component doesn't exist
            AttributeError: If component doesn't have joints
        """
        comp_config = self.get_component_config(component)

        # Handle different component types
        if hasattr(comp_config, "joints"):
            return list(comp_config.joints)
        elif hasattr(comp_config, "steer_joints") or hasattr(
            comp_config, "drive_joints"
        ):
            # Chassis special case
            joints = []
            if hasattr(comp_config, "steer_joints"):
                joints.extend(comp_config.steer_joints)
            if hasattr(comp_config, "drive_joints"):
                joints.extend(comp_config.drive_joints)
            return joints
        else:
            raise AttributeError(f"Component '{component}' does not have joints")

    def get_component_parameter(self, component: str, param: str) -> Any:
        """Get a parameter value from a component's configuration.

        Args:
            component: Component name
            param: Parameter name

        Returns:
            Parameter value

        Raises:
            KeyError: If component or parameter doesn't exist
        """
        comp_config = self.get_component_config(component)

        if hasattr(comp_config, param):
            return getattr(comp_config, param)

        raise KeyError(f"Parameter '{param}' not found for component '{component}'")

    def get_pv_components(self) -> list[str]:
        """Get list of components in PV (position-velocity) mode.

        Returns:
            List of component names that use PV mode
        """
        pv_components = []
        for name, comp_config in self._config.components.items():
            if hasattr(comp_config, "pv_mode") and comp_config.pv_mode:
                pv_components.append(name)
        return pv_components

    # =========================================================================
    # Component Boolean Properties
    # =========================================================================

    @property
    def has_left_arm(self) -> bool:
        """Check if robot has left arm."""
        return self.has_component("left_arm")

    @property
    def has_right_arm(self) -> bool:
        """Check if robot has right arm."""
        return self.has_component("right_arm")

    @property
    def has_left_hand(self) -> bool:
        """Check if robot has left hand."""
        return self.has_component("left_hand")

    @property
    def has_right_hand(self) -> bool:
        """Check if robot has right hand."""
        return self.has_component("right_hand")

    @property
    def has_torso(self) -> bool:
        """Check if robot has torso."""
        return self.has_component("torso")

    @property
    def has_head(self) -> bool:
        """Check if robot has head."""
        return self.has_component("head")

    @property
    def has_chassis(self) -> bool:
        """Check if robot has chassis."""
        return self.has_component("chassis")

    # =========================================================================
    # URDF Queries (delegated to URDFParser)
    # =========================================================================

    def get_joint_names(self, joint_type: str | None = None) -> list[str]:
        """Get joint names from URDF.

        Args:
            joint_type: Optional joint type filter (e.g., "revolute", "prismatic")

        Returns:
            List of joint names

        Raises:
            RuntimeError: If URDF not loaded
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")
        return self._urdf_parser.get_joint_names(joint_type)

    def get_movable_joint_names(self) -> list[str]:
        """Get names of movable joints from URDF.

        Returns:
            List of movable joint names

        Raises:
            RuntimeError: If URDF not loaded
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")
        return self._urdf_parser.get_movable_joint_names()

    def get_joint_limits(
        self, joint_names: list[str] | None = None
    ) -> dict[str, dict[str, float]]:
        """Get joint limits from URDF.

        Args:
            joint_names: Optional list of joint names to get limits for

        Returns:
            Dictionary mapping joint names to limit dictionaries

        Raises:
            RuntimeError: If URDF not loaded
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")
        return self._urdf_parser.get_joint_limits(joint_names)

    def get_joint_pos_limits(self, joint_names: list[str] | None = None) -> np.ndarray:
        """Get position limits (lower, upper) for joints from URDF.

        Args:
            joint_names: Optional list of joint names to get limits for.
                        If None, returns limits for all movable joints.

        Returns:
            Numpy array of shape (N, 2) where N is number of joints.
            Each row is [lower, upper] for a joint.

        Raises:
            RuntimeError: If URDF not loaded

        Example:
            >>> robot = RobotInfo("vega_1")
            >>> pos_limits = robot.get_joint_pos_limits()
            >>> pos_limits.shape
            (3, 2)
            >>> pos_limits[0]  # First joint limits
            array([-1.57,  1.57])
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")

        limits = self._urdf_parser.get_joint_limits(joint_names)
        pos_limits = [
            [limit.lower, limit.upper]
            for limit in limits.values()
            if limit.lower is not None and limit.upper is not None
        ]
        return np.array(pos_limits)

    def get_joint_vel_limits(self, joint_names: list[str] | None = None) -> np.ndarray:
        """Get velocity limits for joints from URDF.

        Args:
            joint_names: Optional list of joint names to get limits for.
                        If None, returns limits for all movable joints.

        Returns:
            Numpy array of shape (N,) where N is number of joints.
            Each element is the velocity limit for a joint.

        Raises:
            RuntimeError: If URDF not loaded

        Example:
            >>> robot = RobotInfo("vega_1")
            >>> vel_limits = robot.get_joint_vel_limits()
            >>> vel_limits.shape
            (3,)
            >>> vel_limits[0]  # First joint velocity limit
            1.0
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")

        limits = self._urdf_parser.get_joint_limits(joint_names)
        vel_limits = [
            limit.velocity for limit in limits.values() if limit.velocity is not None
        ]
        return np.array(vel_limits)

    def get_joint_effort_limits(
        self, joint_names: list[str] | None = None
    ) -> np.ndarray:
        """Get effort limits for joints from URDF.

        Args:
            joint_names: Optional list of joint names to get limits for.
                        If None, returns limits for all movable joints.

        Returns:
            Numpy array of shape (N,) where N is number of joints.
            Each element is the effort limit for a joint.

        Raises:
            RuntimeError: If URDF not loaded

        Example:
            >>> robot = RobotInfo("vega_1")
            >>> effort_limits = robot.get_joint_effort_limits()
            >>> effort_limits.shape
            (3,)
            >>> effort_limits[0]  # First joint effort limit
            100.0
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")

        limits = self._urdf_parser.get_joint_limits(joint_names)
        effort_limits = [
            limit.effort for limit in limits.values() if limit.effort is not None
        ]
        return np.array(effort_limits)

    def get_link_names(self) -> list[str]:
        """Get link names from URDF.

        Returns:
            List of link names

        Raises:
            RuntimeError: If URDF not loaded
        """
        if not self.has_urdf:
            raise RuntimeError("URDF not loaded. Call load_urdf() first.")
        return self._urdf_parser.get_link_names()

    # =========================================================================
    # Utility Methods
    # =========================================================================

    def __repr__(self) -> str:
        """String representation."""
        urdf_status = f"urdf={self._urdf_path.name if self.has_urdf else 'not loaded'}"
        return (
            f"RobotInfo("
            f"model='{self.robot_model}', "
            f"components={len(self._config.components)}, "
            f"{urdf_status})"
        )
