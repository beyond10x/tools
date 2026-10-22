---
format: aep.planning-md/3
id: review-result:adversary-declared-timeouts-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:declared-timeouts-are-enforced: pass-1 findings fixed, 2 red notes on CRLF at the 16 MiB bound'
relations:
- reviews: story:declared-timeouts-are-enforced
revision: 1
---
unit: story:declared-timeouts-are-enforced pass 2, working tree at base `ae027db` plus the uncommitted correction round 1 in `$HOME/.local/state/worktree/trees/b10x/tools/impl-declared-timeouts`
verdict: CONFIRMED (2 note-level findings; my pass-1 findings 1–4 now pass)
cases: executed 123→137, red 2 (taken from the same suite run with my two `adversary_deadline.rs` binaries left out of the count, not from a separate run); scenarios fs 88→116, search 62→74, all pass
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, `$HOME/.cache/tools-wave-w4/declared-timeouts/scratch/adversary/`
needs-coordinator: none

**1. Diff proof.** The tracked diff is the implementor's alone, because everything I added is untracked. My paths (all test files; no implementation file touched):
```
?? contracts/fs/v1alpha2/scenarios/adversary-deadline.yaml        (pass 1, unchanged)
?? contracts/fs/v1alpha2/scenarios/adversary-deadline-order.yaml  (pass 2, 28 scenarios)
?? contracts/search/v1alpha2/scenarios/adversary-deadline.yaml    (pass 1, unchanged)
?? contracts/search/v1alpha2/scenarios/adversary-deadline-order.yaml (pass 2, 12 scenarios)
?? crates/tool-fs/tests/adversary_deadline.rs                     (pass 1 + 3 tests)
?? crates/tool-search/tests/adversary_deadline.rs                 (pass 1 + 1 test)
```
**My pass-1 cases were not weakened:**
- The two YAML files still carry my write times (04:50).
- Both `.rs` files differ from my pass-1 copy only by my own `rustfmt`.
- `tests/adversary.rs` changed its count from 21 to 21 + 3, which counts my three scenarios; every scenario it counts must still pass.
- My new order scenarios start with `adversary2-`, so that count does not move.

**2. Cases added, each run alone before the suite**

| case | asserts | now |
|---|---|---|
| fs order scenarios (28) | every input refusal of each operation wins at `timeout_ms` 60001 and at 0 (refusals below) | green |
| search order scenarios (12) | empty pattern, broken regex, empty glob, broken glob, outside path, missing path, each at 60001 and at 0 | green |
| `a_line_the_counting_bound_stops_before_its_crlf_is_as_whole_as_one_before_its_lf` | a line the 16 MiB bound stops right before its `\r\n` is answered like the same line before `\n` | **red** |
| `a_line_the_counting_bound_stops_between_its_cr_and_its_lf_hands_out_no_cr` | no `\r` in the window text | **red** |

The fs refusals covered:
- read: outside path, missing path, a directory, `offset: 0`, `limit: 0`
- list: outside path, missing path, a file
- glob: empty glob, broken glob, outside path, missing path
- stat: outside path, missing path

Red output, verbatim:
```
assertion `left == right` failed: line 16384 is named cut in the CRLF file only
  left: [16384]
 right: []
a `\r` is handed out: "bb\r\n"
```

**Origin checks, on copies in scratch:**
- The CRLF case is green against my pass-1 copy of the code (`pass2-crlf-base.log`), so the correction round introduced it.
- The `\r` case is red against `git archive ae027db` (`base-cr.log`), so it was already there at the base.
- My pass-1 UTF-8 probe passes against a copy of the current code (`pass2-probe-utf8.log`), so finding 4 is fixed.

**3. Suite run:** `cargo-serial.sh test -p b10x-tool-fs -p b10x-tool-search --locked --no-fail-fast`, EXIT=101, 135 passed and 2 failed.
- The only failures are the 2 cases above.
- Conformance passes, and both `each_mutant_fails_at_least_one_scenario` tests pass.
- `fmt --check` gives FMT=0; `clippy` finished with no warnings.

**4. Findings**

| # | file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|---|
| 1 | `crates/tool-fs/src/line.rs:77` | The new peek treats only `\n` as "the newline is next". A CRLF line the bound stops before its `\r` is listed in `truncated_lines` although the text holds all of it. The same line in an LF file is not listed. This goes against `Line::whole`'s own reasoning about CRLF files. | CONFIRMED | introduced | `fs.read` of a CRLF file over 16 MiB, with the window on the line where the bound falls |
| 2 | `crates/tool-fs/src/line.rs:109` | When the bound falls between a line's `\r` and its `\n`, the `\r` is not dropped (`pop` runs only once the newline is read). The window text then holds a `\r`, against line.rs's own rule. | CONFIRMED | pre-existing | Same setup, with the bound exactly after the `\r` |

Possible fixes (I applied neither):
- **Finding 1:** count a next byte of `\r` followed by `\n` as "newline next" as well.
- **Finding 2:** drop a trailing `\r` from `kept` when the peek finds `\n`.

**5. Attacked and not broken**
- Refusal order: all 40 combinations of operation, input refusal and `timeout_ms` 60001 or 0 pass. My pass-1 order scenarios (findings 1 and 2) now pass too.
- `fs.read` with an offset past the end and `timeout_ms: 60001` answers `fs.InvalidInput`. I read this as by design: that refusal needs the scan, and D3 names the path, the window and the glob. I did not add a case for it.
- `IgnoreDeadline` mutant: still refused above 60000, and still caught by the mutants tests.
- A `timeout_ms` above the maximum is capped by `.min(MAX)` when the deadline is built, then refused. That deadline is never checked.
- The real-clock 1 ms cases pass (finding 3 from pass 1 is closed).
- `whole_prefix` on a line not held whole: the probe is green, and the 2000-character cut is unchanged.
- The `\n` peek at the counting bound: the implementor's test passes, and so does the LF half of my CRLF case.

**6. Written outside the worktree**, all under `$HOME/.cache/tools-wave-w4/declared-timeouts/scratch/adversary/`:
- Pass 1 files: `red-fs.log`, `red-search.log`, `suite.log`, `mutant-run.log`, `probe-utf8.log`, `probe-utf8.rs.txt`
- Pass 2 files: `gen_order.py`, `base_cr_probe.rs`, `pass2-red.log`, `pass2-cr.log`, `pass2-crlf-base.log`, `pass2-probe-utf8.log`, `base-cr.log`, `pass2-suite.log`
- `mutant-copy/`: now synced to the current tree with the UTF-8 probe appended, with its own build output in `mutant-copy/target`
- `base-copy/`: `git archive ae027db` plus `crates/tool-fs/tests/base_cr_probe.rs`, with its own build output in `base-copy/target`

```findings
- file: crates/tool-fs/src/line.rs
  line: 77
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the counting-bound peek treats only a following newline as a whole line, so a CRLF line the bound stops right before its carriage return is listed in truncated_lines while the same LF line is not
- file: crates/tool-fs/src/line.rs
  line: 109
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: when the counting bound falls between a line's carriage return and its newline the carriage return is kept and handed out in the window text, against the reader's rule that it is dropped
```
