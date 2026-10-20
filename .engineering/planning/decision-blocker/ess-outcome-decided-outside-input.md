---
format: aep.planning-md/3
id: decision-blocker:ess-outcome-decided-outside-input
kind: decision-blocker
status: open
title: Nobody has decided how an ESS command declares an outcome the environment decides
relations:
- blocks: story:ess-domains-for-contracts
revision: 1
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
