---
format: aep.planning-md/3
id: story:tool-contract-profile
kind: story
status: implemented
title: The tool-contract/1 profile is written down and validated
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: contracts/profile/tool-contract-1.md
- confidence: cited
  path: crates/tool-contract
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T01:52:29Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-20T01:54:15Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-20T05:03:56Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Outcome

The `tool-contract/1` profile is written down (`contracts/profile/tool-contract-1.md`) and checked by
a validator: a contract whose `risk` is lower than its `effects` allow, whose `bounds` lack a
maximum, whose `subjects` name an input field that does not exist, or whose `capability` is not a
Canon capability id is refused, naming the operation and the property. The same crate carries the
`tool-conformance/1` runner and the schema shape check every toolset crate uses, so the fs,
code-editor and toolchain crates can be written against one API.

## Context

`docs/design/tool-contracts.md` § Model lists each property and where it was taken from; § T1 in
detail gives the document grammar, the validator's rules, the suite format and the runner API this
story implements. ESS cannot validate the documents yet (§ ESS validation), so the validator reads
the ESS field and type grammar itself. The repository had no Rust workspace before this story.

## Scope

- `Cargo.toml` (new, cited): the workspace, `members = ["crates/*"]`, rust-version 1.98, edition
  2024, no workspace licence (per crate), `unsafe_code = "forbid"`, clippy `all` and `pedantic`
  denied
- `Cargo.lock` (new, cited; regenerated at the last unit merge)
- `contracts/profile/tool-contract-1.md` (new, cited)
- `crates/tool-contract/` (new, cited): `src/{lib,document,validate,types,conformance,mutants,digest}.rs`,
  `src/bin/tool-contract.rs`, `tests/{validate,shapes,conformance,cli,error_fields,adversary_*,adversary2_*}.rs`,
  `tests/defects/*.yaml`, `tests/fixtures/notes/`
- `Taskfile.yml` (cited): `check` gained the cargo steps, the msrv check and `tool-contract validate`
- `.github/workflows/check.yml` (cited): pinned Rust toolchain and cache steps

Confirmed by the implementor on 2026-10-20: every cited path exists as named; no inferred line.

## Acceptance

- `cargo run -p b10x-tool-contract -- validate contracts/` exits 0 on a valid contract document and
  exits 1 on each of four seeded defects (risk below effects, a bound without a maximum, an unknown
  subject field, an unknown capability), naming the operation and the property; the four seeded
  documents live under `crates/tool-contract/tests/defects/`.
- The runner refuses a `tool-conformance/1` suite in which a declared profile property has no
  scenario whose `holds` names it, naming the property.
- The shape check refuses a derived schema that differs from the contract's field list, naming the
  operation, the side and the field, and accepts one that matches.
- `task check` runs the store validation, `cargo test --workspace --locked`, `cargo fmt --check`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings` and `tool-contract validate`.
