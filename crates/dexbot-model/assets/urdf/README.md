# Vendored URDF assets

Canonical URDF XML for the built-in robot profiles, embedded into
`dexbot-model` at compile time. This is the Phase 1 "assets bundled with
`dexbot-model`" strategy from the rewrite plan (Section 6): the Rust model
must not depend on a Python-only `dexmate-urdf` installation.

- Source: `dexmate-urdf` 0.9.0, main branch at commit
  `d5636b5c1c9ecae24ce584459fd6dc8436bf6dff` (merge of upstream PR #88).
  All fifteen files are byte-identical to that commit, so every FT/no-FT
  pair comes from one release: `vega::select_model` swaps one for the
  other, and `tests/vega_wrist_variants.rs` checks the swap only shortens
  the flange by 30.2 mm. Refresh all files together from one commit.
- The gripper open pose follows dexbot-utils PR #27, commit
  `2e0bbe47e1db5fca1715a8eb2346adef051e2c37`: 0.96 rad for both hands.
- Only the `.urdf` XML files used by built-in profiles and Vega hardware selection are
  vendored. Meshes, collision-sphere variants, and every other asset stay in
  the upstream package.
- The directory layout below this file mirrors the upstream package layout,
  so a profile URI `package://dexmate_urdf/<relative-path>` resolves to
  `<this directory>/<relative-path>`.

Resolution order used by `dexbot-model` for `package://dexmate_urdf/...`:

1. An explicit asset root passed through `RobotConfig::with_asset_root`.
2. The `DEXBOT_ASSET_ROOT` environment variable (a directory with this same
   package-relative layout).
3. The embedded copies of the files listed here.

Only the `dexmate_urdf` package name is recognized in `package://` URIs;
other package names fail resolution. Profiles may also use `file://` URIs or
plain filesystem paths within an explicit `with_asset_root` directory;
symlinks outside that root are rejected. Direct trusted files can be read
with `UrdfModel::from_file`. When updating these files, bump the recorded
source package version above and regenerate the resolved-config contract
fixtures in `robots/contracts/`.

## License and scope

Copyright 2025 Dexmate Inc. These vendored XML assets are licensed under
the Apache License, Version 2.0; see [LICENSE](LICENSE). This license is
separate from the library license.

This is a joint-metadata bundle, not a complete visualization or dynamics
package. Meshes remain in `dexmate_urdf`. The metadata parser does not load
geometry or inertial data. Upstream gripper effort/velocity values of zero
are not verified effort/rate specifications; do not use these XML files as validated dynamics
models. Correcting those values requires authoritative motor/CAD data.
