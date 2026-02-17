"""Tests for RobotInfo class."""

import pytest

from dexbot_utils import RobotInfo
from dexbot_utils.configs.components.base import BaseComponentConfig
from dexbot_utils.configs.robots.base import BaseRobotConfig


class TestResolveVariantFromEnv:
    """Tests for _resolve_variant_from_env static method."""

    def test_resolve_from_robot_config_env(self, monkeypatch):
        """ROBOT_CONFIG env var is used when set."""
        monkeypatch.setenv("ROBOT_CONFIG", "vega_1")
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        assert RobotInfo._resolve_variant_from_env() == "vega_1"

    def test_resolve_from_robot_config_strips_py(self, monkeypatch):
        """ROBOT_CONFIG with .py extension is stripped."""
        monkeypatch.setenv("ROBOT_CONFIG", "vega_1.py")
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        assert RobotInfo._resolve_variant_from_env() == "vega_1"

    def test_resolve_from_robot_name_env(self, monkeypatch):
        """ROBOT_NAME env var is used as fallback."""
        monkeypatch.delenv("ROBOT_CONFIG", raising=False)
        monkeypatch.setenv("ROBOT_NAME", "dm/vgabcd123456-1")
        assert RobotInfo._resolve_variant_from_env() == "vega_1"

    def test_resolve_raises_when_no_env(self, monkeypatch):
        """ValueError raised when neither env var is set."""
        monkeypatch.delenv("ROBOT_CONFIG", raising=False)
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        with pytest.raises(ValueError, match="Variant not specified"):
            RobotInfo._resolve_variant_from_env()

    def test_robot_config_takes_priority(self, monkeypatch):
        """ROBOT_CONFIG takes priority over ROBOT_NAME."""
        monkeypatch.setenv("ROBOT_CONFIG", "vega_1_f5d6")
        monkeypatch.setenv("ROBOT_NAME", "dm/vgabcd123456-1")
        assert RobotInfo._resolve_variant_from_env() == "vega_1_f5d6"


class TestGetDefaultConfig:
    """Tests for get_default_config static method."""

    def test_get_default_config_with_variant(self):
        """Returns config for specified variant."""
        config = RobotInfo.get_default_config("vega_1")
        assert config.robot_model == "vega_1"
        assert "left_arm" in config.components

    def test_get_default_config_from_env(self, monkeypatch):
        """Resolves variant from env when not specified."""
        monkeypatch.setenv("ROBOT_CONFIG", "vega_1_f5d6")
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        config = RobotInfo.get_default_config()
        assert config.robot_model == "vega_1"
        assert "left_hand" in config.components

    def test_get_default_config_unknown_variant(self):
        """Raises ValueError for unknown variant."""
        with pytest.raises(ValueError, match="Unknown robot variant"):
            RobotInfo.get_default_config("nonexistent_robot")

    def test_get_default_config_no_env_no_variant(self, monkeypatch):
        """Raises ValueError when no variant and no env vars."""
        monkeypatch.delenv("ROBOT_CONFIG", raising=False)
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        with pytest.raises(ValueError, match="Variant not specified"):
            RobotInfo.get_default_config()

    def test_get_default_config_returns_mutable_copy(self):
        """Returned config can be modified without affecting registry."""
        config1 = RobotInfo.get_default_config("vega_1")
        config1.components.pop("left_arm")
        config2 = RobotInfo.get_default_config("vega_1")
        assert "left_arm" in config2.components


class TestRobotInfoConfigParam:
    """Tests for RobotInfo config parameter."""

    def test_init_with_config(self):
        """RobotInfo accepts a config directly."""
        config = RobotInfo.get_default_config("vega_1")
        robot = RobotInfo(configs=config)
        assert robot.robot_model == "vega_1"
        assert robot.has_left_arm

    def test_init_with_modified_config(self):
        """Modified config is used as-is (no merging)."""
        config = RobotInfo.get_default_config("vega_1")
        config.components["left_arm"].pv_mode = True
        robot = RobotInfo(configs=config)
        assert robot.get_component_parameter("left_arm", "pv_mode") is True

    def test_init_with_config_removed_component(self):
        """Components can be removed from config before passing."""
        config = RobotInfo.get_default_config("vega_1")
        config.components.pop("chassis")
        robot = RobotInfo(configs=config)
        assert not robot.has_chassis
        assert robot.has_left_arm

    def test_init_with_custom_config(self):
        """Fully custom BaseRobotConfig works."""
        custom = BaseRobotConfig(
            robot_model="test_robot",
            components={"left_arm": BaseComponentConfig(enabled=True)},
        )
        robot = RobotInfo(configs=custom)
        assert robot.robot_model == "test_robot"
        assert robot.has_component("left_arm")
        assert not robot.has_urdf

    def test_variant_and_config_raises(self):
        """Passing both variant and config raises ValueError."""
        config = RobotInfo.get_default_config("vega_1")
        with pytest.raises(ValueError, match="Cannot specify both.*configs"):
            RobotInfo("vega_1", configs=config)

    def test_existing_variant_usage_unchanged(self):
        """Existing usage with variant string still works."""
        robot = RobotInfo("vega_1")
        assert robot.robot_model == "vega_1"
        assert robot.has_left_arm

    def test_existing_env_usage_unchanged(self, monkeypatch):
        """Existing usage with env var still works."""
        monkeypatch.setenv("ROBOT_CONFIG", "vega_1")
        monkeypatch.delenv("ROBOT_NAME", raising=False)
        robot = RobotInfo()
        assert robot.robot_model == "vega_1"
