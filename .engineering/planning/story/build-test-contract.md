---
format: aep.planning-md/3
id: story:build-test-contract
kind: story
status: draft
title: The build-test toolset has a static contract and a conformance suite over the toolchain crate
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-toolchain
scope:
- confidence: cited
  path: contracts/build-test/v1alpha1
- confidence: cited
  path: crates/tool-toolchain
revision: 2
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
  a refusal naming the formatting check, as harness's executor does.
