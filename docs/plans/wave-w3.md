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
| D3 | `search.grep` declares `risk: low`, changed by the coordinator from the implementor's `none` before the adversary's pass (`contracts/search/v1alpha1/contract.yaml:145`) | it answers file contents as `fs.read` does, and `fs.read` declares `low`; at `none`, a grant capped at `none` would admit a search that reads what it may not `fs.read` |
| D4 | harness's answer names `skipped_large_files` and `skipped_large_paths` are kept; the story's acceptance line said `withheld` and is corrected | the implementor found both names in harness 0.13.3 and 0.14.1 with `git grep`; the story asked for harness's behaviour |
| D5 | `timeout_ms`, declared because the validator requires it and enforced by neither `tool-fs` nor `tool-search`, is filed as `story:declared-timeouts-are-enforced` | wave t1's D6 forbids declaring a bound nothing enforces; `fs` has done so since 0.1.0 (`grep -n timeout_ms crates/tool-fs/src/*.rs` finds nothing), so the fix covers both toolsets at once |
| D6 | the build-test executor judges each written path by where it lands (links resolved against the root), fixed in this unit's correction round | a `paths: [link/x.rs]` whose link leads outside the workspace is written today (implementor's report; pre-existing in harness 0.14.1); the code-editor toolset already judges by landing place, and the executor is code this unit owns |
| D7 | the build-test contract declares the placement this runner has (`local`), not `confined` | the executor applies no sandbox environment or mounts; a contract that says `confined` would admit it where only a confined run is allowed (wave w2's D6 reasoning for `process.run`) |
| D8 | the pre-existing F2 and F3 are fixed in this unit: an argument item beginning with `-` is refused for `string`, path and `enum` kinds; F4 declares every build-test operation as `process.run` is declared (`effects: [process, write, network, secret]`, every consequence, risk `high`) | F2 makes a declared read-only check write files without the write scope being asked; the fix is one rule in `validate_arguments`, recorded as a documented change to the moved registry. F4: build scripts, `npm` scripts and test binaries are arbitrary programs, and the runner confines nothing beyond environment, time, output and written paths |
| D9 | `search.grep` skips any entry named in `SKIPPED`, a file or a directory, and any file containing a NUL byte; `tool-fs`'s walk and `fs.glob` do not change | the released `fs` v1alpha1 says it skips "the directories `.git`, …" (`contracts/fs/v1alpha1/semantics.md:91`), so changing the shared walk would change a released answer; a worktree's `.git` file holds a host path (`gitdir: …`) a search has no reason to answer; the NUL rule makes the code match `semantics.md:65` |
| D10 | the build-test executor judges every path-kind argument by where it lands, read or written | the field description promises it; a read through a link can answer content from outside the workspace when a check prints what it read; the landing check exists already for written paths |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-20 | integration tree `wave-w3` at `60d4d0e`, branch `wave/w3`; both stories scoped onto `Cargo.lock` and moved to `active` |
| 2026-10-20 | opening commit `1813c93` on `wave/w3` (bot); `b10x-gates check`: common checks passed, 1 commit scanned. Unit trees `impl-build-test`, `impl-search` at `1813c93`, branches `impl/<story>`; briefs under `<root>/<unit>/scratch/brief.md`; two `aep:implementor` agents dispatched. Builds go through `b10x-build-slot` (orchestrator's rule of 2026-10-20) |
| 2026-10-20 | unit 2 (search) implementor green: `b10x-tool-fs` 48→55 (`tests/walk_api.rs`, 7), `b10x-tool-search` 0→29 (unit 20, conformance 2 lanes over 46 scenarios, mutants 7: six mutants, each failing 1 to 5 scenarios); red run 24 against a stub; `tool-fs` exports `pub mod walk` (`walk`, `Walked`, `contained`, `SKIPPED`, `MAX_WALK_DEPTH`, `PathGlob`), visibility, derives and docs only; four changes against harness (truncated only past the cap, empty glob refused, `../nowhere` outside, named pipes skipped); 334,242 tokens, 132 tool uses, 47.6 min. D3, D4, D5 taken; the contract validates and the conformance lane passes after D3. Adversary dispatched |
| 2026-10-20 | unit 1 (build-test) implementor green: 31→54 cases (lib 25→44, conformance 3 lanes over 35 scenarios, mutants over four: `no-empty-stdout-check` 1, `no-declared-programs` 28, `no-time-left` 1, `no-write-scope` 2 of 35); red runs 0 of 35 then green; a drift test holds `contract.yaml` (20 operations) equal to the derivation; the executor runs each argv through `b10x-tool-process` with one deadline per call; CI programs checked against the runner image's published toolset, not a CI run; 403,859 tokens, 152 tool uses, 50.4 min. Its open items: a written path through a symlink lands outside the workspace (pre-existing in harness), `placement: confined` with no sandbox applied, the program-set mutant one-directional. D6 and D7 taken. Adversary dispatched, told K1 and K2 are known |
| 2026-10-20 | unit 1 adversary pass 1: red, 54→60 cases, 8 red; 7 findings (4 `introduced`, 3 `pre-existing` from harness 0.14.1). F1 (K1) a link bypasses the write scope; F2 a path item `-w` turns the read-only `go_fmt_check` into a write; F3 a `package` value beginning with `-` becomes an undeclared option; F4 (K2) `placement: confined` on an unconfined run; F5 an argv started after the deadline; F6 the drift test cannot see a path parameter lose its kind; F7 the story's "refusal" against the executor's `build-test.CheckFailed`. 205,002 tokens, 68 tool uses, 31.9 min. Recorded as `review-result:adversary-build-test-pass-1`; F7 fixed in the story by the coordinator (outcome `fixed`); correction round 1 (F1 to F6) sent to the same implementor (D8) |
| 2026-10-20 | unit 2 adversary pass 1: red, 84→91 cases, 4 red (2 cases, plus the conformance and mutant lanes they hold); 5 findings: F1 (pre-existing) a `.git` file is searched though the guidance says `.git` is skipped; F2 (pre-existing, `INFEASIBLE`) a NUL-carrying UTF-8 file is searched against "a binary file is not searched"; F3 to F5 suite gaps (the 400/401 cut, glob before size, the guidance-figure test), closed by its own green cases (a 960-call harness oracle among them); `fs` reports identical per scenario at base and unit. 267,573 tokens, 93 tool uses, 47.5 min; about 588 M of scratch copies. Recorded as `review-result:adversary-search-pass-1`; F3 to F5 outcomes `no-op`; correction round 1 (F1, F2) sent to the same implementor (D9) |
| 2026-10-20 | unit 1 correction round 1 green: 60→67 cases, the adversary's 8 red now passing, conformance 43 of 43, mutants five (`no-landing` new, failing 2 of 43); F1 landing judged before any start; F2/F3 the dash rule, also on `string-list` (one placed by `each` with no separator reaches argv the same way); F4 every operation `local`, `[process, write, network, secret]`, all consequences, `high`; F5 no start after the deadline; F6 path kinds named; wave t1's unit tests that pinned the old profile values updated. Deviation: the brief's `cargo fmt -p b10x-tool-toolchain` reformatted the adversary's `tests/adversary_executor.rs` and no copy of the original was kept; pass 2 is asked to confirm the change is layout only. 511,581 tokens cumulative, 72 tool uses and 53.9 min this round. Outcomes against pass 1: 6 `fixed`. Adversary pass 2 (the last) sent to the same adversary |
| 2026-10-20 | unit 2 correction round 1: F1 a file named in `SKIPPED` not searched unless `path` names it, in `tool-search` only; F2 a NUL-carrying file skipped as binary; mutants `no-skipped-file-names`, `nul-is-text`; conformance 51 of 51; red 1: the adversary's harness oracle still expected harness's answer for a NUL file. The coordinator applied the implementor's patch to that case (the oracle stops generating the NUL file, keeps the random draw, lists the change; original kept at `coordinator/scratch/adversary_harness_oracle.rs.before-patch`), the skill's row for a case a decision changed under; coordinator run `cargo test -p b10x-tool-search -p b10x-tool-fs --no-fail-fast` exit 0, every lane ok. 410,910 tokens cumulative, 57 tool uses and 48.6 min this round. Outcomes F1, F2 `fixed`. Adversary pass 2 (the last) sent to the same adversary |
| 2026-10-20 | unit 1 adversary pass 2: red, 64→66 cases, 1 red; ledger resolved 7, carried 0, new 2. The reformatting of its pass-1 file confirmed layout only (three calls split over lines). P1: `go_fmt_check.paths` is described as refused when it leaves the workspace, and a read path through a link is read; P2: the go summaries say network and checksum lookup are disabled while the provider's environment is not applied. The implementor's round-1 count of 67 did not match the 64 the adversary ran. 255,302 tokens cumulative, 18 tool uses and 7.8 min this pass. Recorded as `review-result:adversary-build-test-pass-2`; P2 filed as `story:build-test-applies-provider-environment` (draft), outcome `no-op`; correction round 2 (P1) sent to the same implementor (D10); the coordinator reads its diff, no third attack |
| 2026-10-20 ~12:00Z | both agents in flight (build-test correction round 2, search adversary pass 2) stopped on HTTP 429 ("session limit · resets 3pm"); the orchestrator switched the account; both resumed with their context, told to check their tree state first |
| 2026-10-20 ~12:15Z | a second HTTP 429 ("resets 4:40pm") stopped both again; the orchestrator switched the account; both resumed |
| 2026-10-20 | unit 1 correction round 2 green: every `workspace-path` and `workspace-path-list` value judged where it lands, read or written, before anything starts; mutant `no-read-landing`; conformance 44 of 44; the brief's `cargo fmt` re-laid out the adversary's pass-2 additions again. Coordinator read `path_arguments` (`executor.rs` 302-330) and the check (504-512): read paths through `lands`, written paths through the written-path judge; no `#[ignore]` in any test file; coordinator run `cargo test -p b10x-tool-toolchain --no-fail-fast` exit 0 (49, 6, 8, 3, 1 passed), five documents validate. Outcomes against pass 2: P1 `fixed`, P2 `no-op` (filed). Verification recorded |
| 2026-10-20 | unit 2 adversary pass 2: green, 96→100 cases, 0 red; confirmed the coordinator's oracle patch did only what was said; 4 notes: three suite gaps (a NUL past 8,000 bytes, a submodule's `.git` file below the root, the two binary rules over 1 MiB) closed by its `adversary_pass2.rs` and `adversary-pass2.yaml`; one sentence (`semantics.md:69`, guidance) said a binary file is not reported while one over 1 MiB is counted as too large. 319,616 tokens cumulative. The coordinator reworded both sentences (kept "names the first 20." whole, which the adversary's guidance test pins); coordinator run `cargo test -p b10x-tool-search -p b10x-tool-fs --no-fail-fast` exit 0, 15 lanes, four documents validate. Outcomes: 3 `no-op`, 1 `fixed`. Verification recorded |
| 2026-10-20 | unit commits `8fd6532` (build-test) and `3aea79a` (search), bot, gates check passed; merged as `5663fb2` and `aad0a22`. `Cargo.lock` merged without a conflict; the merge message of `aad0a22` says it was regenerated, which is wrong (the bot route has no `notes` verb to correct it, so this row does). Merged workspace `cargo test --workspace --locked` exit 0, 49 lanes, 347 tests |

## Closing

### The gate (at `aad0a22`, with the closing docs and store edits in the tree)

| step | command | exit | what it printed |
|---|---|---|---|
| 1 | `aep plan artifact validate` | 0 | `valid` |
| 2 | `cargo test --workspace --locked` | 0 | 49 lanes `test result: ok`, 347 tests passed, 0 failed |
| 3 | `cargo fmt --all --check` | 0 | nothing |
| 4 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished` |
| 5 | `cargo +1.98.0 check --workspace --locked` | 0 | `Finished` |
| 6 | `cargo run --locked -p b10x-tool-contract -- validate contracts/` | 0 | `ok` for `build-test`, `code-editor`, `fs`, `process` and `search` |

Logs: `<root>/coordinator/scratch/gate/`. No step skipped itself.

### Commits the wave made (all by the bot)

| sha | what |
|---|---|
| `1813c93` | opening store commit |
| `8fd6532` | unit 1, `impl/build-test-contract` |
| `3aea79a` | unit 2, `impl/toolset-search` |
| `5663fb2` | merge of unit 1 |
| `aad0a22` | merge of unit 2 (`Cargo.lock` merged without a conflict) |

The closing store commit and the merge of `wave/w3` into `main` follow this page. No release is
part of this wave's approval.

### What each unit cost (tokens are each agent's cumulative total; uses and minutes summed over the turns that reported them; the turns stopped by HTTP 429 reported none)

| unit | implementor | adversary | passes | corrections | findings |
|---|---|---|---|---|---|
| 1 build-test | 548,265 tokens, 226 uses, 105.3 min reported | 255,302 tokens, 86 uses, 39.7 min | 2 (8 then 1 red) | 2 | 9 (7 + 2): 8 fixed, 1 filed |
| 2 search | 410,910 tokens, 189 uses, 96.2 min | 319,616 tokens, 96 uses, 48.9 min reported | 2 (4 then 0 red) | 1 | 9 (5 + 4): 3 fixed, 6 no-op |
| total | 959,175 tokens | 574,918 tokens | 4 | 3 | 18 |

Sub-agent tokens in total: 1,534,093 over 4 agents. Tests at close: 347 in 49 lanes; scenarios:
44 (`build-test`), 53 (`search`); mutants: 6 (`build-test`), 8 (`search`), each killed by the suite.
Review-results recorded: 4.

### Found along the way and filed, not fixed

- `story:declared-timeouts-are-enforced` (draft): `fs` and `search` declare a `timeout_ms` nothing
  enforces.
- `story:build-test-applies-provider-environment` (draft): the go summaries say network and
  checksum lookup are disabled while the provider's environment is not applied.
