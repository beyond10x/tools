# Wave w4: the process-group kill, declared timeouts, the conformance runner's host keys

Run 2026-10-21 by the `tools` session under `aep:implementing` 0.23.2 (wave mode). Proposed by the
`tools` session and approved by `b10x-chef-orchestrator` under exception 6: "Wave approved under
exception 6: units 1 → 2 → 3, serial, one sub-agent at a time." Unit 1 first because harness pins
its fix.

## Units

| unit | story | branch | scope confidence |
|---|---|---|---|
| 1 | `story:process-kill-never-reaches-a-reused-group` | `impl/process-kill-never-reaches-a-reused-group` | cited |
| 2 | `story:declared-timeouts-are-enforced` | `impl/declared-timeouts-are-enforced` | cited |
| 3 | `story:conformance-runner-hosts-and-sequences` | `impl/conformance-runner-hosts-and-sequences` | inferred |

Serial (N = 1 at a time, the approver's rule): each unit forks from `wave/w4` after the previous
unit merged.

Not in this wave: `story:build-test-applies-provider-environment` (same file as unit 1,
`crates/tool-process/src/lib.rs`), `story:contract-directory-names-validated` (same file as unit 3,
`contracts/profile/tool-contract-1.md`), `story:ess-domains-for-contracts`, `story:docs-site`, and
the web-toolsets stories after unit 3 (`story:connector-projection` and `story:toolset-tool-meta`
wait on an ADR question with the operator).

## What `aep plan artifact waves --kind story --status draft` printed (2026-10-21, before the moves)

```
wave 1
  story:build-test-applies-provider-environment
  story:conformance-runner-hosts-and-sequences (inferred)
wave 2
wave 3
  story:profile-declares-ports (inferred)
...
```

(full output in the `tools-plan-web-toolsets` session report; the three units share no surface:
unit 1 `crates/tool-process/src/lib.rs`; unit 2 `contracts/fs`, `contracts/search`,
`crates/tool-fs/src/{lib,walk}.rs`, `crates/tool-search/src/lib.rs`; unit 3
`contracts/profile/tool-contract-1.md`, `crates/tool-contract/src/conformance.rs`,
`crates/tool-contract/tests`.)

## Triples

Wave root: `$HOME/.cache/tools-wave-w4`. Managed trees under
`$HOME/.local/state/worktree/trees/b10x/tools/`, each building into its own `target/`.
`<root>/bin/cargo-serial.sh` runs one `cargo` at a time through `b10x-build-slot`, with `sccache`.

| unit | managed id | worktree | build dir | scratch |
|---|---|---|---|---|
| coordinator | `wave-w4` | `<trees>/wave-w4` (branch `wave/w4`) | `<tree>/target` | `<root>/coordinator/scratch` |
| 1 | `impl-process-kill` | `<trees>/impl-process-kill` | `<tree>/target` | `<root>/process-kill/scratch` |
| 2 | `impl-declared-timeouts` | `<trees>/impl-declared-timeouts` | `<tree>/target` | `<root>/declared-timeouts/scratch` |
| 3 | `impl-conformance-runner` | `<trees>/impl-conformance-runner` | `<tree>/target` | `<root>/conformance-runner/scratch` |

## Pre-flight (2026-10-21)

| check | value |
|---|---|
| base | `main` at `2363756` (plan commit after tag `0.4.0`) |
| free disk | 51G on `/` (`df -h /`), build-slot floor 20G |
| other tools trees | `tools-plan-web-toolsets` (merged plan branch, kept until this wave closes) |
| dispatch | `aep:implementor`, `aep:adversary` (plugin agents), one at a time |

## Commits this approval covers

1. The opening store commit on `wave/w4` (moves to `active`, this page, the `AGENTS.md` line).
2. One commit per unit on its `impl/*` branch, by the coordinator through the bot route.
3. The merge of each green unit into `wave/w4`.
4. The closing store commit.
5. The merge of `wave/w4` into `main` and its push.

The release after the wave is approved separately by the same message ("After the wave, release on
the newest ess").

## Decisions taken by the coordinator

| id | decision | reason |
|---|---|---|
| D2 | unit 2 enforces the declared `timeout_ms` in new versions `fs` v1alpha2 and `search` v1alpha2 (optional input `timeout_ms`, output `deadline_reached`), and the two crates move their claim to v1alpha2; v1alpha1 stays in `contracts/` unchanged | a released v1alpha1 is never edited (`AGENTS.md` § Rules), and saying the deadline hit needs an answer field; no consumer depends on the crates yet (`grep b10x-tool-` over the harness and metaharness manifests on 2026-10-21: nothing) |
| D3 | in unit 2 the code changes, not the documents: a path, pattern and glob refusal comes before the `timeout_ms` refusal, and a deadline cut inside a multi-byte character ends the window on a whole character | both `semantics.md` already say refusals come first, and that is the refusal a governor reads (adversary pass 1, findings 1, 2, 4) |
| D4 | unit 2's two pass-2 findings (CRLF at the 16 MiB counting bound, one `introduced`, one `pre-existing`) are fixed in a second correction round; the coordinator reads the diff, no third attack | both are one-line changes in `crates/tool-fs/src/line.rs`, a file the unit already owns; the pre-existing one would otherwise leave a red adversary case on the branch |
| D5 | unit 3's correction round 1 fixes F1–F3: the schema check builds its toolset with the first scenario's context; colliding request-log names are refused at parse; a request with `Transfer-Encoding` is answered 501 and logged with `"body": null`. An unknown `{{...}}` stays as written | F1 makes `Missing::Setting` for a host key unusable through the documented `run_with`; F2 lets one route's `{$lines}` count another's requests; a literal `{{` can be payload a toolset needs |
| D6 | unit 3's pass-2 findings N1 (empty-host fallback when the first scenario fails before its factory) and N3 (the 400, 404, 413 and 431 answers close without draining) are fixed in a second correction round; the schema check takes the first scenario whose factory ran; the coordinator reads the diff, no third attack; N2 (the drain observed by no test) is `no-op` | N1 makes a failing suite name the wrong cause and leaves a red adversary case on the branch; N3 is the same close path the 501 already uses; the adversary budget is 2 passes |
| D1 | unit 1 reaps the shell after the group kill and before the settle, not after the settle as the brief said | the settle waits until the group is gone, and an unreaped shell is still a member, so the brief's order always waited the full 500 ms (703 ms against 206 ms on the unit's test, implementor's measurement); no signal is sent after the reap, so the kill can still never name a reused group |

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-21 | integration tree `wave-w4` at `2363756`, branch `wave/w4`; the three stories moved `draft → proposed → active` |
| 2026-10-21 | opening commit `5ac57a7` (bot, hook checks passed); unit tree `impl-process-kill` at `5ac57a7`, branch `impl/process-kill-never-reaches-a-reused-group`; `aep:implementor` dispatched |
| 2026-10-21 | unit 1 implementor green: 44→46 cases, red run `left: Some(Err(ECHILD))` before the fix; `waitid(…WNOWAIT)` through `nix` (no new dependency); `Mutation::ReapBeforeKill` outside `Mutation::ALL`, caught by a unit test; conformance 31 of 31 and the 7 mutant lanes identical to the base (`diff` empty); 84,325 tokens, 36 tool uses, 8.8 min. D1 taken. `aep:adversary` dispatched |
| 2026-10-21 | unit 1 adversary pass 1: no red case, 46→48 cases, 2 notes (`introduced`): a program dead by its own signal and one that stops itself had no case; its `tests/adversary_3.rs` pins both, each red against its mutant in a scratch copy; 80,333 tokens, 26 tool uses, 9.2 min; recorded as `review-result:adversary-process-kill-pass-1`, outcomes 2 `no-op`. Coordinator run `cargo test -p b10x-tool-process --locked --no-fail-fast` exit 0 (7 lanes); verification recorded (base red `left: Some(Err(ECHILD))`, treatment green). Unit commit `e046841`, merge `ae027db` (bot). CHANGELOG `Unreleased` § Fixed line written |
| 2026-10-21 | unit tree `impl-declared-timeouts` at `ae027db`; D2 taken; `aep:implementor` dispatched |
| 2026-10-21 ~04:15Z | unit 2 implementor stopped on HTTP 429 ("session limit · resets 6:20am"), mid-change (`stat` and `digest_file`); the orchestrator switched the account; resumed with its context, told to check its tree state first |
| 2026-10-21 | unit 2 implementor green: 100→120 cases, red 2 lanes (`unknown field timeout_ms`, schema `deadline_reached` absent); conformance v1alpha2 fs 85 of 85, search 59 of 59; `ignore-deadline` fails 5 of 85 and 2 of 59; validate `ok` for 7 documents; v1alpha1 and `crates/tool-contract` untouched; build script picked up the new directories unedited. Its calls: above-max refused, not capped; refusals win at `timeout_ms: 0`; a deadline-stopped `fs.read` answers an empty window; `FsError`/`SearchError` gain `TimeoutAboveMax` (breaking). It reported running three commands that fed stdin, against the machine rule; all returned at once. 244,345 tokens, 76 tool uses, 26.0 min (both turns). `aep:adversary` dispatched |
| 2026-10-21 | unit 2 adversary pass 1: NEEDS-CHANGE, 120→130 cases, 8 red (3 cases, 5 suite lanes); 4 findings (`introduced`): the `timeout_ms` refusal comes before the path and pattern refusals in both crates against `semantics.md`; no real-clock case (a seconds-for-milliseconds mutant kept 120 green; its 1 ms cases catch it); a deadline cut mid-character leaves U+FFFD (INFEASIBLE through the public API); 147,197 tokens, 43 tool uses, 9.4 min. Recorded as `review-result:adversary-declared-timeouts-pass-1`; finding 3 outcome `no-op`; D3 taken; correction round 1 (findings 1, 2, 4) sent to the same implementor |
| 2026-10-21 | unit 2 correction round 1 green: 130→133 cases, red 9 before the fix; every input refusal before `timeout_ms` (`judge_timeout` after the last refusal of each fs operation; grep after pattern, glob and `resolve`); `tests.rs:710` re-pinned to the new order; `whole_prefix` on any line not held whole, which also fixes the same U+FFFD at the 16 MiB counting cut it found; `the_other_adversary_scenarios_hold` counts 21 + 3; conformance fs 88 of 88, search 62 of 62; 292,508 tokens cumulative, 37 tool uses and 11.9 min this round. Outcomes 3 `fixed`. Adversary pass 2 (the last) sent to the same adversary |
| 2026-10-21 | unit 2 adversary pass 2: 123→137 cases, scenarios fs 88→116, search 62→74; 2 red, both notes on CRLF at the 16 MiB bound (`line.rs:77` introduced by round 1, `line.rs:109` pre-existing at `ae027db`); ledger carried 0, new 2, resolved 4; all 40 refusal-order combinations pass; 186,947 tokens, 25 tool uses, 4.5 min. Recorded as `review-result:adversary-declared-timeouts-pass-2`; D4 taken; correction round 2 sent |
| 2026-10-21 ~07:00Z | unit 2 correction round 2 stopped on HTTP 429 ("resets 9:10am") before its red run; the orchestrator switched the account; resumed with its context |
| 2026-10-21 | unit 2 correction round 2 green: 137→138 cases, red 2 (both pass-2 cases); `after_the_bound` in `line.rs` reads what follows the bound (end, `\n`, `\r\n` in one or two buffers, other) and drops a `\r` the bound took before `\n`; conformance fs 116 of 116, search 74 of 74; 309,315 tokens cumulative, 7 tool uses this turn. Coordinator read the round's `line.rs` diff (only the bound branch and the new `after_the_bound`), no `#[ignore]` in either crate, run `cargo test -p b10x-tool-fs -p b10x-tool-search --locked --no-fail-fast` exit 0 (18 lanes ok). Outcomes 2 `fixed`; verification recorded. Unit commit `58d227e`, merge `1393b0e` (bot). CHANGELOG § Added, § Changed, § Fixed written |
| 2026-10-21 | unit tree `impl-conformance-runner` at `1393b0e`; `aep:implementor` dispatched (its gate runs the whole workspace's tests, since every crate uses the runner) |
| 2026-10-21 | unit 3 implementor green: 402→418 workspace tests (54 lanes, 0 failed, `gate-test.log`), new lane `hosts_and_sequences` 16; clippy, fmt, validate exit 0; mutant `runner drops host` fails the new lane (exit 101). Its calls: `holds`/`expect` at scenario level on a `calls` scenario refused at parse; `Scenario.call` holds the first entry; `body_file` relative to the contract directory; `Report.missing` added. Coordinator session respawned; `aep:adversary` pass 1 dispatched |
| 2026-10-21 | unit 3 adversary pass 1: CONFIRMED (blocker), 418→424 cases, 3 red; 7 findings (`introduced`): the schema check builds the toolset with an empty `host` (F1, blocker); routes `/a/b` and `/a_b` share one request log (F2); a chunked body is logged empty and answered 200 (F3, INFEASIBLE); 3 mutants the unit lane missed, each killed by an adversary case; an unknown placeholder stays as written (J1); 108,049 tokens, 28 tool uses, 8.5 min. Recorded as `review-result:adversary-conformance-runner-pass-1`; 4 outcomes `no-op` (M1–M3, J1); D5 taken; correction round 1 (F1–F3) sent to the same implementor |
| 2026-10-21 | unit 3 correction round 1 green for F1–F3 (424 cases, red 3 before: the adversary cases); 69,164 tokens, 55 tool uses, 6.0 min. Its gates 2 and 3 failed only in the adversary file (clippy `format_push_string` at `adversary_hosts.rs:323`, 7 rustfmt hunks); the coordinator changed that file to `write!` and ran `cargo fmt -p b10x-tool-contract`, no assertion changed. Coordinator runs: fmt `--check` exit 0, clippy `-D warnings` exit 0, `test --workspace --locked --no-fail-fast` exit 0, 424 passed in 55 lanes. Outcomes 3 `fixed`. Session restarted on opus-5-5 by the orchestrator; adversary pass 2 (the last) sent to the same adversary |
| 2026-10-21 | unit 3 adversary pass 2: CONFIRMED (warning), 424→428 cases, 1 red; ledger: F1–F3 and M1–M3 resolved, J1 carried; new N1 empty-host fallback (`conformance.rs:852`), N2 `drain` unobserved (INFEASIBLE), N3 error answers close without draining (INFEASIBLE); 2 more mutants killed (M4, M5); 140,311 tokens, 16 tool uses, 5.5 min. Recorded as `review-result:adversary-conformance-runner-pass-2`; outcomes J1, N2 `no-op`; D6 taken; correction round 2 (N1, N3) sent |
| 2026-10-21 | unit 3 correction round 2 green: 428 cases, red 1 before (the N1 case); the schema check takes the first scenario whose factory was called (`type Reached`), an unreached suite builds with an empty host and drops that build's `Missing`; `drain` after 400, 404, 413 and 431 (408 excepted); adversary files untouched (sha256 equal); 88,844 tokens, 19 tool uses, 2.5 min. Coordinator read the round (`conformance.rs:826-872,940-947`, `conformance/serve.rs:220-246,373`), no `#[ignore]` in `crates/tool-contract`; runs `test --workspace --locked --no-fail-fast` exit 0, 428 passed in 56 lanes, fmt `--check` exit 0, `clippy --workspace --all-targets -D warnings` exit 0. Outcomes N1, N3 `fixed`; verification recorded. Unit commit `da2a135`, merge `23e6a47` (bot). CHANGELOG § Added written. Hold (orchestrator): no merge to `main`, no push of `main`, no release |
| 2026-10-21 | wave-close gate on `wave/w4` at `23e6a47`: `aep plan artifact validate` valid; `test --workspace --locked --no-fail-fast` exit 0, 428 passed, 0 failed in 56 lanes; `fmt --all -- --check`, `clippy --workspace --all-targets --locked -- -D warnings`, `+1.98.0 check --workspace --locked` and `tool-contract validate` (7 ok) exit 0. Scope of the 3 stories rewritten to the files that landed (cited); a `test_result` each; all 3 moved to `implemented`. New drafts `story:private-adr-citations` (blocks any merge to `main`) and `story:fs-climb-through-missing-refused`. Closing store commit on `wave/w4`; commit 5 (merge into `main`, push) and the release held by the orchestrator |
