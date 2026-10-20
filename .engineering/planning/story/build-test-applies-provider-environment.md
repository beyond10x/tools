---
format: aep.planning-md/3
id: story:build-test-applies-provider-environment
kind: story
status: draft
title: The build-test executor starts each program with its provider's declared environment
relations:
- decomposes: epic:tool-contracts
- depends_on: story:build-test-contract
scope:
- confidence: cited
  path: contracts/build-test
- confidence: cited
  path: contracts/process
- confidence: cited
  path: crates/tool-process/src/lib.rs
- confidence: cited
  path: crates/tool-toolchain/src/executor.rs
revision: 2
---
## Outcome

The build-test executor starts each program with the environment its provider declares (for the go
provider `GOSUMDB`, `GOENV`, `GOTOOLCHAIN` and the rest of its `sandbox` block), so the builtin
summaries that say network and checksum lookup are disabled are true of a run through
`crates/tool-toolchain`; `b10x-tool-process` gains a way to set a variable to a value, not only to
pass one through by name.

## Context

Found by the adversary's second pass on `story:build-test-contract` (2026-10-20,
`review-result:adversary-build-test-pass-2`, P2, note, `pre-existing`): `contract.yaml:35` and the
other go summaries say "with network and checksum lookup disabled", while the executor applies none
of the provider's environment, because `Process` passes variables by name only (implementor's
report, wave w3). Harness 0.14.1 published the same text over the same unconfined run
(`toolchain.rs:22`, `local.rs:556-563`). That no lookup is disabled is inferred from the environment
not being applied; it was not run against the network.

## Scope

- `crates/tool-process/src/lib.rs` (a value-setting environment entry, still cleared to an allow-list otherwise)
- `crates/tool-toolchain/src/executor.rs` (apply the provider's environment)
- `contracts/process/` (if the input gains a field, a new version), `contracts/build-test/`

## Acceptance

- A go scenario shows `GOSUMDB` (or the provider's declared variable) set in the child to the
  provider's value.
- The go summaries are true of the run, or reworded where a value cannot be applied.
