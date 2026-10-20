# Changelog

All notable changes to this repository are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions are workspace-wide and tags are
the bare version.

## [Unreleased]

### Added

- `process` v1alpha1 (`process.run`) with its conformance suite (31 scenarios), and
  `b10x-tool-process`, harness 0.14.1's local process runner moved here: an argv and never a
  command line, a declared program set, an environment cleared to an allow-list, a caller's
  `timeout_ms` capped at 600000 and by what the run has left, the whole process group killed at the
  bound, and each stream kept at both ends with the omitted byte count. It declares
  `effects: [process, write, network, secret]` and every consequence the profile lists, because it
  confines nothing beyond the environment, time and output.
- `tool_contract::documents` in `b10x-tool-contract`: every `contracts/<toolset>/<version>/contract.yaml`
  embedded at build time (`all`, `get`, `Document::contract`, one constant per document), so an
  adopter reads command names and input schemas without depending on an implementation crate.
  `b10x-tool-fs`, `b10x-tool-code-editor` and `b10x-tool-process` take `CONTRACT_YAML` from it.

### Fixed

- Against harness 0.14.1's runner, `process.run` ends at its bound and kills the process group
  even after the program itself has exited, where a background descendant holding the output kept
  the call open with `timed_out: false`; a descendant that left the group is waited for at most
  500 ms after the kill; a cut moves only for a character it broke.

### Changed

- `b10x-tool-fs`, `b10x-tool-code-editor` and `b10x-tool-toolchain` are Apache-2.0, like every
  other crate in the repository; 0.1.0 released them as `LicenseRef-B10x-Proprietary`.
- `fs.glob` stops walking once `max_results` matching paths are answered (4 entries visited instead
  of 211 on a 1,110-entry tree with `max_results: 1`). `paths` and `truncated` are unchanged; a
  capped glob that stops before reaching a directory past the depth bound now answers
  `depth_bound_reached: false`, where 0.1.0 answered `true`.

## [0.1.0] - 2026-10-20

The first release: ADR 0095 phase T1, seeded from harness 0.13.3 under ADR 0097. harness and
metaharness pin this tag.

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
