---
format: aep.planning-md/3
id: story:toolset-code-editor
kind: story
status: implemented
title: The code-editor toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/code-editor/v1alpha1
- confidence: cited
  path: crates/tool-code-editor
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T01:52:29Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-20T01:54:15Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-20T05:03:56Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Outcome

The `code-editor` v1alpha1 toolset (`code.write`, `code.edit`, `code.patch`, `code.move`,
`code.delete`) has a contract, semantics and a conformance suite, and `crates/tool-code-editor`
passes it. The crate is harness's local writer moved here: `crates/harness-tools/src/local.rs` at
harness 0.13.3 (`798325f`), its `write`, `edit` and `resolve_new` functions and their tests, with the
same refusals (a symlink target, a missing parent, a path that climbs out, an edit that matches zero
or several times, an empty `old`), plus the `expected_sha256` precondition, a unified `diff` in the
reply, and the new `code.patch`, `code.move` and `code.delete`.

## Context

The best forms in the inventory: edit with an `expected_sha256` precondition (workspace), several
edits resolved against the original file with overlaps refused (P1, P2), a unified diff returned
(P1). `docs/design/tool-contracts.md` § Operation sets fixes the field names. Owner decision
2026-10-20 (ADR 0097): harness's tools move here and harness consumes them.

## Scope

- `contracts/code-editor/v1alpha1/` (new, cited): `contract.yaml` (family `code`), `semantics.md`,
  `scenarios/` (8 files, 68 scenarios, 3 of them `adversary-*`), `fixtures/` (`link-to-parent/`,
  `dangling-link/`, `link-inside/`, with committed symlinks)
- `crates/tool-code-editor/` (new, cited): `src/{lib,tests}.rs`,
  `tests/{conformance,mutants,adversary_trailing_slash,adversary_pass2}.rs`
- `Cargo.lock` (cited; shared with the other crates of the wave, regenerated at merge)

Confirmed by the implementor on 2026-10-20: every cited path exists as named; no inferred line.

## Acceptance

- The suite passes against `crates/tool-code-editor` (`cargo test -p b10x-tool-code-editor`).
- An edit whose `expected_sha256` differs from the file is refused with the file unchanged.
- A patch with overlapping edits is refused with the file unchanged.
- Each of these mutants fails at least one scenario: the precondition ignored, edits applied one
  after another instead of against the original, no diff returned.
- harness's `local.rs` tests for write, edit, links and new paths run here unchanged in substance
  (renamed where the API moved) and pass.
