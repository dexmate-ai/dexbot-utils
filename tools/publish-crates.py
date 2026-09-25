#!/usr/bin/env python3
"""Ordered crates.io publication. A retry verifies the existing package digest."""
import hashlib
import json
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

root = Path(__file__).resolve().parents[1]
version = subprocess.check_output([sys.executable, str(root / "tools/check-release.py")], text=True).strip()
for crate in ("dexbot-model", "dexbot-cli"):
    subprocess.run(["cargo", "package", "--locked", "--registry", "crates-io", "-p", crate], cwd=root, check=True)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version=1"], cwd=root))
    package = Path(metadata["target_directory"]) / "package" / f"{crate}-{version}.crate"
    subprocess.run([sys.executable, str(root / "tools/check-crate.py"), str(package), crate, version], check=True)
    request = urllib.request.Request(f"https://crates.io/api/v1/crates/{crate}/{version}", headers={"User-Agent": "dexbot-utils-release (contact@dexmate.ai)"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            published = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
        published = None
    if published is not None:
        if published["yanked"]:
            sys.exit(f"{crate} {version} is yanked; choose a new version")
        if hashlib.sha256(package.read_bytes()).hexdigest() != published["checksum"]:
            sys.exit(f"{crate} {version} exists with different content; bump the version")
        print(f"{crate} {version} already published with identical content")
    else:
        # Cargo waits for registry availability before returning success.
        subprocess.run(["cargo", "publish", "--locked", "--registry", "crates-io", "-p", crate], cwd=root, check=True)
