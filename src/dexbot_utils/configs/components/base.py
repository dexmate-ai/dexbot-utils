"""Base component configuration classes."""

from abc import ABC, abstractmethod
from dataclasses import dataclass


@dataclass
class BaseComponentConfig:
    """Base configuration for a generic robot component.

    Robot components (arm, hand, torso, head, chassis, etc.) inherit the
    default ``enabled=True``. Sensor components override this to
    ``enabled=False`` so they must be explicitly enabled by the user.

    Attributes:
        enabled: Whether the component is enabled in the configuration.
                 Defaults to True for robot components, overridden to
                 False in all sensor configs.
    """

    enabled: bool = True


@dataclass
class BaseJointComponentConfig(BaseComponentConfig, ABC):
    """Base configuration for a robot component with joints.

    Subclasses must implement the `joints` property to return
    the list of joint names for the component.
    """

    @property
    @abstractmethod
    def joints(self) -> list[str]:
        """List of joint names for the component."""
        ...
