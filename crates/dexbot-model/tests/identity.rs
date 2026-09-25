use dexbot_model::{
    available_profiles, profile_for_robot_name, try_profile_for_robot_name, ModelError,
};

#[test]
fn identity_v2_and_legacy_names_select_the_same_variants() {
    for (token, suffix, profile) in [
        ("vg1", "1", "vega_1"),
        ("vg1u", "1u", "vega_1u"),
        ("vg1p", "1p", "vega_1p"),
    ] {
        for identity in [
            format!("dm/{token}-0123456789"),
            format!("dm-{token}-0123456789"),
            format!("dm/vg0123456789-{suffix}"),
        ] {
            assert_eq!(profile_for_robot_name(&identity), profile);
            assert_eq!(
                profile_for_robot_name(&identity.to_ascii_uppercase()),
                profile
            );
        }
    }
    for profile in available_profiles() {
        assert_eq!(profile_for_robot_name(profile), *profile);
    }
    assert_eq!(profile_for_robot_name("unknown"), "vega_1");
}

#[test]
fn fallible_resolution_accepts_every_documented_spelling() {
    for (token, suffix, profile) in [
        ("vg1", "1", "vega_1"),
        ("vg1u", "1u", "vega_1u"),
        ("vg1p", "1p", "vega_1p"),
    ] {
        for identity in [
            format!("dm/{token}-0123456789"),
            format!("dm-{token}-0123456789"),
            format!("dm/vg0123456789-{suffix}"),
            format!("dm/vgabcd123456-{suffix}"),
            // Both spellings of the variant, agreeing.
            format!("dm/{token}-0123456789-{suffix}"),
            format!("  DM/{}-0123456789\n", token.to_ascii_uppercase()),
            // The deployed form: a serial followed by a check character, and
            // serials that are themselves dashed.
            format!("dm/{token}-7k3mp9wd2r6x8t4q-c"),
            format!("DM-{}-7K3MP9WD2R6X8T4Q-C", token.to_ascii_uppercase()),
            format!("dm/{token}-7k3mp9wd2r6x8t4q-1"),
            format!("dm/{token}-unit-test"),
        ] {
            assert_eq!(
                try_profile_for_robot_name(&identity).unwrap(),
                profile,
                "{identity:?}"
            );
            // The legacy total form agrees wherever the fallible form succeeds.
            assert_eq!(profile_for_robot_name(&identity), profile);
        }
    }
    for profile in available_profiles() {
        assert_eq!(try_profile_for_robot_name(profile).unwrap(), *profile);
        assert_eq!(
            try_profile_for_robot_name(&format!(" {} ", profile.to_ascii_uppercase())).unwrap(),
            *profile
        );
    }
}

#[test]
fn fallible_resolution_fails_closed_on_unrecognised_names() {
    for name in [
        "",
        "   ",
        "unknown",
        // Typo of a built-in profile.
        "vega_1p_grippr",
        // Future models and tokens this build has no profile for.
        "dm/vg2-000123",
        "dm/vg1pro-0001",
        "dm/vg2-000123-1",
        "dm/ab0123456789-1",
        // Wrong separator.
        "dm_vg1p-0001",
        // Token and an unmistakable version suffix disagree. (A trailing
        // `-1` is not listed: it may be a check character.)
        "dm/vg1-abc-1p",
        "dm/vg1p-0123456789-1u",
        // Legacy names with an unknown version, short serial or no version.
        "dm/vg0123456789-2",
        "dm/vg0123456789-rc1",
        "dm/xyz-1u",
        "dm/vg0123456789",
        "dm/vg1p-",
        "dm/vg1p-00 01",
    ] {
        let error = try_profile_for_robot_name(name).unwrap_err();
        assert!(
            matches!(&error, ModelError::UnknownRobotName { name: reported, .. } if reported == name),
            "{name:?}: {error}"
        );
    }
    // Case and surrounding whitespace are not identity: these used to fall
    // through to vega_1.
    assert_eq!(try_profile_for_robot_name("VEGA_1P").unwrap(), "vega_1p");
    assert_eq!(profile_for_robot_name("VEGA_1P"), "vega_1p");
    assert_eq!(profile_for_robot_name("vega_1u "), "vega_1u");
    // The legacy total form keeps its historical guess for rejected names.
    assert_eq!(profile_for_robot_name("dm/vg2-000123"), "vega_1");
    assert_eq!(profile_for_robot_name("dm/xyz-1u"), "vega_1u");
    assert_eq!(profile_for_robot_name("dm/vg1-abc-1p"), "vega_1");
}
