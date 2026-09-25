<div align="center">
  <h1>Dexmate Robot Model Utilities</h1>
</div>

The robot model behind every Dexmate client: built-in robot profiles, their
resolution into an operational configuration, URDF-backed joint metadata, and
a CLI for inspecting, validating, diffing and migrating profiles.

The model implementation is Rust, with a C ABI and a C++17 wrapper. There is
no standalone Python library: Python runtime consumers get model data through [dexcontrol](https://github.com/dexmate-ai/dexcontrol), which
embeds this crate and exposes it in every language it supports.

| You are writing… | Use |
| --- | --- |
| Rust model/configuration tooling | `dexbot-model` from crates.io |
| Python that controls a robot | `dexcontrol.robot_config()`, `dexcontrol.available_profiles()` |
| C++ model/configuration tooling | This repository’s SDK (`dexbot::model`) |
| Shell scripts, CI, operators | the `dexbot` CLI |

## Install in a Rust project

```bash
cargo add dexbot-model
```

Source is published to crates.io. Profiles and URDFs are embedded; neither
DexComm nor a robot connection is required. The command above becomes available
when the first crate release is published.

## Install in a C++ project

Download the matching **dexbot-sdk** archive from
[GitHub Releases](https://github.com/dexmate-ai/dexbot-utils/releases).
SDK names follow `dexbot-sdk-v0.2.3-linux-x86_64.tar.gz` (also Linux aarch64,
macOS arm64 and macOS x86_64). A separate `dexbot-cli` archive contains just the
CLI. Check the SHA-256 sidecar before extracting. These assets are produced by
the release workflow; adding this CI does not itself publish a release.

```bash
sha256sum -c dexbot-sdk-v0.2.3-linux-x86_64.tar.gz.sha256
# On macOS: shasum -a 256 -c ARCHIVE.tar.gz.sha256
tar -xzf dexbot-sdk-v0.2.3-linux-x86_64.tar.gz
cmake -S . -B build -DCMAKE_PREFIX_PATH="$PWD/dexbot-sdk-v0.2.3-linux-x86_64"
cmake --build build
```

In your application's CMakeLists.txt:

```cmake
find_package(dexbot 0.2 CONFIG REQUIRED)
target_link_libraries(my_app PRIVATE dexbot::model)
```

```cpp
#include <dexbot/model.hpp>

auto config = dexbot::RobotConfig::from_profile("vega_1p").resolve();
auto arm = config.component("left_arm");
for (const auto& joint : arm.joints()) {
    // joint.name, joint.joint_type, optional lower/upper/effort/velocity
}
auto pose = arm.pose("zero");  // stored joint positions and their reference frame
auto json = config.json();    // complete configuration, including advanced fields
```

C++ model loading, overlays and resolution throw `dexbot::Error` on failure.
Components and document views retain their underlying immutable configuration.
Model poses are stored values, not runtime torso compensation; use dexcontrol
when resolving poses against live robot state. The public C ABI is documented
in `bindings/c/include/dexbot.h`. ABI and SDK version checks reject incompatible
libraries. Keep the SDK `lib/` directory available when running applications:
the CMake build adds its runtime path, but deploying your application requires
shipping the shared library too (or configuring an installation RPATH).

Prebuilt SDKs currently target Linux with glibc 2.35+ and macOS 14+; other
platforms should build from source. Windows and vcpkg registry publication are
not part of this initial release. The prebuilt C++ SDK does not require Rust.

### Build and install from source

Requires Rust/Cargo, CMake 3.20+, and a C/C++17 compiler:

```bash
cmake -S . -B build -DCMAKE_INSTALL_PREFIX="$HOME/.local"
cmake --build build --parallel 2
cmake --install build
```

Use `-DCMAKE_PREFIX_PATH="$HOME/.local"` in consuming projects. See
`examples/cpp` and `crates/dexbot-model/examples/inspect_model.rs` for matching
profile inspection tasks. Release configuration is documented in
[ci/README.md](ci/README.md).

## Layout

- `crates/dexbot-model` — the library. Profiles and URDFs are embedded at
  compile time, so a consumer needs nothing on disk.
- `crates/dexbot-cli` — the `dexbot` binary.
- `crates/dexbot-model/assets/profiles` — the built-in profiles (`vega_1`,
  `vega_1u`, `vega_1p`, and their `_f5d6` / `_gripper` hand variants) and the
  fragments they `extends`. See its `README.md` for the profile format.
- `robots/contracts` — frozen resolved-configuration contracts the tests pin.

## Library

```toml
[dependencies]
dexbot-model = "0.2"
```

```rust
use dexbot_model::{available_profiles, try_profile_for_robot_name, RobotConfig};

// Built-in profiles, and the one a ROBOT_NAME-style identity selects. An
// unrecognised name is an error, never a guess.
assert!(available_profiles().contains(&"vega_1"));
assert_eq!(try_profile_for_robot_name("dm/vg0123456789-1u")?, "vega_1u");
assert!(try_profile_for_robot_name("dm/vg2-000123").is_err());

// Resolve a profile into the operational configuration a robot runs against.
let resolved = RobotConfig::from_profile("vega_1")?
    .with_sensor_enabled("head_camera")
    .resolve()?;
let arm = &resolved.components["left_arm"];
println!("{:?}", arm.joints.as_ref().map(|joints| &joints.names));
println!("{}", resolved.normalized_json()?);

// A custom profile file (YAML or JSON) composed from common/*.yaml fragments.
// To customize a complete built-in profile, apply an overlay instead:
let customized = RobotConfig::from_profile("vega_1")?
    .with_overlay_file("my_overlay.yaml")?
    .resolve()?;
let custom = RobotConfig::from_file("my_robot.yaml")?.resolve()?;
# Ok::<(), dexbot_model::ModelError>(())
```

`package://dexmate_urdf/...` references resolve against an explicit asset
root, then `DEXBOT_ASSET_ROOT`, then the embedded URDFs.

## CLI

```bash
cargo install dexbot-cli --locked       # after registry publication
# From a source checkout: cargo install --locked --path crates/dexbot-cli

dexbot list                              # built-in profiles
dexbot show vega_1                       # normalized resolved configuration
dexbot validate my_robot.yaml            # schema and model validation
dexbot diff vega_1 my_robot.yaml         # field-level diff of two resolutions
dexbot diff --exit-code a.yaml b.yaml    # ...exiting 1 when they differ
dexbot migrate my_robot.yaml             # to the current schema version
dexbot urdf robot.urdf                   # links, joints, movable joints
dexbot profile-for dm/vg0123456789-1p    # -> vega_1p
```

## Development

```bash
tools/verify.sh    # fmt, clippy -D warnings, tests, 90% line coverage
```

## Licensing

This project is **dual-licensed**:

### Open Source License
This software is available under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.
See the [LICENSE](./LICENSE) file for details.

### Commercial License
For businesses that want to use this software in proprietary applications without the AGPL requirements, commercial licenses are available.

---

<div align="center">
  <h3>🤝 Ready to build amazing robots?</h3>
  <p>
    <a href="mailto:contact@dexmate.ai">📧 Contact Us</a> •
  </p>
</div>

### Named pose frames

Each `metadata.pose_pool` entry contains `joint_pos` and `frame` together.
Both fields are required for structured entries. Unknown fields, unknown frames,
non-numeric values and incorrect joint counts fail model validation.

```yaml
metadata:
  pose_pool:
    zero:
      joint_pos: [0, 0, 0, 0, 0, 0, 0]
      frame: torso_upright
    folded:
      joint_pos: [1.57079, 0, 0, -3.07, 0, 0, -0.69813]
      frame: joint
```

Frames are `joint`, `torso_horizontal`, or `torso_upright`. The latter two are
model-specific torso reference conventions, not Cartesian world frames.
Legacy array-only poses retain raw joint semantics. The separate `pose_frames`
map is rejected: move each frame into its pose entry.

Vega `folded` and `folded_closed_hand` are joint-relative. `L_shape` and `lift_up` use the `torso_horizontal` reference. `zero` stores seven zeros
in the `torso_upright` frame: its compensation is relative to an upright torso
(`torso_pitch - pi/2`). Passthrough returns the stored zeros. Head `home` is compensated, while `tucked` stays joint-relative.
Consumers should resolve a named pose once and validate the resulting joint
limits; they must not apply compensation again to a resolved target.
