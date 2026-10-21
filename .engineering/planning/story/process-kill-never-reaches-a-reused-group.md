---
format: aep.planning-md/3
id: story:process-kill-never-reaches-a-reused-group
kind: story
status: implemented
title: process.run never signals a process group by a pid it has already reaped
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-process
scope:
- confidence: cited
  path: crates/tool-process/src/lib.rs
- confidence: cited
  path: crates/tool-process/src/tests.rs
- confidence: cited
  path: crates/tool-process/tests/adversary_3.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:33:22Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T11:48:05Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Outcome

`process.run` never signals a process group by a pid it has already reaped: the shell stays
unreaped (`waitid` with `WNOWAIT`) until the reply is built, so the group id the deadline kills is
reserved for the run's own group.

## Context

Found by the adversary's second pass on `story:toolset-process` (2026-10-20,
`review-result:adversary-toolset-process-pass-2`, P5, `INFEASIBLE`, note): after the shell has been
reaped, `crates/tool-process/src/lib.rs:332` calls `killpg` with the shell's pid; when no member of
that group is left, the pid is free and could be reused by an unrelated process leading its own
group. Not constructed: with `pid_max` 4194304 it needs about 7,000 forks a second for the 600 s
bound. Harness 0.14.1 only killed a child it had not reaped. The wave shipped without it.

## Scope

- `crates/tool-process/src/lib.rs` (the wait and the kill)
- `crates/tool-process/src/tests.rs` (the unit test)
- `crates/tool-process/tests/adversary_3.rs` (the adversary's cases)

Landed in `e046841`, merged into `wave/w4` as `ae027db`.

## Acceptance

- A unit test shows the shell's pid is not reaped before the deadline kill, on a run whose shell
  exits first and whose descendant holds the output.
- The conformance suite and every mutant lane stay as they are.
