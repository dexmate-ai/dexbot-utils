//! Normalized grasp-torque bounds for Dexmate S- and D-grippers.
//!
//! These constants describe the `standard.dex_gripper` driver. They are not
//! physical torque values or universal limits for other gripper models.

/// Lowest valid normalized grip-force limit (no grasping effort).
pub const GRASP_TORQUE_MIN: f64 = 0.0;
/// Highest valid normalized grip-force limit (hardware maximum).
pub const GRASP_TORQUE_MAX: f64 = 1.0;
/// Clients should warn above this value: a stalled gripper can damage its motor.
pub const GRASP_TORQUE_HIGH_THRESHOLD: f64 = 0.5;
