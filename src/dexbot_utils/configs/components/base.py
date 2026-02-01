"""Base component configuration classes."""

from dataclasses import MISSING, dataclass, field


@dataclass
class BaseComponentConfig:
    """Base configuration for a generic robot component.

    Attributes:
        enabled: Whether the component is enabled in the configuration
    """

    enabled: bool = True


@dataclass
class BaseJointComponentConfig(BaseComponentConfig):
    """Base configuration for a robot component with joints.

    Subclasses must define a `joints` attribute, either as a dataclass field
    or as a property that computes joint names dynamically.

    Attributes:
        joints: List of joint names for the component (field or property)
    """

    joints: list[str] = field(default=MISSING, init=False)
