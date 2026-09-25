#!/usr/bin/env python3
"""Inspect the exact Rust archive before publishing approved model source."""
import importlib.util
from pathlib import Path, PurePosixPath
import sys
import tarfile
import tempfile

spec = importlib.util.spec_from_file_location('public_source', Path(__file__).with_name('check-public-source.py'))
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)


def check(archive, crate, version):
    if crate not in {'dexbot-model', 'dexbot-cli'}:
        raise ValueError('Unapproved crate')
    prefix = f'{crate}-{version}'
    seen = set()
    with tempfile.TemporaryDirectory() as temp, tarfile.open(archive, 'r:gz') as packed:
        root = Path(temp)
        for member in packed.getmembers():
            parts = PurePosixPath(member.name).parts
            if (not member.isfile() or not parts or parts[0] != prefix
                    or len(parts) < 2 or '..' in parts or '\\' in member.name
                    or member.name in seen):
                raise ValueError('Unsafe crate entry: ' + member.name)
            seen.add(member.name)
            relative = PurePosixPath(*parts[1:])
            if relative.name == 'Cargo.toml.orig':
                relative = relative.with_name('original-manifest.toml')
            destination = root / 'crates' / crate / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(packed.extractfile(member).read())
        if f'{prefix}/Cargo.toml' not in seen or f'{prefix}/src/lib.rs' not in seen and f'{prefix}/src/main.rs' not in seen:
            raise ValueError('Missing manifest or crate entry point')
        policy.check(root)


if __name__ == '__main__':
    check(Path(sys.argv[1]), sys.argv[2], sys.argv[3])
