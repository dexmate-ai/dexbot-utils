//! The FT and no-FT URDFs of one Vega body and end effector must describe the
//! same robot apart from the wrist: `vega::select_model` swaps one for the
//! other, so any other difference silently moves the tool frame. Dropping the
//! wrist FT sensor shortens the flange by 30.2 mm and changes nothing else.

use dexbot_model::UrdfModel;
use quick_xml::{events::Event, Reader};
use std::collections::BTreeMap;
use std::path::PathBuf;

const WRIST_FT_LENGTH_MM: f64 = 30.2;

type Transform = [[f64; 4]; 4];

fn source(stem: &str) -> String {
    let body = stem.split("_no_ft").next().unwrap();
    let body = ["_gripper", "_f5d6"]
        .iter()
        .fold(body, |name, hand| name.trim_end_matches(hand));
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/urdf/robots/humanoid")
            .join(body)
            .join(format!("{stem}.urdf")),
    )
    .unwrap()
}

fn numbers(value: Option<String>) -> [f64; 3] {
    let values: Vec<f64> = value
        .as_deref()
        .unwrap_or("0 0 0")
        .split_whitespace()
        .map(|item| item.parse().unwrap())
        .collect();
    [values[0], values[1], values[2]]
}

fn origin(xyz: [f64; 3], [r, p, y]: [f64; 3]) -> Transform {
    let (sr, cr, sp, cp, sy, cy) = (r.sin(), r.cos(), p.sin(), p.cos(), y.sin(), y.cos());
    [
        [
            cy * cp,
            cy * sp * sr - sy * cr,
            cy * sp * cr + sy * sr,
            xyz[0],
        ],
        [
            sy * cp,
            sy * sp * sr + cy * cr,
            sy * sp * cr - cy * sr,
            xyz[1],
        ],
        [-sp, cp * sr, cp * cr, xyz[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn compose(a: &Transform, b: &Transform) -> Transform {
    let mut out = [[0.0; 4]; 4];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

/// Joint origins keyed by child link: (parent link, parent-to-child transform).
fn tree(source: &str) -> BTreeMap<String, (String, Transform)> {
    let mut reader = Reader::from_str(source);
    let mut buffer = Vec::new();
    let mut edges = BTreeMap::new();
    let (mut in_joint, mut parent, mut child) = (false, String::new(), String::new());
    let mut transform = origin([0.0; 3], [0.0; 3]);
    loop {
        let event = reader.read_event_into(&mut buffer).unwrap();
        let attribute = |event: &quick_xml::events::BytesStart<'_>, key: &[u8]| {
            event
                .try_get_attribute(key)
                .unwrap()
                .map(|value| value.unescape_value().unwrap().into_owned())
        };
        match &event {
            Event::Start(tag) | Event::Empty(tag) => match tag.name().as_ref() {
                b"joint" => {
                    in_joint = true;
                    transform = origin([0.0; 3], [0.0; 3]);
                }
                b"parent" if in_joint => parent = attribute(tag, b"link").unwrap(),
                b"child" if in_joint => child = attribute(tag, b"link").unwrap(),
                b"origin" if in_joint => {
                    transform = origin(
                        numbers(attribute(tag, b"xyz")),
                        numbers(attribute(tag, b"rpy")),
                    )
                }
                _ => {}
            },
            Event::End(tag) if tag.name().as_ref() == b"joint" => {
                in_joint = false;
                edges.insert(child.clone(), (parent.clone(), transform));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    edges
}

/// Position of `link` in the root frame with every joint at zero, in mm.
fn position_mm(edges: &BTreeMap<String, (String, Transform)>, link: &str) -> [f64; 3] {
    let mut pose = origin([0.0; 3], [0.0; 3]);
    let mut current = link;
    while let Some((parent, transform)) = edges.get(current) {
        pose = compose(transform, &pose);
        current = parent;
    }
    [pose[0][3] * 1e3, pose[1][3] * 1e3, pose[2][3] * 1e3]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

#[test]
fn dropping_the_wrist_sensor_only_shortens_the_flange() {
    for body in ["vega_1p", "vega_1u"] {
        for (hand, tools) in [
            ("", &["L_ee", "R_ee"][..]),
            (
                "_gripper",
                &["L_ee", "R_ee", "L_gripper_base", "R_gripper_base"][..],
            ),
            ("_f5d6", &["L_ee", "R_ee", "L_hand_base", "R_hand_base"][..]),
        ] {
            let fitted = source(&format!("{body}{hand}"));
            let absent = source(&format!("{body}_no_ft{hand}"));
            let name = format!("{body}{hand}");

            let joints = |source: &str| {
                let model = UrdfModel::parse(source).unwrap();
                model
                    .joints
                    .into_iter()
                    .filter(|joint| joint.joint_type != "fixed")
                    .map(|joint| (joint.name, (joint.joint_type, joint.limit, joint.axis)))
                    .collect::<BTreeMap<_, _>>()
            };
            assert_eq!(joints(&fitted), joints(&absent), "{name}: movable joints");

            let (fitted, absent) = (tree(&fitted), tree(&absent));
            for arm_link in ["L_arm_l7", "R_arm_l7"] {
                let moved = distance(
                    position_mm(&fitted, arm_link),
                    position_mm(&absent, arm_link),
                );
                assert!(moved < 0.01, "{name}: {arm_link} moved {moved:.2} mm");
            }
            for tool in tools {
                let moved = distance(position_mm(&fitted, tool), position_mm(&absent, tool));
                assert!(
                    (moved - WRIST_FT_LENGTH_MM).abs() < 0.05,
                    "{name}: {tool} moved {moved:.2} mm, expected {WRIST_FT_LENGTH_MM} mm"
                );
            }
        }
    }
}
