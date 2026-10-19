---
format: aep.planning-md/3
id: story:toolset-fs
kind: story
status: draft
title: The fs toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: contracts/fs/v1alpha1
- confidence: cited
  path: crates/tool-fs
revision: 2
---
## Outcome

The `fs` v1alpha1 toolset (`fs.read`, `fs.list`, `fs.glob`, `fs.stat`) has a contract, semantics and a
conformance suite, and `crates/tool-fs` passes the suite.

## Context

The inventory's 14 file readers; the best forms: paged reads with a whole-file digest (substrate), a
truncation contract (epistemic-knowledge-runtime), read caps (P1).

## Scope

- `contracts/fs/v1alpha1/` (new)
- `crates/tool-fs/` (new)

## Acceptance

- The suite passes against `crates/tool-fs`.
- Each of these mutants of `crates/tool-fs` fails at least one scenario: no read cap, no `truncated`
  flag, a digest of the page instead of the whole file, a path outside the subject root accepted.
- `crates/tool-fs` derives its schemas from its types, and the suite checks they equal the contract's.
