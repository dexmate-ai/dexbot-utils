# Copyright (C) 2025 Dexmate Inc.
#
# This software is dual-licensed:
#
# 1. GNU Affero General Public License v3.0 (AGPL-3.0)
#    See LICENSE-AGPL for details
#
# 2. Commercial License
#    For commercial licensing terms, contact: contact@dexmate.ai

from dataclasses import dataclass

from ...base import BaseComponentConfig


@dataclass
class RPLidarConfig(BaseComponentConfig):
    enabled: bool = False
    topic: str = "sensors/lidar_front/scan"
    name: str = "rplidar"
