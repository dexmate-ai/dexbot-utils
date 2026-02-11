"""Base component configuration classes."""

from abc import ABC, abstractmethod
from dataclasses import dataclass


@dataclass
class BaseComponentConfig:
    """Base configuration for a generic robot component.

    Attributes:
        enabled: Whether the component is enabled in the configuration
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
