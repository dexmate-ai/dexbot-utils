"""Tests for gripper component configuration validation."""

from dataclasses import fields

import pytest

from dexbot_utils.configs.components.vega_1.hand import (
    DexDGripperConfig,
    DexSGripperConfig,
)


def test_grasp_torque_defaults_to_valid_value():
    assert DexSGripperConfig().grasp_torque == 0.2


def test_grasp_torque_bounds_are_ordered():
    # The high-torque threshold must sit strictly inside the valid range so
    # clients can warn above it without excluding legal values.
    config = DexSGripperConfig()
    assert (
        config.GRASP_TORQUE_MIN
        < config.GRASP_TORQUE_HIGH_THRESHOLD
        < config.GRASP_TORQUE_MAX
    )
    assert config.GRASP_TORQUE_MIN <= config.grasp_torque <= config.GRASP_TORQUE_MAX


def test_grasp_torque_bounds_are_not_dataclass_fields():
    # ClassVar annotations must stay off the constructor: the bounds describe
    # the gripper, they are not per-instance settings.
    field_names = {f.name for f in fields(DexSGripperConfig)}
    assert "grasp_torque" in field_names
    assert field_names.isdisjoint(
        {"GRASP_TORQUE_MIN", "GRASP_TORQUE_MAX", "GRASP_TORQUE_HIGH_THRESHOLD"}
    )


def test_double_gripper_inherits_bounds():
    assert DexDGripperConfig().GRASP_TORQUE_MAX == DexSGripperConfig().GRASP_TORQUE_MAX


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
