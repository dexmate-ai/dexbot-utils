# Copyright (C) 2025 Dexmate Inc.
#
# This software is dual-licensed:
#
# 1. GNU Affero General Public License v3.0 (AGPL-3.0)
#    See LICENSE-AGPL for details
#
# 2. Commercial License
#    For commercial licensing terms, contact: contact@dexmate.ai

"""Configuration for ZED camera sensor."""

from dataclasses import dataclass

from ...base import BaseComponentConfig
from .camera import CameraConfig


@dataclass
class ZedXCameraConfig(BaseComponentConfig):
    """Configuration for ZED stereo camera sensor.

    This configuration supports multi-stream ZED cameras with independent
    transport settings for each RGB stream. Depth always uses Zenoh.

    Attributes:
        name: Camera name identifier
        transport: Transport protocol for RGB streams ("zenoh" or "rtc")
        enable_rgb: Enable left and right RGB stream capture
        enable_depth: Enable depth stream capture
        left_rgb: Property returning left RGB camera configuration
        right_rgb: Property returning right RGB camera configuration
        depth: Property returning depth camera configuration
    """

    enabled: bool = False
    name: str = "head_camera"
    transport: str = "zenoh"
    enable_rgb: bool = True
    enable_depth: bool = True

    @property
    def left_rgb(self) -> CameraConfig:
        return CameraConfig(
            enabled=self.enabled & self.enable_rgb,
            name="left_rgb",
            transport=self.transport,
            topic=f"sensors/{self.name}/left_rgb",
            rtc_channel=f"sensors/{self.name}/left_rgb_rtc",
        )

    @property
    def right_rgb(self) -> CameraConfig:
        return CameraConfig(
            enabled=self.enabled & self.enable_rgb,
            name="right_rgb",
            transport=self.transport,
            topic=f"sensors/{self.name}/right_rgb",
            rtc_channel=f"sensors/{self.name}/right_rgb_rtc",
        )

    @property
    def depth(self) -> CameraConfig:
        return CameraConfig(
            enabled=self.enabled & self.enable_depth,
            name="depth",
            transport=self.transport,
            topic=f"sensors/{self.name}/depth",
        )


@dataclass
class ZedXOneCameraConfig(BaseComponentConfig):
    """Configuration for ZED stereo camera sensor.

    This configuration supports multi-stream ZED cameras with independent
    transport settings for each RGB stream. Depth always uses Zenoh.

    Attributes:
        name: Camera name identifier
        transport: Transport protocol for RGB streams ("zenoh" or "rtc")
        enable_rgb: Enable left and right RGB stream capture
        enable_depth: Enable depth stream capture
        left_rgb: Property returning left RGB camera configuration
        right_rgb: Property returning right RGB camera configuration
        depth: Property returning depth camera configuration
    """

    enabled: bool = False
    name: str = "wrist_camera"
    transport: str = "zenoh"
    side: str = "left"

    @property
    def rgb(self) -> CameraConfig:
        return CameraConfig(
            enabled=self.enabled,
            name="rgb",
            transport=self.transport,
            topic=f"sensors/{self.side}_{self.name}/rgb",
            rtc_channel=f"sensors/{self.name}/rgb_rtc",
        )
