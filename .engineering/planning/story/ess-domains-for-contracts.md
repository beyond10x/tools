---
format: aep.planning-md/3
id: story:ess-domains-for-contracts
kind: story
status: draft
title: Each contract document is lifted into an ESS domain that ess validates
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-fs
- depends_on: story:toolset-code-editor
scope:
- confidence: inferred
  path: crates/tool-contract
revision: 2
---
## Outcome

Each toolset's `contract.yaml` is lifted into an ESS domain (`contracts/<toolset>/<version>/ess/`)
that `ess specify validate` accepts, with one event per operation outcome, and the `tool-contract`
validator checks that the domain and the document agree field for field.

## Context

On 2026-10-20 `ess specify validate` (ess 0.60.0) refused a one-command probe with
`ESS-COMMAND-007 empty_change` (a success outcome that emits no event) and `ESS-COMMAND-004
conflicting_declaration` (outcomes without `when`). A tool operation's outcome is decided by the
filesystem, not by its input. Either ESS gains a way to declare an outcome decided outside the input,
or each outcome gains an event and a `when` over a declared probe field; which one is the question
this story settles with the ESS owner. `docs/design/tool-contracts.md` § ESS validation.

## Scope

- `contracts/*/v1alpha1/ess/` (new)
- `crates/tool-contract/` (the agreement check)

## Acceptance

- `ess specify validate --path contracts/fs/v1alpha1/ess` exits 0.
- `tool-contract validate contracts/` refuses a document whose ESS domain declares a field the
  document does not, naming both.
