---
format: aep.planning-md/3
id: story:declared-timeouts-are-enforced
kind: story
status: active
title: Every declared timeout_ms is enforced or no longer declared
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-search
scope:
- confidence: cited
  path: contracts/fs
- confidence: cited
  path: contracts/search
- confidence: cited
  path: crates/tool-fs/src/lib.rs
- confidence: cited
  path: crates/tool-fs/src/walk.rs
- confidence: cited
  path: crates/tool-search/src/lib.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 4}
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

- `crates/tool-fs/src/lib.rs`, `crates/tool-fs/src/walk.rs`
- `crates/tool-search/src/lib.rs`
- `contracts/fs/`, `contracts/search/` (scenarios, or a new version)

## Acceptance

- A mutant that ignores the deadline fails at least one scenario of each toolset.
- Or, where enforcement is not taken: the profile no longer requires `timeout_ms` for an operation
  that starts no process, and the new contract version does not declare it.
