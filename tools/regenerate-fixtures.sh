#!/usr/bin/env bash
# Rewrites the frozen resolved-config contracts from the built-in profiles.
# Review the resulting diff: every change is a contract change for consumers.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/robots/contracts/resolved-config/v0"
# Capture failures before touching any fixture; stage on the same filesystem.
stage="$(mktemp -d "$out/.regenerate.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
cargo build --quiet --manifest-path "$root/Cargo.toml" -p dexbot-cli --message-format=json >"$stage/build.jsonl"
# Use Cargo's exact executable path, including custom target directories.
cli="$(python3 - "$stage/build.jsonl" <<'PYTHON'
import json, sys
artifacts = [json.loads(line) for line in open(sys.argv[1])]
paths = [item['executable'] for item in artifacts
         if item.get('reason') == 'compiler-artifact'
         and item.get('target', {}).get('name') == 'dexbot' and item.get('executable')]
if len(paths) != 1:
    sys.exit('Expected exactly one dexbot executable; fixtures unchanged.')
print(paths[0])
PYTHON
)"
profiles="$("$cli" list)"
[[ -n "$profiles" ]] || { echo 'No profiles returned; fixtures unchanged.' >&2; exit 1; }
while IFS= read -r profile; do
    [[ "$profile" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Invalid profile name; fixtures unchanged.' >&2; exit 1; }
    "$cli" show "$profile" >"$stage/$profile.json"
done <<<"$profiles"
for fixture in "$stage"/*.json; do
    mv "$fixture" "$out/"
done
