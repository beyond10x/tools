---
format: aep.planning-md/3
id: approval-record:takt-delivers-contract-directory-names
kind: approval-record
status: approved
title: takt 0.17.0 delivers story:contract-directory-names-validated through the bot route
summary: 'Decided by the orchestrator on 2026-10-28: accept the story and let takt deliver it, merge grant only'
relations:
- decides: story:contract-directory-names-validated
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-28T10:52:52Z", actor: "human:timo", revision: 3, executor: "agent:takt-proof"}
---
## Decision

`story:contract-directory-names-validated` is accepted (`draft` to `proposed`) and is delivered by
takt 0.17.0 (https://github.com/beyond10x/takt/releases/tag/0.17.0) on `story.delivery/2`, with
Claude Code as the harness and the bot route as the forge: one run to the merge approval, one
grant of `repository.merge` at the exact revision, one run to the merge. No hand-written code: the
story's change is takt's and its harness's.

It is the first takt delivery of a story from a beyond10x repository's own planning store; earlier
deliveries ran on the scratch repository todo-app-example only.

## Instruction

Decided by `b10x-chef-orchestrator` on 2026-10-28 under the owner's standing grant, with the
owner's words quoted verbatim:

> what we're needing soon now actually is really proof that we can take this thing, point it to a
> story within the repo and see it solving that story.

The orchestrator's item: prove takt 0.17.0 delivers a real story from a real beyond10x
repository's AEP planning store, unattended apart from the merge grant, ending with a merged bot
pull request on that repository's `main` and the story implemented in the store.

## Why this story

Shortlist of three, read 2026-10-28:

- `story:contract-directory-names-validated` (tools): one crate (`crates/tool-contract`), an
  acceptance a test shows (`validate` exits 1 on `build_test/v1alpha1`, naming the grammar), and a
  CI run of about 2.5 minutes on `main`. Chosen.
- `story:fs-climb-through-missing-refused` (tools): one crate (`crates/tool-fs`) plus a new
  conformance scenario, but no acceptance section and a claim not run yet.
- `story:routes-refusal-names-a-later-route-field-line` (llm): one crate with an exact
  acceptance, but a 21-minute gate.

## What it changes

- `story:contract-directory-names-validated`: `draft` to `proposed`.
- `.engineering/gates.yaml` and `.engineering/assertions-project.yaml`: the gates takt's gate step
  runs, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`
  and `tool-contract validate contracts/`, three of `task check`'s steps; takt runs the tests
  itself.
- `.engineering/commit-identity.conf`: takt commits as `b10x-bot[bot]`, the identity this
  repository's rulesets admit on `main`.
- `.engineering/delegations.yaml`: the owner delegates `repository.merge.judge_surface` to
  `agent:takt-delivery`, so the grant `takt approve --by agent:takt-delivery` merges a change to a
  judge surface. Revocable by a later record that removes it.
