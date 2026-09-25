pub const PROFILE_NAMES: &[&str] = &[
    "vega_1",
    "vega_1_f5d6",
    "vega_1_gripper",
    "vega_1p",
    "vega_1p_f5d6",
    "vega_1p_gripper",
    "vega_1u",
    "vega_1u_f5d6",
    "vega_1u_gripper",
];

pub fn source(name: &str) -> Option<&'static str> {
    match name {
        "vega_1" => Some(include_str!("../assets/profiles/vega_1.yaml")),
        "vega_1_f5d6" => Some(include_str!("../assets/profiles/vega_1_f5d6.yaml")),
        "vega_1_gripper" => Some(include_str!("../assets/profiles/vega_1_gripper.yaml")),
        "vega_1p" => Some(include_str!("../assets/profiles/vega_1p.yaml")),
        "vega_1p_f5d6" => Some(include_str!("../assets/profiles/vega_1p_f5d6.yaml")),
        "vega_1p_gripper" => Some(include_str!("../assets/profiles/vega_1p_gripper.yaml")),
        "vega_1u" => Some(include_str!("../assets/profiles/vega_1u.yaml")),
        "vega_1u_f5d6" => Some(include_str!("../assets/profiles/vega_1u_f5d6.yaml")),
        "vega_1u_gripper" => Some(include_str!("../assets/profiles/vega_1u_gripper.yaml")),
        _ => None,
    }
}

/// Embedded plain fragments referenced by built-in profiles through
/// `extends`. Fragments are not standalone profiles and are therefore not
/// listed in [`PROFILE_NAMES`].
pub fn fragment_source(name: &str) -> Option<&'static str> {
    match name {
        "common/vega_upper_body.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_upper_body.yaml"
        )),
        "common/vega_mobile_base.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_mobile_base.yaml"
        )),
        "common/vega_1_sensors.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_1_sensors.yaml"
        )),
        "common/vega_1p_sensors.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_1p_sensors.yaml"
        )),
        "common/vega_hands_f5d6.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_hands_f5d6.yaml"
        )),
        "common/vega_hands_gripper.yaml" => Some(include_str!(
            "../assets/profiles/common/vega_hands_gripper.yaml"
        )),
        _ => None,
    }
}
