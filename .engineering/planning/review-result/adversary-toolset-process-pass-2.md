---
format: aep.planning-md/3
id: review-result:adversary-toolset-process-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:toolset-process: pass-1 findings fixed, 2 red cases on the cut sentence, a scenario that misses a mutant, 5 findings'
relations:
- reviews: story:toolset-process
revision: 1
---
```
unit: story:toolset-process
verdict: red
cases: executed 36→40, red 4
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 8 paths under $HOME/.cache/tools-wave-w2/process/scratch/adversary/, one of them a 348M build directory (listed in part 6)
needs-coordinator: yes
```

This is the last pass. The correction fixes every pass-1 finding, and all 9 pass-1 cases pass, 8 of them because of the fix (case notes in part 2). Two new red cases remain: the sentence the correction added about where a cut moves (semantics.md:83) does not match the code at either end. These go to a person.

**1. `git --no-pager diff --stat`**
```
 Cargo.lock | 32 ++++++++++++++++++++++++++++++++
 1 file changed, 32 insertions(+)
```
That line is the implementor's change. The crate and the contract are untracked, so `git diff` cannot see my files. `find -newer src/lib.rs` lists only my three pass-2 files, all test paths:
- `crates/tool-process/tests/adversary_2.rs`
- `contracts/process/v1alpha1/scenarios/adversary-bounds-2.yaml`
- `contracts/process/v1alpha1/scenarios/adversary-output-2.yaml`

My pass-1 files have not changed since my pass-1 run.

**2. Cases** (each run alone before the suite)

| case | now | output when run alone |
|---|---|---|
| `adversary-a-byte-that-is-not-utf-8-closing-the-head-is-answered-and-not-moved` | red | `output.omitted_bytes: expected 32768, got 32769` |
| `adversary-a-byte-that-is-not-utf-8-opening-the-tail-is-answered-and-not-moved` | red | `output.omitted_bytes: expected 32768, got 32769` |
| `adversary-a-descendant-left-by-a-shell-that-ended-is-killed-at-the-bound-not-only-left-behind` | green | red against the mutant below: `expected "started", got "startedsurvived"` |
| `a_run_answers_within_what_it_had_left_and_half_a_second_…` | green | 0.70s with 200 ms left and a process outside the group holding the output |

Mutant, applied only to a copy in scratch (`mutant-kill-only-a-live-shell.diff`): `lib.rs:330` `if timed_out {` becomes `if timed_out && ended.is_none() {`, so the group is killed only while the shell is still alive. Against it:
- All 18 unit tests pass, including `tests.rs:365`.
- All 28 of the implementor's scenarios pass, including `bounds.yaml:48`.
- My pass-1 `adversary-bounds.yaml` also passes.
- Only `adversary.rs:190` fails (`descendant 739915 was still alive after the call returned`), plus my new scenario.

Pass-1 cases against the new code:
- **Now passing because of the fix:** the held-and-alive test (fails on the mutant), the left-group test (passes because of the 500 ms wait after the kill), the head cut (passes because of the new `whole_prefix`), and the closed workspace (passes because of `enterable`).
- **Not proven by the kill:** my pass-1 YAML `adversary-bounds.yaml` now passes whether or not the group is killed.

**3. Suite** (`cargo-serial.sh test -p b10x-tool-process --no-fail-fast`, exit 101)
- Unit tests: 18 passed.
- adversary: 9 passed.
- adversary_2: 2 passed, 2 failed.
- conformance: 1 passed, 1 failed ("29 of 31 scenarios passed").
- mutants: 6 passed, 1 failed (it requires the unmutated suite to pass first).
- `tool-contract validate contracts/` and clippy `--tests -D warnings`: both clean.

**4. Findings**

| # | file:line | what was measured | what reaches it | verdict / origin | fix not applied |
|---|---|---|---|---|---|
| P1 | output.rs:193 | A non-UTF-8 byte (0xE9) as the head's last byte, followed in the stream by `b`, is cut off and counted in N. semantics.md:83 says "Only a character the cut itself broke moves it". | Output over 64 KiB that is not UTF-8 (Latin-1, binary), with a high byte at offset 32767. Harness local.rs:1049-1053 did the same, but made no such claim. | CONFIRMED / introduced, note | Keep up to 3 bytes past the head in `Ends` and move the cut only when they complete the character, or narrow the sentence. |
| P2 | output.rs:213 | A stray 0xA9 that opens the tail after a whole character is skipped and counted. Same sentence. | The same streams, with a continuation-looking byte at `total - 32768`. | CONFIRMED / introduced, note | Keep 3 bytes before the tail in the ring, or narrow the sentence. |
| P3 | scenarios/bounds.yaml:48 | Neither this scenario nor tests.rs:365 can fail when the kill after the shell ended is skipped: the descendant writes at 2–3 s, and the reply is due by the 300 ms bound plus 500 ms. Shown with the mutant above. | Any other implementation is judged by this suite: one that leaves descendants running passes `bounds.yaml` (now caught only by my `adversary-bounds-2.yaml`). This is the story's acceptance property. | CONFIRMED / introduced, warning | My green scenario now catches it. Still needed: a `Mutation` that kills only a live shell, named against that scenario in mutants.rs (a source change). |
| P4 | contract.yaml:92 | `consequences: []`, while semantics.md:43-45 says the program "may write and delete files, reach the network" and that the operation "declares every effect that allows". | Anything that filters on consequences. The risk floor would not change (already `high`). | CONFIRMED / introduced, note | Declare `delete`, `send-external`, `irreversible` and `sensitive-data`, or say why consequences differ from effects. |
| P5 | lib.rs:332 | After the shell has been reaped, `killpg` addresses a group by the shell's pid. When no member of that group is left (the output held only by a process outside it), that pid is free and could be reused. I did not construct this. | It needs the pid reused as a group id within the bound. pid_max here is 4194304, so that means about 7000 forks/s for 600 s. Harness only killed a child it had not reaped. | INFEASIBLE / introduced, note | Watch the exit with `waitid(…, WEXITED\|WNOHANG\|WNOWAIT)` so the shell stays unreaped until the reply, then reap it. |

needs-coordinator: yes, for three things:
- P1 and P2 stay red and go to a person (change the code or narrow semantics.md:83).
- P3 needs a source change (the new `Mutation`) that is not mine to make.
- The copy's `target/` (348M) is in my scratch, and the rules forbid me to delete a build directory.

**5. Attacked and not broken**
- **Locking:** the shared output state is locked once per chunk read, never while waiting on the pipe, so taking the output cannot block.
- **Byte counts:** the head, tail and omitted arithmetic in `Ends::take` is exact, and the normal path waits for both streams to finish.
- **Answer time:** the reply comes within what the run had left plus 500 ms (measured 0.70s).
- **A gone group:** when the group is gone but a process outside it holds the output, the wait still ends at 500 ms.
- **`exit` after the shell ended by itself:** it keeps its own code, matching contract.yaml:63.
- **Symlinked root:** `enterable` follows a root replaced by a symlink, and nothing claims it should not.
- **Not constructed:** a shell the user cannot kill (a setuid program that makes root its real user) would block `child.wait()` at lib.rs:336 with no limit. Building it needs passwordless sudo.

**6. Written outside the worktree**
In $HOME/.cache/tools-wave-w2/process/scratch/adversary/:
- `p2-scenario_a_byte_that_is_not_utf_8_closing_the_head_is_answered_and_not_moved.log`
- `p2-scenario_a_byte_that_is_not_utf_8_opening_the_tail_is_answered_and_not_moved.log`
- `p2-scenario_a_descendant_left_by_a_shell_that_ended_is_killed_at_the_bound_not_only_left_behind.log`
- `p2-a_run_answers_within_what_it_had_left_and_half_a_second_when_a_process_outside_the_group_holds_the_output.log`
- `p2-suite-run.log`
- `mutant-kill-only-a-live-shell.diff`
- `mutant-copy-run.log`
- `mutant-copy/` (348M, including its own `target/`)

The pass-1 logs listed in my pass-1 report are still there. No process of mine is still running, and the lease is released.

```findings
[
  {"file": "crates/tool-process/src/output.rs", "line": 193, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "whole_prefix moves the cut for a stray lead byte that closes the head although the cut broke no character, against semantics.md:83, so omitted_bytes counts the stream's own byte."},
  {"file": "crates/tool-process/src/output.rs", "line": 213, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "whole_suffix skips a stray continuation byte that opens the tail after a whole character, against semantics.md:83, so the byte is dropped and counted as omitted."},
  {"file": "contracts/process/v1alpha1/scenarios/bounds.yaml", "line": 48, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The scenario meant to hold the kill after the shell ended, and tests.rs:365, both pass against an implementation that skips that kill, because the descendant writes only after the reply is due."},
  {"file": "contracts/process/v1alpha1/contract.yaml", "line": 92, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "consequences is empty while semantics.md says the program may delete files and reach the network and that the operation declares everything that allows."},
  {"file": "crates/tool-process/src/lib.rs", "line": 332, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "After the shell has been reaped, killpg sends SIGKILL to a group id that may no longer be reserved, so a reused pid leading an unrelated group could be killed."}
]
```
