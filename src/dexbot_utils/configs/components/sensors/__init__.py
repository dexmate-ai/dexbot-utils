"""Vega-specific component configuration dataclasses.

This module contains Vega robot family specific component configurations.
"""

from .cameras import CameraConfig, ZedXCameraConfig, ZedXOneCameraConfig
from .imu import ChassisIMUConfig, ZedIMUConfig
from .lidar import Lidar3DConfig, RPLidarConfig
from .ultrasonic import UltraSonicConfig

__all__ = [
    "CameraConfig",
    "ZedXCameraConfig",
    "ZedXOneCameraConfig",
    "ChassisIMUConfig",
    "ZedIMUConfig",
    "Lidar3DConfig",
    "RPLidarConfig",
    "UltraSonicConfig",
]
