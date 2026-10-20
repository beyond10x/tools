---
format: aep.planning-md/3
id: review-result:adversary-toolset-fs-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:toolset-fs: 4 red cases, 6 findings (one pre-existing from harness)'
tags:
- adversary
relations:
- reviews: story:toolset-fs
revision: 1
---
Adversary pass 1 on story:toolset-fs, as returned by `aep:adversary` on 2026-10-20. The coordinator
replaced the expanded home directory in paths with `$HOME` (public repository rule) and changed
nothing else.

```
unit: story:toolset-fs
verdict: red
cases: executed 29→34 test functions (scenarios 44→61), red 4 of mine (6 total: the suite's own intact-toolset test and the mutants' intact precondition go red with them)
origin: introduced 3, pre-existing 1, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-t1/toolset-fs/scratch/adversary/gen-long-line.py (generator, deleted after use; directory now empty)
needs-coordinator: no
```

Findings cover the working tree of `$HOME/.local/state/worktree/trees/b10x/tools/impl-toolset-fs` (branch `impl/toolset-fs`, uncommitted, base `796937c`).

## 1. Diff stat

`git --no-pager diff --stat`: only ` Cargo.lock | 70 +` (the implementor's, untouched by me). Everything of mine is untracked; `git status --short` shows `?? contracts/fs/` and `?? crates/tool-fs/`. My files, all test/scenario/fixture paths:

| path | lines |
|---|---|
| `crates/tool-fs/tests/adversary.rs` | 164 |
| `contracts/fs/v1alpha1/scenarios/adversary-glob.yaml` | 45 (3 scenarios) |
| `contracts/fs/v1alpha1/scenarios/adversary-read.yaml` | 64 (6) |
| `contracts/fs/v1alpha1/scenarios/adversary-links.yaml` | 85 (7) |
| `contracts/fs/v1alpha1/scenarios/adversary-long-line.yaml` | 19 (1, generated) |
| `contracts/fs/v1alpha1/fixtures/adversary-links/{real/keep.txt, alias -> real, nowhere -> missing.txt}` | fixture |
| `contracts/fs/v1alpha1/fixtures/adversary-long-line/long.txt` | 2507 bytes |

No non-test path touched. `cargo fmt --check`, `cargo clippy --all-targets -D warnings` and `validate contracts/` pass with my files in place.

## 2. Cases added and their red output (each run alone, before the suite)

**F1** `adversary-a-glob-with-exactly-max-results-matches-is-not-truncated` (adversary-glob.yaml:5; test `a_glob_with_exactly_max_results_matches_is_not_truncated`). Fixture `a.rs`, `b.txt`; `fs.glob {glob: "*.rs", max_results: 1}`; expects `truncated: false`. RED:
```
FAIL adversary-a-glob-with-exactly-max-results-matches-is-not-truncated: output.truncated: expected false, got true
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
```

**F4** `adversary-a-path-through-a-file-does-not-exist-and-is-not-found` (adversary-read.yaml:5; test `a_path_through_a_file_does_not_exist_and_is_not_found`). `fs.read {path: a.txt/x}` where `a.txt` is a file; expects `fs.NotFound`. RED:
```
FAIL adversary-a-path-through-a-file-does-not-exist-and-is-not-found: expected error fs.NotFound, got error fs.Io: `a.txt/x`: Not a directory (os error 20)
```

**F2** `a_file_one_byte_under_the_counting_bound_is_counted_and_digested_whole` (adversary.rs:100). A 16,777,215-byte file (16 MiB − 1) without a trailing newline; expects `lines.total: Some(16384)` and `read.sha256 == stat.sha256`. RED:
```
assertion `left == right` failed: a file under 16 MiB has a known line count; lines_counted_to Some(16384), note Some("line counting stopped after 16777216 bytes of `under.txt`, at line 16384, so how many lines it has is not known. ...")
  left: None
 right: Some(16384)
```

**F3** `a_window_one_below_its_start_is_only_an_empty_file_read_from_line_one` (adversary.rs:140). `fs.read {offset: 1, limit: 0}` on a 3-line file; asserts the contract's `fs.Window.to` sentence. RED:
```
assertion `left == right` failed: limit 0 at offset 1 answered lines Window { from: 1, to: 0, total: Some(3) } with text ""
  left: (1, Some(3))
 right: (1, Some(0))
```

**Green now (15 scenarios, test `the_other_adversary_scenarios_hold`, asserts exactly 15 ran):** depth bound pinned at exactly 12 (file under 12 directories found, under the 13th not; the existing fixture at glob.yaml:586 is 14 deep and is satisfied by any bound in 3..=13); all five SKIPPED directories (the suite had only `target` and `node_modules`); a 2500-char line cut to 2000 and named in `truncated_lines` with the whole-file digest (semantics.md:41, no scenario held it); byte ceiling at exactly its value (8 bytes under `max_bytes: 8` whole, 7 cuts; pins `<=` at lib.rs:585, which no scenario did); offset on the last line answered and offset last+1 refused (pins `>` at lib.rs:321; a `>=` mutant passed every existing scenario and every unit test); missing `path` is `fs.InvalidInput`; dangling link: `fs.stat` and `fs.read` NotFound, `fs.list` shows `symlink`; link to a directory inside: `fs.list` under the target's name, `fs.stat` through it under the target's path, `fs.stat` of it as the link, `fs.glob` under the link's own path.

## 3. Suite run

`cargo-serial.sh test -p b10x-tool-fs --no-fail-fast` (after the cases existed; first non-`--no-fail-fast` run exited 101 at the `adversary` binary):
```
unittests src/lib.rs: 22 passed
tests/adversary.rs:   1 passed; 4 failed   (F1, F2, F3, F4 above)
tests/conformance.rs: 1 passed; 1 failed   the_suite_passes_against_the_intact_toolset: 59 of 61 scenarios passed
tests/mutants.rs:     4 passed; 1 failed   each_mutant_fails_at_least_one_scenario: "the intact toolset must pass before a mutant's failure means anything"
error: 3 targets failed
```
The four `killing()` tests stay green: each declared mutant still fails its named scenario.

## 4. Findings

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F1 | `crates/tool-fs/src/lib.rs:468` | `paths.len() >= limit` is checked on every visited file before the glob is matched, so the first non-matching file after the `limit`-th match sets `truncated: true` with nothing cut; contract.yaml:218 says `truncated` means "More paths matched than were answered" | any `fs.glob` with `max_results` where exactly that many files match and another file follows in walk order (`max_results: 1` as an existence probe is the common shape); the default 500 the same way | CONFIRMED, warning | pre-existing: snapshot local.rs:785-788 has the same check before `filter.matches` at :790 |
| F2 | `crates/tool-fs/src/lib.rs:567,572` and `contracts/fs/v1alpha1/contract.yaml:75` | `scanned += line.length + 1` charges a separator the last line does not have, and `>=` stops at the bound inclusive; a 16 MiB − 1 file with no trailing newline (and a file of exactly 16 MiB) is consumed to its last byte and then answered `lines.total: null`, `sha256: null`, `truncated: true`, while `fs.stat` (lib.rs:523, `< 16 MiB`) digests the same file; contract says "over 16 MiB", stat guidance says "under 16 MiB" / "larger file" | a read of any file whose size lands in that one-byte band; a consumer comparing `paging.digest` between read and stat trips on it | CONFIRMED, warning | introduced: the arithmetic is harness's (snapshot :336, :341) but harness never claimed "over"; the contract sentence and `fs.stat` are the unit's |
| F3 | `contracts/fs/v1alpha1/contract.yaml:23` (`fs.Window.to`) vs `crates/tool-fs/src/lib.rs:376` | `limit: 0` answers `lines: {from: 1, to: 0, total: 3}` on a non-empty file; the contract says `to` one below `from` "is only an empty file read from line 1" | a caller passing `limit: 0` to get `bytes`, `lines.total` and `sha256` without text, which the input allows and nothing refuses | CONFIRMED, warning | introduced (harness had the same claim as a code comment, snapshot :409-410 over :417; the unit made it a contract sentence) |
| F4 | `crates/tool-fs/src/error.rs:154` via `lib.rs:226` | `a.txt/x` canonicalises to ENOTDIR, `FsError::io` maps only `NotFound` to `fs.NotFound`, so the answer is `fs.Io: Not a directory`; semantics.md:15 says a path that lands inside and does not exist is `fs.NotFound` | a model mistyping a path through a file (`src/lib.rs/foo`) | CONFIRMED, warning | introduced: harness returned an unclassified string (snapshot :205-207); the error classification is the unit's |
| J1 | `contracts/fs/v1alpha1/semantics.md:23` | "an unknown [field] is `fs.InvalidInput`" cannot be held by any scenario: the runner refuses a `call.input` key that is not an input field at coverage (conformance.rs:525-531, profile line 181); only a unit test could hold it, and none does | the semantics' own claim that each guarantee has a scenario (semantics.md:4-5) | INFEASIBLE, note | introduced |
| J2 | `crates/tool-fs/src/lib.rs:288` | the `.min(MAX_READ_BYTES_CEILING)` cap and the 16 MiB counting bound are held by unit tests only; the largest fixture is 71,680 bytes and no scenario asks `max_bytes` above 1024, so a mutant dropping the 262,144 cap passes the suite | the acceptance line "the suite checks" the bounds; nothing else measures the cap | CONFIRMED, note | introduced |

Named fixes, not applied: F1 — in `glob`'s visitor, test the limit only after `filter.matches` (count matches, not visits), or set `truncated` when a *matching* path is found past the limit. F2 — either charge the separator only when `next_line` saw one and use `>` (then the stat bound and the contract's "over" agree), or reword contract.yaml:75 and the stat guidance to "16 MiB or more"; the unit test comment at tests.rs:598 ("the read would not have finished counting either") is wrong either way. F3 — refuse `limit: 0` as `fs.InvalidWindow`, or drop "which is only an empty file read from line 1" from `fs.Window.to`. F4 — map `io::ErrorKind::NotADirectory` to `NotFound` in `FsError::io` (or to `NotADirectory` on the operations that declare it) and keep semantics.md:15 as written. J2 — a fixture over 256 KiB or a generated one.

## 5. Attacked and not broken

- Whole-file `sha256` on a middle window, an empty file, a CRLF file (raw bytes incl. `\r` are digested, line.rs:64), a cut long line: all equal the file's digest.
- `truncated` at the exact byte ceiling, first line over the ceiling, `offset` 0 / past end / on the last line; `to`/`from`/`total` on each.
- Containment: climb-out, absolute path, link out via read/list/glob/stat, glob's walk re-check (walk.rs:69), dangling links, links to a directory inside; `$absent` on `bytes: null` is accepted by the runner (conformance.rs:849) so the symlink entries match the contract's `Optional<Integer>`.
- `/`-glob against the whole relative path vs bare-name glob; SKIPPED five; depth bound (now pinned at 12).
- The four declared mutants each fail 3 to 7 scenarios; none is killed only by a unit test.
- Harness port: all 14 read/list/find/link/line tests of snapshot `mod tests` (:1255-2085) are present in tests.rs with their assertions kept or strengthened; none dropped or weakened.
- Schema check, `validate contracts/`, fmt and clippy pass with the adversary files in place.

## 6. Paths written outside the worktree

`$HOME/.cache/tools-wave-t1/toolset-fs/scratch/adversary/gen-long-line.py` — written, run once with `python3 -I`, deleted; the directory is empty. The cargo wrapper appended to its own `$HOME/.cache/tools-wave-t1/cargo-serial.log`.

## 7. Findings block

```findings
- file: crates/tool-fs/src/lib.rs
  line: 468
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: fs.glob sets truncated when the first file visited after the limit-th match does not match, so exactly max_results matches answers truncated true against the contract's "More paths matched than were answered".
- file: crates/tool-fs/src/lib.rs
  line: 572
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a 16 MiB - 1 file with no trailing newline (and one of exactly 16 MiB) is scanned to its last byte yet answered lines.total null and sha256 null while fs.stat digests it, against the contract's "over 16 MiB" and "under 16 MiB".
- file: contracts/fs/v1alpha1/contract.yaml
  line: 23
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: limit 0 answers lines.to one below from on a non-empty file, so fs.Window.to's "which is only an empty file read from line 1" is false.
- file: crates/tool-fs/src/error.rs
  line: 154
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a path through a regular file (a.txt/x) is answered fs.Io "Not a directory" where semantics.md says a path inside the root that does not exist is fs.NotFound.
- file: contracts/fs/v1alpha1/semantics.md
  line: 23
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the guarantee that an unknown input field is fs.InvalidInput cannot be held by any scenario because the runner refuses unknown call.input keys at coverage, and no unit test holds it either.
- file: crates/tool-fs/src/lib.rs
  line: 288
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the 262144 byte-ceiling cap and the 16 MiB counting bound are held by unit tests only; the largest fixture is 71680 bytes and no scenario asks max_bytes above 1024, so a mutant dropping the cap passes the suite.
```
