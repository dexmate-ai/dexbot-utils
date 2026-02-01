"""Miscellaneous component configurations (battery, estop, heartbeat)."""

from dataclasses import dataclass

from ..base import BaseComponentConfig


@dataclass
class BatteryConfig(BaseComponentConfig):
    """Configuration for battery management system.

    Attributes:
        state_sub_topic: Topic for battery state feedback
    """

    state_sub_topic: str = "state/bms"


@dataclass
class EStopConfig(BaseComponentConfig):
    """Configuration for emergency stop component.

    Can be disabled via DEXCONTROL_DISABLE_ESTOP_CHECKING=1 environment variable.

    Attributes:
        state_sub_topic: Topic for emergency stop state feedback
        estop_query_name: Service name for emergency stop queries
    """

    state_sub_topic: str = "state/estop"
    estop_query_name: str = "system/estop"


@dataclass
class HeartbeatConfig(BaseComponentConfig):
    """Configuration for system heartbeat monitoring.

    Can be disabled via DEXCONTROL_DISABLE_HEARTBEAT=1 environment variable.

    Attributes:
        heartbeat_topic: Topic for heartbeat messages
        timeout_seconds: Heartbeat timeout threshold in seconds
    """

    heartbeat_topic: str = "heartbeat"
    timeout_seconds: float = 1.0
