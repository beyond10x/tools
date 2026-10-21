---
format: aep.planning-md/3
id: review-result:adversary-conformance-runner-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:conformance-runner-hosts-and-sequences: 1 red, the schema check falls back to an empty host when the first scenario fails early, 4 findings'
relations:
- reviews: story:conformance-runner-hosts-and-sequences
revision: 1
---
unit: story:conformance-runner-hosts-and-sequences, working tree at base `1393b0e` after correction round 1 (pass 2)
verdict: CONFIRMED (warning 1: the schema check falls back to an empty `host` when the first scenario fails before its factory runs)
cases: executed 424→428, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths under `$HOME/.cache/tools-wave-w4/conformance-runner/scratch/adversary/` (`copy/`, refreshed from the worktree; `suite-2.log`; `helpers.rs.txt`; `tests2.rs.txt`)
needs-coordinator: none

**1. Diff proof.** `git --no-pager diff --stat`: `contracts/profile/tool-contract-1.md` 93,
`crates/tool-contract/src/conformance.rs` 639; 630 insertions, 102 deletions. The adversary added
one untracked test file, `crates/tool-contract/tests/adversary_hosts_2.rs`, and left
`adversary_hosts.rs` and every implementation file alone.

**2. Cases in `crates/tool-contract/tests/adversary_hosts_2.rs`**

| case | asserts | now |
|---|---|---|
| `a_first_scenario_failing_before_its_factory_does_not_make_the_run_miss_a_setting` | the first scenario fails in `host` (`{{serve}}` with no routes); every `host` has `allow`, so `missing` is empty and the second scenario passes | red |
| `the_schema_check_builds_with_the_first_scenarios_host_and_workspace` | the schema build gets the first scenario's `allow`, its workspace, and no `served` | green, kills M4 |
| `transfer_encoding_identity_is_answered_501_and_logged_with_a_null_body` | `Transfer-Encoding: identity` with `Content-Length` is answered 501 and logged with `"body": null` | green |
| `a_501_answer_closes_promptly_and_leaves_nothing_running` | a chunked POST scenario ends in under 1.5 s | green, kills M5 |

Red output (`cargo-serial.sh test --locked -p b10x-tool-contract --test adversary_hosts_2`):

```
missing: setting allow
FAIL first-fails-in-host: host: {{serve}} needs routes in fixture.serve
FAIL second-passes: not counted: the run is missing setting allow
0 of 2 scenarios passed, 1 missing
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Mutants, in a copy refreshed from the worktree (M3 of pass 1 gone; `diff -r` against the worktree
`src` empty after the runs):

| mutant | where | `hosts_and_sequences` | `adversary_hosts` | `adversary_hosts_2` |
|---|---|---|---|---|
| M4 keep every scenario's host (`if true`) | `conformance.rs:835` | 16 ok | 6 ok | schema-host case FAILED (`left: Some("second")`) |
| M5 drain without `shutdown(Write)` | `conformance/serve.rs:369` | 16 ok | 6 ok | 501 case FAILED, "took 3.010032856s" |
| M6 no `drain` call | `conformance/serve.rs:241` | 16 ok | 6 ok | only the red case fails, so M6 survives |

**3. Suite run.** `cargo-serial.sh test --workspace --locked --no-fail-fast`: `EXIT=101`, 56
`test result:` lines, 427 passed, 1 failed (the red case). clippy `-D warnings` `Finished`; fmt
`--check` exit 0.

**4. Ledger.** Pass 1: F1 resolved (its fallback is N1), F2 resolved, F3 resolved, M1–M3 resolved
(the pass-1 cases are in the unit's lanes), J1 carried as decided. New: N1 the empty-host fallback
(`conformance.rs:852`), N2 the `drain` call observed by no test, N3 the 400, 404, 413 and 431
answers close without draining an unread body (`conformance/serve.rs:227-237`, code read only).

**5. Attacked and not broken.** A `calls` scenario first; `{{serve}}` in the first scenario's host
(the schema build gets the stopped server's URL with `served: None`, as the profile says); a first
scenario's factory reporting `Missing` while a later one would not (the run fails, as specified);
routes `/a` and `/a/` (distinct logs); routes differing by case (collide only on a case-insensitive
filesystem; CI is `ubuntu-latest`, `.github/workflows/check.yml:18`); `Transfer-Encoding: identity`;
the 501 path ends at once.

```findings
- file: crates/tool-contract/src/conformance.rs
  line: 852
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: when the first scenario fails before its factory runs, the schema check builds with an empty host, so the run reports a setting no scenario lacks and marks a passing scenario not counted
- file: crates/tool-contract/src/conformance/serve.rs
  line: 241
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: removing the drain call survives every lane, because no deterministic case produces the reset it prevents
- file: crates/tool-contract/src/conformance/serve.rs
  line: 227
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the 400, 404, 413 and 431 answers close without draining an unread request body, so a client can see a reset instead of the status
- file: crates/tool-contract/src/conformance.rs
  line: 1059
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: carried from pass 1 as decided by the coordinator, a misspelled placeholder reaches the toolset as literal text with no failure
```
