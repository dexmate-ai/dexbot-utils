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
class Lidar3DConfig(BaseComponentConfig):
    """Configuration for 3D LIDAR point cloud sensor."""

    enabled: bool = False
    topic: str = ""
    name: str = "lidar_3d"

    def __post_init__(self) -> None:
        if not self.topic:
            self.topic = f"sensors/{self.name}/point_cloud"
