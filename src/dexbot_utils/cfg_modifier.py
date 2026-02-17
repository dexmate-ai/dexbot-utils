from loguru import logger

from .configs import BaseRobotConfig
from .configs.components.vega_1.hand import (
    DexDGripperConfig,
    DexSGripperConfig,
    F5D6HandV1Config,
    F5D6HandV2Config,
)
from .hand import HandType


def runtime_override_robot_config(
    config: BaseRobotConfig,
    hand_types: dict[str, HandType],
    enable_hand_type_override: bool = True,
    disable_estop_checking: bool = False,
    disable_heartbeat: bool = False,
):
    """Apply runtime-driven overrides to a robot's configuration.

    This function mutates the config in place based on runtime inputs:
    - Optionally disables the `estop` and `heartbeat` components.
    - Updates end-effector (hand) components per detected `hand_types`:
      - Disables a hand component (`enabled = False`) when the detected type
        is `HandType.UNKNOWN` (server has no topics for it).
      - Injects a new hand component when a known hand type is detected but
        no hand entry exists in the config.
      - Replaces a mismatched hand component when `enable_hand_type_override`
        is True.
      - Disables arm end-effector pass-through (`enable_ee_pass_through = False`)
        when a concrete hand type is provided.
    - Emits warnings via the logger when overrides, injections, or disables occur.

    Args:
        config: Robot configuration to be mutated.
        hand_types: Mapping from side name (`"left"`, `"right"`) to detected
            `HandType`. Use `HandType.UNKNOWN` when no hand is detected.
        enable_hand_type_override: If True, replaces an existing hand component in
            the config when it does not match the detected `HandType`. If False,
            leaves the original component and logs a warning.
        disable_estop_checking: If True, disables the `estop` component (when
            present) and logs a warning.
        disable_heartbeat: If True, disables the `heartbeat` component (when
            present) and logs a warning.

    """

    if disable_estop_checking:
        if "estop" in config.components:
            config.components["estop"].enabled = False
            logger.warning("EStop checking is disabled via environment variable")

    if disable_heartbeat:
        if "heartbeat" in config.components:
            config.components["heartbeat"].enabled = False
            logger.warning("Heartbeat is disabled via environment variable")

    # Handling hand types
    hand_type_mapping = {
        DexSGripperConfig: HandType.DexGripper,
        DexDGripperConfig: HandType.DexGripper,
        F5D6HandV1Config: HandType.HandF5D6_V1,
        F5D6HandV2Config: HandType.HandF5D6_V2,
    }
    hand_type_reverse_mapping = {
        HandType.DexGripper: DexDGripperConfig,
        HandType.HandF5D6_V1: F5D6HandV1Config,
        HandType.HandF5D6_V2: F5D6HandV2Config,
    }
    for side in ("left", "right"):
        if side in hand_types:
            hand_type = hand_types[side]
            if f"{side}_arm" in config.components:
                arm = config.components[f"{side}_arm"]
                # EE pass-through is disabled when hand type is specified
                if hand_type != HandType.UNKNOWN and hasattr(
                    arm, "enable_ee_pass_through"
                ):
                    arm.enable_ee_pass_through = False  # type: ignore[attr-defined]

            hand_key = f"{side}_hand"
            if hand_key in config.components:
                if hand_type == HandType.UNKNOWN:
                    config.components[hand_key].enabled = False
                    logger.warning(
                        f"Disabling {side}_hand: can not detect known end-effector from robot-controller."
                    )
                else:
                    desired_hand_type = hand_type_mapping[
                        type(config.components[hand_key])
                    ]
                    if desired_hand_type != hand_type:
                        if enable_hand_type_override:
                            override_hand_cfg = hand_type_reverse_mapping[hand_type](
                                side=side
                            )
                            config.components[hand_key] = override_hand_cfg
                            logger.warning(
                                f"Override {side}_hand config to {override_hand_cfg} based on detected hand type {hand_type}"
                            )
                        else:
                            logger.warning(
                                f"Detected {side}_hand type is {hand_type}, but the input config is {desired_hand_type}."
                            )
            else:
                # Hand not in config — inject if a known type was detected
                if hand_type != HandType.UNKNOWN:
                    new_hand_cfg = hand_type_reverse_mapping[hand_type](side=side)
                    config.components[hand_key] = new_hand_cfg
                    logger.warning(
                        f"Auto-adding {side}_hand config ({hand_type}) based on runtime detection"
                    )
