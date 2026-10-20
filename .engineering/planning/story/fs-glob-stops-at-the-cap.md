---
format: aep.planning-md/3
id: story:fs-glob-stops-at-the-cap
kind: story
status: active
title: fs.glob stops walking once max_results matching paths are answered
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-fs
scope:
- confidence: cited
  path: crates/tool-fs/src/lib.rs
- confidence: cited
  path: crates/tool-fs/src/walk.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T07:48:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-20T07:48:36Z", actor: "human:timo", revision: 4}
---
## Outcome

`fs.glob` stops walking the tree once `max_results` matching paths are answered, instead of
visiting every remaining entry with a `canonicalize` each; the answer is unchanged.

## Context

Found by the adversary's second pass on `story:toolset-fs` (2026-10-20,
`review-result:adversary-toolset-fs-pass-2`, finding J3): in `crates/tool-fs/src/walk.rs` the
visitor's `false` returns from the current frame only, so parent frames keep iterating after the
cap. Pre-existing in harness 0.13.3 (`crates/harness-tools/src/local.rs` `walk`, snapshot lines
951-953). The answer is unaffected, so the wave shipped without it.

## Scope

- `crates/tool-fs/src/walk.rs` (`walk` returns whether the visitor asked to stop as well as whether
  the depth bound bit)
- `crates/tool-fs/src/lib.rs` (`glob`'s visitor)

## Acceptance

- A unit test counts visited paths on a tree of 1,000 files with `max_results: 1` and asserts the
  count is under 10.
- The conformance suite stays at every scenario passing.
