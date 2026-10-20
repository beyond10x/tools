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
| D6 | the `code-editor` contract declares no `input_bytes` bound, and the toolchain crate pins `regex` itself rather than through the workspace | the profile's bound names are the five the validator knows; a bound no implementation enforces would be declared and not kept. A workspace `regex` entry is one line the root manifest can gain later without a unit touching it |
| D7 | the `fs` contract declares its 500-entry, 500-path and depth-12 caps through `paging` fields and `semantics.md`, not as named bounds | the profile's bound names are the design's five (`timeout_ms`, `output_bytes`, `processes`, `cpu_ms`, `memory_bytes`); widening the vocabulary is a profile change for a later version, recorded in the implementor's patch under the wave root |
| D8 | the pre-existing defect the code-editor adversary found in harness's edit (overlapping occurrences of `old` counted as one) is fixed in this unit rather than filed | the fix is one loop change inside code this unit already owns, and the adversary's scenario for it is in the suite; the crate rustdoc records the difference from harness 0.13.3 |
| D10 | `tool-toolchain`'s `lib.rs` gains a third documented change beyond the crate rustdoc and `pub mod contract;`: `ResolvedProvider::plan` falls back to the mangled-name lookup when the exact tool name misses | the adversary's case pins that a published operation name plans through `plan`; the fallback fires only on a miss, so every builtin and every harness call by a tool's own name behaves as before. Measured by the implementor: 6 of 6 adversary cases green with the patch, 5 of 6 without |
| D9 | a toolchain tool name outside the profile's `[a-z][a-z0-9_-]*` grammar derives a mangled operation name (lower-cased, `_` for other characters, `t_` prefix when not letter-initial) instead of a refusal; the adversary's case pinning the raw names is rewritten by the adversary to the decision | the derivation must stay total over what the registry admits, and the registry's `legal_name` is in the verbatim-moved `lib.rs` |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 03:50 | integration tree `wave-t1` created at `dabd034`; branch `wave/t1`; store planned; proposal sent to `adr-changes` |
| 2026-10-20 04:00 | approved by `adr-changes` under standing exception 5, with D2's licence part changed; a licence gate refusing a proprietary crate stops units 2 to 4 before `main` and closes the wave with unit 1 |
| 2026-10-20 04:08 | opening commit `9b97da8` on `wave/t1` (bot); `b10x-gates check`: common checks passed, 3 commits scanned; unit 1 tree `impl-tool-contract-profile` at `9b97da8`, branch `impl/tool-contract-profile`, implementor dispatched; free disk 29 G (other sessions building) |
| 2026-10-20 04:12 | unit 1 implementor refused with HTTP 429 at its first step; account rotated by `ais`; same agent resumed |
| 2026-10-20 04:34 | build wrapper hung 15 min on its lock: the sccache daemon started under the first build inherited the lock's fd. Wrapper fixed (fd closed for children, new lock file), the waiting process killed, implementor told to rerun |
| 2026-10-20 05:05 | unit 1 implementor green: 31 cases (lib 2, cli 6, conformance 9, shapes 6, validate 8), 0 red; cold build 76 s, `target/` 338 M after tests, 546 M after clippy and the msrv check; 212,639 sub-agent tokens, 49 tool uses, 31.7 min. Claim verified by the coordinator: the four defect documents exit 1 naming operation and property, the valid fixture exits 0 (`coordinator/scratch/verify-unit1-treatment.log`); the base has no crate, so the behaviour is new. Adversary pass 1 dispatched |
| 2026-10-20 05:30 | adversary pass 1: red, 31→68 cases, 5 red, 5 findings all `introduced` (2 CONFIRMED in the runner: expected errors not checked against the operation's outcomes; fixture symlinks copied as files; 2 INFEASIBLE shape edge cases; 1 design-doc signature, fixed by the coordinator). Recorded as `review-result:adversary-tool-contract-profile-pass-1`; 182,249 tokens, 34 tool uses, 16.9 min. Correction round 1 sent to the same implementor |
| 2026-10-20 05:45 | correction round 1 green: 68→68 cases, 5 red → 0, no adversary file touched; profile document's scenario, post-condition, coverage, runner and shape sections changed with the rules; 258,586 tokens, 25 tool uses, 5.2 min. `review_outcome fixed` recorded against pass 1. Adversary pass 2 (the last) dispatched to the same adversary |
| 2026-10-20 06:00 | adversary pass 2: no failing case, 68→81 cases, 0 red; pass-1 findings resolved 5, carried 1 (the design-doc line, fixed in the integration tree, not visible in the unit tree), new 2 judgement items (profile document contradiction at its shape table; returned error fields unchecked). Recorded as `review-result:adversary-tool-contract-profile-pass-2`; 228,748 tokens, 12 tool uses, 6.0 min. Correction round 2 sent to the same implementor; the coordinator reads its diff, no third attack |
| 2026-10-20 06:15 | correction round 2 green: 81→86 cases (a new `error_fields` lane of 5), the returned error's fields checked against the declaration, the profile document's shape sentence fixed; 289,224 tokens, 11 tool uses, 4.2 min. Coordinator read the diff (`check_error_fields`, the two document hunks; adversary files unchanged by line count). Unit commit `4ebd7e8` on `impl/tool-contract-profile` (bot), `b10x-gates check`: passed, 4 commits scanned. `review_outcome fixed` recorded against pass 2. Merged into `wave/t1` with `--no-ff` as `796937c` (bot); the merged base builds and its 86 tests pass; `target/` 438 M |
| 2026-10-20 06:20 | units 2 to 4 forked from `796937c`: trees `impl-toolset-fs`, `impl-toolset-code-editor`, `impl-toolset-toolchain`, branches `impl/<story>`; three implementors dispatched (N = 3, sized from the unit 1 build: 76 s cold, 546 M to 689 M per tree; free disk 69 G) |
| 2026-10-20 06:40 | unit 4 (toolchain) implementor green: 8→17 cases, `lib.rs` differs from the snapshot by the crate rustdoc and `pub mod contract;` only, `builtins/*.yaml` byte-identical; 166,229 tokens, 53 tool uses, 11.2 min. Its ask for a workspace-level `regex` entry declined: the crate pins `regex = "1.13"` as harness did. Adversary dispatched. Coordinator check: `lib.rs` diff against the snapshot is the rustdoc paragraph and `pub mod contract;`, the five `builtins/*.yaml` are `cmp`-identical; recorded as verification evidence |
| 2026-10-20 03:35Z | the fs and code-editor implementors and the toolchain adversary all refused with HTTP 429 ("out of usage credits") while still reading, before any write; found at 04:03Z when the cron ticks drained. Account rotated by `ais` (active: 96 % headroom); the same three agents resumed. Times in this log before this row are the coordinator's local estimates; from here they are UTC |
| 2026-10-20 04:15Z | unit 3 (code-editor) implementor green: 18 unit tests (9 ported from harness by name, 9 new), 51 scenarios in 5 files, 3 mutants each failing 3 to 5 scenarios; `similar 2.7.0` added; 208,474 tokens, 13 tool uses, 7.7 min. Its ask for an `input_bytes` bound name declined: a bound no implementation enforces is not declared (D6). Adversary dispatched |
| 2026-10-20 04:25Z | unit 4 (toolchain) adversary pass 1: red, 17→22 cases, 5 red, 8 findings all `introduced` (5 CONFIRMED in `contract.rs`: unnamed error on an empty workspace, names outside the grammar, 4 of 8 output fields, for-each and empty-stdout tools equal to plain runs; 3 notes: dead `Local` placement, the story's `argv[0]` wording, parameter defaults lost). Recorded as `review-result:adversary-toolset-toolchain-pass-1`; 148,565 tokens, 34 tool uses, 8.8 min. Correction round 1 sent to the same implementor; the story's Outcome wording corrected by the coordinator |
| 2026-10-20 04:35Z | unit 2 (fs) implementor green: 22 unit tests (14 ported from harness by name, 8 new), 44 scenarios in 5 files with committed symlink fixtures, 4 mutants each failing 3 to 7 scenarios; 320,540 tokens, 58 tool uses, 14.6 min. Its ask for bound names `entries`, `paths`, `depth` declined for this wave (D7): the profile keeps the design's five bound names; the caps are declared through `paging` and semantics. Adversary dispatched |
| 2026-10-20 04:50Z | unit 3 (code-editor) adversary pass 1: red, 51→59 scenarios and 2 test cases, 7 findings (6 `introduced`, 1 `pre-existing` from harness: overlapping matches counted as one); the blocker is a containment bypass through a `link/` spelling. Recorded as `review-result:adversary-toolset-code-editor-pass-1`; 145,472 tokens, 40 tool uses, 10.0 min. Correction round 1 sent to the same implementor, the pre-existing defect included in the fix (D8) |
| 2026-10-20 04:52Z | unit 4 (toolchain) correction round 1: 6 of 7 fixed, lib 17→23 cases; adversary case 2 pins raw names and validator acceptance at once, unsatisfiable under the grammar. Coordinator decision (D9): the derivation mangles a name to the grammar; the adversary rewrites its own case to that decision in pass 2, dispatched to the same adversary; 227,680 tokens, 19 tool uses, 8.4 min |
| 2026-10-20 05:05Z | unit 4 (toolchain) adversary pass 2: 28→29 cases, 1 red (a mangled operation name cannot be routed back to its tool for planning), pass-1 findings resolved 6 plus case 2 rewritten to D9, 3 notes (the harness's run-time array-or-object answer the profile cannot express; case-sensitive collision check in the verbatim `lib.rs`; the story wording, already fixed in the integration tree). Recorded as `review-result:adversary-toolset-toolchain-pass-2`; 191,769 tokens, 17 tool uses, 6.0 min. Correction round 2 sent to the same implementor; the coordinator reads its diff, no third attack |
| 2026-10-20 05:10Z | unit 3 (code-editor) correction round 1 green: lib 18→23, 59 of 59 scenarios, mutants compared against an intact run, `code.NotText` declared, overlapping matches counted (D8), containment bypass closed; 275,531 tokens, 37 tool uses, 8.2 min. `review_outcome fixed` recorded against pass 1. Adversary pass 2 (the last) dispatched to the same adversary |
| 2026-10-20 05:20Z | unit 4 (toolchain) correction round 2: `tool_for` and `plan_operation` added, lib 23→25; adversary case 6 calls `plan` by the published name, unreachable from `contract.rs`; the implementor measured a 3-line `plan` fallback in `lib.rs` (6 of 6 green) and reverted it; D10 taken and the implementor told to apply it; 260,719 tokens, 23 tool uses, 4.8 min |
| 2026-10-20 05:25Z | unit 2 (fs) adversary pass 1: red, 44→61 scenarios and 5 test cases, 4 red, 6 findings (1 `pre-existing` from harness: glob truncation at exactly the limit; 3 `introduced`: 16 MiB boundary, `limit: 0` sentence, `a.txt/x` as `fs.Io`; 2 notes: unheld guarantees). Recorded as `review-result:adversary-toolset-fs-pass-1`; 243,421 tokens, 53 tool uses, 17.0 min. Correction round 1 sent to the same implementor, the pre-existing defect included in the fix (D8) |
| 2026-10-20 05:35Z | unit 4 (toolchain) D10 applied: `plan` falls back to `tool_for`; 25 lib and 6 adversary cases green; `lib.rs` diff against the snapshot 17 lines; 271,963 tokens, 32 tool uses, 6.8 min. Coordinator's own run: exit 0, same counts (`coordinator/scratch/verify-toolchain.log`); `review_outcome fixed` against pass 2; verification evidence recorded. Unit commit `c01bab4` (bot, gates check passed, 6 commits scanned); merged into `wave/t1` as `e46926b`; the merged workspace builds, 17 test lanes green, free disk 67 G |
| 2026-10-20 05:45Z | unit 3 (code-editor) adversary pass 2: pass-1 findings all resolved, 59→65 scenarios and 7 new cases, 1 red note-severity case (`code.delete` of a link pointing outside names the wrong error) and 3 notes (a test racing on a shared marker file; the header-only identity diff; `new/` creating a file). Recorded as `review-result:adversary-toolset-code-editor-pass-2`; 216,480 tokens, 22 tool uses, 9.8 min. Correction round 2 sent to the same implementor; the coordinator reads its diff, no third attack. The monitor asked whether the containment bypass was pre-existing: answered no, with harness `local.rs:203-214` and `:490-503` as the evidence |
| 2026-10-20 05:55Z | unit 2 (fs) correction round 1 green: unit 22→25, 63 of 63 scenarios, 5 mutants (a `NoCeilingCap` mutant and a 301,000-byte fixture added), glob truncation fixed (D8), 16 MiB boundary made exact, `limit: 0` refused, `NotADirectory` as `fs.NotFound`; 385,070 tokens, 59 tool uses, 6.9 min. `review_outcome fixed` against pass 1. Adversary pass 2 (the last) dispatched to the same adversary |
| 2026-10-20 06:05Z | unit 3 (code-editor) correction round 2 green: 68 of 68 scenarios, 35 cases; `delete` refuses a link by name before following it, a trailing-slash spelling on an absent path is `code.NotAFile`, the move test no longer races on a shared marker, the identity diff is documented as read-only; 313,615 tokens, 28 tool uses, 4.4 min. Coordinator read `delete`, `normalise`/`Spelling` and ran the tests (exit 0, same counts, `coordinator/scratch/verify-code-editor.log`); `review_outcome fixed` against pass 2; verification evidence recorded. Unit commit `2f024bf` (bot, gates check passed, 6 commits scanned); `Cargo.lock` auto-merged; merged into `wave/t1` as `2032e2d`; the merged workspace builds `--locked`, 23 test lanes green |
| 2026-10-20 06:20Z | unit 2 (fs) adversary pass 2: pass-1 findings 6 of 6 resolved, 63→69 scenarios and 1 new case, 1 red (a one-line file over 16 MiB counted and digested by `read` while `stat` answers null), 4 notes (2 pre-existing from harness: the scan is bounded in lines not bytes; a capped glob keeps walking). Recorded as `review-result:adversary-toolset-fs-pass-2`; 330,559 tokens, 23 tool uses, 8.0 min. Correction round 2 sent to the same implementor (R1, N1, J4, J5); J3 to a follow-up story at close; the coordinator reads the diff, no third attack |
| 2026-10-20 06:30Z | the store refused the fs pass-2 review's YAML findings block (an unquoted `: ` in a message); the refusal was sent back verbatim and the adversary re-emitted the five findings as JSON, recorded unchanged in substance. Ledger for `story:toolset-fs`: resolved 6, new 5, carried 0. J3 filed as `story:fs-glob-stops-at-the-cap` (draft). One duplicate `review_outcome fixed` record against pass 1 was appended by mistake; the store keeps both |
| 2026-10-20 06:40Z | unit 2 (fs) correction round 2 green: unit 25→28, 70 of 70 scenarios, the scan bounded in bytes read (`next_line` takes a budget; the bound tested after every line), the path judged before the window, the first-line cut reserving a separator only where one exists; 428,790 tokens, 29 tool uses, 5.0 min. Coordinator read `next_line`/`Scan::take` and ran the tests (exit 0, same counts, `coordinator/scratch/verify-fs.log`); `review_outcome fixed` against pass 2; verification evidence recorded. Unit commit `4779bc4` (bot, gates check passed, 6 commits scanned). The merge into `wave/t1` conflicted on `Cargo.lock` as D5 planned; resolved by `git checkout --ours Cargo.lock` and `cargo update --workspace --offline` (added `globset 0.4.20`, `log 0.4.34`), committed as the merge `0b45907` (bot); the merged workspace builds `--locked`, 28 test lanes green |
| 2026-10-20 06:50Z | all four units merged. Whole gate on `wave/t1` at `0b45907`, one exit status per step, logs under `coordinator/scratch/gate/`: steps 1 to 5 exit 0 (28 lanes, 194 tests), step 6 `tool-contract validate contracts/` exit 1 with `Too many levels of symbolic links (os error 40)`: the document walk followed the fixture links back up the tree (`fs/.../linked/up -> ..`, `code-editor/.../link-to-parent/link -> ..`); each unit tree alone had passed because the kernel cut the loop at a different depth |
| 2026-10-20 07:05Z | coordinator fix on the integration branch (a shared file no implementor held): the walk reads each entry's own file type and skips symlinks; a test lays out a parent link and an alias to a second document and asserts each document is reported once (red on the unfixed binary: the same document reported under every loop spelling). Commit `15f367e` (bot, gates check passed, 12 commits scanned). Whole gate re-run at `15f367e`, logs under `coordinator/scratch/gate2/`: all six steps exit 0; 28 lanes, 195 tests passed, 0 failed; `ok contracts/code-editor/v1alpha1/contract.yaml`, `ok contracts/fs/v1alpha1/contract.yaml`; free disk 48 G |

## Closing

### The gate (at `15f367e`)

| step | command | exit | what it printed |
|---|---|---|---|
| 1 | `aep plan artifact validate` | 0 | `valid` |
| 2 | `cargo test --workspace --locked` | 0 | 28 lanes `test result: ok`, 195 tests passed, 0 failed |
| 3 | `cargo fmt --all --check` | 0 | nothing |
| 4 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished` |
| 5 | `cargo +1.98.0 check --workspace --locked` | 0 | `Finished` |
| 6 | `cargo run --locked -p b10x-tool-contract -- validate contracts/` | 0 | `ok` for both documents |

No step skipped itself; every step printed what its exit status says.

### Commits the wave made (all by the bot)

| sha | what |
|---|---|
| `9b97da8` | opening store commit |
| `4ebd7e8` | unit 1, `impl/tool-contract-profile` |
| `796937c` | merge of unit 1 into `wave/t1` |
| `c01bab4` | unit 4, `impl/toolset-toolchain` |
| `e46926b` | merge of unit 4 |
| `2f024bf` | unit 3, `impl/toolset-code-editor` |
| `2032e2d` | merge of unit 3 (`Cargo.lock` auto-merged) |
| `4779bc4` | unit 2, `impl/toolset-fs` |
| `0b45907` | merge of unit 2 (`Cargo.lock` regenerated, D5) |
| `15f367e` | fix: the document walk never follows a symlink (coordinator, on the integration branch) |
| closing | the closing store commit: evidence, moves, scopes, README, CHANGELOG, design, this page |

### What each unit cost (sub-agent tokens are each agent's cumulative total; tool uses and minutes are summed over its turns)

| unit | implementor | adversary | passes | corrections | findings |
|---|---|---|---|---|---|
| 1 tool-contract-profile | 289,224 tokens, 85 uses, 41.1 min | 228,748 tokens, 46 uses, 22.9 min | 2 (5 then 0 red) | 2 | 8 (5 + 3) |
| 4 toolset-toolchain | 271,963 tokens, 127 uses, 31.2 min | 191,769 tokens, 51 uses, 14.8 min | 2 (5 then 1 red) | 2 plus D10 | 12 (8 + 4) |
| 3 toolset-code-editor | 313,615 tokens, 78 uses, 20.3 min | 216,480 tokens, 62 uses, 19.8 min | 2 (10 then 1 red) | 2 | 11 (7 + 4) |
| 2 toolset-fs | 428,790 tokens, 146 uses, 26.5 min | 331,780 tokens, 76 uses, 25.2 min | 2 (4 then 1 red) | 2 | 11 (6 + 5) |
| total | 1,303,592 tokens | 968,777 tokens | 8 | 8 | 42 |

Sub-agent tokens in total: 2,272,369 over 8 agents and 20 turns. Tests at close: 195 in 28 lanes;
scenarios: 70 (`fs`), 68 (`code-editor`); mutants: 5 (`fs`), 3 (`code-editor`), each killed by the
suite. Review-results recorded: 8; review outcomes: 9 (one duplicate); verification records: 4;
test_result records: 4.

### Found along the way and filed, not fixed

- `story:fs-glob-stops-at-the-cap` (draft): a capped `fs.glob` keeps walking the tree after the cap;
  pre-existing in harness; the answer is unaffected.
- The implementors' two profile patches (bound names `entries`/`paths`/`depth`; `input_bytes`) under
  the wave root, declined by D6 and D7, for a later profile version.
- The harness answer shape for a multi-process toolchain run (an object or an array by run-time
  cardinality) that the profile cannot express: for `story:build-test-contract`.
