# Wave t1: the profile, the fs and code-editor toolsets, the toolchain crate

Proposed 2026-10-20 by the `tools` session under `aep:implementing` 0.23.2 (wave mode). Basis: the
owner's decision of 2026-10-20 recorded in atlas ADR 0097 ("the tools from harness must move into
the tools repo"), ADR 0095 phase T1, and this repository's design
(`docs/design/tool-contracts.md` § T1 in detail).

## Units

| unit | story | serves | branch | scope confidence |
|---|---|---|---|---|
| 1 | `story:tool-contract-profile` | O1, O2 | `impl/tool-contract-profile` | cited |
| 2 | `story:toolset-fs` | O1, O2, O3 | `impl/toolset-fs` | cited |
| 3 | `story:toolset-code-editor` | O1, O2, O3 | `impl/toolset-code-editor` | cited |
| 4 | `story:toolset-toolchain` | O1, O3 | `impl/toolset-toolchain` | cited |

Unit 1 runs first; units 2 to 4 fork from the integration branch after unit 1 has merged, because
all three build on the `b10x-tool-contract` crate it creates. Both steps run under this one
approval. The release of `0.1.0` is a separate step after the wave closes, and is asked for in the
same proposal because harness and metaharness need a tag to pin.

Left out, as drafts for T3: `story:toolset-search`, `story:toolset-process` (the `grep` and `exec`
halves of harness's `local.rs`), `story:build-test-contract` (a static contract over the toolchain
crate), `story:ess-domains-for-contracts` (lifting the documents into ESS once ESS can express an
outcome decided outside the input).

## What `aep plan artifact waves --kind story --status draft` printed

```
wave 1
  story:tool-contract-profile
wave 2
  story:toolset-code-editor
  story:toolset-process
wave 3
  story:toolset-fs
wave 4
  story:ess-domains-for-contracts (inferred)
  story:toolset-search
  story:toolset-toolchain
wave 5
  story:build-test-contract
collision: story:build-test-contract story:toolset-toolchain crates/tool-toolchain
collision: story:ess-domains-for-contracts story:tool-contract-profile crates/tool-contract (inferred)
collision: story:tool-contract-profile story:toolset-code-editor Cargo.lock
collision: story:tool-contract-profile story:toolset-fs Cargo.lock
collision: story:tool-contract-profile story:toolset-toolchain Cargo.lock
collision: story:toolset-code-editor story:toolset-fs Cargo.lock
collision: story:toolset-code-editor story:toolset-toolchain Cargo.lock
collision: story:toolset-fs story:toolset-toolchain Cargo.lock
5 wave(s), 8 collision(s), 0 unassessed
```

Selection path: the verb. Where this plan departs from the verb: units 2, 3 and 4 collide only on
`Cargo.lock`, which each unit regenerates by adding its own crate; the coordinator resolves that
file at each merge with `cargo update --workspace --offline` and the gate's `--locked` build checks
the result. The two collisions with `crates/tool-toolchain` and `crates/tool-contract` are with
stories this wave leaves out. The six units `waves` places in waves 2 to 5 that are not in this
plan are the T3 drafts.

## Triples

Wave root: `$HOME/.cache/tools-wave-t1`. Managed trees live under the worktree manager's state
directory; each builds into its own `target/` inside the tree.

| unit | managed id | worktree | build dir | scratch |
|---|---|---|---|---|
| coordinator | `wave-t1` | `<state>/worktree/trees/b10x/tools/wave-t1` | `<tree>/target` | `<root>/coordinator/scratch` |
| 1 | `impl-tool-contract-profile` | `<state>/worktree/trees/b10x/tools/impl-tool-contract-profile` | `<tree>/target` | `<root>/tool-contract-profile/scratch` |
| 2 | `impl-toolset-fs` | `<state>/worktree/trees/b10x/tools/impl-toolset-fs` | `<tree>/target` | `<root>/toolset-fs/scratch` |
| 3 | `impl-toolset-code-editor` | `<state>/worktree/trees/b10x/tools/impl-toolset-code-editor` | `<tree>/target` | `<root>/toolset-code-editor/scratch` |
| 4 | `impl-toolset-toolchain` | `<state>/worktree/trees/b10x/tools/impl-toolset-toolchain` | `<tree>/target` | `<root>/toolset-toolchain/scratch` |

`<state>` is `$HOME/.local/state`; `<root>` is the wave root. Builds across trees are serialised by
`<root>/bin/cargo-serial.sh` (one `cargo` at a time, refused under 25 G free, killed under 25 G
free, `sccache` as the compiler wrapper), which is the operator's rule for this machine.

## Pre-flight (2026-10-20)

| check | value |
|---|---|
| base | `main` at `dabd034`, clean |
| `git worktree list` | one tree, this wave's integration tree |
| free disk on `/` | 38 G (`df -h /`); floor 25 G |
| compiler cache | `sccache` installed, 9.4 G cache, wired through `cargo-serial.sh` |
| one measured build | none yet: the repository has no crate before unit 1. Unit 1's build is measured and written here before units 2 to 4 are sized |
| model budget | active account 5 h headroom 42 %, 9 of 14 accounts usable (`ais`); N = 1 then 3 |
| dispatch | `aep:implementor`, `aep:adversary` (plugin agents) |

## Commits this approval authorises

1. The opening store commit on `wave/t1` (this page, the design section, the story bodies and
   scopes, the moves to `active`, AGENTS.md).
2. One commit per unit on its `impl/*` branch, by the implementor's work as committed by the
   coordinator through the bot route.
3. The merge of each green unit into `wave/t1`, and `Cargo.lock` regeneration commits where a merge
   needs one.
4. The closing store commit (evidence, moves to `implemented`, README).
5. The merge of `wave/t1` into `main`, its push, and the pull request that carries CI.

Not covered: the tag, the version bump and the GitHub release. Those are the `0.1.0` release,
asked for separately below.

## Release asked for

After the wave closes: workspace version `0.1.0`, `CHANGELOG.md`, tag `0.1.0` on the merge commit
and a GitHub release `tools 0.1.0`, all through the bot route (`AGENTS.md` § Releases).

## Decisions taken by the coordinator

| id | decision | reason |
|---|---|---|
| D1 | the contract document is `tool-contract/1` YAML with ESS-shaped commands; `ess specify validate` does not run on it in T1 | ess 0.60.0 refuses a command without an emitted event and without `when` (ESS-COMMAND-007, -004, probe of 2026-10-20) |
| D2 | workspace `rust-version = "1.98"`, edition 2024; licence per crate: the crates moved from harness (`tool-fs`, `tool-code-editor`, `tool-toolchain`) keep `LicenseRef-B10x-Proprietary` as harness declares it, `tool-contract` (written new) is Apache-2.0, and the README's crate table names each crate's licence | llm and metaharness are at 1.98; harness moves to 1.98 under ADR 0097. The licence part was changed at approval (2026-10-20, `adr-changes`): relabelling harness's proprietary code Apache-2.0 is a grant neither ADR 0095 nor ADR 0097 makes, so it waits for the owner; the change is then one field per crate |
| D3 | the toolchain registry moves as `crates/tool-toolchain` and derives profile-conforming operations; the static `build-test` contract is a draft story | its YAML already declares programs, environment, roots and written paths; its operations are resolved per workspace |
| D4 | `search` and `process` stay in harness's `local.rs` for T1 and are drafted as T3 stories | the T1 scope is fs and code-editor; the drafts keep ADR 0097's "all of harness's tools" visible |
| D5 | `Cargo.lock` is declared in every unit's scope and the collision is worked | hiding a shared file from the verb would make the selection look safer than it is |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 03:50 | integration tree `wave-t1` created at `dabd034`; branch `wave/t1`; store planned; proposal sent to `adr-changes` |
| 2026-10-20 04:00 | approved by `adr-changes` under standing exception 5, with D2's licence part changed; a licence gate refusing a proprietary crate stops units 2 to 4 before `main` and closes the wave with unit 1 |
