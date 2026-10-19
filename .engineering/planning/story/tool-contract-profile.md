---
format: aep.planning-md/3
id: story:tool-contract-profile
kind: story
status: draft
title: The tool-contract/1 profile is written down and validated
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: contracts/profile/tool-contract-1.md
- confidence: inferred
  path: crates/tool-contract
revision: 3
---
## Outcome

The `tool-contract/1` profile is written down (`contracts/profile/tool-contract-1.md`) and checked by
a validator: a contract whose `risk` is lower than its `effects` allow, whose `bounds` lack a
maximum, whose `subjects` name an input field that does not exist, or whose `capability` is not a
Canon capability id is refused, naming the operation and the property.

## Context

`docs/design/tool-contracts.md` § Model lists each property and where it was taken from.

## Scope

- `contracts/profile/tool-contract-1.md` (new)
- `crates/tool-contract/` (new): the profile types and the validator (inferred)

## Acceptance

`tool-contract validate contracts/` exits 0 on a valid contract and exits 1 on each of four seeded
defects (risk below effects, a bound without a maximum, an unknown subject field, an unknown
capability), naming the operation and the property.
