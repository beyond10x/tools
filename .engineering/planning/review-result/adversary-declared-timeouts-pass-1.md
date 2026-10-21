---
format: aep.planning-md/3
id: review-result:adversary-declared-timeouts-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:declared-timeouts-are-enforced: 3 red, the timeout refusal comes before the path and pattern refusals, 4 findings'
relations:
- reviews: story:declared-timeouts-are-enforced
revision: 1
---
unit: story:declared-timeouts-are-enforced, working tree at base `ae027db` plus uncommitted and untracked changes in `$HOME/.local/state/worktree/trees/b10x/tools/impl-declared-timeouts`
verdict: NEEDS-CHANGE
cases: executed 120→130, red 8 (3 adversary cases plus 5 suite-level tests that run the merged scenario suite; scenarios fs 85→88 with 1 red, search 59→62 with 2 red)
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `$HOME/.cache/tools-wave-w4/declared-timeouts/scratch/adversary/`
needs-coordinator: you decide whether `semantics.md` or the code is the side that changes for finding 1; until one does, the conformance and mutants tests stay red by design

**1. Diff proof.** `git --no-pager diff --stat` is unchanged from the implementor's 20 files, because everything I added is untracked. `git status --short -uall`, filtered to my files:
```
?? contracts/fs/v1alpha2/scenarios/adversary-deadline.yaml
?? contracts/search/v1alpha2/scenarios/adversary-deadline.yaml
?? crates/tool-fs/tests/adversary_deadline.rs
?? crates/tool-search/tests/adversary_deadline.rs
```
All four are test files. I changed no implementation file. I ran `rustfmt` on my two `.rs` files only.

**2. Cases added**, each run alone before the suite. Logs are `scratch/adversary/red-fs.log` and `red-search.log`.

| case | asserts | now |
|---|---|---|
| fs `adversary-a-path-that-climbs-out-is-refused-for-the-path-whatever-its-deadline-asks` | fs.read `../x.txt` with `timeout_ms: 60001` gives `fs.OutsideWorkspace` | red |
| search `…-path-that-climbs-out-…` | grep with `path: ..` and `60001` gives `search.OutsideWorkspace` | red |
| search `…-broken-expression-…` | `fn (alpha` with `60001` gives `search.InvalidPattern` | red |
| -1 / `"10"` scenarios (3) | answered `InvalidInput` | green |
| `*_with_a_deadline_of_1_ms_stops_near_it_*` (5: read, list, glob, stat, grep) | the uncut workload takes 5 ms or more; `timeout_ms: 1` gives `deadline_reached` true in under 500 ms, partial output | green on the code, red on a mutant copy |

Red output, verbatim:
```
FAIL adversary-a-path-that-climbs-out-is-refused-for-the-path-whatever-its-deadline-asks: expected error fs.OutsideWorkspace, got error fs.InvalidInput: `timeout_ms` is 60001 and the most a call may name is 60000 milliseconds. Nothing was read.
FAIL adversary-a-broken-expression-is-refused-for-the-pattern-whatever-its-deadline-asks: expected error search.InvalidPattern, got error search.InvalidInput: `timeout_ms` is 60001 …
FAIL adversary-a-path-that-climbs-out-…: expected error search.OutsideWorkspace, got error search.InvalidInput: …
```

**Mutant copy:** `Duration::from_millis` changed to `from_secs` at `crates/tool-fs/src/deadline.rs:50`. Against it, the unit's 120 tests passed and none failed (log `scratch/adversary/mutant-run.log`). My 5 real-clock cases went red, for example: `timeout_ms: 1 on a 254.108016ms read` and `timeout_ms: 1 on a 236.511494ms search`.

**3. Suite run:** `cargo-serial.sh test -p b10x-tool-fs -p b10x-tool-search --locked --no-fail-fast`, EXIT=101, 122 passed and 8 failed (log `scratch/adversary/suite.log`).
- The 3 red adversary cases listed above.
- 5 tests that run the merged suite, all red because of my red scenarios:
  - fs `the_other_adversary_scenarios_hold`
  - `the_suite_passes_against_the_intact_toolset` (fs, search)
  - `each_mutant_fails_at_least_one_scenario` (fs, search)
- Scenario counts: fs "87 of 88 scenarios passed", search "60 of 62 scenarios passed".
- `fmt --check` gives FMT=0; `clippy` finished with no warnings.

**4. Findings**

| # | file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|---|
| 1 | `contracts/search/v1alpha2/semantics.md:51` | Says "The pattern, the glob and the path are judged first", but `crates/tool-search/src/lib.rs:286` refuses `timeout_ms` before any of them. The unit's own test `crates/tool-search/src/tests.rs:710` asserts the opposite of that document. | NEEDS-CHANGE | introduced | Any grep with `timeout_ms` above 60000 plus an escaping path: a governor reads `InvalidInput`, not `OutsideWorkspace`. |
| 2 | `contracts/fs/v1alpha2/semantics.md:39` | "Path resolution and every refusal come first", but `crates/tool-fs/src/lib.rs:231` refuses `timeout_ms` before `resolve`. The crate's own policy at lib.rs (the path is judged before the window, "the refusal a governor reads") points the same way as the document. | NEEDS-CHANGE | introduced | Same as 1, for any fs operation. |
| 3 | `crates/tool-fs/src/deadline.rs:50` | No case drives a `timeout_ms` other than 0 or 60000 through the real clock. A seconds-for-milliseconds mutant (`from_secs` for `from_millis`) keeps all 120 tests green. | CONFIRMED | introduced | Every call that names `timeout_ms` from 1 to 60000. My 5 real-clock cases now catch the mutant. |
| 4 | `crates/tool-fs/src/line.rs:74` | When the deadline cuts a line mid-character, the window text ends in U+FFFD, a character the file does not hold. `admit` at `crates/tool-fs/src/lib.rs:760` runs `from_utf8_lossy` without the `whole_prefix` cut that the byte-ceiling path uses. A probe in the scratch copy using `Deadline::after_checks(2)` showed it: text ends `"€€\u{FFFD}\n"`, and the line is in `truncated_lines`. | INFEASIBLE | introduced | Only a real deadline passing between two 8 KiB buffers of a line inside the window. I built that state; I could not drive it through the public API. |

Possible fixes (I applied none):
- **Findings 1 and 2:** either move the `timeout_ms` check after the path, pattern and glob checks, or reword both `semantics.md` files to say the `timeout_ms` refusal comes first.
- **Finding 4:** apply `whole_prefix` to `kept` when `!line.whole`.

**5. Attacked and not broken**
- `timeout_ms` edge values: 0, 1, 60000, 60001, -1 and `"10"` are each answered as the contract says.
- `deadline_reached` false on an incomplete answer: none found. Every early stop is a check that sets `reached`.
- `sha256` or `lines.total` answered from a partial scan: none. Both are tied to `stopped` / `reached`.
- Loops with no check: none. 50,000 entries, 100×200 trees, 16 MiB `fs.stat`, 1M-line read and a 200-file grep all stop under 500 ms.
- Scenarios copied from v1alpha1: only the `version:` line differs, and no meaning changed.
- `counting_bounded` and the deadline can never both be set, so there is no note conflict.
- No crate in the workspace builds `StatInput` / `ListInput` from literals, so the new fields break no consumer.

**6. Written outside the worktree**, all under `$HOME/.cache/tools-wave-w4/declared-timeouts/scratch/adversary/`:
- `red-fs.log`, `red-search.log`, `suite.log`, `mutant-run.log`, `probe-utf8.log`, `probe-utf8.rs.txt`
- `mutant-copy/`: a copy of the repository, about 8 MB, with its own build output in `mutant-copy/target`

```findings
- file: contracts/search/v1alpha2/semantics.md
  line: 51
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: semantics says pattern, glob and path are judged before the deadline, but grep refuses timeout_ms above 60000 first, so an escaping path or broken regex is answered search.InvalidInput instead of search.OutsideWorkspace or search.InvalidPattern
- file: contracts/fs/v1alpha2/semantics.md
  line: 39
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: semantics says path resolution and every refusal come first, but every fs operation refuses timeout_ms above 60000 before resolving the path, so a containment probe is answered fs.InvalidInput instead of fs.OutsideWorkspace
- file: crates/tool-fs/src/deadline.rs
  line: 50
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no case drives a nonzero timeout_ms through the real clock, so a seconds-for-milliseconds mutant passes all 120 unit tests; the added 1 ms real-clock cases catch it
- file: crates/tool-fs/src/line.rs
  line: 74
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a deadline cut inside a multi-byte character leaves U+FFFD in the read window text because admit decodes lossily without the whole_prefix cut the byte-ceiling path uses
```
