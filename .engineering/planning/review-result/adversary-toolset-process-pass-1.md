---
format: aep.planning-md/3
id: review-result:adversary-toolset-process-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:toolset-process: 8 red cases, a descendant holds the call past its bound (pre-existing in harness), 6 findings'
relations:
- reviews: story:toolset-process
revision: 1
---
```
unit: story:toolset-process
verdict: red
cases: executed 24→33, red 8
origin: introduced 2, pre-existing 4, undecided 0
wrote-outside-worktree: 10 logs in $HOME/.cache/tools-wave-w2/process/scratch/adversary/
needs-coordinator: yes
```

Findings cover the uncommitted tree on `impl/toolset-process` (base bf8deee) at $HOME/.local/state/worktree/trees/b10x/tools/impl-toolset-process. The acceptance statement fails: a run whose shell has already exited can run past its bound with a descendant still alive, and the reply says `timed_out: false`.

**1. `git --no-pager diff --stat`**
```
 Cargo.lock | 32 ++++++++++++++++++++++++++++++++
 1 file changed, 32 insertions(+)
```
That Cargo.lock line is the implementor's change. The crate and the contract are untracked, so `git diff` cannot see my files. `find -newer adversary-brief.md` lists only my three files, all test paths:
- `contracts/process/v1alpha1/scenarios/adversary-bounds.yaml`
- `contracts/process/v1alpha1/scenarios/adversary-output.yaml`
- `crates/tool-process/tests/adversary.rs`

No source, contract, semantics, existing scenario or Cargo file was touched.

**2. Cases** (each run alone before the suite; red output verbatim)

| case | now | red output |
|---|---|---|
| `adversary-a-descendant-holding-the-output-after-its-shell-ended-is-killed-at-the-bound` (YAML) | red | `output.stdout: expected "started", got "startedlate"` (2.13s against 300 ms) |
| `a_descendant_in_the_group_that_outlives_its_shell_does_not_hold_the_call_past_the_bound` (:184) | red | `the call was held 3.002354585s against a bound of 300 ms … exit: Some(0), stdout: "started" … timed_out: false, timeout_ms: 300` |
| `a_descendant_that_left_the_group_does_not_hold_the_call_past_the_bound` (:221) | red | `the call was held 3.008385728s against a bound of 300 ms, by a descendant outside the run's process group … timed_out: true` |
| `adversary-the-head-cut-lands-between-characters-after-a-byte-that-is-not-utf-8` (YAML) | red | `output.omitted_bytes: expected 32771, got 32770` |
| `the_head_does_not_close_on_a_broken_character_after_a_byte_that_is_not_utf_8` (:100) | red | `the head closes on the character the cut broke: "aaaaa�\n… 32770 bytes omitted here …\nbbbbbb"` |
| `a_workspace_that_can_no_longer_be_entered_is_unavailable_and_not_a_spawn_failure` (:258) | red | `` `sh`: Permission denied (os error 13) `` left `"process.SpawnFailed"` right `"process.WorkspaceUnavailable"` |
| `adversary-one-byte-over-the-cap-…`, `adversary-both-streams-over-the-cap-…` (YAML) | green | adds coverage: no earlier scenario had both streams over the cap |
| `every_allow_listed_variable_set_here_reaches_the_child_unchanged` | green | checked 11 names, from PATH to SSL_CERT_DIR |

**3. Suite** (`cargo-serial.sh test -p b10x-tool-process --no-fail-fast`, exit 101)
- Unit tests: 16 passed.
- adversary: 3 passed, 6 failed.
- conformance: 1 passed, 1 failed ("24 of 26 scenarios passed"; the two failures are my red YAML scenarios).
- mutants: 5 passed, 1 failed (`each_mutant_fails_at_least_one_scenario` requires the unmutated suite to pass first).
- `tool-contract validate contracts/` and clippy `--tests -D warnings`: both clean.

**4. Findings**

| # | file:line | what was measured | what reaches it | verdict / origin | fix not applied |
|---|---|---|---|---|---|
| F1 | lib.rs:319, :346 | The shell exits, so the poll loop stops. The output threads are then joined with no deadline (rows 1–2). | Any `sh -c "cmd &"` whose background command keeps its output open, such as a server or watcher. The guidance itself sends callers to `sh -c`. With the default bound the call can wait forever. Harness 0.14.1 local.rs:604 and :622-623 do the same. | CONFIRMED / pre-existing, blocker | Keep the deadline running until both output threads finish (`is_finished`). At the deadline, kill the whole process group even after the shell has been reaped. |
| F2 | lib.rs:346 | `set -m` puts a background job in its own process group. Killing the run's group does not reach it, so it holds the call for 3s. | `set -m`, `setsid`, or a program that daemonises without closing its output. Same harness lines. | CONFIRMED / pre-existing, warning | At the deadline, stop waiting on the pipes and answer with what was read so far. Otherwise semantics.md:45 has to drop "or hold it open". |
| F3 | output.rs:113 | The head ends on U+FFFD and the count is 1 short. One earlier invalid byte makes `from_utf8` report that byte (`error_len` Some) instead of the cut at the end. This contradicts semantics.md:57. | A stream over 64 KiB with any byte in its first 32 KiB that is not UTF-8. Harness local.rs:1053. | CONFIRMED / pre-existing, note | Check only the last 1 to 3 bytes for a character the cut broke. |
| F4 | lib.rs:259 | `is_dir()` is still true after the root loses its search permission. The run then fails as `SpawnFailed` naming `sh`, against semantics.md:25. | Nothing found except a run that removes the permission itself (`chmod -x .`). I built this state. | INFEASIBLE / introduced, note | Treat a root that cannot be entered (or a permission error from changing into it) as `WorkspaceUnavailable`, or narrow the sentence. |
| F5 | src/tests.rs:47 | The only assertions that a variable crosses are about `PATH` (tests.rs:47, environment.yaml:35). Removing any other name from `INHERITED_ENV` passes the suite. This is from reading the tests; I did not run a mutated copy. | The relocated-rustup failure that lib.rs:63 describes would come back unseen. Harness local.rs:1574 had the same gap. | CONFIRMED / pre-existing, note | The green case above now catches it. |
| F6 | contract.yaml:87 | `effects: [process]`, and semantics.md:36 reasons "not `secret`" from the environment alone. lib.rs:22 says nothing confines the program, so a declared shell can reach the network and read credential files under `HOME`. The design table (tool-contracts.md:54) says "no network unless granted". | Anything that decides on declared effects (the repository's goal O1). | CONFIRMED / introduced, note | Declare `network` (and probably `secret`), or say that `effects` describes only the runner, not the program it starts. |

needs-coordinator: F1's origin is pre-existing, but it fails the story's own acceptance sentence. Routing it (back to the implementor or into a story of its own) is your call. My two red YAML scenarios also keep the existing conformance and mutants tests red until it is fixed.

**5. Attacked and not broken**
- **Program spellings:** `/bin/sh`, `./sh`, `sh ` and `""` are all compared as text. The `/bin/sh` scenario rules out a basename-only check.
- **SIGTERM:** a program that ignores it is still stopped, because the kill is SIGKILL.
- **Closed output:** a program that closes its output and keeps running is still caught by the deadline poll.
- **Bounds:** a `timeout_ms` above the maximum is capped, `remaining` takes the smaller value, and 0 kills at once.
- **Output edges:** exactly at the cap, one byte over, and both streams over all pass.
- **Unset name:** `inheriting` a name that is not set adds nothing.
- **Errors:** every error name is declared and its fields match.
- **Port:** every harness exec test was ported with its assertions kept.
- **Profile:** `risk: high` is above the `process` floor.
- **Not tested:** whether a missing `Stdio::null()` would pass the stdin scenario when the test runner's own stdin is `/dev/null`.

**6. Written outside the worktree**
In $HOME/.cache/tools-wave-w2/process/scratch/adversary/:
- red-1-yaml-descendant.log
- red-2-descendant-timing.log
- red-3-left-group.log
- red-4-head-cut.log
- red-5-head-cut-rust.log
- red-6-workspace-closed.log
- green-7-8-output.log
- green-9-env.log
- suite-run.log
- suite-run-nff.log

I deleted the two probe scripts I wrote there. No process of mine is still alive, and the lease is released.

```findings
[
  {"file": "crates/tool-process/src/lib.rs", "line": 319, "category": "acceptance", "severity": "blocker", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "Once the shell has exited, the output threads are joined with no deadline, so a background descendant in the run's own group holds the call past its bound and the reply says timed_out false."},
  {"file": "crates/tool-process/src/lib.rs", "line": 346, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A descendant that leaves the process group (set -m, setsid) survives the group kill and holds the call open until it exits, against semantics.md's 'does not outlive the call or hold it open'."},
  {"file": "crates/tool-process/src/output.rs", "line": 113, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "whole_prefix finds the earliest invalid byte rather than the cut, so after any byte that is not UTF-8 the head closes on a broken character and omitted_bytes is short by its bytes."},
  {"file": "crates/tool-process/src/lib.rs", "line": 259, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A workspace root that can no longer be entered passes the is_dir check and is answered as process.SpawnFailed naming the program, where semantics.md promises process.WorkspaceUnavailable."},
  {"file": "crates/tool-process/src/tests.rs", "line": 47, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "Only PATH is ever asserted to reach the child, so removing any other name from INHERITED_ENV passes the suite."},
  {"file": "contracts/process/v1alpha1/contract.yaml", "line": 87, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "effects: [process] declares no network or secret effect although nothing stops a declared program from using either, and the design says process.run has no network unless granted."}
]
```
