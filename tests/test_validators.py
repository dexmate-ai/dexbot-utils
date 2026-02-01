"""Tests for validators module."""

import os

import pytest

from dexbot_utils import RobotInfo
from dexbot_utils.validators import (
    ComponentValidationError,
    get_missing_components,
    has_all_components,
    has_any_component,
    has_component,
    require_component,
    require_components,
    validate_component,
    validate_components,
)


class TestValidateComponent:
    """Tests for validate_component function."""

    def test_validate_component_exists(self):
        """Test validating existing component."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            result = validate_component("left_arm")
            assert result is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_component_missing_raises(self):
        """Test validation raises for missing component."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            with pytest.raises(
                ComponentValidationError, match="does not have component"
            ):
                validate_component("nonexistent_component")
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_component_missing_no_raise(self):
        """Test validation returns False without raising."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            result = validate_component("nonexistent_component", raise_on_missing=False)
            assert result is False
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_component_with_robot_config(self):
        """Test validation with explicit RobotInfo."""
        robot = RobotInfo("vega_1")
        result = validate_component("left_arm", robot_config=robot)
        assert result is True


class TestValidateComponents:
    """Tests for validate_components function."""

    def test_validate_components_all_exist(self):
        """Test validating multiple existing components."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            result = validate_components(["left_arm", "right_arm", "torso"])
            assert result is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_components_some_missing_raises(self):
        """Test validation raises when some components missing."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            with pytest.raises(ComponentValidationError, match="missing components"):
                validate_components(["left_arm", "nonexistent1", "nonexistent2"])
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_components_some_missing_no_raise(self):
        """Test validation returns False without raising."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            result = validate_components(
                ["left_arm", "nonexistent"], raise_on_missing=False
            )
            assert result is False
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_validate_components_with_robot_config(self):
        """Test validation with explicit RobotInfo."""
        robot = RobotInfo("vega_1")
        result = validate_components(["left_arm", "right_arm"], robot_config=robot)
        assert result is True


class TestRequireComponent:
    """Tests for require_component function."""

    def test_require_component_exists(self):
        """Test requiring existing component."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            # Should not raise
            require_component("left_arm")
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_require_component_missing(self):
        """Test requiring missing component raises."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            with pytest.raises(ComponentValidationError):
                require_component("nonexistent_component")
        finally:
            os.environ.pop("ROBOT_CONFIG", None)


class TestRequireComponents:
    """Tests for require_components function."""

    def test_require_components_all_exist(self):
        """Test requiring multiple existing components."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            # Should not raise
            require_components(["left_arm", "right_arm", "torso"])
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_require_components_some_missing(self):
        """Test requiring components with some missing raises."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            with pytest.raises(ComponentValidationError):
                require_components(["left_arm", "nonexistent"])
        finally:
            os.environ.pop("ROBOT_CONFIG", None)


class TestHasComponent:
    """Tests for has_component function."""

    def test_has_component_true(self):
        """Test has_component returns True for existing component."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_component("left_arm") is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_component_false(self):
        """Test has_component returns False for missing component."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_component("nonexistent") is False
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_component_with_config(self):
        """Test has_component with explicit config."""
        robot = RobotInfo("vega_1")
        assert has_component("left_arm", robot_config=robot) is True


class TestHasAllComponents:
    """Tests for has_all_components function."""

    def test_has_all_components_true(self):
        """Test has_all_components returns True when all present."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_all_components(["left_arm", "right_arm", "torso"]) is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_all_components_false(self):
        """Test has_all_components returns False when some missing."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_all_components(["left_arm", "nonexistent"]) is False
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_all_components_with_config(self):
        """Test has_all_components with explicit config."""
        robot = RobotInfo("vega_1")
        assert has_all_components(["left_arm", "right_arm"], robot_config=robot) is True


class TestHasAnyComponent:
    """Tests for has_any_component function."""

    def test_has_any_component_all_present(self):
        """Test has_any_component returns True when all present."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_any_component(["left_arm", "right_arm"]) is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_any_component_some_present(self):
        """Test has_any_component returns True when some present."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_any_component(["left_arm", "nonexistent"]) is True
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_any_component_none_present(self):
        """Test has_any_component returns False when none present."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            assert has_any_component(["nonexistent1", "nonexistent2"]) is False
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_has_any_component_with_config(self):
        """Test has_any_component with explicit config."""
        robot = RobotInfo("vega_1")
        assert has_any_component(["left_arm"], robot_config=robot) is True


class TestGetMissingComponents:
    """Tests for get_missing_components function."""

    def test_get_missing_components_none_missing(self):
        """Test get_missing_components returns empty list when all present."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            missing = get_missing_components(["left_arm", "right_arm", "torso"])
            assert missing == []
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_get_missing_components_some_missing(self):
        """Test get_missing_components returns missing components."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            missing = get_missing_components(
                ["left_arm", "nonexistent1", "nonexistent2"]
            )
            assert set(missing) == {"nonexistent1", "nonexistent2"}
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_get_missing_components_all_missing(self):
        """Test get_missing_components when all missing."""
        os.environ["ROBOT_CONFIG"] = "vega_1"
        try:
            missing = get_missing_components(["nonexistent1", "nonexistent2"])
            assert set(missing) == {"nonexistent1", "nonexistent2"}
        finally:
            os.environ.pop("ROBOT_CONFIG", None)

    def test_get_missing_components_with_config(self):
        """Test get_missing_components with explicit config."""
        robot = RobotInfo("vega_1")
        missing = get_missing_components(
            ["left_arm", "nonexistent"], robot_config=robot
        )
        assert missing == ["nonexistent"]
