---
format: aep.planning-md/3
id: story:toolset-search
kind: story
status: implemented
title: The search toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-fs
- depends_on: story:fs-glob-stops-at-the-cap
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/search/v1alpha1
- confidence: inferred
  path: crates/tool-fs/src/lib.rs
- confidence: inferred
  path: crates/tool-fs/src/walk.rs
- confidence: cited
  path: crates/tool-search
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:18:49Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-20T09:18:50Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-20T12:41:29Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":9,"verification":1}}}
---
## Outcome

The `search` v1alpha1 toolset (`search.grep`) has a contract, semantics and a conformance suite, and
`crates/tool-search` passes it, built from harness's `grep` (`crates/harness-tools/src/local.rs` at
harness 0.13.3: `grep`, `search_expression`, `around`, `cut_to`, and the `walk` the fs crate holds).

## Context

Drafted during T1 (2026-10-20) so that the part of harness's `local.rs` that T1 did not move has a
named home; ADR 0097 moves all of harness's tools here. Phase T3 in
`docs/design/tool-contracts.md`.

## Scope

- `contracts/search/v1alpha1/` (new)
- `crates/tool-search/` (new)
- `crates/tool-fs/src/walk.rs` and `crates/tool-fs/src/lib.rs` (inferred, 2026-10-20): the `walk`
  this story reuses is `pub(crate)` in tool-fs (`crates/tool-fs/src/walk.rs:43`, `mod walk;` at
  `crates/tool-fs/src/lib.rs:42`), so tool-search reaches it only once tool-fs exports it.
  `story:fs-glob-stops-at-the-cap` changes what `walk` returns in the same file, so this story lands
  after that one rather than beside it.

## Acceptance

- The suite passes against `crates/tool-search`.
- A literal pattern, a regular expression, a glob filter and context lines answer as harness's
  `search` does, including the large files it did not inspect (`skipped_large_files`,
  `skipped_large_paths`, the names harness 0.13.3 and 0.14.1 answer; this line said `withheld`
  until 2026-10-20, corrected after the implementor checked both tags) and `depth_bound_reached`.
