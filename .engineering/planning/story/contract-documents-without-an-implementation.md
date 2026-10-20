---
format: aep.planning-md/3
id: story:contract-documents-without-an-implementation
kind: story
status: active
title: The released contract documents are reachable through b10x-tool-contract alone
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: crates/tool-code-editor/src/lib.rs
- confidence: cited
  path: crates/tool-contract
- confidence: cited
  path: crates/tool-fs/src/toolset.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T07:48:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-20T07:48:36Z", actor: "human:timo", revision: 4}
---
## Outcome

An adopter reads the released `fs` and `code-editor` v1alpha1 contract documents through the
Apache-2.0 crate `b10x-tool-contract` alone, without depending on an implementation crate.

## Context

- In tools 0.1.0 the contract documents are embedded only by the implementation crates:
  `crates/tool-fs/src/toolset.rs:20` and `crates/tool-code-editor/src/lib.rs:39-40`
  (`include_str!` of `contracts/<toolset>/v1alpha1/contract.yaml`). 0.1.0 released both crates as
  `LicenseRef-B10x-Proprietary`; `crates/tool-contract/Cargo.toml:4` is `Apache-2.0`.
- The first adopter that linked only the Apache-2.0 crate needs the contract's command names and
  input schemas for its published catalogue, and reaches neither through `b10x-tool-contract` today.
- 2026-10-20, later the same day: the owner relicensed the crates moved from harness Apache-2.0
  for 0.2.0 (`CHANGELOG.md` § Unreleased). The story stands on the design alone:
  `docs/design/tool-contracts.md` says a contract names no implementation, and reaching it only
  through one inverts that.

## Scope

- `crates/tool-contract/` (the embedded documents and their accessor)
- `crates/tool-fs/src/toolset.rs` and `crates/tool-code-editor/src/lib.rs` (take `CONTRACT_YAML` from
  `b10x-tool-contract` instead of embedding their own copy)

## Acceptance

- A test in `crates/tool-contract` parses each embedded document and finds the commands of
  `contracts/fs/v1alpha1/contract.yaml` and `contracts/code-editor/v1alpha1/contract.yaml` by name.
- `cargo tree -p b10x-tool-contract` lists no `b10x-tool-fs`, `b10x-tool-code-editor` or
  `b10x-tool-toolchain`.
- tool-fs and tool-code-editor keep every conformance scenario passing.
