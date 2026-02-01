"""Internal helper functions for robot configuration."""

from functools import lru_cache


@lru_cache(maxsize=None)
def build_robot_type_abbreviation_mapping() -> dict[str, str]:
    """Build robot type abbreviation mapping from registered configurations.

    Scans all registered robot configurations and builds a mapping of abbreviations
    to robot types (e.g., "vg" -> "vega").

    Note: vega_1u (upper body) uses "vg" abbreviation with "u" suffix in robot name
    (e.g., "dm/vg0123456789-1u" for vega upper body).

    Returns:
        Dictionary mapping abbreviations to robot types.

    Example:
        >>> mapping = build_robot_type_abbreviation_mapping()
        >>> print(mapping)
        {'vg': 'vega'}
    """
    from .configs import ConfigRegistry

    mapping = {}

    # Get all registered variants
    for variant in ConfigRegistry.list_variants():
        config = ConfigRegistry.get(variant)

        # Get abbreviation and robot_model from config
        abbr = config.abbr
        robot_model = config.robot_model

        # Extract robot type from robot_model (e.g., "vega_1" -> "vega")
        # Split on '_' and take everything except the last part (version)
        parts = robot_model.rsplit("_", 1)
        robot_type = parts[0] if len(parts) > 1 else robot_model

        # Add to mapping if not already present
        if abbr not in mapping:
            mapping[abbr] = robot_type

    return mapping


# Build once at module load
ROBOT_TYPE_ABB_MAPPING = build_robot_type_abbreviation_mapping()
