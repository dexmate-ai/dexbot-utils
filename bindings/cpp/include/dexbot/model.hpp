// Copyright (C) 2026 Dexmate Inc.
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Dexmate-Commercial
#pragma once
#include <dexbot.h>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace dexbot {
class Error : public std::runtime_error { public: using std::runtime_error::runtime_error; };
namespace detail {
inline void check(bool ok) { if (!ok) throw Error(dexbot_last_error()); }
inline const char* cstr(const std::string& s) {
    if (s.find('\0') != std::string::npos) throw Error("input contains a NUL byte");
    return s.c_str();
}
inline void abi() { if (dexbot_abi_version() != 1) throw Error("incompatible dexbot native ABI"); }
}
// Immutable document views retain their root, including after config destruction.
class Value {
    std::shared_ptr<dexbot_value> owner_;
    const dexbot_value* node_;
    Value(std::shared_ptr<dexbot_value> owner, const dexbot_value* node)
        : owner_(std::move(owner)), node_(node) { detail::check(node_ != nullptr); }
public:
    explicit Value(dexbot_value* owned) : owner_(owned, dexbot_document_free), node_(owned) {
        detail::check(owned != nullptr);
    }
    Value(const Value&) = default;
    Value& operator=(const Value&) = default;
    Value(Value&& other) noexcept
        : owner_(std::move(other.owner_)), node_(std::exchange(other.node_, nullptr)) {}
    Value& operator=(Value&& other) noexcept {
        if (this != &other) {
            owner_ = std::move(other.owner_);
            node_ = std::exchange(other.node_, nullptr);
        }
        return *this;
    }
    Value get(const std::string& key) const { return Value(owner_, dexbot_get(node_, detail::cstr(key))); }
    Value at(std::size_t i) const { return Value(owner_, dexbot_at(node_, i)); }
    int type() const { auto t = dexbot_type(node_); detail::check(t >= 0); return t; }
    std::size_t size() const { if (type() < 4) throw Error("expected collection"); return dexbot_size(node_); }
    bool is_null() const { return type() == 0; }
    std::string string() const {
        std::size_t length = 0;
        const auto* data = dexbot_string(node_, &length);
        detail::check(data != nullptr);
        return std::string(data, length);
    }
    double number() const { double v = 0; detail::check(dexbot_number(node_, &v)); return v; }
    bool boolean() const { bool v = false; detail::check(dexbot_boolean(node_, &v)); return v; }
    std::optional<double> optional_number() const { return is_null() ? std::nullopt : std::optional<double>(number()); }
    std::vector<std::string> strings() const {
        std::vector<std::string> result;
        const auto count = size();
        result.reserve(count);
        for (std::size_t i = 0; i < count; ++i) result.push_back(at(i).string());
        return result;
    }
    std::vector<std::string> keys() const { return Value(dexbot_keys(node_)).strings(); }
    std::string json() const {
        std::unique_ptr<char, decltype(&dexbot_string_free)> data(dexbot_json(node_), dexbot_string_free);
        detail::check(data != nullptr);
        return data.get();
    }
};
struct Joint {
    std::string name, joint_type;
    std::optional<double> lower, upper, effort, velocity;
};
struct Pose { std::vector<double> joint_pos; std::string frame; };
class Component {
    Value config_, metadata_;
    std::string name_;
public:
    Component(Value config, Value metadata, std::string name)
        : config_(std::move(config)), metadata_(std::move(metadata)), name_(std::move(name)) {}
    const std::string& name() const { return name_; }
    std::string driver() const { return config_.get("driver").string(); }
    bool enabled() const { return config_.get("enabled").boolean(); }
    std::vector<std::string> joint_names() const {
        auto joints = config_.get("joints");
        return joints.is_null() ? std::vector<std::string>{} : joints.get("names").strings();
    }
    std::vector<Joint> joints() const {
        const auto names = metadata_.keys();
        bool present = false;
        for (const auto& name : names) if (name == name_) present = true;
        if (!present) return {};
        auto values = metadata_.get(name_);
        std::vector<Joint> result;
        const auto count = values.size();
        result.reserve(count);
        for (std::size_t i = 0; i < count; ++i) {
            auto j = values.at(i);
            result.push_back({j.get("name").string(), j.get("joint_type").string(),
                j.get("lower").optional_number(), j.get("upper").optional_number(),
                j.get("effort").optional_number(), j.get("velocity").optional_number()});
        }
        return result;
    }
    std::vector<std::string> pose_names() const {
        auto metadata = config_.get("metadata");
        for (const auto& key : metadata.keys()) if (key == "pose_pool") return metadata.get(key).keys();
        return {};
    }
    // Stored model pose, without runtime torso compensation (no live joint state).
    Pose pose(const std::string& name) const {
        auto p = config_.get("metadata").get("pose_pool").get(name);
        // Model validation accepts legacy arrays with raw joint semantics.
        Value positions = p;
        std::string frame = "joint";
        if (p.type() != 4) { positions = p.get("joint_pos"); frame = p.get("frame").string(); }
        Pose result{{}, frame};
        const auto count = positions.size();
        result.joint_pos.reserve(count);
        for (std::size_t i = 0; i < count; ++i) result.joint_pos.push_back(positions.at(i).number());
        return result;
    }
    std::string json() const { return config_.json(); }
};
class ResolvedConfig {
    Value value_;
public:
    explicit ResolvedConfig(Value value) : value_(std::move(value)) {}
    std::string profile_name() const { return value_.get("profile_name").string(); }
    std::string model_name() const { return value_.get("robot").get("model").string(); }
    std::string content_hash() const { return value_.get("content_hash").string(); }
    std::vector<std::string> component_names() const { return value_.get("components").keys(); }
    std::vector<std::string> sensor_names() const { return value_.get("sensors").keys(); }
    Component component(const std::string& name) const { return Component(value_.get("components").get(name), value_.get("joint_metadata"), name); }
    Component sensor(const std::string& name) const { return Component(value_.get("sensors").get(name), value_.get("joint_metadata"), name); }
    std::string json() const { return value_.json(); }
    Value document() const { return value_; }
};
class RobotConfig {
    std::string source_;
    bool file_;
    std::optional<std::string> overlay_;
    RobotConfig(std::string source, bool file) : source_(std::move(source)), file_(file) {}
public:
    static RobotConfig from_profile(std::string name) { return RobotConfig(std::move(name), false); }
    static RobotConfig from_file(std::string path) { return RobotConfig(std::move(path), true); }
    RobotConfig with_overlay_yaml(std::string yaml) const {
        auto result = *this; result.overlay_ = std::move(yaml); return result;
    }
    ResolvedConfig resolve() const {
        detail::abi();
        return ResolvedConfig(Value(dexbot_resolve(detail::cstr(source_), file_, overlay_ ? detail::cstr(*overlay_) : nullptr)));
    }
};
inline std::vector<std::string> available_profiles() { detail::abi(); return Value(dexbot_profiles()).strings(); }
inline std::string profile_for_robot_name(const std::string& name) { detail::abi(); return Value(dexbot_profile_for(detail::cstr(name))).string(); }
inline Value parse_urdf(const std::string& xml) { detail::abi(); return Value(dexbot_parse_urdf(detail::cstr(xml))); }
} // namespace dexbot
