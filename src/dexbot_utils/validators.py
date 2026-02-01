"""Validation utilities for robot components and configurations."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .robot_info import RobotInfo


class ComponentValidationError(Exception):
    """Exception raised when component validation fails."""

    pass


def validate_component(
    component: str,
    robot_config: RobotInfo | None = None,
    raise_on_missing: bool = True,
) -> bool:
    """Validate that a robot has a specific component.

    Args:
        component: The component name to validate (e.g., "left_arm", "chassis").
        robot_config: RobotInfo instance. If None, created from environment.
        raise_on_missing: If True, raises exception when component is missing.

    Returns:
        True if robot has the component.

    Raises:
        ComponentValidationError: If component is missing and raise_on_missing is True.
    """
    # Get robot configuration
    if robot_config is None:
        from .robot_info import RobotInfo

        robot_config = RobotInfo()

    components = robot_config.get_component_list()
    robot_type = robot_config.robot_type

    # Check if component exists
    has_component = component in components

    if not has_component and raise_on_missing:
        raise ComponentValidationError(
            f"Robot '{robot_type}' does not have component '{component}'. "
            f"Available components: {components}"
        )

    return has_component


def validate_components(
    components: list[str],
    robot_config: RobotInfo | None = None,
    raise_on_missing: bool = True,
) -> bool:
    """Validate that a robot has all specified components.

    Args:
        components: List of component names to validate.
        robot_config: RobotInfo instance. If None, created from environment.
        raise_on_missing: If True, raises exception when any component is missing.

    Returns:
        True if robot has all components.

    Raises:
        ComponentValidationError: If any component is missing and raise_on_missing is True.
    """
    # Get robot configuration
    if robot_config is None:
        from .robot_info import RobotInfo

        robot_config = RobotInfo()

    available = robot_config.get_component_list()
    robot_type = robot_config.robot_type

    # Check all components
    missing = [comp for comp in components if comp not in available]

    if missing and raise_on_missing:
        raise ComponentValidationError(
            f"Robot '{robot_type}' is missing components: {missing}. "
            f"Available components: {available}"
        )

    return len(missing) == 0


def require_component(component: str) -> None:
    """Decorator-friendly function to require a component.

    Args:
        component: The required component name.

    Raises:
        ComponentValidationError: If component is not available.
    """
    validate_component(component, raise_on_missing=True)


def require_components(components: list[str]) -> None:
    """Decorator-friendly function to require multiple components.

    Args:
        components: List of required component names.

    Raises:
        ComponentValidationError: If any component is not available.
    """
    validate_components(components, raise_on_missing=True)


def has_component(
    component: str,
    robot_config: RobotInfo | None = None,
) -> bool:
    """Check if robot has a component without raising exceptions.

    Args:
        component: The component name to check.
        robot_config: RobotInfo instance. If None, created from environment.

    Returns:
        True if robot has the component, False otherwise.
    """
    return validate_component(component, robot_config, raise_on_missing=False)


def has_all_components(
    components: list[str],
    robot_config: RobotInfo | None = None,
) -> bool:
    """Check if robot has all specified components without raising exceptions.

    Args:
        components: List of component names to check.
        robot_config: RobotInfo instance. If None, created from environment.

    Returns:
        True if robot has all components, False otherwise.
    """
    return validate_components(components, robot_config, raise_on_missing=False)


def has_any_component(
    components: list[str],
    robot_config: RobotInfo | None = None,
) -> bool:
    """Check if robot has at least one of the specified components.

    Args:
        components: List of component names to check.
        robot_config: RobotInfo instance. If None, created from environment.

    Returns:
        True if robot has at least one component, False otherwise.
    """
    for component in components:
        if has_component(component, robot_config):
            return True
    return False


def get_missing_components(
    required: list[str],
    robot_config: RobotInfo | None = None,
) -> list[str]:
    """Get list of missing components from required list.

    Args:
        required: List of required component names.
        robot_config: RobotInfo instance. If None, created from environment.

    Returns:
        List of missing component names as strings.
    """
    # Get robot configuration
    if robot_config is None:
        from .robot_info import RobotInfo

        robot_config = RobotInfo()

    available = robot_config.get_component_list()

    # Check for missing components
    missing = [comp for comp in required if comp not in available]

    return missing
