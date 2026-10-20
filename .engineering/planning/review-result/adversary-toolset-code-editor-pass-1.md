---
format: aep.planning-md/3
id: review-result:adversary-toolset-code-editor-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:toolset-code-editor: a containment bypass through a trailing-slash spelling, 7 findings'
tags:
- adversary
relations:
- reviews: story:toolset-code-editor
revision: 1
---
Adversary pass 1 on story:toolset-code-editor, as returned by `aep:adversary` on 2026-10-20. The
coordinator replaced the expanded home directory in paths with `$HOME` (public repository rule) and
changed nothing else.

```
unit: story:toolset-code-editor — worktree impl/toolset-code-editor (uncommitted work on base 796937c7450b00c309e3301ead279dd3b171ea05)
verdict: red
cases: executed 21→23 test functions (suite scenarios 51→59), red 4 (2 adversary tests, the conformance lane with 8 red scenarios, 1 mutants-lane knock-on)
origin: introduced 6, pre-existing 1, undecided 0
wrote-outside-worktree: 4 log files under the assigned scratch directory (part 6)
needs-coordinator: no
```

## 1. What I touched

`git --no-pager diff --stat 796937c…` shows only ` Cargo.lock | 20 +` (the implementor's, untracked crate and contract excluded). My additions are untracked, so `git status --short` / `ls-files --others`:

| path (worktree-relative) | lines | kind |
|---|---|---|
| `contracts/code-editor/v1alpha1/scenarios/adversary-trailing-slash.yaml` | 51 | scenario file, 4 scenarios |
| `contracts/code-editor/v1alpha1/scenarios/adversary-semantics.yaml` | 53 | scenario file, 4 scenarios |
| `crates/tool-code-editor/tests/adversary_trailing_slash.rs` | 70 | integration test, 2 cases |

No `src/`, `Cargo.*`, `contract.yaml`, `semantics.md` or existing scenario touched. No existing case changed.

## 2. Cases added (written before anything ran)

**`tests/adversary_trailing_slash.rs`** — a link in the workspace pointing at a file *outside*, named `link/` (and `link/.`); asserts the victim is unchanged, then `code.OutsideWorkspace`. Both **red**. Run alone first (`cargo-serial.sh test -p b10x-tool-code-editor --test adversary_trailing_slash`, exit 101):

```
test an_edit_through_a_link_out_of_the_workspace_spelled_with_a_trailing_slash_is_refused ... FAILED
test a_patch_through_a_link_out_of_the_workspace_spelled_with_a_trailing_slash_is_refused ... FAILED
thread '…edit…' panicked at crates/tool-code-editor/tests/adversary_trailing_slash.rs:40:9:
assertion `left == right` failed: link/: the edit followed the link out of the workspace
  left: "owned\n"
 right: "original\n"
thread '…patch…' panicked at crates/tool-code-editor/tests/adversary_trailing_slash.rs:63:5:
assertion `left == right` failed: the patch followed the link out of the workspace
  left: "owned\n"
 right: "original\n"
test result: FAILED. 0 passed; 2 failed
```

**Scenarios** — each drives the implementation from `semantics.md`. A single scenario file cannot run alone (the runner's coverage rule refuses a suite that does not hold every property for every operation), so their first execution was the conformance lane alone (`--test conformance -- --nocapture`, exit 101, `51 of 59 scenarios passed`). All 8 **red**, verbatim:

```
FAIL adversary-move-onto-a-dangling-link-is-refused-as-exists: expected error code.Exists, got error code.IsSymlink: `link`: `link` is a link that leads nowhere this workspace can write (No such file or directory (os error 2))
FAIL adversary-edit-an-old-that-fits-at-two-overlapping-places-is-ambiguous: expected error code.Ambiguous, got ok: {"bytes":3,"diff":"--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-aaa\n+ba\n","path":"a.txt","sha256":"8bca2b27…"}
FAIL adversary-edit-an-old-equal-to-new-still-answers-a-non-empty-diff: output.diff: expected a length of at least 1, got 0
FAIL adversary-patch-a-non-empty-patch-that-changes-nothing-answers-a-non-empty-diff: output.diff: expected a length of at least 1, got 0
FAIL adversary-edit-a-link-spelled-with-a-trailing-slash-still-lands-under-the-targets-own-name: output.path: expected "real.txt", got "link"
FAIL adversary-patch-a-link-spelled-with-a-trailing-slash-still-lands-under-the-targets-own-name: output.path: expected "real.txt", got "link"
FAIL adversary-delete-a-link-spelled-with-a-trailing-slash-is-still-refused-by-name: expected error code.IsSymlink, got ok: {"path":"link"}
FAIL adversary-move-a-link-spelled-with-a-trailing-slash-still-moves-its-target: output.from: expected "real.txt", got "link"
```

## 3. Suite run

`cargo-serial.sh test -p b10x-tool-code-editor --no-fail-fast` (a first run without the flag stopped at the first failing binary; log 03), exit 101:

| lane | result |
|---|---|
| `src/lib.rs` unit tests | 18 passed |
| `tests/adversary_trailing_slash.rs` | 0 passed, 2 failed |
| `tests/conformance.rs` | failed: `51 of 59 scenarios passed` (the 8 above) |
| `tests/mutants.rs` | `each_mutant_fails_at_least_one_scenario` ok; `the_ignored_precondition_fails_exactly_the_stale_digest_scenarios` FAILED at mutants.rs:76 — knock-on: my 8 red scenarios appear in its failed list beside the 5 digest ones |

## 4. Findings (tree: the worktree above, uncommitted)

| # | measured | reaches it | verdict / origin / sev |
|---|---|---|---|
| 1 | `src/lib.rs:552-553`: `resolve` delegates to `resolve_new`; for `link/` or `link/.` lstat fails (ENOTDIR on a link to a file), the walk steps past the link and rebuilds `<root>/link` uncanonicalised. `read_text` then reads and `fs::write`s through it: victim outside the workspace rewritten (test :40, :63); inside, `output.path`/`from` is `link` not `real.txt` and `code.move` moves the link, not its target (semantics.md:15, :88) | any caller of `code.edit`/`code.patch` choosing the path string; the configuration (a link inside to a file outside) is the one `src/tests.rs::a_link_to_a_file_outside_the_workspace_does_not_get_to_overwrite_it` exists to guard. harness local.rs:203-214 canonicalised the whole candidate, so `link/` was ENOTDIR or outside there | CONFIRMED / introduced / **blocker** |
| 2 | `src/lib.rs:518`: `delete`'s link guard lstats `root.join(relative)`; `link/` → ENOTDIR → not a link → `remove_file` unlinks it, `ok {path: link}`; semantics.md:96 says `code.IsSymlink` by the caller's name | any `code.delete` with `link/`. Fixing #1 alone makes this worse: `link/` canonicalised to `real.txt` passes this guard and the target is removed through the link | CONFIRMED / introduced / warning |
| 3 | `src/lib.rs:490` vs `semantics.md:89`: `to` naming a dangling link → `code.IsSymlink`; the document says `code.Exists` ("a link, dangling or not") | any `code.move` whose `to` is a dangling link | CONFIRMED / introduced / warning |
| 4 | `src/lib.rs:772`: `match_indices` counts non-overlapping; `old: aa` in `aaa\n` counts 1 and is applied at offset 0 though it fits at two places; semantics.md:54 "exactly once" | any `old` that overlaps itself (`))` in `)))`, a run of blank lines) | CONFIRMED / **pre-existing** (harness local.rs:522 `text.matches(old).count()`, same non-overlapping count) / note |
| 5 | `semantics.md:61`, `:81`: "`diff` is never empty" / "cannot happen" — `old == new` answers `diff: ""` with `applied: 1` | any edit or patch with `old == new` | CONFIRMED / introduced / note |
| 6 | `tests/mutants.rs:78`: asserts every failure under the mutant contains "digest", assuming the unmutated suite is all green; it went red for my scenarios, not for the precondition | any red scenario anywhere in the suite | CONFIRMED / introduced / note |
| 7 | `src/lib.rs:636`: a UTF-8 decode failure is answered as `code.Io`, whose summary is "The operating system refused" (by reading; no case) | any edit of a binary file | CONFIRMED / introduced / note |

**Named fixes, not applied:** (1) canonicalise the whole candidate when it exists, as harness did, or normalise `/` and `/.` spellings before the walk, plus a belt-and-braces lstat in `read_text` and `rename` as `write` has; (2) `delete` lstats the normalised final component, not `root.join(relative)`; (3) `rename` lstats `root.join(to)` before `resolve_new` → `Exists`, or semantics.md:89 says `IsSymlink`; (4) `locate` advances one byte per find, or semantics.md:54 says non-overlapping; (5) semantics.md:61/:81 add "empty when `old` equals `new`"; (6) compare the mutant's failures against the unmutated run's.

## 5. Attacked, not broken

- Precondition: stale, missing file (`actual` absent), uppercase, wrong length — refused as mismatch with the file unchanged, consistent with "lowercase hex" (read + existing scenarios).
- Patch: one-byte overlap, identical, nested, adjacent, match-only-after-another, triple ranges — `apply_against_original` sort+windows is correct (read).
- Start/end-of-file matches, CRLF-spanning `old`, diff for one-line/no-trailing-newline files (read; `similar` emits the no-newline hint).
- `code.write` to a link, through a link parent, parent-is-a-file, `path: ""`, `/` — refused or lands correctly (read).
- `code.move` onto itself, onto a directory, missing parent, from a link inside — as documented (read).
- Mutants: each fails ≥3 scenarios through the suite (`each_mutant_fails` ok in my run).
- Harness port: all 9 write/edit/link tests present, assertions strengthened (message word → error name); none dropped. No test-that-cannot-fail shape in `src/tests.rs`.

## 6. Paths written outside the worktree

```
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary/01-rust-case-alone.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary/02-conformance-lane.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary/03-full-suite.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary/04-full-suite-no-fail-fast.log
```
(`cargo-serial.sh` also appended to its own shared `$HOME/.cache/tools-wave-t1/cargo-serial.log`.) Test tempdirs went under the worktree's `target/tmp/adversary/`. Lease released with `session-end`.

```findings
[
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 552, "category": "acceptance", "severity": "blocker", "verdict": "CONFIRMED", "origin": "introduced", "message": "resolve() rebuilds an uncanonicalised <root>/link for a path spelled link/ or link/., so code.edit and code.patch read and write through a link to a file outside the workspace, and inside the reply names the link instead of its target."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 518, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "delete's link guard lstats the spelled path, so link/ is not seen as a link and is removed with outcome ok where semantics.md:96 promises code.IsSymlink."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 490, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "code.move onto a dangling link answers code.IsSymlink where semantics.md:89 says code.Exists for a link dangling or not."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 772, "category": "property", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "match_indices counts non-overlapping occurrences, so old=aa in aaa counts once and is applied at offset 0 although it fits at two places; harness local.rs:522 counted the same way."},
  {"file": "contracts/code-editor/v1alpha1/semantics.md", "line": 61, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The document says the diff is never empty (and at :81 cannot be empty with a non-empty edits) but old == new answers diff \"\" with applied 1."},
  {"file": "crates/tool-code-editor/tests/mutants.rs", "line": 78, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The test assumes the unmutated suite is all green, so any red scenario makes it fail for a reason unrelated to the precondition; compare against the unmutated run's failures instead."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 636, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A UTF-8 decode failure is reported as code.Io, whose contract summary is an operating-system refusal (by reading, no case run)."}
]
```
