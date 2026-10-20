---
format: aep.planning-md/3
id: story:contract-directory-names-validated
kind: story
status: draft
title: tool-contract validate refuses a contract directory name the build script cannot embed
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: contracts/profile/tool-contract-1.md
- confidence: cited
  path: crates/tool-contract/src/documents.rs
- confidence: cited
  path: crates/tool-contract/src/validate.rs
revision: 2
---
## Outcome

`tool-contract validate contracts/` refuses a toolset or version directory name that
`crates/tool-contract/build.rs` cannot embed, naming the path and the grammar, so a document the
validator passes never stops the workspace from building; and `tool_contract::documents::all`'s
rustdoc says its order is the byte order of the directory names, not version precedence.

## Context

Found by the adversary's pass on `story:contract-documents-without-an-implementation` (2026-10-20,
`review-result:adversary-contract-documents-pass-1`, findings F1 and F2, both `INFEASIBLE`, severity
note): `validate` accepts `build_test/v1alpha1`, `web/v1.2` and `web/V1`, while `build.rs:103`
panics on them (`"build_test" is not [a-z][a-z0-9-]*`) and so stops every crate building, the
validator included; `all()` orders `v1 < v10 < v1alpha1 < v2` while `src/documents.rs:34` reads as
version order. Nothing reaches either today: every planned toolset name and `v1alpha1` fit the
grammar, and no caller treats position as precedence. The wave shipped without them.

## Scope

- `crates/tool-contract/src/validate.rs` (the directory-name rule)
- `crates/tool-contract/src/documents.rs` (the rustdoc line)
- `contracts/profile/tool-contract-1.md` (the grammar stated)

## Acceptance

- `tool-contract validate` on a tree holding `build_test/v1alpha1/contract.yaml` exits 1 naming the
  directory and `[a-z][a-z0-9-]*`.
- The rustdoc of `documents::all` names byte order.
