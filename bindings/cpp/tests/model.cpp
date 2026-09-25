#include <dexbot/model.hpp>
#include <iostream>
#include <stdexcept>

static void require(bool ok) { if (!ok) throw std::runtime_error("SDK regression failed"); }
int main(int argc, char** argv) {
    try {
        for (const auto& profile : dexbot::available_profiles()) {
            auto config = dexbot::RobotConfig::from_profile(profile).resolve();
            require(config.profile_name() == profile);
            if (argc > 1 && std::string(argv[1]) == "--json") {
                std::cout << config.json() << '\n';
                continue;
            }
            auto arm = config.component("left_arm");
            require(arm.joint_names().size() == 7);
            require(arm.joints().size() == 7);
            require(arm.pose("zero").joint_pos.size() == 7);
            std::cout << profile << ": " << arm.name() << " has " << arm.joints().size() << " joints\n";
        }
        auto source = dexbot::parse_urdf("<robot name='move-test'/>");
        {
            auto moved = std::move(source);
            require(moved.get("robot_name").string() == "move-test");
        } // The source must no longer expose a dangling pointer to this root.
        bool empty_rejected = false;
        try { source.json(); } catch (const dexbot::Error&) { empty_rejected = true; }
        require(empty_rejected);
        empty_rejected = false;
        try { source.is_null(); } catch (const dexbot::Error&) { empty_rejected = true; }
        require(empty_rejected);
        auto replacement = dexbot::parse_urdf("<robot name='replacement'/>");
        source = std::move(replacement);
        auto copied = source;
        require(copied.get("robot_name").string() == "replacement");
        empty_rejected = false;
        try { replacement.get("robot_name"); } catch (const dexbot::Error&) { empty_rejected = true; }
        require(empty_rejected);
        auto arm = dexbot::RobotConfig::from_profile("vega_1p").resolve().component("left_arm");
        require(arm.joints().size() == 7); // Root outlives temporary config.
        require(dexbot::profile_for_robot_name("dm/vg0123456789-1p") == "vega_1p");
        bool rejected = false;
        try { dexbot::RobotConfig::from_profile("missing").resolve(); }
        catch (const dexbot::Error&) { rejected = true; }
        require(rejected);
        rejected = false;
        try { dexbot::RobotConfig::from_profile(std::string("vega_1\0oops", 11)).resolve(); }
        catch (const dexbot::Error&) { rejected = true; }
        require(rejected);
        auto overlaid = dexbot::RobotConfig::from_profile("vega_1p")
            .with_overlay_yaml("sensors:\n  head_camera:\n    enabled: true\n").resolve();
        require(overlaid.sensor("head_camera").enabled());
        require(dexbot::RobotConfig::from_profile("vega_1p").resolve().component("battery").joints().empty());
        auto raw = dexbot::RobotConfig::from_profile("vega_1p").with_overlay_yaml(
            "components:\n  left_arm:\n    metadata:\n      pose_pool:\n        raw_test: [0, 0, 0, 0, 0, 0, 0]\n").resolve();
        require(raw.component("left_arm").pose("raw_test").frame == "joint");
        require(raw.component("left_arm").pose("raw_test").joint_pos.size() == 7);
        require(dexbot::parse_urdf("<robot name=\"sample\"><link name=\"base\"/></robot>").get("robot_name").string() == "sample");
        if (argc > 1 && std::string(argv[1]) != "--json") {
            auto custom = dexbot::RobotConfig::from_file(argv[1]).resolve();
            require(custom.model_name() == "test");
            require(custom.component_names().empty());
        }
        return 0;
    } catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
