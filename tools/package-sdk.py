#!/usr/bin/env python3
"""Archive an installed SDK and standalone CLI with SHA-256 sidecars."""
from pathlib import Path
import hashlib
import platform
import re
import subprocess
import sys
import tarfile

root = Path(__file__).resolve().parents[1]
version = subprocess.check_output([sys.executable, str(root / "tools/check-release.py")], text=True).strip()
stage, output = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
system = "macos" if platform.system() == "Darwin" else platform.system().lower()
label = sys.argv[3] if len(sys.argv) > 3 else f"{system}-{platform.machine().lower()}"
if not re.fullmatch(r"(?:linux|macos)-(?:x86_64|aarch64|arm64)", label):
    sys.exit("Unsupported release platform")
output.mkdir(parents=True, exist_ok=True)
required = {
    "include/dexbot.h", "include/dexbot/model.hpp", "bin/dexbot",
    "lib/cmake/dexbot/dexbotConfig.cmake", "lib/cmake/dexbot/dexbotConfigVersion.cmake",
    "share/dexbot/LICENSE", "share/dexbot/LICENSE-URDF", "share/dexbot/README.md",
    "share/dexbot/examples/cpp/CMakeLists.txt", "share/dexbot/examples/cpp/inspect_model.cpp",
    "lib/libdexbot_model.dylib" if label.startswith("macos-") else "lib/libdexbot_model.so",
}
found = set()
for path in stage.rglob("*"):
    name = path.relative_to(stage).as_posix()
    if path.is_symlink() or not (path.is_file() or path.is_dir()):
        sys.exit("Unsupported SDK entry: " + name)
    if path.is_dir():
        if not any(item.startswith(name + "/") for item in required):
            sys.exit("Unapproved SDK directory: " + name)
    elif name not in required:
        sys.exit("Unapproved SDK file: " + name)
    else:
        found.add(name)
if found != required:
    sys.exit("Missing SDK files: " + ", ".join(sorted(required - found)))
for product in ("sdk", "cli"):
    stem = f"dexbot-{product}-v{version}-{label}"
    path = output / (stem + ".tar.gz")
    with tarfile.open(path, "w:gz") as archive:
        if product == "sdk":
            archive.add(stage, arcname=stem)
        else:
            archive.add(stage / "bin/dexbot", arcname=stem + "/bin/dexbot")
            archive.add(root / "LICENSE", arcname=stem + "/LICENSE")
            archive.add(stage / "share/dexbot/LICENSE-URDF", arcname=stem + "/LICENSE-URDF")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    path.with_name(path.name + ".sha256").write_text(digest + "  " + path.name + "\n")
    print(path)
