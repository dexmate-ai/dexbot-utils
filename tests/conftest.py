"""Pytest configuration and shared fixtures."""

import os
import tempfile
from pathlib import Path
from typing import Generator

import pytest


@pytest.fixture
def mock_robot_name() -> str:
    """Return a mock robot name for testing."""
    return "dm/vgabcd123456-1"


@pytest.fixture
def mock_robot_name_vega_u() -> str:
    """Return a mock robot name for vega-1u (upper body) for testing."""
    return "dm/vgabcd123456-1u"


@pytest.fixture
def set_robot_env(mock_robot_name: str) -> Generator[str, None, None]:
    """Set ROBOT_NAME environment variable for testing."""
    old_value = os.environ.get("ROBOT_NAME")
    os.environ["ROBOT_NAME"] = mock_robot_name
    yield mock_robot_name
    if old_value is None:
        os.environ.pop("ROBOT_NAME", None)
    else:
        os.environ["ROBOT_NAME"] = old_value


@pytest.fixture
def set_robot_env_vega_u(mock_robot_name_vega_u: str) -> Generator[str, None, None]:
    """Set ROBOT_NAME environment variable for vega-1u (upper body) testing."""
    old_value = os.environ.get("ROBOT_NAME")
    os.environ["ROBOT_NAME"] = mock_robot_name_vega_u
    yield mock_robot_name_vega_u
    if old_value is None:
        os.environ.pop("ROBOT_NAME", None)
    else:
        os.environ["ROBOT_NAME"] = old_value


@pytest.fixture
def unset_robot_env() -> Generator[None, None, None]:
    """Ensure ROBOT_NAME environment variable is not set."""
    old_value = os.environ.get("ROBOT_NAME")
    os.environ.pop("ROBOT_NAME", None)
    yield
    if old_value is not None:
        os.environ["ROBOT_NAME"] = old_value


@pytest.fixture
def sample_urdf() -> str:
    """Return sample URDF content for testing."""
    return """<?xml version="1.0"?>
<robot name="test_robot">
  <link name="base_link">
    <visual>
      <geometry>
        <box size="0.1 0.1 0.1"/>
      </geometry>
    </visual>
  </link>

  <link name="link1">
    <visual>
      <geometry>
        <cylinder radius="0.05" length="0.3"/>
      </geometry>
    </visual>
  </link>

  <link name="link2">
    <visual>
      <geometry>
        <cylinder radius="0.05" length="0.3"/>
      </geometry>
    </visual>
  </link>

  <link name="end_effector">
    <visual>
      <geometry>
        <sphere radius="0.03"/>
      </geometry>
    </visual>
  </link>

  <joint name="joint1" type="revolute">
    <parent link="base_link"/>
    <child link="link1"/>
    <origin xyz="0 0 0.1" rpy="0 0 0"/>
    <axis xyz="0 0 1"/>
    <limit lower="-1.57" upper="1.57" effort="100" velocity="1.0"/>
  </joint>

  <joint name="joint2" type="revolute">
    <parent link="link1"/>
    <child link="link2"/>
    <origin xyz="0 0 0.3" rpy="0 0 0"/>
    <axis xyz="0 1 0"/>
    <limit lower="-2.0" upper="2.0" effort="150" velocity="1.5"/>
  </joint>

  <joint name="joint3" type="prismatic">
    <parent link="link2"/>
    <child link="end_effector"/>
    <origin xyz="0 0 0.3" rpy="0 0 0"/>
    <axis xyz="0 0 1"/>
    <limit lower="0.0" upper="0.2" effort="50" velocity="0.5"/>
  </joint>

  <joint name="fixed_joint" type="fixed">
    <parent link="link1"/>
    <child link="link2"/>
    <origin xyz="0 0 0" rpy="0 0 0"/>
  </joint>
</robot>
"""


@pytest.fixture
def sample_urdf_file(sample_urdf: str) -> Generator[Path, None, None]:
    """Create a temporary URDF file for testing."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".urdf", delete=False) as f:
        f.write(sample_urdf)
        temp_path = Path(f.name)

    yield temp_path

    # Cleanup
    if temp_path.exists():
        temp_path.unlink()
