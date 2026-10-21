---
format: aep.planning-md/3
id: story:declared-timeouts-are-enforced
kind: story
status: implemented
title: Every declared timeout_ms is enforced or no longer declared
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-search
scope:
- confidence: cited
  path: contracts/fs/v1alpha2
- confidence: cited
  path: contracts/search/v1alpha2
- confidence: cited
  path: crates/tool-fs/src/deadline.rs
- confidence: cited
  path: crates/tool-fs/src/error.rs
- confidence: cited
  path: crates/tool-fs/src/lib.rs
- confidence: cited
  path: crates/tool-fs/src/line.rs
- confidence: cited
  path: crates/tool-fs/src/tests.rs
- confidence: cited
  path: crates/tool-fs/src/toolset.rs
- confidence: cited
  path: crates/tool-fs/src/types.rs
- confidence: cited
  path: crates/tool-fs/src/walk.rs
- confidence: cited
  path: crates/tool-fs/tests
- confidence: cited
  path: crates/tool-search/src/error.rs
- confidence: cited
  path: crates/tool-search/src/lib.rs
- confidence: cited
  path: crates/tool-search/src/tests.rs
- confidence: cited
  path: crates/tool-search/src/toolset.rs
- confidence: cited
  path: crates/tool-search/src/types.rs
- confidence: cited
  path: crates/tool-search/tests
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T11:48:05Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
---
## Outcome

Every `timeout_ms` bound a released contract declares is enforced by the default implementation, or
the bound is removed from a new contract version: `fs.read`, `fs.list`, `fs.glob`, `fs.stat` and
`search.grep` stop at their deadline and say so in the answer, with a scenario that fails when the
deadline is ignored.

## Context

- The validator requires a `timeout_ms` bound on every operation, so `contracts/fs/v1alpha1` and
  `contracts/search/v1alpha1` declare `{default: 10000, max: 60000}`, while neither crate reads it:
  `grep -n timeout_ms crates/tool-fs/src/*.rs` finds nothing (2026-10-20), and the search
  implementor reported the same for `crates/tool-search`.
- Wave t1's D6 set the rule this breaks: a bound no implementation enforces is not declared.
- A released `v1alpha1` is never edited in place (`AGENTS.md` § Rules); enforcing a declared bound
  narrows nothing a caller relies on, but adding an answer field that says the deadline hit is a
  change a conforming implementation could fail, so it may need a new version.

## Scope

- `contracts/fs/v1alpha2/`, `contracts/search/v1alpha2/` (new versions; v1alpha1 unchanged)
- `crates/tool-fs/src/`: `deadline.rs` (new), `error.rs`, `lib.rs`, `line.rs`, `tests.rs`, `toolset.rs`,
  `types.rs`, `walk.rs`; `crates/tool-fs/tests/`
- `crates/tool-search/src/`: `error.rs`, `lib.rs`, `tests.rs`, `toolset.rs`, `types.rs`;
  `crates/tool-search/tests/`

Landed in `58d227e`, merged into `wave/w4` as `1393b0e`.

## Acceptance

- A mutant that ignores the deadline fails at least one scenario of each toolset.
- Or, where enforcement is not taken: the profile no longer requires `timeout_ms` for an operation
  that starts no process, and the new contract version does not declare it.
