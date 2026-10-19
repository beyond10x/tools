---
format: aep.planning-md/3
id: story:toolset-code-editor
kind: story
status: draft
title: The code-editor toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: contracts/code-editor/v1alpha1
- confidence: cited
  path: crates/tool-code-editor
revision: 2
---
## Outcome

The `code-editor` v1alpha1 toolset (`code.write`, `code.edit`, `code.patch`, `code.move`,
`code.delete`) has a contract, semantics and a conformance suite, and `crates/tool-code-editor`
passes it.

## Context

The best forms in the inventory: edit with an `expected_sha256` precondition (workspace), several
edits resolved against the original file with overlaps refused (P1, P2), a unified diff returned
(P1).

## Scope

- `contracts/code-editor/v1alpha1/` (new)
- `crates/tool-code-editor/` (new)

## Acceptance

- The suite passes against `crates/tool-code-editor`.
- An edit whose `expected_sha256` differs from the file is refused with the file unchanged.
- A patch with overlapping edits is refused with the file unchanged.
- Each of these mutants fails at least one scenario: the precondition ignored, edits applied one
  after another instead of against the original, no diff returned.
