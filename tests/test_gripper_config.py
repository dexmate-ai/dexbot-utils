"""Tests for gripper component configuration validation."""

import pytest

from dexbot_utils.configs.components.vega_1.hand import (
    DexDGripperConfig,
    DexSGripperConfig,
)


def test_grasp_torque_defaults_to_valid_value():
    assert DexSGripperConfig().grasp_torque == 0.2


@pytest.mark.parametrize("value", [0.0, 0.5, 1.0])
def test_grasp_torque_accepts_inclusive_bounds(value):
    assert DexSGripperConfig(grasp_torque=value).grasp_torque == value


@pytest.mark.parametrize("value", [-0.1, 1.1, 2.0])
def test_grasp_torque_rejects_out_of_range(value):
    with pytest.raises(ValueError, match="grasp_torque"):
        DexSGripperConfig(grasp_torque=value)


def test_double_gripper_inherits_validation():
    # DexDGripperConfig subclasses DexSGripperConfig, so it inherits __post_init__.
    with pytest.raises(ValueError, match="grasp_torque"):
        DexDGripperConfig(grasp_torque=5.0)
