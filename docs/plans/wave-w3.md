# Wave w3: the build-test contract on the process runner, the search toolset

Run 2026-10-20 by the `tools` session under `aep:implementing` 0.23.2 (wave mode). Approved with
wave w2 by `adr-changes` (now `b10x-chef-orchestrator`): "then `toolset-search`", and
`build-test-contract` moved here by wave w2's D1, which the approver accepted.

## Units

| unit | story | serves | branch | scope confidence |
|---|---|---|---|---|
| 1 | `story:build-test-contract` | O1, O2, O3 | `impl/build-test-contract` | cited |
| 2 | `story:toolset-search` | O1, O2, O3 | `impl/toolset-search` | cited, two `inferred` (`crates/tool-fs/src/walk.rs`, `lib.rs`) |

Both fork from `wave/w3` at the opening commit and run at once (N = 2). Both depend on stories
`implemented` in wave w2 (`toolset-process`, `fs-glob-stops-at-the-cap`).

Not in this wave: `story:contract-directory-names-validated`, `story:process-kill-never-reaches-a-reused-group`
(findings filed by wave w2), `story:docs-site`, and `story:ess-domains-for-contracts` (widened to
the profile and every toolset in ESS; it waits for both units of this wave).

## What `aep plan artifact waves --kind story --status draft` printed

Before `Cargo.lock` was added to both stories' scope:

```
wave 1
  story:build-test-contract
  story:contract-directory-names-validated
  story:docs-site
  story:process-kill-never-reaches-a-reused-group
  story:toolset-search (inferred)
wave 2
  story:ess-domains-for-contracts (inferred)
2 wave(s), 0 collision(s), 0 unassessed
```

Both units add a dependency or a crate, so both change `Cargo.lock`; it is now in both scopes and
the coordinator regenerates it at the second merge (wave t1's D5).

## Triples

Wave root: `$HOME/.cache/tools-wave-w3`. Managed trees under `$HOME/.local/state/worktree/trees/b10x/tools/`,
each building into its own `target/`. `<root>/bin/cargo-serial.sh` runs one `cargo` at a time for
the wave, through `b10x-build-slot` (the orchestrator's machine-wide slots and free-space check,
rule of 2026-10-20), with `sccache` started before a slot is taken. Harness source read-only at
`<root>/harness-0.14.1`.

| unit | managed id | worktree | build dir | scratch |
|---|---|---|---|---|
| coordinator | `wave-w3` | `<trees>/wave-w3` (branch `wave/w3`) | `<tree>/target` | `<root>/coordinator/scratch` |
| 1 | `impl-build-test` | `<trees>/impl-build-test` | `<tree>/target` | `<root>/build-test/scratch` |
| 2 | `impl-search` | `<trees>/impl-search` | `<tree>/target` | `<root>/search/scratch` |

## Pre-flight (2026-10-20)

| check | value |
|---|---|
| base | `main` at `60d4d0e` (tag `0.2.0`), primary checkout clean |
| `git worktree list` | the primary checkout and this wave's integration tree |
| free disk, accounts | the orchestrator's (rule of 2026-10-20) |
| one measured build | wave w2: `target/` 494 M to 1,336 M per unit tree |
| dispatch | `aep:implementor`, `aep:adversary` (plugin agents) |

## Commits this approval covers

1. The opening store commit on `wave/w3` (moves to `active`, scopes, this page).
2. One commit per unit on its `impl/*` branch, by the coordinator through the bot route.
3. The merge of each green unit into `wave/w3`, and the `Cargo.lock` regeneration at the second.
4. The closing store commit.
5. The merge of `wave/w3` into `main`, its push, and the pull request that carries CI.

No release is part of this approval.

## Decisions taken by the coordinator

| id | decision | reason |
|---|---|---|
| D1 | the build-test executor runs each planned argv through `b10x-tool-process` | wave w2's D1; harness's `invoke_toolchain` runs them through its process runner (`catalogue.rs:534`) |
| D2 | `tool-search` reuses `tool-fs`'s walk, exported from `tool-fs`, rather than a copy | the walk was fixed in wave w2 (`fs-glob-stops-at-the-cap`); a copy would not carry the next fix |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 | integration tree `wave-w3` at `60d4d0e`, branch `wave/w3`; both stories scoped onto `Cargo.lock` and moved to `active` |
