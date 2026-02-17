"""Tests for runtime_override_robot_config hand auto-detection."""

from dexbot_utils.cfg_modifier import runtime_override_robot_config
from dexbot_utils.configs.components.vega_1.hand import (
    DexDGripperConfig,
    F5D6HandV1Config,
    F5D6HandV2Config,
)
from dexbot_utils.configs.robots.base import BaseRobotConfig
from dexbot_utils.hand import HandType


def _make_config(components: dict) -> BaseRobotConfig:
    """Helper to build a BaseRobotConfig with given components."""
    return BaseRobotConfig(components=components)


# -------------------------------------------------------------------
# Case: hand NOT in config + known type detected → inject
# -------------------------------------------------------------------


class TestAutoAddHand:
    """Test injecting hand config when detected but missing from config."""

    def test_adds_gripper_when_detected_but_missing(self):
        config = _make_config({})
        runtime_override_robot_config(config, hand_types={"left": HandType.DexGripper})
        assert "left_hand" in config.components
        assert isinstance(config.components["left_hand"], DexDGripperConfig)
        assert config.components["left_hand"].side == "left"
        assert config.components["left_hand"].enabled is True

    def test_adds_f5d6_v2_when_detected_but_missing(self):
        config = _make_config({})
        runtime_override_robot_config(
            config, hand_types={"right": HandType.HandF5D6_V2}
        )
        assert "right_hand" in config.components
        assert isinstance(config.components["right_hand"], F5D6HandV2Config)
        assert config.components["right_hand"].side == "right"

    def test_adds_f5d6_v1_when_detected_but_missing(self):
        config = _make_config({})
        runtime_override_robot_config(config, hand_types={"left": HandType.HandF5D6_V1})
        assert "left_hand" in config.components
        assert isinstance(config.components["left_hand"], F5D6HandV1Config)

    def test_adds_both_hands_when_both_detected(self):
        config = _make_config({})
        runtime_override_robot_config(
            config,
            hand_types={
                "left": HandType.DexGripper,
                "right": HandType.HandF5D6_V2,
            },
        )
        assert "left_hand" in config.components
        assert "right_hand" in config.components
        assert isinstance(config.components["left_hand"], DexDGripperConfig)
        assert isinstance(config.components["right_hand"], F5D6HandV2Config)

    def test_does_not_add_when_unknown_and_missing(self):
        config = _make_config({})
        runtime_override_robot_config(config, hand_types={"left": HandType.UNKNOWN})
        assert "left_hand" not in config.components


# -------------------------------------------------------------------
# Case: hand in config + UNKNOWN detected → soft disable
# -------------------------------------------------------------------


class TestSoftDisableHand:
    """Test disabling hand config when server reports UNKNOWN."""

    def test_disables_gripper_when_unknown(self):
        config = _make_config({"left_hand": DexDGripperConfig(side="left")})
        runtime_override_robot_config(config, hand_types={"left": HandType.UNKNOWN})
        assert "left_hand" in config.components
        assert config.components["left_hand"].enabled is False

    def test_disables_f5d6_when_unknown(self):
        config = _make_config({"right_hand": F5D6HandV2Config(side="right")})
        runtime_override_robot_config(config, hand_types={"right": HandType.UNKNOWN})
        assert config.components["right_hand"].enabled is False

    def test_disables_both_hands_when_both_unknown(self):
        config = _make_config(
            {
                "left_hand": DexDGripperConfig(side="left"),
                "right_hand": F5D6HandV2Config(side="right"),
            }
        )
        runtime_override_robot_config(
            config,
            hand_types={"left": HandType.UNKNOWN, "right": HandType.UNKNOWN},
        )
        assert config.components["left_hand"].enabled is False
        assert config.components["right_hand"].enabled is False


# -------------------------------------------------------------------
# Existing behavior: hand in config + known type matches → no-op
# -------------------------------------------------------------------


class TestExistingMatchNoOp:
    """Existing behavior: matching hand type is left unchanged."""

    def test_matching_type_unchanged(self):
        original = DexDGripperConfig(side="left")
        config = _make_config({"left_hand": original})
        runtime_override_robot_config(config, hand_types={"left": HandType.DexGripper})
        assert config.components["left_hand"] is original
        assert config.components["left_hand"].enabled is True


# -------------------------------------------------------------------
# Existing behavior: hand in config + known type mismatch → replace
# -------------------------------------------------------------------


class TestExistingMismatchOverride:
    """Existing behavior: mismatched hand type gets replaced."""

    def test_replaces_mismatched_type(self):
        config = _make_config({"left_hand": DexDGripperConfig(side="left")})
        runtime_override_robot_config(config, hand_types={"left": HandType.HandF5D6_V2})
        assert isinstance(config.components["left_hand"], F5D6HandV2Config)

    def test_no_replace_when_override_disabled(self):
        original = DexDGripperConfig(side="left")
        config = _make_config({"left_hand": original})
        runtime_override_robot_config(
            config,
            hand_types={"left": HandType.HandF5D6_V2},
            enable_hand_type_override=False,
        )
        assert config.components["left_hand"] is original
