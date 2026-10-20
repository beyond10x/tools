---
format: aep.planning-md/3
id: review-result:adversary-search-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:toolset-search: no red case, three suite gaps closed by its cases, one sentence on large binary files'
relations:
- reviews: story:toolset-search
revision: 1
---
```
unit: story:toolset-search (worktree impl/toolset-search: base 1813c93 plus correction round 1, uncommitted, 2026-10-20)
verdict: green
cases: executed 96→100, red 0
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-w3/search/scratch/adversary/ (32 entries, copies-target now 601 MB; part 6)
needs-coordinator: no
```

The two pass-1 red cases are fixed and nothing is red now. I found three gaps in the correction's suite and closed each with a case. The fourth finding is one sentence in the contract that the code contradicts for binary files over 1 MiB.

**Your patch to `adversary_harness_oracle.rs`:** it does only what you said. The diff against `adversary_harness_oracle.rs.before-patch` removes the `nul.dat` write, keeps the random draw as `let _ = rng.chance(30);` so every other tree is unchanged, and updates the doc comment. Nothing else moved.

**1. Diff stat.** `git --no-pager diff --stat` is unchanged (Cargo.lock 15, `crates/tool-fs/src/lib.rs` 7, `crates/tool-fs/src/walk.rs` 43). This pass added two untracked files:
- `contracts/search/v1alpha1/scenarios/adversary-pass2.yaml` (34 lines, 2 scenarios)
- `crates/tool-search/tests/adversary_pass2.rs` (173 lines, 4 tests)

The pass-1 files are as I left them, apart from your patch. Clippy with `-D warnings`, `tool-contract validate contracts/` and rustfmt all exit 0.

**2. Cases, run alone first** (`cargo test --no-fail-fast -p b10x-tool-search --test adversary_pass2`): 4 passed, exit 0.

To show what each case holds, I ran three hypothetical mutants of the corrected code. They live only in the scratch copy (`mutants-2.patch`):
- **M7:** only the first 8000 bytes are checked for a NUL byte.
- **M10:** a skipped name is compared with the workspace-relative path, so only entries at the root are skipped.
- **M14:** a file over 1 MiB is read for a NUL byte before the size check.

Each mutant passes all 41 cases that existed before this pass: 23 unit, 2 conformance, 9 mutants, and my 5 scenarios, 1 guidance and 1 oracle test from pass 1. Each one fails exactly one case from this pass:

| Case | Mutant | Red output under the mutant |
|---|---|---|
| `a_nul_byte_anywhere_makes_a_file_binary_up_to_the_last_byte_of_the_largest_file_read` | M7 | `left: ["last.dat", "past-buffer.dat", "text.txt"]` / `right: ["text.txt"]` |
| `a_submodules_git_file_below_the_root_is_skipped_too` | M10 | `output.matches: expected 2 items, got 4` / answered `src/target` and `vendor/lib/.git` |
| `the_two_kinds_of_binary_file_are_answered_alike_on_both_sides_of_one_mebibyte` | M14 | `["odd-big.dat"]` `left: false` `right: true` |
| `a_skipped_file_below_the_root_named_as_path_is_searched` | — | green; it holds the "path names it" exception below the root |

**3. Suite run**, `cargo test --no-fail-fast -p b10x-tool-fs -p b10x-tool-search`: exit 0, 100 ok, 0 failed. The conformance run has 53 scenarios. The tool-fs counts are the same as before (30/6/4/2/6/7). The search mutants test passes 9 of 9; my pass-1 scenarios kill both new mutants, `no-skipped-file-names` and `nul-is-text`.

**4. Findings**

| # | File:line | Measured | What reaches it | Verdict / origin | Fix (not applied) |
|---|---|---|---|---|---|
| P2-1 | `crates/tool-search/src/lib.rs:397` | The shipped cases only put a NUL at byte 5007, so M7 survives; semantics says "anywhere" | Any binary file whose first NUL is past 8000 bytes | CONFIRMED / introduced | Keep `adversary_pass2.rs` |
| P2-2 | `crates/tool-search/src/lib.rs:362` | The shipped cases only put skipped names at the root, so M10 survives | A submodule checkout has its `.git` file below the root | CONFIRMED / introduced | Keep `adversary-pass2.yaml` |
| P2-3 | `crates/tool-search/src/lib.rs:382` | No shipped case holds that the NUL rule and the non-UTF-8 rule answer the same over 1 MiB, so M14 survives | Any binary file over 1 MiB | CONFIRMED / introduced | Keep `adversary_pass2.rs` |
| P2-4 | `contracts/search/v1alpha1/semantics.md:69` (and `contract.yaml:80`) | The documents say a binary file "is not searched and is not reported, whether the walk reaches it or path names it". A binary file over 1 MiB of either kind is counted in `skipped_large_files`, named in `skipped_large_paths` and sets `truncated: true`. Measured: under M14 the non-UTF-8 file over 1 MiB is still named; the intact code passes the equality test, so it names both | A workspace with a binary file over 1 MiB. None was shown: the only file over 1 MiB in this repo is the text fixture `large-files/over.txt` | INFEASIBLE / introduced (the sentence is new; harness also counted large non-UTF-8 files) | Reword to "A binary file of at most 1 MiB …; a larger one is counted as too large, because its size is checked before it is read", or check the first bytes of a large file for binary before counting it |

**5. Attacked and could not break**
- **Skipped names:**
  - A skipped name at depth 0 and 2.
  - A file named `target`.
  - A link with a skipped name, to a file and to a directory.
  - A skipped entry named as `path`, at the root and below it.
  - `path: target` searching what is under it.
  - A large file with a skipped name is not counted.
  - The tool-fs walk and `fs.glob` are unchanged.
- **NUL bytes:** at the very start, past 64 KiB, as the last byte of a file of exactly 1048576 bytes, and in a file named as `path`. All are not searched and not counted; `\x07` is still text.
- **The two binary rules:** they answer alike on both sides of 1 MiB, whether the walk reaches the file or `path` names it.
- **Guidance and semantics, sentence by sentence:** they match the code, apart from P2-4.
- **The harness oracle:** it still agrees on all 960 calls with your patch.

**6. Written outside the worktree.** All under `$HOME/.cache/tools-wave-w3/search/scratch/adversary/`.
- New in this pass:
  - Scripts and patch: `apply_mutants_2.py`, `probe_mutants_2.sh`, `mutants-2.patch`
  - Logs: `probe-2.log`, `pass2-alone.log`, `suite-2.log`, `clippy-2.log`, `validate-2.log`
- Changed in this pass:
  - `mutant-copy/`, resynced to the corrected tree with the pass-2 mutants applied
  - `copies-target/`, now 601 MB; I did not delete it
  - `parked/` (empty)
- Pass-1 entries are unchanged.

The wrapper also appended its own lines to `$HOME/.cache/tools-wave-w3/cargo-serial.log`. I have released the lease.

**7. Findings block**

```findings
[
  {"file": "crates/tool-search/src/lib.rs", "line": 397, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No shipped case puts a NUL byte past 8000 bytes, so a check limited to the first 8000 bytes passes every case that existed before this pass; closed by adversary_pass2.rs."},
  {"file": "crates/tool-search/src/lib.rs", "line": 362, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Every shipped skipped-name case is at the root, so comparing the skipped names with the workspace-relative path lets a submodule's .git file be searched; closed by adversary-pass2.yaml."},
  {"file": "crates/tool-search/src/lib.rs", "line": 382, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No shipped case holds that a NUL file and a non-UTF-8 file over 1 MiB are answered alike, so reading large files for NUL before the size check passes every case that existed before this pass; closed by adversary_pass2.rs."},
  {"file": "contracts/search/v1alpha1/semantics.md", "line": 69, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "semantics.md and the guidance say a binary file is not reported, but a binary file over 1 MiB is counted in skipped_large_files, named in skipped_large_paths and sets truncated."}
]
```
