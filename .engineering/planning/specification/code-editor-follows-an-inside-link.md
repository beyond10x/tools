---
format: aep.planning-md/3
id: specification:code-editor-follows-an-inside-link
kind: specification
status: draft
title: The code-editor toolset follows a link that lands inside the workspace, as v1alpha1 says
relations:
- informed_by: story:code-editor-trailing-slash-refused
revision: 1
---
## Finding

`b10x-tool-code-editor` and `contracts/code-editor/v1alpha1/semantics.md:14-17` agree: a write or an
edit to a link that lands inside the workspace is followed to its target. Nothing was built.

## Asked

2026-10-20, from the approving session, on metaharness's reading of `tool-code-editor` 0.2.0:
`refuse_link` (`crates/tool-code-editor/src/lib.rs:847`, called by `code.write` at `:401`) refuses a
write whose target is itself a link, which would contradict the contract; file a story to make code
and contract agree.

## What the code does

- `code.write` resolves the path first (`lib.rs:400`, `resolve_new`), which canonicalises the deepest
  existing entry, the link included, so `target` is where the link lands, never the link.
- `refuse_link(&target, …)` at `:401` therefore sees a link only when one was put at the resolved
  place between the resolve and the write; its rustdoc says so (`lib.rs:844-846`: "a link put at
  `target` between that check and the call that follows would still be followed").
- `code.edit` and `code.patch` do the same through `read_text` (`lib.rs:685-686`).

## What shows it

- `crates/tool-code-editor/src/tests.rs:78`,
  `a_link_that_stays_inside_the_workspace_is_still_a_way_to_reach_its_target`: a write to `link`
  pointing at `real.txt` inside succeeds and answers `path: real.txt`. It passed on the 0.3.0
  release gate (`cargo test --workspace --locked`, `test tests::a_link_that_stays_inside_the_workspace_is_still_a_way_to_reach_its_target ... ok`).
- `contracts/code-editor/v1alpha1/scenarios/adversary-trailing-slash.yaml` holds the same for
  `link/` through edit, patch and move.

## Not established

Where the refusal of `link-in` without a trailing `/` that the probe through harness 0.15.0 saw
comes from: not from `tool-code-editor`, by the test above; the layer above it was not examined.
