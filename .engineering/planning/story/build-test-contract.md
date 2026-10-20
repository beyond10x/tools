---
format: aep.planning-md/3
id: story:build-test-contract
kind: story
status: implemented
title: The build-test toolset has a static contract and a conformance suite over the toolchain crate
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-toolchain
- depends_on: story:toolset-process
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/build-test/v1alpha1
- confidence: cited
  path: crates/tool-toolchain
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:18:48Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-20T09:18:49Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-20T12:41:27Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":9,"verification":1}}}
---
## Outcome

`contracts/build-test/v1alpha1/` is a static contract document and conformance suite for the
operations `crates/tool-toolchain` derives from its five built-in providers, and the crate passes
the suite against fixture workspaces (a Cargo workspace, a Go module, a Taskfile, an npm and a yarn
project).

## Context

`story:toolset-toolchain` moves the registry and derives profile-conforming operations per resolved
workspace; it does not ship a document a harness can read without resolving a workspace. This story
freezes the built-in providers' operations into a document, and the suite exercises the executor
that `harness-tools` holds today in `catalogue.rs` (`invoke_toolchain`) and `toolchain.rs`.

## Scope

- `contracts/build-test/v1alpha1/` (new)
- `crates/tool-toolchain/` (the executor and the conformance test)

## Acceptance

- `tool-contract validate contracts/build-test/v1alpha1` exits 0.
- The suite passes against `crates/tool-toolchain`; a tool whose `empty_stdout` check fails answers
  the error `build-test.CheckFailed` naming the formatting check (harness answered a refusal with
  the message `formatting check failed: …`; amended 2026-10-20 after
  `review-result:adversary-build-test-pass-1`, F7, to the error the executor declares).
