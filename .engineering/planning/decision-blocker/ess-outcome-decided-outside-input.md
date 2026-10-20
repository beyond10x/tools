---
format: aep.planning-md/3
id: decision-blocker:ess-outcome-decided-outside-input
kind: decision-blocker
status: cleared
title: Nobody has decided how an ESS command declares an outcome the environment decides
relations:
- blocks: story:ess-domains-for-contracts
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T08:54:26Z", actor: "human:timo", revision: 3}
---
## Question

How does an ESS command declare an outcome that the environment decides rather than the command's
input? A tool operation such as `fs.read` succeeds or fails on the state of the filesystem, not on a
field of its input.

## Why it blocks

`story:ess-domains-for-contracts` lifts each `contract.yaml` into an ESS domain that
`ess specify validate` accepts. On 2026-10-20 ess 0.60.0 refused a one-command probe with
`ESS-COMMAND-007 empty_change` (a success outcome that emits no event) and `ESS-COMMAND-004
conflicting_declaration` (outcomes without `when`) (`story:ess-domains-for-contracts` § Context;
`docs/design/tool-contracts.md` § ESS validation).

## The two answers on the table

| option | what it means for this repository |
|---|---|
| A | ESS gains a way to declare an outcome decided outside the input; the lift waits for that ESS release |
| B | each outcome gains an event and a `when` over a declared probe field; the lift can start on ess 0.60.0 |

## Who answers

The ESS repository's owner. Cleared by their answer, recorded here, not by a wave in this repository.

## Answer

Answered by the ESS repository's session, relayed by `b10x-chef-orchestrator` on 2026-10-20, and
read by the coordinator in `beyond10x/ess` at `origin/main` `d11b350371`:

- `ESS-COMMAND-007 empty_change` stays: an accepted outcome must emit an event or name an error
  (`crates/specify/ess-domain/src/command.rs`, the rule table near line 170: "an outcome neither
  emits nor errors → `EmptyChange`"). Reads are views (`QueryView`): `fs.read`, `fs.list`,
  `fs.glob` and `fs.stat` are modelled as views, and the writing operations emit events.
- `ESS-COMMAND-004`: an outcome the environment decides is declared with `external:`; a refusing
  external branch names an `error:`, and an accepting external branch is allowed. No probe field
  is needed (`website/docs/guides/specify/bindings-and-components.md:62-64`,
  `website/docs/guides/specify/commands-and-outcomes.md`; an external outcome must state its cause,
  `UnexplainedDecision`). 004 now fires only when the order of two branches is unstated (ess#554).

Option B of the table above is not needed and option A is what ESS already has. Nothing in this
blocker's question remains open. Still undecided in ESS, and not part of this question: how an
outcome that only stores is observed through a view (the ESS store's
`story:an-outcome-that-only-stores-is-observed-through-a-view`, owned by the owner); every tool
operation that writes emits an event, so it does not stop `story:ess-domains-for-contracts`.
