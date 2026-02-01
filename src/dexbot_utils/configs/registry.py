"""Configuration registry for robot variants."""

from typing import Callable, ClassVar, TypeVar

from .robots.base import BaseRobotConfig

T = TypeVar("T", bound=BaseRobotConfig)


def register_variant(variant: str) -> Callable[[type[T]], type[T]]:
    """Decorator to register a robot configuration variant.

    Use this decorator on robot config dataclasses to auto-register them.

    Args:
        variant: Variant name (e.g., "vega_1", "vega_1u", "vega_1p")

    Returns:
        Decorator function

    Example:
        @register_variant("vega_1")
        @dataclass
        class Vega1Config(BaseRobotConfig):
            robot_model: str = "vega_1"
            ...
    """

    def decorator(config_class: type[T]) -> type[T]:
        ConfigRegistry.register(variant, config_class)
        return config_class

    return decorator


class ConfigRegistry:
    """Registry for robot configuration variants.

    Provides a central registry for managing robot configuration classes
    and factory methods for retrieving configurations.
    """

    _registry: ClassVar[dict[str, type[BaseRobotConfig]]] = {}

    @classmethod
    def register(cls, variant: str, config_class: type[BaseRobotConfig]) -> None:
        """Register a robot configuration variant.

        Args:
            variant: Variant name (e.g., "vega_1", "vega_1u")
            config_class: Configuration dataclass for the variant
        """
        cls._registry[variant] = config_class

    @classmethod
    def get(cls, variant: str) -> BaseRobotConfig:
        """Get a robot configuration instance.

        Args:
            variant: Variant name

        Returns:
            Configuration instance for the variant

        Raises:
            ValueError: If variant is not registered
        """
        if variant not in cls._registry:
            raise ValueError(
                f"Unknown robot variant: '{variant}'. "
                f"Available variants: {list(cls._registry.keys())}"
            )

        config_class = cls._registry[variant]
        return config_class()

    @classmethod
    def list_variants(cls) -> list[str]:
        """List all registered robot variants.

        Returns:
            List of variant names
        """
        return list(cls._registry.keys())


def get_robot_config(variant: str) -> BaseRobotConfig:
    """Get robot configuration for a specific variant.

    Convenience function that delegates to ConfigRegistry.get().

    Args:
        variant: Robot variant name (e.g., "vega_1")

    Returns:
        Configuration instance

    Raises:
        ValueError: If variant is not registered
    """
    return ConfigRegistry.get(variant)


def get_available_variants() -> list[str]:
    """Get list of available robot variants.

    Convenience function that delegates to ConfigRegistry.list_variants().

    Returns:
        List of registered variant names
    """
    return ConfigRegistry.list_variants()
