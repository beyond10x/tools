---
format: aep.planning-md/3
id: story:toolset-process
kind: story
status: draft
title: The process toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: contracts/process/v1alpha1
- confidence: cited
  path: crates/tool-process
revision: 2
---
## Outcome

The `process` v1alpha1 toolset (`process.run`) has a contract, semantics and a conformance suite, and
`crates/tool-process` passes it, built from harness's `exec` (`crates/harness-tools/src/local.rs` at
harness 0.13.3: `exec`, `drain`, `read_ends`, `keep_both_ends`, `terminate`, `INHERITED_ENV`): an
argv, never a command line; a declared program set; an allow-listed environment; a wall-clock bound
that kills the process group; output kept at both ends with the omitted byte count.

## Context

Drafted during T1 (2026-10-20) so that the part of harness's `local.rs` that T1 did not move has a
named home; atlas ADR 0097 moves all of harness's tools here. Phase T3 in
`docs/design/tool-contracts.md`.

## Scope

- `contracts/process/v1alpha1/` (new)
- `crates/tool-process/` (new)

## Acceptance

- The suite passes against `crates/tool-process`.
- A program outside the declared set is refused by name with nothing started; a run over its bound
  is killed with its descendants and the reply says so.
