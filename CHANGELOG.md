# Changelog

All notable changes to this repository are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions are workspace-wide and tags are
the bare version.

## [Unreleased]

### Added

- The `tool-contract/1` profile (`contracts/profile/tool-contract-1.md`): effects, consequences,
  risk with a floor set by effects, idempotency, preconditions, determinism, bounds with a required
  maximum, paging, requires, subjects, placement, capability, output marks and presentation, plus
  the `tool-conformance/1` scenario format.
- `b10x-tool-contract`: the document parser, the validator (every refusal names the operation and
  the property), the ESS type grammar and the shape check against `schemars`-derived schemas, the
  conformance runner (symlink-faithful fixtures, coverage over the behavioural properties, outcome
  and error-field checks), the mutants helper and the `tool-contract validate` binary.
- `fs` v1alpha1 (`fs.read`, `fs.list`, `fs.glob`, `fs.stat`) with its conformance suite, and
  `b10x-tool-fs`, harness 0.13.3's local reader moved here with a whole-file digest on reads.
- `code-editor` v1alpha1 (`code.write`, `code.edit`, `code.patch`, `code.move`, `code.delete`) with
  its conformance suite, and `b10x-tool-code-editor`, harness 0.13.3's local writer moved here with
  `expected_sha256` preconditions, unified diffs and patches resolved against the original.
- `b10x-tool-toolchain`, harness 0.13.3's declarative toolchain registry moved here, deriving
  `build-test` operations under the profile.
