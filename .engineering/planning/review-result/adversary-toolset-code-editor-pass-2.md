---
format: aep.planning-md/3
id: review-result:adversary-toolset-code-editor-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:toolset-code-editor: pass-1 findings resolved, 1 red note case, 3 notes'
tags:
- adversary
relations:
- reviews: story:toolset-code-editor
revision: 1
---
Adversary pass 2 on story:toolset-code-editor, as returned by `aep:adversary` on 2026-10-20 after
correction round 1. The coordinator replaced the expanded home directory in paths with `$HOME`
(public repository rule) and changed nothing else.

```
unit: story:toolset-code-editor, pass 2 — worktree impl/toolset-code-editor (uncommitted, base 796937c7450b00c309e3301ead279dd3b171ea05)
verdict: red (1 note-severity case; nothing holds the unit)
cases: executed 28→35 test functions (suite scenarios 59→65), red 1
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 4 log files under the assigned pass-2 scratch directory (part 6)
needs-coordinator: no
```

**Pass-1 findings:** all 7 resolved — my 2 tests and 8 scenarios are green (65 of 65), `mutants.rs` compares against an intact run, `code.NotText` is declared; none carried; 1 new (below).

## 1. What I touched

`git --no-pager diff --stat 796937c…` still shows only ` Cargo.lock | 20 +` (implementor's). Untracked, mine (pass 1 files unchanged in substance; `adversary_trailing_slash.rs` was rustfmt'd by the implementor, 70→74 lines, same assertions):

| path | lines | kind |
|---|---|---|
| `crates/tool-code-editor/tests/adversary_pass2.rs` | 287 | 7 tests (6 green probes, 1 red) |
| `contracts/code-editor/v1alpha1/scenarios/adversary-pass2.yaml` | 76 | 6 scenarios, all green |

No `src/`, `Cargo.*`, `contract.yaml`, `semantics.md` or existing scenario touched.

## 2. Cases added (written before anything ran)

Six probes executed the OS-dependent paths the coordinator named; all green on first run (`--test adversary_pass2`, exit 0, log 01). Then one more case, run alone (`--test adversary_pass2 a_delete_of_a_link_is_refused_by_name_whatever_it_points_at`, exit 101, log 02), **red**:

```
thread 'a_delete_of_a_link_is_refused_by_name_whatever_it_points_at' panicked at crates/tool-code-editor/tests/adversary_pass2.rs:265:9:
assertion `left == right` failed: link: `link` resolves outside the workspace and was refused
  left: "code.OutsideWorkspace"
 right: "code.IsSymlink"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out
```

Scenarios (`adversary-pass2.yaml`): trailing `/` on a directory for write/edit/delete (`NotAFile {path: "dir/"}`) and move-to (`Exists`), identity edit beside a real one (`applied: 2`, hunk only for the real one), `old == new` answers exactly `--- src/a.txt\n+++ src/a.txt\n`. First execution in the conformance lane: `65 of 65 scenarios passed`, exit 0 (log 03).

## 3. Suite run

`cargo-serial.sh test -p b10x-tool-code-editor --no-fail-fast`, exit 101 (log 04):

| lane | result |
|---|---|
| `src/lib.rs` unit tests | 23 passed (9 harness-ported still present) |
| `tests/adversary_trailing_slash.rs` | 2 passed |
| `tests/adversary_pass2.rs` | 6 passed, 1 failed (the case above) |
| `tests/conformance.rs` | 1 passed, 65 of 65 scenarios |
| `tests/mutants.rs` | 2 passed (`each_mutant_fails_at_least_one_scenario_the_intact_editor_passes`, `the_ignored_precondition_fails_exactly_…`) |

## 4. Findings (tree: the worktree above, uncommitted)

| # | measured | reaches it | verdict / origin / sev |
|---|---|---|---|
| 1 | `src/lib.rs:513`: `delete` calls `resolve` before `refuse_link`, so a link pointing outside answers `code.OutsideWorkspace`; `semantics.md:105-106` says a link is `code.IsSymlink` "by the name the caller gave … whatever it points at". Nothing is removed either way (test :265) | any `code.delete` naming a link whose target is outside the workspace | CONFIRMED / introduced / note |
| 2 | `src/tests.rs:669-674`: the test writes a fixed-name marker `../code-editor-move-marker.txt` into the tempdir's parent (the shared `TMPDIR`) and removes it; two concurrent runs race on one file and one `expect` panics (by reading) | any two `cargo test` runs of this crate at once, e.g. two worktrees | CONFIRMED / introduced / note |
| 3 | `src/lib.rs:832-834`: the identity diff is two header lines and no hunk; a consumer that feeds `diff` to `patch(1)` or `git apply` gets nothing applicable (whether they reject it was not run) | any consumer applying `diff` rather than displaying it | CONFIRMED / introduced / note |
| 4 | `src/lib.rs:794` `normalise` + `semantics.md:16`: `code.write` to `new/` (absent) creates a regular file `new`; a trailing `/` on a path to *create* reads as "directory" to a caller, and the write silently makes a file | any `code.write` whose path ends in `/` and does not exist | CONFIRMED / introduced / note |

**Named fixes, not applied:** (1) in `delete`, `refuse_link(root.join(normalise(relative)))` before `resolve`, or semantics.md:105 says a link out is `code.OutsideWorkspace`; (2) make the workspace a subdirectory of the tempdir so `..` lands inside it; (3) document that `diff` is for reading, or return `""` for an identity change and keep `applied`; (4) refuse `NotAFile` for a *new* path spelled with a trailing `/`, or keep and say so under `code.write`.

## 5. Attacked, not broken

- Link in the middle of a path to an outside file: 5 spellings (`outdir/deep.txt`, `/./`, trailing `/`, `outdir/../victim.txt` with and without the file) × 6 operations, all `OutsideWorkspace`, nothing touched outside (probe green).
- `to` whose directory lands outside while `to` is absent: `outdir/new.txt`, `outdir/new.txt/`, `outdir/deep.txt`, `outdir/..`, `outdir/../new.txt`, `../new.txt` — all `OutsideWorkspace`, nothing created (probe green).
- Trailing `/`, `/.`, `/./` on a directory: `NotAFile` for write/edit/delete/move-from, `Exists` for move-to (probe + 4 scenarios green).
- Header-only diff vs `applied`: identity + real → `applied 2`, one hunk; identity alone → `applied 1`, header only (probe + 2 scenarios green).
- Overlapping count over multi-byte text: `日日日日`/`日日`→3, `aéaéa`/`aéa`→2, `éaéa`/`éa`→2, `日本日本語`/`日本`→2, `ééé`/`éé`→2, `éé`/`éé` unique (probe green).
- `NotText` with invalid bytes after the match: refused, bytes unchanged (probe green).
- Mutant set-difference with a red intact scenario: by reading `mutants.rs` — an unrelated red is in both sets and cancels; a red scenario that *expects* `PreconditionFailed` drops out of the difference and the equality fails, which is the right outcome. Not run.
- `link/..`, `sub/..`, `..` tails, absolute paths, `""`/`.`: refused or `NotAFile` by reading `normalise`/`resolve_new`; no case run.

## 6. Paths written outside the worktree

```
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary-2/01-rust-cases-alone.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary-2/02-delete-case-alone.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary-2/03-conformance-lane.log
$HOME/.cache/tools-wave-t1/toolset-code-editor/scratch/adversary-2/04-full-suite-no-fail-fast.log
```
(`cargo-serial.sh` appended to its shared `$HOME/.cache/tools-wave-t1/cargo-serial.log`.) Test tempdirs under the worktree's `target/tmp/adversary-2/`. Lease released with `session-end`.

```findings
[
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 513, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "code.delete resolves before refusing a link by name, so a link pointing outside answers code.OutsideWorkspace where semantics.md:105 promises code.IsSymlink whatever it points at."},
  {"file": "crates/tool-code-editor/src/tests.rs", "line": 669, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The move test writes and removes a fixed-name marker in the shared tempdir parent, so two concurrent test runs race on one file."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 832, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The identity diff is two header lines with no hunk, which a consumer applying diff rather than displaying it cannot use (not run against patch or git apply)."},
  {"file": "crates/tool-code-editor/src/lib.rs", "line": 794, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "normalise makes code.write to an absent path spelled new/ create a regular file named new, where the trailing slash reads as a directory to the caller."}
]
```
