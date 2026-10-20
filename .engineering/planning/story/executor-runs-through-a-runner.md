---
format: aep.planning-md/3
id: story:executor-runs-through-a-runner
kind: story
status: implemented
title: The build-test executor runs every argv through a runner the caller supplies
relations:
- decomposes: epic:tool-contracts
- depends_on: story:build-test-contract
scope:
- confidence: cited
  path: crates/tool-toolchain/src/executor.rs
- confidence: cited
  path: crates/tool-toolchain/src/executor/tests.rs
- confidence: cited
  path: crates/tool-toolchain/src/lib.rs
- confidence: cited
  path: crates/tool-toolchain/src/toolset.rs
- confidence: cited
  path: crates/tool-toolchain/tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T14:23:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-20T14:23:56Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-20T15:37:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Outcome

`tool_toolchain::executor::Executor` runs every planned argv through a runner the caller supplies,
so one executor serves a local unconfined run and a confined one: a `Runner` trait (one argv, the
provider's declared sandbox, the time left → the run's output), `b10x-tool-process`'s `Process`
as the default runner, and every check the executor makes today (declared programs, the dash rule,
landing, the write scope, the deadline, `empty_stdout`) made before or around the runner, whatever
the runner is.

## Context

- Asked 2026-10-20 by the approving session (`b10x-chef-orchestrator`, under the owner's standing
  grant) for harness, which runs toolchain plans through its `Operations::run_within(argv,
  remaining)` so local and confined runs share one path (harness 0.15.0
  `crates/harness-catalogue/src/catalogue.rs:519-558`, `operations.rs:271-290`). tools 0.3.0's
  executor is tied to `tool_process::Process` (`crates/tool-toolchain/src/executor.rs:337-340`).
- The providers' sandbox environment (`builtins/*.yaml` `sandbox.env`) describes a confined run:
  `{mount:goroot}`, `{mount:rustup}`, `HOME` and `PATH` moved into the workspace. The runner
  receives it; a confined runner applies it; the local `Process` runner does not, because those
  values relocate the toolchains a local run uses. This is the part of
  `story:build-test-applies-provider-environment` that fits this seam.
- The type change breaks callers that name the executor's fields or constructor shape: release
  0.4.0.

## Scope

- `crates/tool-toolchain/src/executor.rs`, `crates/tool-toolchain/src/executor/tests.rs`
- `crates/tool-toolchain/src/toolset.rs`, `crates/tool-toolchain/tests/`
- `crates/tool-toolchain/src/lib.rs` (re-exports)

## Acceptance

- A test runner supplied by the caller receives every argv of a plan, with the provider's name and
  sandbox spec and the time left, and its output is what the call answers; a refusal (declared
  programs, dash rule, landing, write scope, deadline) happens before the test runner is called.
- `Executor::new` with `Process` passes the `build-test` v1alpha1 suite and every mutant lane as
  before.
