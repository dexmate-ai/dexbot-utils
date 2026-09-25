#!/usr/bin/env python3
"""Validate versions before building or publishing. Python 3.11+."""
import re
import sys
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parents[1]
version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
if not re.fullmatch(r"\d+\.\d+\.\d+", version):
    sys.exit("Only stable x.y.z releases are supported")
if len(sys.argv) > 1 and sys.argv[1] != "v" + version:
    sys.exit("Release tag must match workspace version v" + version)
for crate in ("dexbot-cli", "dexbot-capi"):
    manifest = tomllib.loads((root / "crates" / crate / "Cargo.toml").read_text())
    dep = manifest["dependencies"].get("dexbot-model", manifest["dependencies"].get("model"))
    if dep["version"] != "=" + version:
        sys.exit(crate + " model dependency version mismatch")
if f"project(dexbot VERSION {version} " not in (root / "CMakeLists.txt").read_text():
    sys.exit("CMake project version mismatch")
print(version)
