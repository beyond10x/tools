# Wave w2: contract documents without an implementation, the capped glob walk, the process toolset

Proposed 2026-10-20 by the `tools` session under `aep:implementing` 0.23.2 (wave mode). Approved the
same day by the `adr-changes` session (atlas ADR 0097 point 3, ADR 0095): the store commit of the
plan made after wave t1, and the stories `contract-documents-without-an-implementation`,
`fs-glob-stops-at-the-cap`, `build-test-contract` and `toolset-process`, then `toolset-search`, and
release `0.2.0` once `contract-documents-without-an-implementation` is merged.

## Units

| unit | story | serves | branch | scope confidence |
|---|---|---|---|---|
| 1 | `story:contract-documents-without-an-implementation` | O1, O3 | `impl/contract-documents-without-an-implementation` | cited |
| 2 | `story:fs-glob-stops-at-the-cap` | O1 | `impl/fs-glob-stops-at-the-cap` | cited |
| 3 | `story:toolset-process` | O1, O2, O3 | `impl/toolset-process` | cited |

The three fork from `wave/w2` at the opening commit and run at once (N = 3).

Left for wave w3, under the same approval: `story:build-test-contract` and `story:toolset-search`.
`build-test-contract` runs the toolchain's planned argv through a process runner, which in harness is
`Operations::run_within` over `local.rs` `exec` (harness 0.14.1 `crates/harness-tools/src/catalogue.rs`
`invoke_toolchain`, line 534); `toolset-process` moves that runner here, so the build-test executor
is built on `crates/tool-process` rather than on a second copy of it (decision D1, edge
`depends_on: story:toolset-process` added). `toolset-search` reuses the walk that
`fs-glob-stops-at-the-cap` changes (`depends_on` edge added at planning). Release `0.2.0` is cut
after this wave closes, so the adopter waiting on unit 1 gets a tag before wave w3 runs.

Not in either wave: `story:ess-domains-for-contracts` (open
`decision-blocker:ess-outcome-decided-outside-input`) and `story:docs-site`.

## What `aep plan artifact waves --kind story --status draft` printed

Before the `build-test-contract → toolset-process` edge was added:

```
wave 1
  story:build-test-contract
  story:contract-documents-without-an-implementation
  story:docs-site
  story:fs-glob-stops-at-the-cap
  story:toolset-process
wave 2
  story:ess-domains-for-contracts (inferred)
  story:toolset-search (inferred)
collision: story:contract-documents-without-an-implementation story:ess-domains-for-contracts crates/tool-contract (inferred)
collision: story:fs-glob-stops-at-the-cap story:toolset-search crates/tool-fs/src/lib.rs (inferred)
collision: story:fs-glob-stops-at-the-cap story:toolset-search crates/tool-fs/src/walk.rs (inferred)
2 wave(s), 3 collision(s), 0 unassessed
```

Selection path: the verb, narrowed by D1. The three collisions are with stories outside this wave.
Unit 3 adds a crate and so changes `Cargo.lock`; units 1 and 2 add no dependency, so no unit
shares a file with another.

## Triples

Wave root: `$HOME/.cache/tools-wave-w2`. Managed trees live under `$HOME/.local/state/worktree/trees/b10x/tools/`;
each builds into its own `target/` inside the tree. Builds across trees are serialised by
`<root>/bin/cargo-serial.sh` (one `cargo` at a time, refused and killed under 25 G free on `/`,
`sccache` as the compiler wrapper). The harness source the units port is snapshotted read-only at
`<root>/harness-0.14.1` (`git archive 0.14.1`; `local.rs` differs from the 0.13.3 snapshot wave t1
used by one test assertion, `local.rs:1703-1709`).

| unit | managed id | worktree | build dir | scratch |
|---|---|---|---|---|
| coordinator | `plan-after-t1` | `<trees>/plan-after-t1` (branch `wave/w2`) | `<tree>/target` | `<root>/coordinator/scratch` |
| 1 | `impl-contract-documents` | `<trees>/impl-contract-documents` | `<tree>/target` | `<root>/contract-documents/scratch` |
| 2 | `impl-fs-glob` | `<trees>/impl-fs-glob` | `<tree>/target` | `<root>/fs-glob/scratch` |
| 3 | `impl-toolset-process` | `<trees>/impl-toolset-process` | `<tree>/target` | `<root>/process/scratch` |

## Pre-flight (2026-10-20)

| check | value |
|---|---|
| base | `main` at `046dfb1` (tag `0.1.0`), primary checkout clean |
| `git worktree list` | the primary checkout and this wave's integration tree |
| free disk on `/` | 66 G (`df -h /`); floor 25 G |
| compiler cache | `sccache`, 20 GiB maximum, wired through `cargo-serial.sh` |
| one measured build | wave t1: 76 s cold, 546 M to 689 M of `target/` per tree |
| model budget | 9 of 14 accounts usable, weekly headroom 6 % to 16 % (`ais status`); N = 3 |
| dispatch | `aep:implementor`, `aep:adversary` (plugin agents) |

## Commits this approval authorises

1. The opening store commit on `wave/w2` (the plan made after wave t1, the moves to `active`, this page).
2. One commit per unit on its `impl/*` branch, made by the coordinator through the bot route.
3. The merge of each green unit into `wave/w2`, and a `Cargo.lock` regeneration where a merge needs one.
4. The closing store commit (evidence, moves to `implemented`, scopes, README, CHANGELOG).
5. The merge of `wave/w2` into `main`, its push, and the pull request that carries CI.

Then, as a separate step (AGENTS.md § Releases): release `0.2.0`.

## Decisions taken by the coordinator

| id | decision | reason |
|---|---|---|
| D1 | `build-test-contract` waits for `toolset-process` and runs in wave w3 | its executor runs each planned argv through harness's process runner (`catalogue.rs:534`); building it before `crates/tool-process` exists would copy `exec` a second time |
| D2 | `b10x-tool-contract` discovers every `contracts/<toolset>/<version>/contract.yaml` at build time rather than listing two | a toolset added in a later unit then needs no edit to `crates/tool-contract`, so no unit of this wave or the next shares that crate with unit 1 |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 | integration tree `plan-after-t1` at `046dfb1`, branch `wave/w2`; store plan written; approved by `adr-changes` |
