# Copyright (C) 2025 Dexmate Inc.
#
# This software is dual-licensed:
#
# 1. GNU Affero General Public License v3.0 (AGPL-3.0)
#    See LICENSE-AGPL for details
#
# 2. Commercial License
#    For commercial licensing terms, contact: contact@dexmate.ai

"""Configuration for USB camera sensor."""

from dataclasses import dataclass

from ...base import BaseComponentConfig


@dataclass
class CameraConfig(BaseComponentConfig):
    """Configuration for camera sensor.

    This configuration supports both Zenoh (reliable, JPEG compressed) and
    RTC (low latency, hardware accelerated video) transports.

    Attributes:
        name: Camera name identifier
        transport: Transport protocol ("zenoh" or "rtc")
        topic: Topic for camera data publishing
        rtc_channel: Channel name for RTC transport
    """

    enabled: bool = False

    name: str = "usb_camera"
    transport: str = "zenoh"  # or "rtc"
    topic: str = "sensors/camera/rgb"

    rtc_channel: str = "sensors/camera/rgb_rtc"
