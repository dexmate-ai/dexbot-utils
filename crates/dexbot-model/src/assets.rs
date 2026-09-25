//! Embedded URDF assets vendored from the `dexmate-urdf` distribution.
//!
//! See `assets/urdf/README.md` for the vendoring policy and the recorded
//! source package version.

/// The only `package://` name resolvable by the bundled asset tree.
pub const URDF_PACKAGE_NAME: &str = "dexmate_urdf";

/// Environment variable naming an external asset root directory that mirrors
/// the package-relative layout of `assets/urdf/`.
pub const ASSET_ROOT_ENV: &str = "DEXBOT_ASSET_ROOT";

macro_rules! urdf_asset {
    ($relative:literal) => {
        (
            $relative,
            include_str!(concat!("../assets/urdf/", $relative)),
        )
    };
}

const URDF_SOURCES: &[(&str, &str)] = &[
    urdf_asset!("robots/humanoid/vega_1p/vega_1p_no_ft.urdf"),
    urdf_asset!("robots/humanoid/vega_1p/vega_1p_no_ft_gripper.urdf"),
    urdf_asset!("robots/humanoid/vega_1p/vega_1p_no_ft_f5d6.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u_no_ft.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u_no_ft_gripper.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u_no_ft_f5d6.urdf"),
    urdf_asset!("robots/humanoid/vega_1/vega_1.urdf"),
    urdf_asset!("robots/humanoid/vega_1/vega_1_f5d6.urdf"),
    urdf_asset!("robots/humanoid/vega_1/vega_1_gripper.urdf"),
    urdf_asset!("robots/humanoid/vega_1p/vega_1p.urdf"),
    urdf_asset!("robots/humanoid/vega_1p/vega_1p_f5d6.urdf"),
    urdf_asset!("robots/humanoid/vega_1p/vega_1p_gripper.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u_f5d6.urdf"),
    urdf_asset!("robots/humanoid/vega_1u/vega_1u_gripper.urdf"),
];

/// Returns the canonical `'static` asset path together with the embedded
/// URDF XML for a package-relative path, if bundled. Callers key
/// process-wide caches by the returned asset path.
pub fn urdf_entry(relative_path: &str) -> Option<(&'static str, &'static str)> {
    URDF_SOURCES
        .iter()
        .find(|(path, _)| *path == relative_path)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_asset_is_nonempty_xml() {
        for (path, source) in URDF_SOURCES {
            assert!(
                source.contains("<robot"),
                "asset {path} does not look like URDF"
            );
        }
        assert!(urdf_entry("robots/humanoid/vega_1/vega_1.urdf").is_some());
        assert!(urdf_entry("missing.urdf").is_none());
    }
}
