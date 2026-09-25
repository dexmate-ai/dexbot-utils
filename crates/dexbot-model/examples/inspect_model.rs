use dexbot_model::{available_profiles, RobotConfig};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for profile in available_profiles() {
        let config = RobotConfig::from_profile(profile)?.resolve()?;
        let arm = &config.components["left_arm"];
        println!(
            "{}: left_arm has {} joints",
            profile,
            arm.joints.as_ref().unwrap().names.len()
        );
    }
    Ok(())
}
