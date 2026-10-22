---
format: aep.planning-md/3
id: review-result:adversary-process-kill-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:process-kill-never-reaches-a-reused-group: no red case, 2 notes closed by its own cases'
relations:
- reviews: story:process-kill-never-reaches-a-reused-group
revision: 1
---
unit: story:process-kill-never-reaches-a-reused-group, uncommitted working tree on base 5ac57a7
verdict: CONFIRMED (2 notes, both test gaps; I found no reachable defect)
cases: executed 46→48, red 0 on the tree (each new case is red against its own mutant in a scratch copy)
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: 1 directory, `$HOME/.cache/tools-wave-w4/process-kill/scratch/adversary/` (376M)
needs-coordinator: none

**1. Diff stat**
```
 crates/tool-process/src/lib.rs   |  80 +++++++++++++++++++++-----
 crates/tool-process/src/tests.rs | 118 +++++++++++++++++++++++++++++++++++++++
```
Both of those are the implementor's changes. My only change is one new untracked test file, `crates/tool-process/tests/adversary_3.rs`. No implementation path was touched.

**2. Cases added** (`crates/tool-process/tests/adversary_3.rs`)

| Case | Asserts | Tree | Scratch-copy mutant | Red output there |
|---|---|---|---|---|
| `a_program_ended_by_its_own_signal_is_answered_at_once_and_not_as_timed_out` (:40) | `sh -c 'kill -KILL $$'` with a 3000 ms bound gives `timed_out: false`, `exit: None` and answers in under 1.5 s | green | `Signaled` treated as not ended | `a program dead by its own signal was waited on to the bound: RunOutput { argv: ["sh", "-c", "kill -KILL $$"], exit: None, stdout: "", stderr: "", truncated: false, omitted_bytes: 0, timed_out: true, timeout_ms: 3000 }` |
| `a_program_that_stops_itself_is_not_ended_and_is_killed_at_the_bound` (:66) | a shell that closes both streams and then sends itself SIGSTOP is killed at the 300 ms bound: `timed_out: true`, `exit: None`; a 10 s watchdog turns a hang into a failure | green | `WSTOPPED` added to the `waitid` flags | `the run did not answer within 10 s of a 300 ms bound: a stop was taken as an end` |

Run alone on the tree: `cargo-serial.sh test -p b10x-tool-process --locked --test adversary_3` gave `test result: ok. 2 passed; 0 failed`. The red lines above were captured before `rustfmt`, at file lines 46 and 79; after formatting those are lines 51 and 84.

**3. Suite run** (after the cases existed): `cargo-serial.sh test -p b10x-tool-process --locked --no-fail-fast` exited 0.
```
test result: ok. 23 passed  (unittests)
test result: ok. 9 passed   (adversary)
test result: ok. 4 passed   (adversary_2)
test result: ok. 2 passed   (adversary_3)
test result: ok. 2 passed   (conformance)
test result: ok. 8 passed   (mutants)
test result: ok. 0 passed   (doctests)
```
The 46 before my cases comes from the implementor's `green-test.log`.

Against each mutant in the scratch copy, the other 46 cases all stay green, conformance and mutant lanes included. Only the new case fails, exit 101. `fmt -- --check` and `clippy --all-targets -D warnings` are both clean with my file in the tree.

**4. Findings**

| # | file:line | Finding | Verdict / origin | What reaches it |
|---|---|---|---|---|
| A1 | `crates/tool-process/src/lib.rs:510` | No existing test or scenario covers a program that dies by its own signal. A mutant treating `WaitStatus::Signaled` as "not ended" passes all 46 existing cases, although the run then waits to the bound and reports `timed_out: true`. `adversary_3.rs:40` catches it. | CONFIRMED / introduced, note | Any declared program that crashes (SIGSEGV, OOM kill) before its bound. |
| A2 | `crates/tool-process/src/lib.rs:507` | No existing test covers a stopped program. Adding `WSTOPPED` takes a stop as the end, and the blocking `child.wait()` then hangs the call with no limit; all 46 existing cases stay green. `adversary_3.rs:66` catches it. | CONFIRMED / introduced, note | A program that stops itself or is stopped while its streams are closed. |

The base used `Child::try_wait`, so neither line existed before this change. Both findings are on the working tree over 5ac57a7.

**5. Attacked and not broken**
- **Exit between the last poll and the kill:** the shell stays unreaped, since only `child.wait()` reaps it, so `killpg` names its own group.
- **Deadline `None`:** it only arises under the NoTimeoutCap mutant, and it never reaches the kill.
- **`terminate` on the wait-error path:** with `WNOHANG` the error is ECHILD, meaning someone else already reaped the shell. Nothing in the repository ignores SIGCHLD or calls `waitpid(-1)`, so I found nothing that reaches this; the base behaves the same.
- **Leaked zombies:** I found none. The normal path and the path after a kill both reap. The `ended` error path does not reap, but it only fires after the shell was already reaped.
- **`RunOutput` against the base:** `exit` keeps the shell's own code, because a zombie's status is fixed. `timed_out` comes from the same loop and output from the same drains. Lanes match (`after-lanes.sum` and `base-lanes.sum`, 31 of 31).
- **Stop or continue reported as an end:** neither `WSTOPPED` nor `WCONTINUED` is requested.
- **`child.kill()` on the unreaped zombie:** harmless.
- **The settle's `killpg(pid, 0)` after the reap:** it sends no signal, as the code comment says.

**6. Written outside the worktree**, all in `$HOME/.cache/tools-wave-w4/process-kill/scratch/adversary/`:
- `cases-alone.log` and `suite.log`
- `mutant-waitid.diff`, `mutant-signal.log` and `mutant-stop.log`
- `mutant-copy/`: a full copy of the repository (no `.git`) plus its own `target/`, which accounts for most of the 376M. Hard rule 6 stops me deleting a build directory, so it is yours to clean up.

```findings
- file: crates/tool-process/src/lib.rs
  line: 510
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a mutant that takes WaitStatus::Signaled as not ended passes all 46 existing cases and turns a self-signalled program into timed_out true at the bound; crates/tool-process/tests/adversary_3.rs:40 catches it
- file: crates/tool-process/src/lib.rs
  line: 507
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a mutant adding WSTOPPED to the waitid flags passes all 46 existing cases and hangs the call in child.wait() on a stopped program with closed streams; crates/tool-process/tests/adversary_3.rs:66 catches it
```
