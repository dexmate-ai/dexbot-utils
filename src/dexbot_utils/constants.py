"""Constants and configuration settings for dexbot-utils."""

import re

# Environment variable configuration
ROBOT_NAME_ENV_VAR = "ROBOT_NAME"

ROBOT_CONFIG_FILE_ENV_VAR = "ROBOT_CONFIG"

# Compiled regex pattern for robot name validation (performance optimization)
# Format: dm/[abbr][serial]-[version][suffix]
# Examples: dm/vgabcd123456-1, dm/vg0123456789-1p (p=pro), dm/vg0123456789-1u (u=upper)
ROBOT_NAME_PATTERN = re.compile(r"^dm/[a-zA-Z0-9]{12}-(?:\d+[a-z]?|rc\d+)$")

# Version suffix to variant suffix mapping
VERSION_SUFFIX_MAPPING = {
    "p": "pro",
    "u": "upper",
}
