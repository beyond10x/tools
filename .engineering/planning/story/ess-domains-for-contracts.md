---
format: aep.planning-md/3
id: story:ess-domains-for-contracts
kind: story
status: draft
title: The profile, every toolset's operations and their suites are modelled and checked in ESS
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-fs
- depends_on: story:toolset-code-editor
- depends_on: story:toolset-process
- depends_on: story:build-test-contract
scope:
- confidence: inferred
  path: contracts/code-editor/v1alpha1
- confidence: inferred
  path: contracts/fs/v1alpha1
- confidence: inferred
  path: contracts/process/v1alpha1
- confidence: inferred
  path: contracts/profile
- confidence: inferred
  path: crates/tool-contract
revision: 6
---
## Outcome

The repository's inner workings are modelled in ESS, written and checked through the
`ess:specifying` skill:

- the `tool-contract/1` profile itself is an ESS domain (`contracts/profile/`): its properties
  (effects, consequences, risk and its floors, idempotency, determinism, bounds, paging, requires,
  subjects, placement, capability, output and presentation) are typed, and the rules the validator in
  `crates/tool-contract` enforces today are ESS invariants, guards and relations that
  `ess specify validate` checks;
- each toolset's operations are ESS commands and views in one domain per toolset and version:
  `contracts/fs/v1alpha1/` (`fs.read`, `fs.list`, `fs.glob`, `fs.stat` as views),
  `contracts/code-editor/v1alpha1/` (`code.write`, `code.edit`, `code.patch`, `code.move`,
  `code.delete` as commands that emit an event on every accepted outcome),
  `contracts/process/v1alpha1/` (`process.run`) and the `build-test` toolset once
  `story:build-test-contract` has written it; an outcome the filesystem or the program decides is an
  `external:` branch with its cause stated;
- each toolset's conformance suite is an ESS conformance suite (`ess verify conform synthesize`),
  run against the crate (`ess verify conform run`), and the report is recorded in the AEP store with
  `aep plan artifact evidence --from <report>`; the repository's own `tool-conformance/1` runner and
  the validator in `crates/tool-contract` are retired where ESS does the same check, and the
  `tool-contract validate` binary keeps only what ESS cannot express, each such rule named.

## The two decisions

`decision-blocker:ess-outcome-decided-outside-input` was cleared on 2026-10-20 with the ESS
repository's answer (read in `beyond10x/ess` at `d11b350371`; the blocker's `## Answer` holds the
citations):

| code | what ess refused | what was decided |
|---|---|---|
| `ESS-COMMAND-007 empty_change` | a success outcome that emits no event | an accepted outcome emits an event or names an error. `fs.read`, `fs.list`, `fs.glob` and `fs.stat` are views (`QueryView`), not commands; `code.*` and `process.run` emit an event on every accepted outcome |
| `ESS-COMMAND-004 conflicting_declaration` | several outcomes on one command with no `when` | an outcome the environment decides is an `external:` branch, with an `error:` when it refuses and a stated cause; no probe field. 004 now fires only when two branches' order is unstated (ess#554) |

Open in ESS and not blocking here: how an outcome that only stores is observed through a view (the
ESS store's `story:an-outcome-that-only-stores-is-observed-through-a-view`).

## Context

- Owner's direction of 2026-10-20, relayed by `b10x-chef-orchestrator`: tools models its inner
  workings in ESS with the `ess:specifying` skill; this story was widened from "lift each contract
  document into an ESS domain" to the profile, every toolset and the suites.
- Today `contracts/<toolset>/v1alpha1/contract.yaml` is `tool-contract/1` YAML with ESS-shaped
  commands, checked by `crates/tool-contract` (`validate.rs`, `conformance.rs`); ADR 0095
  point 2 says a contract is an ESS command plus the profile.
- A released `v1alpha1` is never edited in place (`AGENTS.md` § Rules): modelling `fs.read` as a
  view changes what an adopter calls, so the ESS form of a released toolset is a new version beside
  `v1alpha1`, and `v1alpha1` stays as released.

## Scope

- `contracts/profile/` (the profile as an ESS domain)
- `contracts/*/` (one ESS domain per toolset, in a version beside `v1alpha1`)
- `crates/tool-contract/` (the validator and the runner retired where ESS covers them; the
  conformance target that answers ESS's `ExecuteCommand` and `QueryView` per toolset)

## Acceptance

- `ess specify validate` exits 0 on the profile domain and on each toolset domain.
- `ess verify conform run` against each toolset crate exits 0, and its report, recorded with
  `aep plan artifact evidence --from`, moves each toolset's `executable-system-specification`
  artifact to `conforming`.
- Every rule `crates/tool-contract`'s validator enforces today is either checked by ESS (named) or
  listed in the profile domain as a rule ESS cannot express, with the reason.
