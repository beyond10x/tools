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
| D3 | the licence change lands as its own coordinator commit on `wave/w2`, before the units merge | the owner's decision came after dispatch; no unit holds the three `Cargo.toml` files, README, AGENTS.md or CHANGELOG, so the commit conflicts with none of them. Approved by `adr-changes` with the decision |
| D4 | the capped `fs.glob` answering `depth_bound_reached: false` when the walk stopped at the cap before reaching a directory past the bound is accepted, in `v1alpha1` | `contracts/fs/v1alpha1/semantics.md:94`: "The walk stops descending at depth 12; `depth_bound_reached: true` says it did somewhere". A walk that stopped at the cap first did not stop descending at depth 12; `truncated: true` already says the answer is partial. No scenario pins the old answer |
| D5 | unit 1's two notes ship unchanged and are filed as `story:contract-directory-names-validated` | both `INFEASIBLE`: no planned toolset name breaks the grammar and no caller reads order as precedence; a correction round would cost a second adversary pass for a rustdoc line and a validator rule outside the story |
| D6 | unit 3's pre-existing defects F1 to F3 are fixed in this unit, not filed; F6 declares `effects: [process, write, network, secret]` | F1 fails the story's own acceptance sentence; F2 and F3 sit in the same functions the unit already owns (wave t1 D8 did the same). F6: nothing confines a declared program beyond environment, time and output, so a grant that admits by declared effects must see what the program can do; the design's "no network unless granted" is a property of a confined implementation, which is a narrower contract |
| D7 | after pass 2, `process.run` keeps the stronger cut sentence and the code is made to match it (P1, P2); a `Mutation` that kills only a live shell is added (P3); every profile consequence is declared (P4); P5 is filed | the adversary's two cases state the sentence exactly and the fix keeps three bytes on each side; P3 is the story's acceptance property and a mutant is the profile's way to hold it; P4 follows D6; P5 needs about 7,000 forks a second to reach |
| D8 | the four adversary review records were recreated, before their first commit, with the operator's home directory written as `$HOME`; nothing else in them changed | `b10x-gates scan-text` refused them (`personal-paths`: 8, 4, 3 and 2 lines) and this repository is public; a review record is immutable, so the uncommitted file was removed and created again under the same id. The adversaries' texts as returned are kept in `<root>/coordinator/scratch/review-*.md`, and `diff` against the `.redacted` copies shows only those lines |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 | integration tree `plan-after-t1` at `046dfb1`, branch `wave/w2`; store plan written; approved by `adr-changes` |
| 2026-10-20 | opening commit `bf8deee` on `wave/w2` (bot); `b10x-gates check`: common checks passed, 1 commit scanned; `aep plan artifact validate`: valid. Unit trees `impl-contract-documents`, `impl-fs-glob`, `impl-toolset-process` at `bf8deee`, branches `impl/<story>`; briefs under `<root>/<unit>/scratch/brief.md`; three `aep:implementor` agents dispatched; free disk 66 G |
| 2026-10-20 07:50Z | the owner's licence decision, relayed by `adr-changes` and read in its transcript as typed: "pick a free licence, one which is dacor with most other libs we own". Every crate becomes Apache-2.0 (D3). The process implementor told mid-flight; its brief updated |
| 2026-10-20 | commit `d5614d8` on `wave/w2` (bot): the three moved crates, README, AGENTS.md, CHANGELOG, the contract-documents story's context; `b10x-gates check`: common checks passed, 2 commits scanned |
| 2026-10-20 | unit 1 (contract-documents) implementor green: 164→173 cases (a `tests/documents.rs` lane of 9), red run `E0432 unresolved import tool_contract::documents`; `build.rs` 108 lines, `src/documents.rs` 73; `Cargo.lock` unchanged; `target/` 1.3 G; 91,533 tokens, 36 tool uses, 6.5 min. Adversary dispatched. Free disk 43 G (other sessions building) |
| 2026-10-20 | unit 2 (fs-glob) implementor green: lib 28→30, red run 2 cases (`the walk visited 211 of 1,110 entries for one answered path`); after: 4 visits; conformance 70 of 70 and mutant counts unchanged; 92,017 tokens, 42 tool uses, 6.6 min. One answer changes: a capped glob that stops before reaching a directory past the depth bound answers `depth_bound_reached: false` (D4). CHANGELOG patch under its scratch. Adversary dispatched |
| 2026-10-20 | unit 3 (process) implementor green: 0→24 cases (unit 16, conformance 2 lanes over 22 scenarios, mutants 6: `NoProgramCheck`, `InheritAll`, `NoKill`, `NoGroupKill`, `NoOutputCap`, `NoTimeoutCap`, each failing 1 to 3 scenarios); 8 harness tests ported by name; `risk: high` as harness's run envelope; licence Apache-2.0 after D3; `Cargo.lock` +32; CHANGELOG and README patch under its scratch; 235,562 tokens, 84 tool uses, 19.2 min. Adversary dispatched |
| 2026-10-20 | unit 1 adversary pass 1: green, 173→178 cases (`tests/adversary_documents.rs`, 5), 0 red; 2 notes `INFEASIBLE`, `introduced` (F1: `build.rs:103` refuses names `validate` accepts; F2: `all()` byte order against a rustdoc that reads as version order); rebuilds in a path-dependency consumer checked for add, edit, rename and remove; 149,536 tokens, 51 tool uses, 17.9 min; ~740 M of consumer builds under its scratch. Recorded as `review-result:adversary-contract-documents-pass-1`; both findings filed as `story:contract-directory-names-validated` (draft), outcomes `no-op` (D5). Unit commit `794a179` (bot, gates check passed) |
| 2026-10-20 08:16Z | free disk on `/` 19 G then 17 G: builds in other sessions' trees (loom, llm, inference, ess, epistemic-knowledge-runtime). The wrapper killed the fs-glob adversary's test run at 24 G (exit 143) and refused the coordinator's verification run; reported to `b10x-chef-orchestrator`. Waiting for 28 G |
| 2026-10-20 | free disk back to 34 G. Unit 1 claim `VERIFIED`: the base has no `documents` module; on `794a179` `tests/documents.rs` 9 passed and `tests/adversary_documents.rs` 5 passed, exit 0, `cargo tree -e all` lists no implementation crate (`coordinator/scratch/verify-cd-treatment.log`); verification evidence recorded. Merged into `wave/w2` with `--no-ff` as `d14b7ac` (bot); the merged workspace `cargo test --workspace --locked` exit 0, 30 lanes ok; `target/` 848 M |
| 2026-10-20 | unit 2 adversary pass 1: green, 44→48 cases (`tests/adversary_w2_glob.rs`, 4, one over 150 seeded random trees at every cap), 0 red; 3 of the 4 red on a base copy, only on `depth_bound_reached`; 1 note `CONFIRMED`, `introduced` (J1: the story said the answer is unchanged); 166,847 tokens, 58 tool uses, 23.7 min; 365 M scratch copy. Recorded as `review-result:adversary-fs-glob-pass-1`; the story's Outcome and the CHANGELOG corrected by the coordinator, outcome `fixed`. Claim `VERIFIED`: 211 visits on the base (implementor's red run), the test asserting under 10 passes on the unit, `cargo test -p b10x-tool-fs` exit 0 (`coordinator/scratch/verify-fsglob-treatment.log`). Unit commit `692f992` (bot, gates check passed); merged as `d655d49`; merged workspace exit 0, 31 lanes ok |
| 2026-10-20 | unit 3 adversary pass 1: red, 24→33 cases, 8 red; 6 findings (2 `introduced`, 4 `pre-existing` from harness 0.14.1). The blocker (F1, pre-existing, `local.rs:604`, `:622-623`): once the shell exits, the output threads are joined with no deadline, so a background descendant holding the output keeps the call past its bound and the reply says `timed_out: false`; F2 a descendant that left the group holds the call; F3 `whole_prefix` after a non-UTF-8 byte; F4 an unenterable root answered as `SpawnFailed`; F5 only `PATH` asserted (covered by the adversary's green case); F6 `effects: [process]` for a runner that confines nothing but environment, time and output. 178,718 tokens, 68 tool uses, 16.7 min. Recorded as `review-result:adversary-toolset-process-pass-1`. Correction round 1 sent to the same implementor (D6) |
| 2026-10-20 | unit 3 correction round 1 green: 33→36 cases (unit 18, adversary 9, conformance 2 lanes over 28 scenarios, mutants 7), the adversary's 9 unedited; F1 the deadline runs until the program and both streams end, the group killed at the bound even after the shell was reaped; F2 output in shared state, never joined, at most 500 ms settle after the kill; F3 `whole_prefix` and `whole_suffix` look at 3 bytes at most; F4 `process.WorkspaceUnavailable` for an unenterable root; F6 `effects: [process, write, network, secret]`; a new scenario restores the `NoGroupKill` catch the F2 change had lost; 312,335 tokens, 136 tool uses, 13.7 min. Outcomes against pass 1: 5 `fixed`, 1 `no-op` (F5) written; the store kept 4 records (3 `fixed`, 1 `no-op`; `aep` 0.74.0 printed `on hand: review_outcome=1, 1, 2, 2, 3, 4` for the six writes); why two identical writes were not kept is not established. Adversary pass 2 (the last) sent to the same adversary |
| 2026-10-20 | unit 3 adversary pass 2: red, 36→40 cases, 2 new red cases; ledger (`aep plan artifact findings`): resolved 6, carried 0, new 5. P1 and P2: a stray non-UTF-8 byte at the head's end or the tail's start is moved and counted, against the sentence the correction added (`semantics.md:83`); P3 (warning): `bounds.yaml:48` and `tests.rs:365` pass against a runner that skips the kill after the shell ended (the adversary's new scenario catches it); P4: `consequences: []` beside an unconfined runner; P5 (`INFEASIBLE`): `killpg` by a reaped pid could reach a reused group. 252,384 tokens, 24 tool uses, 8.2 min; a 348 M mutant copy under its scratch. Recorded as `review-result:adversary-toolset-process-pass-2`. P5 filed as `story:process-kill-never-reaches-a-reused-group` (draft). Correction round 2 (P1 to P4) sent to the same implementor; the coordinator reads its diff, no third attack (D7) |
| 2026-10-20 ~10:55 local | operator's rule relayed by `b10x-chef-orchestrator` and present in the operator's agent instructions: disk space and accounts are the orchestrator's. The coordinator's periodic account check was cancelled and `cargo-serial.sh` replaced (by `mv`, so a running copy kept its file) with a version that keeps the lock and `sccache` and has no disk floor or watch |
| 2026-10-20 | `story:ess-domains-for-contracts` widened on the owner's direction (relayed by `b10x-chef-orchestrator`) to the profile, every toolset and the suites in ESS; `decision-blocker:ess-outcome-decided-outside-input` cleared with the ESS repository's answer, read in `beyond10x/ess` at `d11b350371` (reads are views; environment outcomes are `external:` branches) |
| 2026-10-20 | unit 3 correction round 2 green: 40→44 cases (unit 21, adversary 9, adversary_2 4, conformance 2 lanes over 31 scenarios, mutants 8); P1 and P2 keep 3 bytes beside each cut and move it only for a character it broke; P3 `Mutation::KillOnlyALiveProgram`, failing the adversary's scenario; P4 every profile consequence declared; 365,734 tokens, 180 tool uses, 33.0 min. Coordinator read the round's diff (`process/scratch/c2-round.diff`, 461 lines): no assertion removed, the ported tests changed only their call to the new `Cut` argument, the mutation reachable only through `mutated_by`; the adversary's six files byte-identical to the round's starting snapshot. Coordinator run: `cargo test -p b10x-tool-process --locked --no-fail-fast` exit 0 (21, 9, 4, 2, 8 passed), three documents validate. Outcomes against pass 2: 4 `fixed`, 1 `no-op` (P5), written a second apart, all five kept. Verification recorded. Unit commit `ae8527a` (bot, gates check passed); merged as `9fed5ee` |
| 2026-10-20 | coordinator commit `015f428` on `wave/w2`: `b10x-tool-process` takes `CONTRACT_YAML` from `tool_contract::documents::PROCESS_V1ALPHA1`, as the unit 3 brief left to the coordinator at merge; the merged workspace `cargo test --workspace --locked` exit 0, 37 lanes ok |

## Closing

### The gate (at `015f428`, with the closing docs and store edits in the tree)

| step | command | exit | what it printed |
|---|---|---|---|
| 1 | `aep plan artifact validate` | 0 | `valid` |
| 2 | `cargo test --workspace --locked` | 0 | 37 lanes `test result: ok`, 259 tests passed, 0 failed |
| 3 | `cargo fmt --all --check` | 0 | nothing |
| 4 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished` |
| 5 | `cargo +1.98.0 check --workspace --locked` | 0 | `Finished` |
| 6 | `cargo run --locked -p b10x-tool-contract -- validate contracts/` | 0 | `ok` for the `code-editor`, `fs` and `process` documents |

Logs: `<root>/coordinator/scratch/gate/`. No step skipped itself.

### Commits the wave made (all by the bot)

| sha | what |
|---|---|
| `bf8deee` | opening store commit |
| `d5614d8` | the licence change (D3) |
| `794a179` | unit 1, `impl/contract-documents-without-an-implementation` |
| `d14b7ac` | merge of unit 1 |
| `692f992` | unit 2, `impl/fs-glob-stops-at-the-cap` |
| `d655d49` | merge of unit 2 |
| `ae8527a` | unit 3, `impl/toolset-process` |
| `9fed5ee` | merge of unit 3 (`Cargo.lock` merged without conflict) |
| `015f428` | `tool-process` reads its contract from the embedded documents (coordinator, planned in the unit 3 brief) |

The closing store commit and the merge of `wave/w2` into `main` follow this page.

### What each unit cost (tokens are each agent's cumulative total; uses and minutes summed over its turns)

| unit | implementor | adversary | passes | corrections | findings |
|---|---|---|---|---|---|
| 1 contract-documents | 91,533 tokens, 36 uses, 6.5 min | 149,536 tokens, 51 uses, 17.9 min | 1 (0 red) | 0 | 2 notes, filed |
| 2 fs-glob | 92,017 tokens, 42 uses, 6.6 min | 166,847 tokens, 58 uses, 23.7 min | 1 (0 red) | 0 (store and changelog by the coordinator) | 1 note, fixed |
| 3 process | 365,734 tokens, 400 uses, 65.9 min | 252,384 tokens, 92 uses, 24.8 min | 2 (8 then 2 red) | 2 | 11 (6 + 5): 9 fixed, 1 no-op, 1 filed |
| total | 549,284 tokens | 568,767 tokens | 4 | 2 | 14 |

Sub-agent tokens in total: 1,118,051 over 6 agents and 9 turns. Tests at close: 259 in 37 lanes;
scenarios: 70 (`fs`), 68 (`code-editor`), 31 (`process`); mutants: 7 (`process`), each killed by the
suite. Review-results recorded: 4.

### Found along the way and filed, not fixed

- `story:contract-directory-names-validated` (draft): `validate` accepts directory names the build
  script refuses; `all()`'s order reads as version order.
- `story:process-kill-never-reaches-a-reused-group` (draft): after the shell is reaped, the deadline
  kill addresses its group by a pid that could be reused.
- Harness 0.14.1's own `exec` keeps the descendant-holds-the-call defect until harness moves to
  `tool-process`; reported to `b10x-chef-orchestrator`.
