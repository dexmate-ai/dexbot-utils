//! The URDF parser and resource resolution, exercised directly.
//!
//! Profile resolution parses the embedded Vega URDFs as a side effect, but
//! that covers one document shape from one source. What the parser promises
//! is broader: both element forms, attribute unescaping, default axes, and a
//! typed error for every way a hand-edited URDF goes wrong -- which is how
//! URDFs arrive, since they are exported and then patched by hand.

use dexbot_model::{load_profile_urdf, ModelError, PackageResolver, ResourceResolver, UrdfModel};
use std::path::PathBuf;

/// A realistic document: nested elements, a fixed joint, a mimic-style
/// self-closing joint, entity-escaped attribute, explicit and default axes.
const ARM: &str = r#"<?xml version="1.0"?>
<robot name="test&amp;bot">
  <link name="base"/>
  <link name="upper"/>
  <link name="tool"/>
  <joint name="shoulder" type="revolute">
    <parent link="base"/>
    <child link="upper"/>
    <limit lower="-1.5" upper="1.5" effort="10.0" velocity="2.0"/>
    <axis xyz="0 0 1"/>
  </joint>
  <joint name="slide" type="prismatic">
    <parent link="upper"/>
    <child link="tool"/>
    <limit lower="0.0" upper="0.2"/>
    <axis/>
  </joint>
  <joint name="mount" type="fixed"/>
  <joint name="spin" type="continuous">
    <parent link="tool"/>
    <child link="base"/>
  </joint>
</robot>
"#;

#[test]
fn a_realistic_document_parses_completely() {
    let model = UrdfModel::parse(ARM).unwrap();
    assert_eq!(model.robot_name, "test&bot", "attributes are unescaped");
    assert_eq!(model.links, ["base", "tool", "upper"], "sorted");
    assert_eq!(model.joints.len(), 4);

    let shoulder = model.joint("shoulder").unwrap();
    assert_eq!(shoulder.parent.as_deref(), Some("base"));
    assert_eq!(shoulder.child.as_deref(), Some("upper"));
    assert_eq!(shoulder.axis, Some([0.0, 0.0, 1.0]));
    let limit = shoulder.limit.as_ref().unwrap();
    assert_eq!(
        (limit.lower, limit.upper, limit.effort, limit.velocity),
        (Some(-1.5), Some(1.5), Some(10.0), Some(2.0))
    );

    // <axis/> with no xyz is URDF's documented default axis.
    let slide = model.joint("slide").unwrap();
    assert_eq!(slide.axis, Some([1.0, 0.0, 0.0]));
    // A limit may carry only some attributes.
    assert_eq!(slide.limit.as_ref().unwrap().effort, None);

    // A self-closing joint has no children to populate.
    let mount = model.joint("mount").unwrap();
    assert_eq!(mount.joint_type, "fixed");
    assert_eq!((mount.parent.as_deref(), mount.axis), (None, None));

    // No <axis> element at all is None, distinct from <axis/>.
    assert_eq!(model.joint("spin").unwrap().axis, None);
    assert!(model.joint("elbow").is_none());
}

#[test]
fn movable_joints_exclude_fixed_ones() {
    let model = UrdfModel::parse(ARM).unwrap();
    let movable: Vec<&str> = model.movable_joint_names().collect();
    // The fixed mount must not appear: callers map these names onto
    // controllable joints, and a fixed joint has no actuator behind it.
    assert_eq!(movable, ["shoulder", "slide", "spin"]);
}

#[test]
fn every_malformation_is_a_typed_error() {
    let cases: &[(&str, &str)] = &[
        ("<robot><joint name=\"j\" type=\"fixed\"/></robot>", "robot name is missing"),
        ("<robot name=\"r\"><joint type=\"fixed\"/></robot>", "joint name is missing"),
        ("<robot name=\"r\"><joint name=\"j\"/></robot>", "joint type is missing"),
        (
            "<robot name=\"r\"><joint name=\"j\" type=\"revolute\"><axis xyz=\"1 2\"/></joint></robot>",
            "exactly three",
        ),
        (
            "<robot name=\"r\"><joint name=\"j\" type=\"revolute\"><axis xyz=\"1 2 nan\"/></joint></robot>",
            "exactly three finite",
        ),
        (
            "<robot name=\"r\"><joint name=\"j\" type=\"revolute\"><axis xyz=\"a b c\"/></joint></robot>",
            "invalid joint axis",
        ),
        (
            "<robot name=\"r\"><joint name=\"j\" type=\"revolute\"><limit lower=\"wide\"/></joint></robot>",
            "invalid numeric attribute",
        ),
        // Truncation -- an interrupted copy into an asset root -- must not
        // parse as a smaller robot.
        ("<robot name=\"r\"><link name=\"a\">", "unexpected end of file"),
        ("<robot name=\"r\"><joint </robot>", ""),
    ];
    for (source, needle) in cases {
        match UrdfModel::parse(source) {
            Err(ModelError::Urdf(message)) => {
                assert!(message.contains(needle), "{source}: {message}")
            }
            other => panic!("{source}: expected a Urdf error, got {other:?}"),
        }
    }
}

#[test]
fn stray_child_elements_outside_a_joint_are_ignored() {
    // A <parent> or <limit> outside any joint (transmission blocks do this)
    // must not panic or attach to a previous joint.
    let model = UrdfModel::parse(
        "<robot name=\"r\"><parent link=\"x\"/><limit lower=\"1\"/><axis xyz=\"0 0 1\"/>\
         <joint name=\"j\" type=\"fixed\"/></robot>",
    )
    .unwrap();
    let joint = model.joint("j").unwrap();
    assert_eq!(
        (joint.parent.as_deref(), joint.limit.as_ref()),
        (None, None)
    );
}

#[test]
fn file_and_plain_paths_read_the_filesystem() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mini.urdf");
    std::fs::write(&path, "<robot name=\"mini\"><link name=\"l\"/></robot>").unwrap();

    let plain = load_profile_urdf(path.to_str().unwrap(), Some(&dir)).unwrap();
    assert_eq!(plain.robot_name, "mini");
    let uri = format!("file://{}", path.display());
    assert_eq!(load_profile_urdf(&uri, Some(&dir)).unwrap(), plain);

    // A missing file is an Io error naming the path, not a panic.
    let missing = dir.join("absent.urdf");
    match UrdfModel::from_file(&missing) {
        Err(ModelError::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("{other:?}"),
    }
}

#[test]
fn package_uris_resolve_through_the_documented_ladder() {
    // 1. An explicit asset root wins.
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path().to_path_buf();
    std::fs::create_dir_all(dir.join("robots")).unwrap();
    std::fs::write(
        dir.join("robots/override.urdf"),
        "<robot name=\"override\"/>",
    )
    .unwrap();
    let model =
        load_profile_urdf("package://dexmate_urdf/robots/override.urdf", Some(&dir)).unwrap();
    assert_eq!(model.robot_name, "override");

    // 2. With no root, a bundled path parses from the embedded sources --
    //    twice to check deterministic results.
    let first = load_profile_urdf(
        "package://dexmate_urdf/robots/humanoid/vega_1/vega_1.urdf",
        None,
    )
    .unwrap();
    let second = load_profile_urdf(
        "package://dexmate_urdf/robots/humanoid/vega_1/vega_1.urdf",
        None,
    )
    .unwrap();
    assert_eq!(first, second);
    assert!(!first.links.is_empty());

    // 3. Everything unresolvable is a typed error carrying the URI.
    for uri in [
        "package://other_pkg/x.urdf",
        "package://dexmate_urdf/not/bundled.urdf",
        "package://no-slash",
        "http://example.com/x.urdf",
    ] {
        match load_profile_urdf(uri, None) {
            Err(ModelError::UnresolvedResource(reported)) => assert_eq!(reported, uri),
            other => panic!("{uri}: {other:?}"),
        }
    }
}

#[test]
fn the_package_resolver_maps_registered_packages_only() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    std::fs::create_dir(root.join("meshes")).unwrap();
    std::fs::write(root.join("meshes/a.stl"), "mesh").unwrap();
    let resolver = PackageResolver::new().with_package("my_pkg", root);
    assert_eq!(
        resolver.resolve("package://my_pkg/meshes/a.stl").unwrap(),
        root.join("meshes/a.stl").canonicalize().unwrap()
    );
    assert_eq!(
        resolver.resolve("file:///tmp/x.urdf").unwrap(),
        PathBuf::from("/tmp/x.urdf")
    );
    assert_eq!(
        resolver.resolve("plain/path.urdf").unwrap(),
        PathBuf::from("plain/path.urdf")
    );
    for uri in ["package://unknown/x", "package://no-slash", "ftp://host/x"] {
        assert!(
            matches!(
                resolver.resolve(uri),
                Err(ModelError::UnresolvedResource(_))
            ),
            "{uri}"
        );
    }
}

#[test]
fn a_self_closing_robot_element_still_names_the_robot() {
    // Degenerate but legal XML; the parser used to miss the Empty-event form
    // of <robot/> and blame a "missing" name that was present.
    let model = UrdfModel::parse("<robot name=\"bare\"/>").unwrap();
    assert_eq!(model.robot_name, "bare");
    assert!(model.links.is_empty() && model.joints.is_empty());
}

/// `<transmission>` and Gazebo blocks name robot joints with their own
/// `<joint>` elements. Those are references, not joint definitions.
#[test]
fn joint_and_link_elements_outside_the_robot_root_are_references() {
    let model = UrdfModel::parse(
        r#"<robot name="t">
  <link name="base"/><link name="tip"/>
  <joint name="axis" type="revolute">
    <parent link="base"/><child link="tip"/>
    <limit lower="-1" upper="1"/>
  </joint>
  <transmission name="axis_trans">
    <type>transmission_interface/SimpleTransmission</type>
    <joint name="axis">
      <hardwareInterface>hardware_interface/PositionJointInterface</hardwareInterface>
      <limit lower="-9" upper="9"/>
    </joint>
    <joint name="axis"/>
    <actuator name="axis_motor"><mechanicalReduction>1</mechanicalReduction></actuator>
  </transmission>
  <gazebo><link name="base"/><joint name="ghost" type="revolute"/></gazebo>
</robot>"#,
    )
    .unwrap();
    assert_eq!(model.joints.len(), 1);
    assert_eq!(model.links, ["base", "tip"]);
    let limit = model.joint("axis").unwrap().limit.as_ref().unwrap();
    assert_eq!((limit.lower, limit.upper), (Some(-1.0), Some(1.0)));
}

#[test]
fn duplicate_joint_and_link_names_are_rejected() {
    for (source, expected) in [
        (
            r#"<robot name="d"><link name="a"/><link name="a"/></robot>"#,
            "duplicate link name \"a\"",
        ),
        (
            r#"<robot name="d"><joint name="j" type="fixed"/><joint name="j" type="revolute"></joint></robot>"#,
            "duplicate joint name \"j\"",
        ),
        (
            r#"<robot name="d"><joint name="j" type="revolute"><limit lower="0" upper="1"/></joint><joint name="j" type="fixed"/></robot>"#,
            "duplicate joint name \"j\"",
        ),
    ] {
        match UrdfModel::parse(source) {
            Err(ModelError::Urdf(message)) => assert!(message.contains(expected), "{message}"),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn mimic_joints_are_parsed_and_not_listed_as_movable() {
    let model = UrdfModel::parse(
        r#"<robot name="m">
  <joint name="leader" type="revolute"><limit lower="0" upper="1"/></joint>
  <joint name="follower" type="revolute">
    <limit lower="0" upper="1"/>
    <mimic joint="leader" multiplier="1.5" offset="-0.25"/>
  </joint>
  <joint name="shadow" type="revolute"><mimic joint="leader"/></joint>
</robot>"#,
    )
    .unwrap();
    let follower = model.joint("follower").unwrap().mimic.as_ref().unwrap();
    assert_eq!(
        (
            follower.joint.as_str(),
            follower.multiplier,
            follower.offset
        ),
        ("leader", 1.5, -0.25)
    );
    // URDF defaults: multiplier 1, offset 0.
    let shadow = model.joint("shadow").unwrap().mimic.as_ref().unwrap();
    assert_eq!((shadow.multiplier, shadow.offset), (1.0, 0.0));
    assert_eq!(model.joint("leader").unwrap().mimic, None);
    assert_eq!(model.movable_joint_names().collect::<Vec<_>>(), ["leader"]);
    assert!(matches!(
        UrdfModel::parse(r#"<robot name="m"><joint name="j" type="revolute"><mimic/></joint></robot>"#),
        Err(ModelError::Urdf(message)) if message.contains("mimic joint is missing")
    ));
}

#[test]
fn the_shipped_hand_urdfs_expose_their_mimic_joints() {
    let model = load_profile_urdf(
        "package://dexmate_urdf/robots/humanoid/vega_1/vega_1_f5d6.urdf",
        None,
    )
    .unwrap();
    assert_eq!(
        model
            .joint("L_th_j2")
            .unwrap()
            .mimic
            .as_ref()
            .unwrap()
            .joint,
        "L_th_j1"
    );
    assert!(!model.movable_joint_names().any(|name| name == "L_th_j2"));
    assert!(model.movable_joint_names().any(|name| name == "L_th_j1"));
}

#[test]
fn invalid_roots_and_empty_names_are_rejected() {
    for source in [
        "<not_robot name='r'/>",
        "<robot name='a'/><robot name='b'/>",
        "<robot name=''/>",
        "<robot name='r'><link/></robot>",
        "<robot name='r'><link name=' '/></robot>",
        "<robot name='r'><joint name='' type='fixed'/></robot>",
        "<robot name='r'><joint name='j' type=''/></robot>",
        "text<robot name='r'/>",
        "<robot name='r'/>text",
    ] {
        assert!(
            matches!(UrdfModel::parse(source), Err(ModelError::Urdf(_))),
            "{source}"
        );
    }
}

#[test]
fn profile_resources_stay_inside_the_authorized_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("assets");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(dir.path().join("outside.urdf"), "<robot name='outside'/>").unwrap();
    std::fs::write(root.join("inside.urdf"), "<robot name='inside'/>").unwrap();
    assert_eq!(
        load_profile_urdf("inside.urdf", Some(&root))
            .unwrap()
            .robot_name,
        "inside"
    );
    assert!(load_profile_urdf(root.join("inside.urdf").to_str().unwrap(), None).is_err());
    for uri in ["../outside.urdf", "package://dexmate_urdf/../outside.urdf"] {
        assert!(load_profile_urdf(uri, Some(&root)).is_err(), "{uri}");
    }
    let resolver = PackageResolver::new().with_package("p", &root);
    assert!(resolver.resolve("package://p/../outside.urdf").is_err());
    assert!(resolver.resolve("package://p//etc/passwd").is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("outside.urdf"), root.join("escape.urdf"))
            .unwrap();
        assert!(load_profile_urdf("escape.urdf", Some(&root)).is_err());
        assert!(resolver.resolve("package://p/escape.urdf").is_err());
    }
}

#[test]
fn extends_cannot_escape_the_profile_directory() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("profiles");
    std::fs::create_dir(&root).unwrap();
    let outside = dir.path().join("outside.yaml");
    std::fs::write(&outside, "robot: {model: outside}\n").unwrap();
    for fragment in ["../outside.yaml", outside.to_str().unwrap()] {
        let text = format!("schema_version: 1\nextends: [{fragment:?}]\n");
        assert!(dexbot_model::RobotConfig::from_yaml_in("test", &text, &root).is_err());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, root.join("escape.yaml")).unwrap();
        assert!(dexbot_model::RobotConfig::from_yaml_in(
            "test",
            "schema_version: 1\nextends: [escape.yaml]\n",
            &root
        )
        .is_err());
    }
}

#[test]
fn duplicate_attributes_cannot_silently_override_metadata() {
    for source in [
        r#"<robot name="first" name="second"/>"#,
        r#"<robot name="r"><link name="a" name="b"/></robot>"#,
        r#"<robot name="r"><joint name="a" type="fixed" type="revolute"/></robot>"#,
        r#"<robot name="r"><joint name="a" type="revolute"><limit lower="0" upper="1" upper="2"/></joint></robot>"#,
        r#"<robot name="r"><joint name="a" type="revolute"><mimic joint=""/></joint></robot>"#,
    ] {
        assert!(
            matches!(UrdfModel::parse(source), Err(ModelError::Urdf(_))),
            "{source}"
        );
    }
}
