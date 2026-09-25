#!/usr/bin/env python3
"""Check a source snapshot before public sync; dexbot implementation is public."""
from pathlib import Path
import re
import sys

ROOT_FILES = {"Cargo.toml", "Cargo.lock", "CMakeLists.txt", "LICENSE", "README.md", "CHANGELOG.md", ".gitignore"}
ROOTS = {"crates/dexbot-model", "crates/dexbot-cli", "crates/dexbot-capi", "bindings/c", "bindings/cpp", "cmake", "examples", "robots", "tools", "ci"}
SUFFIXES = {".rs", ".toml", ".lock", ".md", ".yaml", ".yml", ".json", ".urdf", ".xml", ".py", ".sh", ".h", ".hpp", ".cpp", ".cmake", ".in"}
SECRET = re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,}|AKIA[0-9A-Z]{16}")


def check(root):
    for path in root.rglob("*"):
        name = path.relative_to(root).as_posix()
        if name.split("/")[0] == ".git":
            continue
        if path.is_symlink():
            raise ValueError("Public source contains a symlink: " + name)
        if path.is_dir():
            if not any(item == name or item.startswith(name + "/") or name.startswith(item + "/") for item in ROOTS):
                raise ValueError("Unapproved source directory: " + name)
            continue
        if not path.is_file() or (name not in ROOT_FILES and not (any(name.startswith(p + "/") for p in ROOTS) and (path.suffix in SUFFIXES or path.name in {"Cargo.toml", "Cargo.lock", "LICENSE", "CMakeLists.txt"}))):
            raise ValueError("Unapproved source file: " + name)
        data = path.read_bytes()
        if SECRET.search(data):
            raise ValueError("Possible credential in " + name)
        if path.name in {"Cargo.toml", "Cargo.lock"} and re.search(rb"dexcomm|dexcontrol|git\.dexmate|registry\s*=", data):
            raise ValueError("Private dependency in " + name)


if __name__ == "__main__":
    check(Path(sys.argv[1]))
