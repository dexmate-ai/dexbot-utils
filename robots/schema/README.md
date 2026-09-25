# Robot schemas

`profile-v1.schema.json` describes a composed source profile after `extends`
fragments have been merged. It requires `schema_version: 1` and `robot.model`;
it is not a schema for individual fragments or partial overlays.

`resolved-config-v0.schema.json` describes the normalized output from
`dexbot show` and `dexbot validate --json`. Frozen fixtures are checked against
it in the Rust test suite. It includes readiness and safety vocabularies.

Use `dexbot validate` for full semantic validation, including dependency
cycles, component-specific safety roles and timeout bounds, cross-field
consistency, joint limits, and filesystem resource containment. JSON Schema
validation alone does not perform those checks.
