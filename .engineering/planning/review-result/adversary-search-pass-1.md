---
format: aep.planning-md/3
id: review-result:adversary-search-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:toolset-search: 4 red, a .git file and a NUL-carrying file are searched, three suite gaps closed by its cases'
relations:
- reviews: story:toolset-search
revision: 1
---
```
unit: story:toolset-search (worktree impl/toolset-search: base 1813c93 plus uncommitted work, 2026-10-20)
verdict: red
cases: executed 84→91, red 4
origin: introduced 3, pre-existing 2, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-w3/search/scratch/adversary/ (24 entries, about 588 MB; part 6)
needs-coordinator: yes. The two red scenarios assume the guidance's wording is the rule; you decide whether the fix is in the code or in the wording (the F1 code fix changes tool-fs's walk, so `fs.glob` too). You also decide whether pre-existing findings hold this unit.
```

**1. Diff stat.** `git --no-pager diff --stat` is unchanged by me: Cargo.lock +15, `crates/tool-fs/src/lib.rs` 7, `crates/tool-fs/src/walk.rs` 43. Every file I added is untracked and on an allowed path:

| File | Lines |
|---|---|
| `contracts/search/v1alpha1/scenarios/adversary-held.yaml` | 63 |
| `contracts/search/v1alpha1/scenarios/adversary-red.yaml` | 32 |
| `contracts/search/v1alpha1/fixtures/adversary-sub-link/` (`other/x.txt`, `sub/own.txt`, link `sub/across` → `../other`, link `sub/alias.txt` → `../other/x.txt`) | — |
| `crates/tool-search/tests/adversary_scenarios.rs` (5 tests) | 82 |
| `crates/tool-search/tests/adversary_guidance.rs` (1 test) | 33 |
| `crates/tool-search/tests/adversary_harness_oracle.rs` (1 test) | 569 |

No non-test path is in the diff. Clippy with `-D warnings`, `tool-contract validate contracts/` and rustfmt all exit 0.

**2. Cases, each run alone before the suite** (`cargo test --no-fail-fast -p b10x-tool-search --test adversary_scenarios --test adversary_guidance --test adversary_harness_oracle`):

| Case | Now | Red output, verbatim |
|---|---|---|
| `a_git_file_is_skipped_as_the_guidance_says` | red | `output.matches: expected 1 items, got 2` / `answered: {..."matches":[{"line":1,...,"path":".git","text":"gitdir: ../main/.git/worktrees/impl-search"},{...,"path":"notes.md",...}]...}` |
| `a_binary_file_that_is_valid_utf_8_is_not_searched` | red | `output.matches: expected 1 items, got 2` / `answered: {..."matches":[{...,"path":"blob.dat","text":"needle\u0000\u0000\u0000\u0001\u0002"},{...,"path":"text.txt",...}]...}` |
| 400 vs 401 characters, 2- and 4-byte characters, with context | green | red under M1: `output.matches[0].line_truncated: expected false, got true`; red under M2: `output.matches[0].after[0].text: expected "needleééé…` |
| a file over 1 MiB that the glob does not name | green | red under M6: `output.skipped_large_files: expected 0, got 1` |
| links below `path: sub` that land elsewhere inside | green | red under M3: `output.matches: expected 3 items, got 1` |
| guidance figures checked in their own phrase | green | with the guidance edited to "capped at 6" and "first 21": `"capped at 5;" is not in: …` |
| harness oracle: 80 seeded trees, 960 calls | green | 427 answered, 185 with matches; red under M1, M2 and M6 |

The M probes ran in a scratch copy with env-gated edits (`mutants.patch` in the scratch directory); the tree was not touched. M1: `over` uses `nth(399)`. M2: the cut and `over` count bytes, not characters. M3: the walk's boundary is the search's start, not the root. M6: the glob filter runs after the size check. With my scenarios parked, M1, M2 and M6 each pass all 29 cases the unit shipped. M3 does not: the shipped `no_containment_is_caught…` test catches it.

**3. The suite run**, `cargo test --no-fail-fast -p b10x-tool-fs -p b10x-tool-search`: exit 101, 87 ok, 4 FAILED.
- The two red cases above.
- `conformance::the_suite_passes_against_the_intact_toolset`: `49 of 51 scenarios passed`. The two failures are the two adversary-red scenarios.
- `mutants::each_mutant_fails_at_least_one_scenario`: fails only because the intact run fails those same two scenarios.
- All 48 tool-fs tests give the same results as the coordinator's `baseline-fs.log`.

**4. Findings.** All cover the worktree at base 1813c93 plus the uncommitted work.

| # | File:line | Measured | What reaches it | Verdict / origin | Fix (not applied) |
|---|---|---|---|---|---|
| F1 | `crates/tool-fs/src/walk.rs:112` (against `contract.yaml:79` "The walk skips .git") | A `.git` file is searched and answered; the skip only applies to directories | Every git worktree or submodule root has a `.git` file; this worktree's root `.git` is a 62-byte file. Harness 0.14.1 `local.rs:946` `if path.is_dir() { if SKIPPED.contains…` has the same gate | CONFIRMED / pre-existing | Check the skipped names before the `is_dir` test (fixes `fs.glob` too), or write "the directories .git, …" in both guidances |
| F2 | `contracts/search/v1alpha1/semantics.md:65` | "a binary file among them, is not searched", yet a file with NUL bytes that is valid UTF-8 is searched and its NUL bytes are answered | Nothing found: I built the file | INFEASIBLE / pre-existing (harness `local.rs:691` "Binary or unreadable. Skipping is right") | Drop the phrase, or skip files that contain NUL |
| F3 | `crates/tool-search/src/lib.rs:435` (`over`), `:412` (`cut_to`) | The shipped suite does not hold the 400/401 edge or the character-vs-byte cut (M1 and M2 survive) | Any line of 400 or more characters, or with multi-byte characters | CONFIRMED / introduced | Keep `adversary-held.yaml` |
| F4 | `crates/tool-search/src/lib.rs:343` vs `:354` | The glob-before-size order is not held (M6 survives); harness filters first | Any search with a `glob` in a tree that has a file over 1 MiB | CONFIRMED / introduced | Keep `adversary-held.yaml` |
| F5 | `crates/tool-search/src/tests.rs:573` | The figure check cannot fail for `5` (it is inside `1048576`) or `20` (inside `200`) | The test is the guard for guidance drift | CONFIRMED / introduced | Keep `adversary_guidance.rs`, or match each figure in the phrase that states it |

For F3 to F5 I chose "introduced" over "pre-existing": the gap is in the new conformance suite, and harness had no conformance suite.

**5. Attacked and could not break**
- **Same answers as harness:** the 960 oracle calls cover:
  - links in, out, to the root and dangling
  - skipped directories and a 14-level chain
  - files at both 1 MiB edges
  - CRLF and a lone `\r`
  - non-UTF-8 and NUL files, and a socket

  Outside the documented cap changes, every answer equals harness's, and the errors fall on the same calls.
- **The tool-fs export:** the fs conformance and mutant reports are identical per scenario at the base and in the unit (500 lines after normalising scratch paths). The fs diff only changes visibility, derives and docs.
- **Covered by the shipped suite plus the oracle:** the cap at exactly N and N+1, `max_results: 0`, 1048576 vs 1048577 bytes, context at file edges and overlapping, depth bound, links out (walked and named directly), broken and oversized regex, globs with and without `/`, FIFO.
- **The four changes against harness:** none is a regression for a harness caller.
- **Harness tests dropped or weakened in the port:** none. All 9 search tests are ported with their assertions, and 2 are stronger.
- **Not run:** a slow-but-valid regex on a 1 MiB line. It is pre-existing, and the time bound is the unenforced `timeout_ms` already filed.

**6. Written outside the worktree.** All under `$HOME/.cache/tools-wave-w3/search/scratch/adversary/`:
- Scripts: `gen_held_yaml.py`, `apply_mutants.py`, `probe_mutants.sh`, `compare_fs.sh`
- Copies: `mutant-copy/`, `base-copy/` (a `git archive` of 1813c93), and `copies-target/` (581 MB build directory; I did not delete it)
- `parked/` (empty)
- `mutants.patch`
- Logs: `red-alone.log`, `red-alone-2.log`, `suite.log`, `probe.log`, `clippy.log`, `validate.log`, `fs-base-nocapture.log`, `fs-unit-nocapture.log`
- Comparison files: `fs-base-scenarios.txt`, `fs-unit-scenarios.txt`, `fs-base-norm.txt`, `fs-unit-norm.txt`, `fs-scenarios.diff`, `fs-base.txt`, `all-now.txt`

The wrapper also appended its own lines to `$HOME/.cache/tools-wave-w3/cargo-serial.log`.

**7. Findings block**

```findings
[
  {"file": "crates/tool-fs/src/walk.rs", "line": 112, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The skipped names are only checked for directories, so a worktree's .git file is searched and answered although the search.grep guidance says the walk skips .git."},
  {"file": "contracts/search/v1alpha1/semantics.md", "line": 65, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "semantics.md says a binary file is not searched, but a file with NUL bytes that is valid UTF-8 is searched and its NUL bytes are answered."},
  {"file": "crates/tool-search/src/lib.rs", "line": 435, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No shipped case holds the 400/401-character edge or the character-vs-byte cut; an off-by-one or byte-based line_truncated passes all 29 shipped cases."},
  {"file": "crates/tool-search/src/lib.rs", "line": 343, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No shipped case holds that the glob filter runs before the 1 MiB check; moving it after counts files the glob excludes in skipped_large_files and passes every shipped case."},
  {"file": "crates/tool-search/src/tests.rs", "line": 573, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The guidance-figure test matches substrings, so it cannot fail for 5 (inside 1048576) or 20 (inside 200), and stays green with the guidance edited to capped at 6 and first 21."}
]
```
