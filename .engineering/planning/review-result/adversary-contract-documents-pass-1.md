---
format: aep.planning-md/3
id: review-result:adversary-contract-documents-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:contract-documents-without-an-implementation: no red case, 2 notes (build-script name grammar, byte order)'
relations:
- reviews: story:contract-documents-without-an-implementation
revision: 1
---
```
unit: story:contract-documents-without-an-implementation (uncommitted working tree over bf8deee)
verdict: green
cases: executed 173→178, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary (bin/, consumer/ incl. target/, repo/ incl. target/, trees/, logs/; about 740M)
needs-coordinator: no
```

I found no red case. The unit's claims held under every attack I ran. Two findings remain; both are notes, both are INFEASIBLE, and neither holds the unit.

**1. `git --no-pager diff --stat`**
```
 crates/tool-code-editor/src/lib.rs | 3 +--
 crates/tool-contract/src/lib.rs    | 1 +
 crates/tool-fs/src/toolset.rs      | 2 +-
 3 files changed, 3 insertions(+), 3 deletions(-)
```
Those three are the implementor's changes. `git status --short` adds one untracked file of mine: `crates/tool-contract/tests/adversary_documents.rs`. I changed no non-test path.

**2. Case added:** `$HOME/.local/state/worktree/trees/b10x/tools/impl-contract-documents/crates/tool-contract/tests/adversary_documents.rs`, 5 cases, all green:
- `FS_V1ALPHA1` and `CODE_EDITOR_V1ALPHA1` are byte-equal to the files the base `include_str!` read.
- `get` returns the same entry for every element of `all()`, and nothing for swapped, empty, upper-case or truncated keys.
- `all()` is strictly ordered.
- Each document's cached contract equals `Contract::read` of its file, and no two documents share a cache entry.
- 8 threads calling `contract()` at once get the same pointers.

Run alone: `test result: ok. 5 passed; 0 failed`. There is no red output to quote.

**3. Suite:** `cargo-serial.sh test -p b10x-tool-contract -p b10x-tool-fs -p b10x-tool-code-editor --locked` gave `EXIT=0` and 178 passed, 0 failed, including `adversary_documents: ok. 5 passed`. Clippy `-D warnings` on all targets of `b10x-tool-contract` passes, and so does `fmt --check` on the three crates.

**4. Findings**

| # | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|
| F1 `crates/tool-contract/build.rs:103` | `tool-contract validate contracts` passes `build_test/v1alpha1`, `web/v1.2` and `web/V1` (exit 0). The build script panics on the same trees (`"build_test" is not [a-z][a-z0-9-]*`, exit 101), and that stops every crate in the workspace from building, the validator included. Evidence: `bin/names.sh`, `logs/names.log` | Nothing found. Every toolset name planned in `contracts/README.md` and every `v1alpha1` fits the grammar, and the profile (`contracts/profile/tool-contract-1.md:19-21`) states no grammar | INFEASIBLE / introduced |
| F2 `crates/tool-contract/src/documents.rs:34` | Measured order is `fs/v1`, `fs/v10`, `fs/v1alpha1`, `fs/v2`: byte order. The rustdoc ("sorted by toolset and then by version") reads like version order, so `v1` comes before `v1alpha1` and `v10` before `v2` | Nothing found. There is one version per toolset today, and no caller treats position as precedence | INFEASIBLE / introduced |

**Fixes I did not apply:**
- F1: state `[a-z][a-z0-9-]*` for `toolset` and `version` in the profile, and have `validate` refuse names outside it, so the refusal comes from the validator rather than a build panic.
- F2: change the rustdoc to "byte order of the directory names, not version precedence".

**5. Attacked and could not break**
- **Rebuilds in a path-dependency consumer workspace:** adding, editing, renaming the version directory of and removing a document each reran the build script, and `all()` matched the tree each time. A no-op rebuild stayed `Fresh`; the fixture symlink loops do not force reruns.
- **Trees the build script skips:** a document at depth 1 or 3, a version directory with no document, a symlinked toolset, version or document, and a directory named `contract.yaml`.
- **Trees the build script refuses** (it panics and names the path): an upper-case name, a `_` name, a leading `-`, a `.draft` directory, a non-UTF-8 name, the constant collision `a-b/v1` + `a/b-v1`, and a missing `contracts/`.
- **Empty document:** it is embedded, and `validate` refuses it with ``missing field `format` ``.
- **Acceptance 2:** `cargo tree -p b10x-tool-contract -e all` lists no other b10x crate.
- **MSRV:** `cargo +1.98.0 check` on the three crates, all targets, exits 0. CI runs this check; the unit gate does not.
- **Integration preview:** I copied the sibling unit's uncommitted `process/v1alpha1` document into my scratch copy. Both document test files ran 14 of 14 green, and the consumer parses `process.run`.
- **Not run:** a consumer that depends on the crate by git revision. That needs a commit in a scratch repository, and the launch message forbids git write commands. The path-dependency consumer builds the same directory layout.

**6. Written outside the worktree**
- `$HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary/bin/` (`probe.sh`, `trees.sh`, `names.sh`, `rustc-direct.sh`)
- `$HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary/consumer/` (with `target/`, 236M)
- `$HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary/repo/` (copy of the tree plus the process document, with `target/`, 501M)
- `$HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary/trees/`
- `$HOME/.cache/tools-wave-w2/contract-documents/scratch/adversary/logs/`
- Through the build wrapper: appends to `$HOME/.cache/tools-wave-w2/cargo-serial.log`, and sccache entries from the worktree builds.

**7.**
```findings
[
  {"file": "crates/tool-contract/build.rs", "line": 103, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the build script refuses toolset and version names that tool-contract validate accepts (build_test, v1.2, V1), so a document the validator passes stops the whole workspace building, the validator included"},
  {"file": "crates/tool-contract/src/documents.rs", "line": 34, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "all() is in byte order of the directory names (v1 < v10 < v1alpha1 < v2) while its rustdoc reads as version order"}
]
```
