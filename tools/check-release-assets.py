#!/usr/bin/env python3
"""Require one valid sidecar for every approved release archive."""
from pathlib import Path
import hashlib
import re
import sys


def check(root, tag):
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("Expected stable release tag")
    files = {}
    pattern = rf"dexbot-(sdk|cli)-{re.escape(tag)}-(linux|macos)-(x86_64|aarch64|arm64)\.tar\.gz"
    for path in root.iterdir():
        if path.is_symlink() or not path.is_file():
            raise ValueError("Unexpected release entry")
        name = path.name.removesuffix(".sha256")
        if not re.fullmatch(pattern, name):
            raise ValueError("Unapproved release asset: " + path.name)
        files[path.name] = path
    archives = {name for name in files if name.endswith(".tar.gz")}
    if not archives or set(files) != archives | {name + ".sha256" for name in archives}:
        raise ValueError("Release files and checksums do not match")
    for name in archives:
        with files[name].open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if files[name + ".sha256"].read_text() != digest + "  " + name + "\n":
            raise ValueError("Invalid checksum: " + name)


if __name__ == "__main__":
    check(Path(sys.argv[1]), sys.argv[2])
