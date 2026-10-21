---
format: aep.planning-md/3
id: review-result:adversary-conformance-runner-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:conformance-runner-hosts-and-sequences: 3 red, the schema check builds the toolset with an empty host, 7 findings'
relations:
- reviews: story:conformance-runner-hosts-and-sequences
revision: 1
---
unit: story:conformance-runner-hosts-and-sequences, working tree at base `1393b0e` plus uncommitted and untracked changes in `$HOME/.local/state/worktree/trees/b10x/tools/impl-conformance-runner`
verdict: CONFIRMED (blocker 1: the schema check builds the toolset with an empty `host`)
cases: executed 418→424, red 3
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths under `$HOME/.cache/tools-wave-w4/conformance-runner/scratch/adversary/` (`copy/`, the mutation copy; `suite.log`)
needs-coordinator: none

**1. Diff proof.** `git --no-pager diff --stat` is the implementor's two files
(`contracts/profile/tool-contract-1.md` 83, `crates/tool-contract/src/conformance.rs` 619; 602
insertions, 100 deletions). The adversary added one untracked test file,
`crates/tool-contract/tests/adversary_hosts.rs`, and changed no implementation file.

**2. Cases in `crates/tool-contract/tests/adversary_hosts.rs`**

| case | asserts | now |
|---|---|---|
| `a_factory_needing_a_host_key_every_scenario_provides_passes` | a factory answers `Missing::Setting("allow")` when `allow` is absent from `host`; every scenario sets it, so the run passes | red |
| `two_routes_sharing_one_request_log_are_refused_at_parse` | routes `/a/b` and `/a_b` both log to `.serve/_a_b.jsonl`, so parse refuses them | red |
| `a_chunked_request_body_is_not_silently_logged_empty` | a chunked POST body is logged as `hello` or answered `501` | red |
| `a_program_missing_for_one_scenario_leaves_no_other_scenario_counted` | `Missing` from one scenario's factory call fails the other scenario too | green, kills M2 |
| `the_lines_matcher_counts_exactly_at_its_boundaries` | `{$lines}` on an empty file (0), `a\nb` (2), `a\n` is not 2, `a\nb\nc` is not 2, `a\n\n` (2) | green, kills M1 |
| `a_delayed_answer_does_not_hold_its_scenario_past_the_end` | route `delay_ms: 30000`, the client gives up after 200 ms, the scenario ends in under 5 s | green, kills M3 |

Red output (`cargo-serial.sh test --locked -p b10x-tool-contract --test adversary_hosts`):

```
---- a_factory_needing_a_host_key_every_scenario_provides_passes stdout ----
missing: setting allow
FAIL allow-is-given: not counted: the run is missing setting allow
0 of 1 scenarios passed, 1 missing
---- two_routes_sharing_one_request_log_are_refused_at_parse stdout ----
routes /a/b and /a_b both log to .serve/_a_b.jsonl and the suite parsed
---- a_chunked_request_body_is_not_silently_logged_empty stdout ----
answered 200, logged body Some(String(""))
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
```

Mutants, applied to a copy in scratch and run against `hosts_and_sequences` and `adversary_hosts`:

| mutant | where | unit's lane | adversary file |
|---|---|---|---|
| M1 `count == *lines` to `<=` | `conformance.rs:1409` | `16 passed; 0 failed` | lines case FAILED |
| M2 the not-counted pass skipped (`filter(\|_\| false)`) | `conformance.rs:848` | `16 passed; 0 failed` | missing-for-one case FAILED |
| M3 stop check in the delay loop removed (`if false`) | `conformance/serve.rs:218` | `16 passed; 0 failed` | delay case FAILED, "the scenario took 30.001260108s" |

**3. Suite run.** `cargo-serial.sh test --workspace --locked --no-fail-fast`: `EXIT=101`, 55
`test result:` lines, 421 passed, 3 failed, the only failing line `adversary_hosts`; every released
suite's lane `ok`.

**4. Attacked and not broken.** `{{calls.1...}}` naming a later or missing entry fails the
scenario; a substituted value is not substituted again and keys are not substituted; `Missing`
never counts as passed; released suites unchanged; a client that connects and never sends delays the
scenario's end by at most `IDLE` (5 s) and its thread is joined; `{$lines}` with `\r\n` and without
a final newline counts correctly; a delayed route stops when the scenario ends.

```findings
- file: crates/tool-contract/src/conformance.rs
  line: 1429
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: check_schemas builds the toolset with an empty host, so a factory reporting Missing::Setting for an absent host key, as the contract allows, fails every run even when every scenario provides the key
- file: crates/tool-contract/src/conformance/serve.rs
  line: 241
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: routes /a/b and /a_b share .serve/_a_b.jsonl and the suite parses, so one route's request log counts the other route's requests
- file: crates/tool-contract/src/conformance/serve.rs
  line: 289
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a chunked request body is answered 200 and logged as an empty body instead of being decoded or refused with 501
- file: crates/tool-contract/src/conformance.rs
  line: 1409
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the lines matcher mutated from == to <= survives hosts_and_sequences (16 passed) and is killed by the_lines_matcher_counts_exactly_at_its_boundaries
- file: crates/tool-contract/src/conformance.rs
  line: 848
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: skipping the not-counted pass survives hosts_and_sequences because its factory is Missing for every scenario, and is killed by a_program_missing_for_one_scenario_leaves_no_other_scenario_counted
- file: crates/tool-contract/src/conformance/serve.rs
  line: 218
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: removing the stop check in the delay loop survives hosts_and_sequences and is killed by a_delayed_answer_does_not_hold_its_scenario_past_the_end (30.0 s against a 5 s bound)
- file: crates/tool-contract/src/conformance.rs
  line: 1059
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a misspelled placeholder such as {{calls0.output.id}} or {{ serve }} reaches the toolset as literal text with no failure, as the spec's "left as written" rule allows
```
