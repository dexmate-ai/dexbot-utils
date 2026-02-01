"""Hand and gripper type enumeration."""

from enum import Enum


class HandType(Enum):
    """Enumeration of supported hand and gripper types.

    Attributes:
        UNKNOWN: Unknown or unspecified hand type
        HandF5D6_V1: F5D6 hand version 1
        HandF5D6_V2: F5D6 hand version 2 with touch sensors
        DexGripper: Dex gripper (single or double)
    """

    UNKNOWN = "UNKNOWN"
    HandF5D6_V1 = "HandF5D6_V1"
    HandF5D6_V2 = "HandF5D6_V2"
    DexGripper = "DexGripper"
