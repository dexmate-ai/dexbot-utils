"""URDF parsing utilities for extracting joint information."""

import functools
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path


@dataclass
class JointLimit:
    """Joint limit information."""

    lower: float
    upper: float
    effort: float | None = None
    velocity: float | None = None

    def __repr__(self) -> str:
        """String representation of joint limit."""
        return (
            f"JointLimit(lower={self.lower}, upper={self.upper}, "
            f"effort={self.effort}, velocity={self.velocity})"
        )


@dataclass
class JointInfo:
    """Complete joint information from URDF."""

    name: str
    joint_type: str
    parent_link: str
    child_link: str
    limit: JointLimit | None = None
    axis: tuple[float, float, float] | None = None

    def __repr__(self) -> str:
        """String representation of joint info."""
        return (
            f"JointInfo(name='{self.name}', type='{self.joint_type}', "
            f"parent='{self.parent_link}', child='{self.child_link}')"
        )


class URDFParser:
    """Parser for URDF files to extract robot joint information.

    Note: For optimal performance when calling the same URDF multiple times,
    consider reusing the URDFParser instance or use the convenience functions
    which cache parsers internally.
    """

    # Class-level constant for movable joint types (performance optimization)
    _MOVABLE_JOINT_TYPES = frozenset({"revolute", "continuous", "prismatic"})

    def __init__(self, urdf_path: str | Path):
        """Initialize URDF parser.

        Args:
            urdf_path: Path to the URDF file.

        Raises:
            FileNotFoundError: If URDF file does not exist.
            ET.ParseError: If URDF file is not valid XML.
        """
        self.urdf_path = Path(urdf_path)
        if not self.urdf_path.exists():
            raise FileNotFoundError(f"URDF file not found: {urdf_path}")

        self.tree = ET.parse(self.urdf_path)
        self.root = self.tree.getroot()
        self._joints_cache: dict[str, JointInfo] | None = None

    def get_robot_name(self) -> str:
        """Get the robot name from URDF.

        Returns:
            Robot name from URDF root element.
        """
        return self.root.get("name", "unknown")

    def get_all_joints(self) -> dict[str, JointInfo]:
        """Get all joints from URDF.

        Returns:
            Dictionary mapping joint names to JointInfo objects.
        """
        if self._joints_cache is not None:
            return self._joints_cache

        joints = {}
        for joint in self.root.findall("joint"):
            joint_info = self._parse_joint(joint)
            joints[joint_info.name] = joint_info

        self._joints_cache = joints
        return joints

    def _parse_joint(self, joint_element: ET.Element) -> JointInfo:
        """Parse a joint XML element into JointInfo.

        Args:
            joint_element: XML element for a joint.

        Returns:
            JointInfo object with parsed data.
        """
        name = joint_element.get("name", "")
        joint_type = joint_element.get("type", "unknown")

        # Get parent and child links
        parent = joint_element.find("parent")
        child = joint_element.find("child")
        parent_link = parent.get("link", "") if parent is not None else ""
        child_link = child.get("link", "") if child is not None else ""

        # Parse limit if exists
        limit_elem = joint_element.find("limit")
        limit = None
        if limit_elem is not None:
            limit = JointLimit(
                lower=float(limit_elem.get("lower", "0")),
                upper=float(limit_elem.get("upper", "0")),
                effort=float(limit_elem.get("effort"))
                if limit_elem.get("effort")
                else None,
                velocity=float(limit_elem.get("velocity"))
                if limit_elem.get("velocity")
                else None,
            )

        # Parse axis if exists
        axis_elem = joint_element.find("axis")
        axis = None
        if axis_elem is not None:
            xyz = axis_elem.get("xyz", "1 0 0")
            axis = tuple(map(float, xyz.split()))

        return JointInfo(
            name=name,
            joint_type=joint_type,
            parent_link=parent_link,
            child_link=child_link,
            limit=limit,
            axis=axis,
        )

    def get_joint_names(self, joint_type: str | None = None) -> list[str]:
        """Get list of joint names, optionally filtered by type.

        Args:
            joint_type: Filter by joint type (e.g., 'revolute', 'prismatic', 'fixed').
                       If None, returns all joints.

        Returns:
            List of joint names.
        """
        joints = self.get_all_joints()
        if joint_type is None:
            return list(joints.keys())

        return [name for name, info in joints.items() if info.joint_type == joint_type]

    def get_movable_joint_names(self) -> list[str]:
        """Get list of movable (non-fixed) joint names.

        Returns:
            List of movable joint names.
        """
        joints = self.get_all_joints()
        return [
            name
            for name, info in joints.items()
            if info.joint_type in self._MOVABLE_JOINT_TYPES
        ]

    def get_joint_limits(
        self,
        joint_names: list[str] | None = None,
    ) -> dict[str, JointLimit]:
        """Get joint limits for specified joints.

        Args:
            joint_names: List of joint names. If None, returns limits for all joints.

        Returns:
            Dictionary mapping joint names to JointLimit objects.
            Joints without limits are excluded.
        """
        joints = self.get_all_joints()

        if joint_names is None:
            joint_names = list(joints.keys())

        limits = {}
        for name in joint_names:
            if name in joints and joints[name].limit is not None:
                limits[name] = joints[name].limit

        return limits

    def get_joint_info(self, joint_name: str) -> JointInfo | None:
        """Get detailed information for a specific joint.

        Args:
            joint_name: Name of the joint.

        Returns:
            JointInfo object, or None if joint not found.
        """
        joints = self.get_all_joints()
        return joints.get(joint_name)

    def get_link_names(self) -> list[str]:
        """Get list of all link names in the URDF.

        Returns:
            List of link names.
        """
        links = self.root.findall("link")
        return [link.get("name", "") for link in links]

    def get_joint_tree(self) -> dict[str, list[str]]:
        """Get tree structure of joints organized by parent links.

        Returns:
            Dictionary mapping parent links to lists of child joints.
        """
        joints = self.get_all_joints()
        tree = {}

        for joint_info in joints.values():
            parent = joint_info.parent_link
            if parent not in tree:
                tree[parent] = []
            tree[parent].append(joint_info.name)

        return tree


# Convenience functions for direct usage
# Performance optimization: Cache URDFParser instances to avoid re-parsing


@functools.lru_cache(maxsize=4)
def _get_cached_parser(urdf_path_str: str) -> URDFParser:
    """Get cached URDFParser for path. Cache up to 4 recent URDFs.

    Args:
        urdf_path_str: String path to URDF file.

    Returns:
        Cached URDFParser instance.
    """
    return URDFParser(urdf_path_str)


def parse_urdf(urdf_path: str | Path) -> URDFParser:
    """Parse a URDF file and return parser instance.

    This function caches up to 4 recent URDF parsers for performance.
    Repeated calls with the same path will return the cached parser.

    Args:
        urdf_path: Path to the URDF file.

    Returns:
        URDFParser instance (may be cached).
    """
    return _get_cached_parser(str(urdf_path))


def get_joint_names(
    urdf_path: str | Path,
    joint_type: str | None = None,
) -> list[str]:
    """Get joint names from URDF file.

    Uses cached parser for performance. Repeated calls with same path are very fast.

    Args:
        urdf_path: Path to the URDF file.
        joint_type: Filter by joint type. If None, returns all joints.

    Returns:
        List of joint names.
    """
    parser = _get_cached_parser(str(urdf_path))
    return parser.get_joint_names(joint_type)


def get_movable_joint_names(urdf_path: str | Path) -> list[str]:
    """Get movable joint names from URDF file.

    Uses cached parser for performance. Repeated calls with same path are very fast.

    Args:
        urdf_path: Path to the URDF file.

    Returns:
        List of movable joint names.
    """
    parser = _get_cached_parser(str(urdf_path))
    return parser.get_movable_joint_names()


def get_joint_limits(
    urdf_path: str | Path,
    joint_names: list[str] | None = None,
) -> dict[str, JointLimit]:
    """Get joint limits from URDF file.

    Uses cached parser for performance. Repeated calls with same path are very fast.

    Args:
        urdf_path: Path to the URDF file.
        joint_names: List of joint names. If None, returns all joints with limits.

    Returns:
        Dictionary mapping joint names to JointLimit objects.
    """
    parser = _get_cached_parser(str(urdf_path))
    return parser.get_joint_limits(joint_names)


def get_joint_info(urdf_path: str | Path, joint_name: str) -> JointInfo | None:
    """Get information for a specific joint from URDF file.

    Uses cached parser for performance. Repeated calls with same path are very fast.

    Args:
        urdf_path: Path to the URDF file.
        joint_name: Name of the joint.

    Returns:
        JointInfo object, or None if joint not found.
    """
    parser = _get_cached_parser(str(urdf_path))
    return parser.get_joint_info(joint_name)


def get_link_names(urdf_path: str | Path) -> list[str]:
    """Get all link names from URDF file.

    Uses cached parser for performance. Repeated calls with same path are very fast.

    Args:
        urdf_path: Path to the URDF file.

    Returns:
        List of link names.
    """
    parser = _get_cached_parser(str(urdf_path))
    return parser.get_link_names()
