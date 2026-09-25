# Public CI templates

These templates are copied into the public repository's `.github/workflows/`:

- `test.yml` verifies Rust code and the installed C++ SDK.
- `public_release.yml` and `pypi.yml` are intentionally inactive publishers.
  Both are needed to replace older public workflows at their original paths.

Package builds and publication run in the private repository. The public
repository hosts reviewed source and prebuilt SDK downloads; publishing a public
GitHub release does not publish Rust crates or Python wheels.

Keep `ci/test.yml` synchronized with the private test workflow. The release
checks require the destination's workflows to match these templates before
syncing. Private release setup and the release sequence have one maintained
owner: `.github/RELEASING.md` in the private repository.

For local Rust validation run `bash tools/verify.sh`. For C++ installation and
build instructions, see the repository README. Legacy Python releases remain
available, but this Rust SDK does not build a Python package.
