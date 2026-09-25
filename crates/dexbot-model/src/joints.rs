//! URDF-backed joint resolution.
//!
//! During resolution every enabled runtime object with a joint configuration
//! is checked against the robot URDF: `source: urdf` components materialize
//! their ordered joint names from the profile's explicit `joint_groups`
//! table, and every referenced joint must exist in the URDF, be movable, and
//! carry internally consistent limits. The extracted per-joint metadata is
//! exported on the resolved configuration.

use crate::types::{ProfileDocument, ResolvedJoint};
use crate::urdf::{load_profile_urdf, UrdfModel};
use crate::{ModelError, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// The only supported value of `joints.source`.
pub const JOINT_SOURCE_URDF: &str = "urdf";

/// Materializes joint groups and produces URDF-derived joint metadata for
/// every enabled runtime object. `document` is updated in place so that
/// `source: urdf` objects carry their resolved ordered names.
pub fn resolve_joints(
    document: &mut ProfileDocument,
    asset_root: Option<&Path>,
) -> Result<BTreeMap<String, Vec<ResolvedJoint>>> {
    let groups = document.joint_groups.clone();
    let urdf_uri = document.robot.urdf.clone();
    let mut urdf: Option<UrdfModel> = None;
    let mut metadata = BTreeMap::new();
    let objects = document
        .components
        .iter_mut()
        .chain(document.sensors.iter_mut());
    for (name, object) in objects {
        let Some(joints) = object.joints.as_mut() else {
            continue;
        };
        if !object.enabled {
            continue;
        }
        if joints.source.as_deref() == Some(JOINT_SOURCE_URDF) {
            let group = joints.group.as_deref().unwrap_or_default();
            let Some(group_names) = groups.get(group) else {
                return Err(ModelError::Validation(format!(
                    "runtime object {name:?} references URDF joint group {group:?}, \
                     which does not exist in joint_groups"
                )));
            };
            if !joints.names.is_empty() && joints.names != *group_names {
                return Err(ModelError::Validation(format!(
                    "runtime object {name:?} lists joint names that disagree with \
                     joint group {group:?}"
                )));
            }
            if urdf_uri.is_none() {
                return Err(ModelError::Validation(format!(
                    "runtime object {name:?} declares URDF-sourced joints but the \
                     profile has no robot.urdf"
                )));
            }
            joints.names = group_names.clone();
        }
        let Some(uri) = urdf_uri.as_deref() else {
            continue;
        };
        if urdf.is_none() {
            urdf = Some(load_profile_urdf(uri, asset_root)?);
        }
        let model = urdf.as_ref().expect("URDF loaded above");
        let mut resolved = Vec::with_capacity(joints.names.len());
        for joint_name in &joints.names {
            resolved.push(resolve_joint(name, joint_name, model)?);
        }
        metadata.insert(name.clone(), resolved);
    }
    Ok(metadata)
}

fn resolve_joint(object: &str, joint_name: &str, model: &UrdfModel) -> Result<ResolvedJoint> {
    let Some(joint) = model.joint(joint_name) else {
        return Err(ModelError::Validation(format!(
            "runtime object {object:?} references joint {joint_name:?}, which does \
             not exist in URDF {:?}",
            model.robot_name
        )));
    };
    if joint.joint_type == "fixed" {
        return Err(ModelError::Validation(format!(
            "runtime object {object:?} references fixed joint {joint_name:?}"
        )));
    }
    if !matches!(
        joint.joint_type.as_str(),
        "revolute" | "continuous" | "prismatic"
    ) {
        return Err(ModelError::Validation(format!(
            "runtime object {object:?} references unsupported non-scalar joint {joint_name:?} of type {:?}",
            joint.joint_type
        )));
    }
    if let Some(mimic) = &joint.mimic {
        return Err(ModelError::Validation(format!(
            "runtime object {object:?} references joint {joint_name:?}, which mimics \
             {:?} and cannot be commanded",
            mimic.joint
        )));
    }
    let limit = joint.limit.as_ref();
    let lower = limit.and_then(|limit| limit.lower);
    let upper = limit.and_then(|limit| limit.upper);
    let effort = limit.and_then(|limit| limit.effort);
    let velocity = limit.and_then(|limit| limit.velocity);
    // Absent position limits read as "unbounded" downstream. That is what a
    // continuous joint means and never what a revolute or prismatic one
    // does, so a missing or partial <limit> is a broken URDF, not a default.
    if joint.joint_type != "continuous" && (lower.is_none() || upper.is_none()) {
        return Err(ModelError::Validation(format!(
            "{} joint {joint_name:?} needs a <limit> with both lower and upper \
             position limits",
            joint.joint_type
        )));
    }
    for (field, value) in [
        ("lower", lower),
        ("upper", upper),
        ("effort", effort),
        ("velocity", velocity),
    ] {
        // `UrdfModel::parse` already rejects these; a model built or
        // deserialized by hand reaches this function without that check.
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(ModelError::Validation(format!(
                "joint {joint_name:?} has a non-finite {field} limit"
            )));
        }
    }
    if let (Some(lower), Some(upper)) = (lower, upper) {
        if lower > upper {
            return Err(ModelError::Validation(format!(
                "joint {joint_name:?} has inconsistent limits: lower {lower} \
                 exceeds upper {upper}"
            )));
        }
    }
    for (field, value) in [("effort", effort), ("velocity", velocity)] {
        if value.is_some_and(|value| value < 0.0) {
            return Err(ModelError::Validation(format!(
                "joint {joint_name:?} has a negative {field} limit"
            )));
        }
    }
    Ok(ResolvedJoint {
        name: joint.name.clone(),
        joint_type: joint.joint_type.clone(),
        lower,
        upper,
        effort,
        velocity,
    })
}
