//! Vega-specific hardware-to-model selection. No hardware queries are performed
//! here: transports supply reports and the effective hand-component overlay.
use crate::{DiscoveryFacts, ModelError, ResolvedRobotConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The end effector carried by both arms of a supported Vega model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndEffector {
    Bare,
    Gripper,
    F5d6,
}

/// Complete hardware evidence is required; omitted or unknown wrists never
/// imply that the force/torque sensor is absent. `reported_hands` records which
/// sides answered, separately from the hand components retained by user policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareReport {
    pub reported_hands: BTreeSet<String>,
    pub wrist_ft: BTreeMap<String, Option<bool>>,
}

/// Callers can display the reason when model selection retains the configured
/// URDF. The model library has no process-global logger or runtime dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSelection {
    Selected { uri: String },
    Unchanged { reason: String },
}

fn unchanged(reason: impl Into<String>) -> ModelSelection {
    ModelSelection::Unchanged {
        reason: reason.into(),
    }
}

fn both_sides<T>(values: &BTreeMap<String, T>) -> bool {
    values.len() == 2 && values.contains_key("left") && values.contains_key("right")
}

/// Selects a bundled model only for complete, agreeing reports from both arms.
/// F5D6 hardware revisions share one model; map both revisions to `F5d6`.
/// Vega-1 has no no-FT model; Vega-1p and Vega-1u support both wrist variants.
pub fn select_model(
    body: &str,
    hands: &BTreeMap<String, EndEffector>,
    wrist_ft: &BTreeMap<String, Option<bool>>,
) -> ModelSelection {
    if !both_sides(hands) || !both_sides(wrist_ft) {
        return unchanged(
            "Both arms must report hand and wrist hardware; configured URDF retained.",
        );
    }
    if hands["left"] != hands["right"] {
        return unchanged("Arms have different end effectors; no matching model exists.");
    }
    let (Some(left), Some(right)) = (wrist_ft["left"], wrist_ft["right"]) else {
        return unchanged("Wrist FT status is unknown; configured URDF retained.");
    };
    if left != right {
        return unchanged("Arms have different wrist hardware; no matching model exists.");
    }
    if !matches!(body, "vega_1" | "vega_1p" | "vega_1u") || (body == "vega_1" && !left) {
        return unchanged("No bundled model supports this body and wrist combination.");
    }
    let wrist = if left { "" } else { "_no_ft" };
    let hand = match hands["left"] {
        EndEffector::Bare => "",
        EndEffector::Gripper => "_gripper",
        EndEffector::F5d6 => "_f5d6",
    };
    ModelSelection::Selected {
        uri: format!("package://dexmate_urdf/robots/humanoid/{body}/{body}{wrist}{hand}.urdf"),
    }
}

/// Applies discovery and selects the URDF for the hand components that will
/// actually run. Transports put hand injection/replacement/disable decisions in
/// `facts.overlay`; a disabled or absent hand means a bare flange. Thus a user
/// retaining a configured hand type does not accidentally get the detected
/// hand's model. Explicit user overlays retain the normal discovery-conflict
/// protection. Missing/inconsistent reports keep the configured URDF.
pub fn apply_discovery(
    config: &ResolvedRobotConfig,
    facts: DiscoveryFacts,
    report: &HardwareReport,
) -> Result<(ResolvedRobotConfig, ModelSelection)> {
    let explicit_urdf = facts
        .overlay
        .0
        .get("robot")
        .and_then(|robot| robot.get("urdf"))
        .is_some();
    crate::model::apply_discovery_with(config, facts, |effective| {
        let selection = if explicit_urdf {
            unchanged("Discovery supplied an explicit URDF; automatic selection skipped.")
        } else {
            select_effective_model(effective, report)
        };
        let empty = || crate::RobotOverlay(serde_json::json!({}));
        if let ModelSelection::Selected { uri } = &selection {
            match crate::load_profile_urdf(uri, config.asset_root.as_deref()) {
                Ok(_) => {
                    return Ok((
                        crate::RobotOverlay(serde_json::json!({"robot": {"urdf": uri}})),
                        selection,
                    ))
                }
                Err(ModelError::Io { source, .. })
                    if source.kind() == std::io::ErrorKind::NotFound =>
                {
                    return Ok((empty(), unchanged("Selected URDF is missing from the configured asset root; configured URDF retained.")));
                }
                Err(error) => return Err(error),
            }
        }
        Ok((empty(), selection))
    })
}

fn select_effective_model(
    effective: &serde_json::Value,
    report: &HardwareReport,
) -> ModelSelection {
    let mut hands = BTreeMap::new();
    for side in &report.reported_hands {
        let component = &effective["components"][format!("{side}_hand")];
        let hand = if component.is_null() || component["enabled"] == false {
            EndEffector::Bare
        } else {
            match component["driver"].as_str() {
                Some("standard.dex_gripper") => EndEffector::Gripper,
                Some("standard.f5d6_hand") => EndEffector::F5d6,
                _ => {
                    return unchanged(
                        "Hand driver has no supported Vega model; configured URDF retained.",
                    )
                }
            }
        };
        hands.insert(side.clone(), hand);
    }
    select_model(
        effective["robot"]["model"].as_str().unwrap_or_default(),
        &hands,
        &report.wrist_ft,
    )
}
