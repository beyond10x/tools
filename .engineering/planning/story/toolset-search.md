---
format: aep.planning-md/3
id: story:toolset-search
kind: story
status: draft
title: The search toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-fs
scope:
- confidence: cited
  path: contracts/search/v1alpha1
- confidence: cited
  path: crates/tool-search
revision: 2
---
## Outcome

The `search` v1alpha1 toolset (`search.grep`) has a contract, semantics and a conformance suite, and
`crates/tool-search` passes it, built from harness's `grep` (`crates/harness-tools/src/local.rs` at
harness 0.13.3: `grep`, `search_expression`, `around`, `cut_to`, and the `walk` the fs crate holds).

## Context

Drafted during T1 (2026-10-20) so that the part of harness's `local.rs` that T1 did not move has a
named home; atlas ADR 0097 moves all of harness's tools here. Phase T3 in
`docs/design/tool-contracts.md`.

## Scope

- `contracts/search/v1alpha1/` (new)
- `crates/tool-search/` (new)

## Acceptance

- The suite passes against `crates/tool-search`.
- A literal pattern, a regular expression, a glob filter and context lines answer as harness's
  `search` does, including `withheld` and `depth_bound_reached`.
