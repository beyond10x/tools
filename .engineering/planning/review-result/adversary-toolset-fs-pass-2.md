---
format: aep.planning-md/3
id: review-result:adversary-toolset-fs-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:toolset-fs: pass-1 findings resolved, 1 red case at the 16 MiB bound, 4 notes'
tags:
- adversary
relations:
- reviews: story:toolset-fs
revision: 1
---
Adversary pass 2 on story:toolset-fs, as returned by `aep:adversary` on 2026-10-20 after correction
round 1. The coordinator replaced the expanded home directory in paths with `$HOME` (public
repository rule) and changed nothing else.

```
unit: story:toolset-fs, pass 2 (working tree of impl/toolset-fs after the correction, base 796937c)
verdict: red
cases: executed 38→39 test functions (scenarios 63→69), red 1
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: none (scratch adversary-2 created, left empty; the cargo wrapper appended to its own log)
needs-coordinator: no
```

Pass-1 findings: 6 of 6 resolved (F1 glob cap, F2 counting bound at 16 MiB − 1 / 16 MiB / 16 MiB + 1 multi-line, F3 limit 0, F4 `a.txt/x`, J1 unknown-field test, J2 262144 cap in the suite); 1 new finding on new ground, plus 1 pre-existing note the new finding exposed.

## 1. Diff stat

`git --no-pager diff --stat`: only ` Cargo.lock | 70 +` (the implementor's). Mine, all untracked test/scenario/fixture paths, no non-test path touched:

| path | state |
|---|---|
| `crates/tool-fs/tests/adversary.rs` | 211 lines; +1 test, pass-1 `limit: 0` test strengthened, scenario count 15→21 |
| `contracts/fs/v1alpha1/scenarios/adversary-read.yaml` | 104 lines; +3 scenarios |
| `contracts/fs/v1alpha1/scenarios/adversary-links.yaml` | 113 lines; +3 scenarios |
| `contracts/fs/v1alpha1/fixtures/adversary-chain/{real/keep.txt, filelink -> real/keep.txt, hop -> filelink}` | new fixture |
| `adversary-glob.yaml`, `adversary-long-line.yaml`, `fixtures/adversary-links`, `fixtures/adversary-long-line` | unchanged from pass 1 |

`cargo fmt --check`, `cargo clippy --all-targets -D warnings` and `validate contracts/` pass with my files in place.

## 2. Cases added, run alone before the suite

**R1 (red)** `a_file_over_the_counting_bound_with_no_newline_is_over_it_for_read_as_well_as_for_stat` (adversary.rs:181). A 16,777,217-byte file holding no newline; asserts the contract's "over 16 MiB: lines.total and sha256 null" and that read and stat agree. Run alone:
```
assertion `left == right` failed: over 16 MiB: (lines.total, read digests, stat digests); lines_counted_to None, truncated_lines [1]
  left: (Some(1), true, false)
 right: (None, false, false)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
```

**Strengthened (green)** `a_window_one_below_its_start_is_only_an_empty_file_read_from_line_one` (adversary.rs:140). After the correction every branch of my pass-1 case hit `continue`, so it asserted nothing; it now asserts the refusal is `fs.InvalidWindow` naming `` `limit` `` at offsets 1, 2 and 9. Green: `LimitZero` wins over the past-the-end refusal (lib.rs:315 runs before the scan).

**Green scenarios (6, in `the_other_adversary_scenarios_hold`, which asserts exactly 21 ran):** `seven.txt: "abc\nxyz"` at `max_bytes: 7` answered whole and not truncated (the unterminated last line weighs 3, pinning `Line.terminated`; a mutant charging the separator always cuts it), and cut at `max_bytes: 6`; `"alpha\r\nbeta"` at `max_bytes: 11` whole with `\r` dropped and charged; a read through `hop -> filelink -> real/keep.txt` answered as `real/keep.txt`; `hop/x` is `fs.NotFound` for `fs.read` and for `fs.list` (the `NotADirectory` mapping through a chain).

## 3. Suite run

`cargo-serial.sh test -p b10x-tool-fs --no-fail-fast`, after the cases existed:
```
unittests src/lib.rs: 25 passed
tests/adversary.rs:   5 passed; 1 failed   (R1 above)
tests/conformance.rs: 2 passed             the_suite_passes_against_the_intact_toolset ... ok  (69 scenarios)
tests/mutants.rs:     6 passed             each_mutant_fails_at_least_one_scenario ... ok; no_ceiling_cap_is_caught_by_the_ask_above_the_maximum ... ok
error: 1 target failed: `-p b10x-tool-fs --test adversary`
```

## 4. Findings

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| R1 | `crates/tool-fs/src/lib.rs:599` (the `total < offset \|\| closed` guard around the bound test at :603) | the counting bound is tested only between lines in the counting branch; a 16 MiB + 1 file with no newline is one line, consumed whole by `next_line` inside the window branch, so `fs.read` answers `lines.total: Some(1)`, a digest, `truncated_lines: [1]`, no `lines_counted_to`, while `fs.stat` (lib.rs:551, `<= 16 MiB`) answers `sha256: null` for the same file; contract.yaml:75 says "On a file over 16 MiB ... lines.total and sha256 are null" and :252 "sha256 only for a file of at most 16 MiB" | any file over 16 MiB whose bound-crossing line is handled by the window branch: a one-line file (minified bundle, a blob with no 0x0A byte, a base64 dump), or a short line followed by one over 16 MiB (that line closes the window at :617-619 and is never bound-checked either) | CONFIRMED, warning | introduced (the two guidance sentences and `fs.stat` are the unit's; the branch structure is harness's, snapshot :337-346) |
| N1 | `crates/tool-fs/src/line.rs:54-82` | the same measurement shows the 16 MiB bound does not bound the scan: `next_line` reads a newline-free file to its end in 8 KiB chunks and only then does `take` run; semantics.md:57 "The scan stops once more than 16 MiB have been scanned" holds only for files with a line end past 16 MiB; memory stays at 8000 bytes, time does not | the same files; the scan-time claim at semantics.md:57 and the crate doc at lib.rs:86-94 | CONFIRMED, note | pre-existing: snapshot :1095-1128 is the same `next_line`, :332-346 the same loop |
| J3 | `crates/tool-fs/src/walk.rs:77-78` | read from the code, not timed: `visit → false` returns from the current frame only; parent frames keep iterating (:76), so after `max_results` is reached the rest of the tree is still visited, each entry a `canonicalize` in `contained`; the result is unaffected | every capped `fs.glob` on a large tree | CONFIRMED, note | pre-existing (snapshot :951-953) |
| J4 | `crates/tool-fs/src/lib.rs:308-320` | `offset: 0` and `limit: 0` are refused before `open_read`, so `fs.read {path: ../x, limit: 0}` answers `fs.InvalidWindow`, not `fs.OutsideWorkspace`; measured for `limit: 0` over `PastEnd`, the path part read from the order of the early returns; both say "nothing was read" | a caller sending a bad window and a bad path together | CONFIRMED, note | introduced for `limit` (the `offset` order is harness's, snapshot :313-320) |
| J5 | `crates/tool-fs/src/lib.rs:623` | read from the code: the first-line cut still reserves a separator byte (`ceiling.saturating_sub(1)`) on an unterminated line, while the weight at :614 no longer charges one; `"abcdefghij"` with no newline at `max_bytes: 4` would answer 3 characters where the terminated rule gives 4; the contract does not specify the cut width | a one-line unterminated file read under a small `max_bytes` | CONFIRMED, note | introduced (the terminated rule is this correction's) |

Named fixes, not applied: R1/N1 — test `scanned > MAX_LINE_COUNT_BYTES` after every line whatever branch handled it (and, to bound time as well as count, stop `next_line` itself once the bytes consumed pass the bound, answering `counting_bounded`), or reword contract.yaml:75, :252 and semantics.md:57 to "the count stops at the first line end past 16 MiB; a file whose line runs past that is scanned to its end". J3 — propagate the visitor's `false` up the frames (return a `(bounded, stop)` pair). J4 — resolve the path before judging the window, or document the order. J5 — `saturating_sub(u64::from(line.terminated))`.

## 5. Attacked and not broken

- `terminated` at exactly the bound: 16 MiB with and without a trailing newline counted and digested whole, 16 MiB + 1 multi-line `null`/`null` on both operations (unit test `stat_digests_a_file_of_at_most_the_counting_bound_and_not_one_over_it`, my pass-1 16 MiB − 1 case, both green).
- CRLF last line without newline; ceiling weight on an unterminated last line at exactly the ceiling and one under; `limit: 0` with `offset` past the end; `NotADirectory` through a two-link chain for read and list; the read through the chain under the target's name: all green scenarios above.
- The fifth mutant: killed by exactly one scenario (`a-byte-ceiling-above-the-maximum-is-capped-at-the-maximum`, the only ask above 262144: 37449 × 7 = 262143 ≤ 262144 < 37450 × 7), named in `killing()`, so a survivor would be caught by name.
- Glob cap: exactly `max_results` matches followed by non-matching files is `truncated: false` (my pass-1 scenario and the new unit test); `max_results: 0` answers `[]` with `truncated` only when something matched, consistent with "more matched than were answered".
- All four pass-1 red cases and all 17 pass-1 scenarios green on the corrected tree.

## 6. Paths written outside the worktree

None. `$HOME/.cache/tools-wave-t1/toolset-fs/scratch/adversary-2/` was created and holds nothing; the cargo wrapper appended to its own `$HOME/.cache/tools-wave-t1/cargo-serial.log`.

## 7. Findings block (re-emitted by the adversary as JSON after the store refused the YAML form)

```findings
[
  {
    "file": "crates/tool-fs/src/lib.rs",
    "line": 599,
    "category": "boundary",
    "severity": "warning",
    "verdict": "CONFIRMED",
    "origin": "introduced",
    "message": "the counting bound is tested only in the counting branch, so a 16 MiB + 1 file with no newline is scanned whole and fs.read answers lines.total 1 with a digest while fs.stat answers sha256 null, against the contract's \"over 16 MiB: lines.total and sha256 are null\"."
  },
  {
    "file": "crates/tool-fs/src/line.rs",
    "line": 54,
    "category": "boundary",
    "severity": "note",
    "verdict": "CONFIRMED",
    "origin": "pre-existing",
    "message": "next_line reads a newline-free file to its end before the bound is consulted, so the 16 MiB bound limits memory but not scan time, and semantics.md:57 \"the scan stops once more than 16 MiB have been scanned\" holds only for files with a line end past the bound."
  },
  {
    "file": "crates/tool-fs/src/walk.rs",
    "line": 77,
    "category": "judgement",
    "severity": "note",
    "verdict": "CONFIRMED",
    "origin": "pre-existing",
    "message": "a visitor's false stops only the current frame, so a capped glob still visits the rest of the tree after max_results is reached; the answer is unaffected."
  },
  {
    "file": "crates/tool-fs/src/lib.rs",
    "line": 315,
    "category": "judgement",
    "severity": "note",
    "verdict": "CONFIRMED",
    "origin": "introduced",
    "message": "limit 0 and offset 0 are refused before the path is resolved, so a bad window on a path outside the workspace answers fs.InvalidWindow rather than fs.OutsideWorkspace; limit 0 also wins over the past-the-end refusal."
  },
  {
    "file": "crates/tool-fs/src/lib.rs",
    "line": 623,
    "category": "judgement",
    "severity": "note",
    "verdict": "CONFIRMED",
    "origin": "introduced",
    "message": "the first-line byte-ceiling cut still reserves a separator byte on an unterminated line while the ceiling weight no longer charges one, so a one-line unterminated file is cut one character shorter than the terminated rule allows."
  }
]
```
