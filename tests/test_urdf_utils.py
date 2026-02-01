"""Tests for urdf_utils module."""

import tempfile
from pathlib import Path

import pytest

from dexbot_utils.urdf_utils import (
    JointInfo,
    JointLimit,
    URDFParser,
    get_joint_info,
    get_joint_limits,
    get_joint_names,
    get_link_names,
    get_movable_joint_names,
    parse_urdf,
)


class TestURDFParser:
    """Tests for URDFParser class."""

    def test_urdf_parser_init(self, sample_urdf_file: Path):
        """Test URDFParser initialization."""
        parser = URDFParser(sample_urdf_file)
        assert parser.urdf_path == sample_urdf_file

    def test_urdf_parser_file_not_found(self):
        """Test URDFParser with non-existent file."""
        with pytest.raises(FileNotFoundError):
            URDFParser("/path/to/nonexistent.urdf")

    def test_get_robot_name(self, sample_urdf_file: Path):
        """Test getting robot name from URDF."""
        parser = URDFParser(sample_urdf_file)
        name = parser.get_robot_name()
        assert name == "test_robot"

    def test_get_all_joints(self, sample_urdf_file: Path):
        """Test getting all joints from URDF."""
        parser = URDFParser(sample_urdf_file)
        joints = parser.get_all_joints()

        assert len(joints) == 4  # joint1, joint2, joint3, fixed_joint
        assert "joint1" in joints
        assert "joint2" in joints
        assert "joint3" in joints
        assert "fixed_joint" in joints

        # Test caching
        joints2 = parser.get_all_joints()
        assert joints is joints2  # Same object

    def test_joint_info_details(self, sample_urdf_file: Path):
        """Test detailed joint information."""
        parser = URDFParser(sample_urdf_file)
        joint1 = parser.get_joint_info("joint1")

        assert joint1 is not None
        assert joint1.name == "joint1"
        assert joint1.joint_type == "revolute"
        assert joint1.parent_link == "base_link"
        assert joint1.child_link == "link1"
        assert joint1.limit is not None
        assert joint1.axis is not None

    def test_joint_limit_details(self, sample_urdf_file: Path):
        """Test joint limit information."""
        parser = URDFParser(sample_urdf_file)
        joint1 = parser.get_joint_info("joint1")

        assert joint1.limit.lower == -1.57
        assert joint1.limit.upper == 1.57
        assert joint1.limit.effort == 100.0
        assert joint1.limit.velocity == 1.0

    def test_joint_axis(self, sample_urdf_file: Path):
        """Test joint axis information."""
        parser = URDFParser(sample_urdf_file)
        joint1 = parser.get_joint_info("joint1")

        assert joint1.axis == (0.0, 0.0, 1.0)

    def test_get_joint_names_all(self, sample_urdf_file: Path):
        """Test getting all joint names."""
        parser = URDFParser(sample_urdf_file)
        names = parser.get_joint_names()

        assert len(names) == 4
        assert "joint1" in names
        assert "joint2" in names
        assert "joint3" in names
        assert "fixed_joint" in names

    def test_get_joint_names_filtered(self, sample_urdf_file: Path):
        """Test getting joint names filtered by type."""
        parser = URDFParser(sample_urdf_file)

        revolute = parser.get_joint_names(joint_type="revolute")
        assert len(revolute) == 2
        assert "joint1" in revolute
        assert "joint2" in revolute

        prismatic = parser.get_joint_names(joint_type="prismatic")
        assert len(prismatic) == 1
        assert "joint3" in prismatic

        fixed = parser.get_joint_names(joint_type="fixed")
        assert len(fixed) == 1
        assert "fixed_joint" in fixed

    def test_get_movable_joint_names(self, sample_urdf_file: Path):
        """Test getting only movable joint names."""
        parser = URDFParser(sample_urdf_file)
        movable = parser.get_movable_joint_names()

        assert len(movable) == 3  # joint1, joint2, joint3 (not fixed_joint)
        assert "joint1" in movable
        assert "joint2" in movable
        assert "joint3" in movable
        assert "fixed_joint" not in movable

    def test_get_joint_limits_all(self, sample_urdf_file: Path):
        """Test getting all joint limits."""
        parser = URDFParser(sample_urdf_file)
        limits = parser.get_joint_limits()

        assert len(limits) == 3  # fixed_joint has no limits
        assert "joint1" in limits
        assert "joint2" in limits
        assert "joint3" in limits

        assert limits["joint1"].lower == -1.57
        assert limits["joint1"].upper == 1.57

    def test_get_joint_limits_specific(self, sample_urdf_file: Path):
        """Test getting limits for specific joints."""
        parser = URDFParser(sample_urdf_file)
        limits = parser.get_joint_limits(joint_names=["joint1", "joint3"])

        assert len(limits) == 2
        assert "joint1" in limits
        assert "joint3" in limits
        assert "joint2" not in limits

    def test_get_joint_info_missing(self, sample_urdf_file: Path):
        """Test getting info for non-existent joint."""
        parser = URDFParser(sample_urdf_file)
        info = parser.get_joint_info("nonexistent_joint")
        assert info is None

    def test_get_link_names(self, sample_urdf_file: Path):
        """Test getting all link names."""
        parser = URDFParser(sample_urdf_file)
        links = parser.get_link_names()

        assert len(links) == 4
        assert "base_link" in links
        assert "link1" in links
        assert "link2" in links
        assert "end_effector" in links

    def test_get_joint_tree(self, sample_urdf_file: Path):
        """Test getting joint tree structure."""
        parser = URDFParser(sample_urdf_file)
        tree = parser.get_joint_tree()

        assert "base_link" in tree
        assert "joint1" in tree["base_link"]
        assert "link1" in tree
        assert "link2" in tree


class TestJointLimit:
    """Tests for JointLimit dataclass."""

    def test_joint_limit_creation(self):
        """Test creating JointLimit."""
        limit = JointLimit(lower=-1.0, upper=1.0, effort=100.0, velocity=1.5)

        assert limit.lower == -1.0
        assert limit.upper == 1.0
        assert limit.effort == 100.0
        assert limit.velocity == 1.5

    def test_joint_limit_optional_fields(self):
        """Test JointLimit with optional fields."""
        limit = JointLimit(lower=-1.0, upper=1.0)

        assert limit.lower == -1.0
        assert limit.upper == 1.0
        assert limit.effort is None
        assert limit.velocity is None

    def test_joint_limit_repr(self):
        """Test JointLimit string representation."""
        limit = JointLimit(lower=-1.0, upper=1.0, effort=100.0, velocity=1.5)
        repr_str = repr(limit)

        assert "JointLimit" in repr_str
        assert "-1.0" in repr_str
        assert "1.0" in repr_str


class TestJointInfo:
    """Tests for JointInfo dataclass."""

    def test_joint_info_creation(self):
        """Test creating JointInfo."""
        limit = JointLimit(lower=-1.0, upper=1.0)
        info = JointInfo(
            name="test_joint",
            joint_type="revolute",
            parent_link="parent",
            child_link="child",
            limit=limit,
            axis=(0.0, 0.0, 1.0),
        )

        assert info.name == "test_joint"
        assert info.joint_type == "revolute"
        assert info.parent_link == "parent"
        assert info.child_link == "child"
        assert info.limit == limit
        assert info.axis == (0.0, 0.0, 1.0)

    def test_joint_info_optional_fields(self):
        """Test JointInfo with optional fields."""
        info = JointInfo(
            name="test_joint",
            joint_type="fixed",
            parent_link="parent",
            child_link="child",
        )

        assert info.name == "test_joint"
        assert info.limit is None
        assert info.axis is None

    def test_joint_info_repr(self):
        """Test JointInfo string representation."""
        info = JointInfo(
            name="test_joint",
            joint_type="revolute",
            parent_link="parent",
            child_link="child",
        )
        repr_str = repr(info)

        assert "JointInfo" in repr_str
        assert "test_joint" in repr_str
        assert "revolute" in repr_str


class TestConvenienceFunctions:
    """Tests for convenience functions."""

    def test_parse_urdf(self, sample_urdf_file: Path):
        """Test parse_urdf convenience function."""
        parser = parse_urdf(sample_urdf_file)

        assert isinstance(parser, URDFParser)
        assert parser.get_robot_name() == "test_robot"

    def test_get_joint_names_function(self, sample_urdf_file: Path):
        """Test get_joint_names convenience function."""
        names = get_joint_names(sample_urdf_file)

        assert len(names) == 4
        assert "joint1" in names

    def test_get_joint_names_filtered_function(self, sample_urdf_file: Path):
        """Test get_joint_names with filter."""
        revolute = get_joint_names(sample_urdf_file, joint_type="revolute")

        assert len(revolute) == 2
        assert "joint1" in revolute

    def test_get_movable_joint_names_function(self, sample_urdf_file: Path):
        """Test get_movable_joint_names convenience function."""
        movable = get_movable_joint_names(sample_urdf_file)

        assert len(movable) == 3
        assert "fixed_joint" not in movable

    def test_get_joint_limits_function(self, sample_urdf_file: Path):
        """Test get_joint_limits convenience function."""
        limits = get_joint_limits(sample_urdf_file)

        assert len(limits) == 3
        assert "joint1" in limits
        assert limits["joint1"].lower == -1.57

    def test_get_joint_limits_specific_function(self, sample_urdf_file: Path):
        """Test get_joint_limits with specific joints."""
        limits = get_joint_limits(sample_urdf_file, joint_names=["joint1"])

        assert len(limits) == 1
        assert "joint1" in limits

    def test_get_joint_info_function(self, sample_urdf_file: Path):
        """Test get_joint_info convenience function."""
        info = get_joint_info(sample_urdf_file, "joint1")

        assert info is not None
        assert info.name == "joint1"
        assert info.joint_type == "revolute"

    def test_get_joint_info_missing_function(self, sample_urdf_file: Path):
        """Test get_joint_info with non-existent joint."""
        info = get_joint_info(sample_urdf_file, "nonexistent")
        assert info is None

    def test_get_link_names_function(self, sample_urdf_file: Path):
        """Test get_link_names convenience function."""
        links = get_link_names(sample_urdf_file)

        assert len(links) == 4
        assert "base_link" in links

    def test_functions_with_string_path(self, sample_urdf_file: Path):
        """Test convenience functions with string path."""
        # Test with string path instead of Path object
        str_path = str(sample_urdf_file)

        names = get_joint_names(str_path)
        assert len(names) == 4

        movable = get_movable_joint_names(str_path)
        assert len(movable) == 3

        limits = get_joint_limits(str_path)
        assert len(limits) == 3


class TestInvalidURDF:
    """Tests for handling invalid URDF files."""

    def test_invalid_xml(self):
        """Test parsing invalid XML."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".urdf", delete=False) as f:
            f.write("This is not valid XML")
            temp_path = Path(f.name)

        try:
            with pytest.raises(Exception):  # Will raise ET.ParseError
                URDFParser(temp_path)
        finally:
            temp_path.unlink()

    def test_empty_urdf(self):
        """Test parsing empty URDF."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".urdf", delete=False) as f:
            f.write('<?xml version="1.0"?><robot name="empty"></robot>')
            temp_path = Path(f.name)

        try:
            parser = URDFParser(temp_path)
            joints = parser.get_all_joints()
            assert len(joints) == 0

            links = parser.get_link_names()
            assert len(links) == 0
        finally:
            temp_path.unlink()
