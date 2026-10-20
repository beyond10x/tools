---
format: aep.planning-md/3
id: review-result:adversary-fs-glob-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:fs-glob-stops-at-the-cap: no red case, 1 note (the story said the answer is unchanged; depth_bound_reached changes)'
relations:
- reviews: story:fs-glob-stops-at-the-cap
revision: 1
---
```
unit: story:fs-glob-stops-at-the-cap (worktree impl-fs-glob, uncommitted on base bf8deee)
verdict: green
cases: executed 44→48, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: 1 directory, $HOME/.cache/tools-wave-w2/fs-glob/scratch/adversary (the assigned scratch, including copy/ with its own target/, 365M); the wrapper also appended lines to $HOME/.cache/tools-wave-w2/cargo-serial.log
needs-coordinator: no
```

**1. Diff**

```
 crates/tool-fs/src/lib.rs   |  4 +-
 crates/tool-fs/src/tests.rs | 89 +++++++++++++++++++++++++++++++++++++++++++++
 crates/tool-fs/src/walk.rs  | 65 +++++++++++++++++++++++++--------
 3 files changed, 141 insertions(+), 17 deletions(-)
?? crates/tool-fs/tests/adversary_w2_glob.rs
```

The three `src/` files are the implementor's changes; I left them alone. My only addition is the untracked test file, 401 lines, which `diff --stat` does not list. I changed no non-test path.

**2. Cases added** in `$HOME/.local/state/worktree/trees/b10x/tools/impl-fs-glob/crates/tool-fs/tests/adversary_w2_glob.rs`. All four were green the first time they ran alone, so there is no red output to quote.

| case | what it asserts | now |
|---|---|---|
| `w2_with_no_max_results_the_501st_match_stops_the_walk_at_exactly_500_paths` | 501 matches across `a/` and `b/` answer exactly the first 500 in walk order, with `truncated: true`. A `c/` deeper than 12 after the stop gives `depth: false`. Asking `max_results: 600` gives the same answer. With 500 matches: `truncated: false`, `depth: true` | green |
| `w2_a_stop_inside_the_deepest_directory_the_walk_enters` | a stop inside `d1/…/d12` with the 13th directory after the stop, before it, or between the last answer and the stop; and `max_results: 2`, where there is no stop | green |
| `w2_skipped_directories_and_links_on_either_side_of_the_stop` | `.git`, `node_modules` and `target` (with deep trees inside), a link to a directory outside, a link to a file outside, a link that leads nowhere, and a link to a directory inside, each placed before or after the stop, at caps 1, 2, 3 and none | green |
| `w2_every_cap_over_seeded_random_trees_answers_what_the_uncapped_walk_owes` | 150 seeded random trees, all 150 checked, including deep chains, skipped directories, links in, out and nowhere, and a `loop` link back to the root. For every cap from 0 to matches+1, the answer must equal what an uncapped walk written from the semantics owes. Takes about 12 s | green |

To check these cases can fail, I ran them against a scratch copy of the workspace, swapping in different sources each time:

| sources in the copy | red of 4 | how it failed |
|---|---|---|
| base `bf8deee` | 3 | only `depth_bound_reached`, e.g. `left: (["a.txt"], true, true)` / `right: (["a.txt"], true, false)` |
| `walk.rs:107` `\|=` changed to `=` | 1 | the property test |
| `walk.rs:108` stop not passed to the parent | 3 | depth flag |
| head (the unit's sources) | 0 | |

My first version of the file failed `clippy::case_sensitive_file_extension_comparisons` at my own line 225. I changed the check to `Path::extension() == "txt"`, ran each case alone again (green) and then ran the suite.

**3. Suite run**: `cargo-serial.sh test -p b10x-tool-fs`. One earlier attempt was stopped by the wrapper's disk floor (`killed at 24G`, exit 143). I re-ran it once 35G was free:

```
unittests src/lib.rs   30 passed
tests/adversary.rs      6 passed
tests/adversary_w2_glob 4 passed
tests/conformance.rs    2 passed
tests/mutants.rs        6 passed
Doc-tests               0 passed
SUITE EXIT=0
```

`clippy -p b10x-tool-fs --all-targets --locked -D warnings` exits 0, and `rustfmt --check` on my file exits 0.

**4. Judgement finding** (worktree impl-fs-glob on bf8deee)

- **J1**, `.engineering/planning/story/fs-glob-stops-at-the-cap.md:23`, note, CONFIRMED, introduced.
  - **Measured:** the story says "the answer is unchanged", but on the base copy 3 of my 4 cases fail, and only on `depth_bound_reached` with `truncated: true`. `paths` and `truncated` are the same as on base.
  - **Reached by:** any capped `fs.glob` (`max_results`, or more than 500 matches) over a tree where a directory deeper than 12 comes after the match past the cap. harness and metaharness pin 0.1.0 (`CHANGELOG.md:13`).
  - **Fix, not applied:** reword the story's Outcome to say that only `depth_bound_reached` changes, and name the change in the 0.2.0 changelog. Both files are the coordinator's.
  - **Why only crate tests pin it:** no v1alpha1 scenario can fix this answer without making the released 0.1.0 implementation fail, and `AGENTS.md` says a change like that needs a new contract version.

**5. Attacked and could not break**
- Other callers of `walk`: there are none besides `lib.rs:500`.
- `truncated` at exactly the cap, at one more, at the 500 ceiling, and with a 600 ask capped to 500: correct.
- A stop in the deepest directory with the depth bound before, between or after it: correct.
- Skipped directories and links on either side of the stop: correct.
- Order of `paths`: always the start of the uncapped walk's order, and identical to base.
- `VISITED` counter: it exists in test builds only. `clippy --lib`, with and without `--features mutants`, exits 0 without it.
- Conformance and mutant results, scenario by scenario, are identical on base and head once temp-dir names are normalized. The mutants fail 11, 12, 7 and 1 of 70 scenarios in both.

**6. Paths written outside the worktree**
- `$HOME/.cache/tools-wave-w2/fs-glob/scratch/adversary/`, containing:
  - logs: `alone-*.log`, `alone2-*.log`, `build.log`, `clippy*.log`, `suite*.log`, `probe-*.log`
  - scripts: `probe.sh`, `probe-suite.sh`
  - comparison files: `mutant-lines-*.txt`, `scenario-lines-*.txt`, `scenario-norm-*.txt`
  - `copy/`, the scratch workspace copy with its own `target/`, 365M, left in place
- `$HOME/.cache/tools-wave-w2/cargo-serial.log` (lines appended by the wrapper)

**7. Findings**

```findings
[{"file": ".engineering/planning/story/fs-glob-stops-at-the-cap.md", "line": 23, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The Outcome says the answer is unchanged, but a capped glob's depth_bound_reached now differs from 0.1.0 whenever the depth bound lies past the stop, and no changelog line or v1alpha1 scenario records that."}]
```
