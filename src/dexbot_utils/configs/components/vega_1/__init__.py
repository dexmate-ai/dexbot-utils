"""Vega-specific component configuration dataclasses.

This module contains Vega robot family specific component configurations.
"""

from .arm import Vega1ArmConfig
from .chassis import Vega1ChassisConfig
from .hand import (
    DexDGripperConfig,
    DexSGripperConfig,
    F5D6HandV1Config,
    F5D6HandV2Config,
)
from .head import Vega1HeadConfig
from .misc import BatteryConfig, EStopConfig, HeartbeatConfig
from .torso import Vega1TorsoConfig

__all__ = [
    "Vega1ArmConfig",
    "Vega1ChassisConfig",
    "F5D6HandV1Config",
    "F5D6HandV2Config",
    "DexSGripperConfig",
    "DexDGripperConfig",
    "Vega1HeadConfig",
    "Vega1TorsoConfig",
    "BatteryConfig",
    "EStopConfig",
    "HeartbeatConfig",
]
