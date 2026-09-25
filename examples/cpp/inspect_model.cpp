// Copyright (C) 2026 Dexmate Inc.
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Dexmate-Commercial
#include <dexbot/model.hpp>
#include <iostream>

int main(int argc, char** argv) {
    try {
        for (const auto& profile : dexbot::available_profiles()) {
            auto config = dexbot::RobotConfig::from_profile(profile).resolve();
            if (argc > 1 && std::string(argv[1]) == "--json") {
                std::cout << config.json() << '\n';
            } else {
                auto arm = config.component("left_arm");
                std::cout << profile << ": " << arm.name() << " has " << arm.joint_names().size() << " joints\n";
            }
        }
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
