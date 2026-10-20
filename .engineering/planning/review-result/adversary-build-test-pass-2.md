---
format: aep.planning-md/3
id: review-result:adversary-build-test-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:build-test-contract: pass-1 findings fixed, 1 red (a read path through a link), go summaries claim a disabled network'
relations:
- reviews: story:build-test-contract
revision: 1
---
## Adversary report, pass 2 (the last)

```
unit: story:build-test-contract (uncommitted tree on impl/build-test-contract after correction round 1, base 1813c93)
verdict: red
cases: executed 64→66, red 1
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-w3/build-test/scratch/adversary/ (pass-2 logs, my pass-1 file rebuilt for comparison); $HOME/.cache/go-build (written by the suite's go runs); $HOME/.cache/tools-wave-w3/cargo-serial.log (wrapper)
needs-coordinator: yes
```

One case is left red: the per-field description of `go_fmt_check.paths` promises more than the code does. All six pass-1 findings (F1 to F6) are fixed as claimed.

**Case count:** this run executed 66 tests: lib 48, `adversary_contract` 6, `adversary_executor` 8, `conformance` 3, `mutants` 1. Without my 2 new cases that is 64, not the 67 the correction reported; I don't know where its count comes from.

**Formatting deviation, checked first.** I rebuilt my pass-1 file in scratch (`pass1-adversary_executor.rs.orig`) and diffed it against the reformatted one. It differs in three places only, all layout: two `std::fs::write(…go.mod…)` calls and one `registry.load_file(…)` call were split over several lines, with the trailing commas rustfmt adds. Every assertion and message is as I left it. My scenario and fixture files are unchanged.

### 1. Additions

All my files are still untracked. The tracked `git diff --stat` (4 files, 308 insertions, 48 deletions) is the correction's work, not mine.

| path | pass-2 change |
|---|---|
| `crates/tool-toolchain/tests/adversary_executor.rs` | now 397 lines; 2 tests added at the end |
| `adversary-links.yaml`, `adversary-arguments.yaml`, `fixtures/adversary-symlink/` | unchanged |

### 2. Cases added (each run alone first)

| case | what it asserts | now |
|---|---|---|
| `a_check_path_that_leaves_through_a_link_is_refused_as_its_field_says` | if the `go_fmt_check.paths` description says "refused when it leaves the workspace", then `up/outside.go` (with `up -> ..`) is refused | **red** |
| `dash_items_the_separator_exemption_and_a_zero_deadline_hold` | `-` and `--` items are refused; a dash item is refused under a `rest` with no separator and allowed after one; a zero deadline is refused | green |

Red output when run alone, verbatim:

```
panicked at crates/tool-toolchain/tests/adversary_executor.rs:301:9:
go_fmt_check.paths is described as "paths relative to the workspace root, each refused when it leaves the workspace; 1 to 128 items", and `up/outside.go`, which leaves the workspace through a link, answered Err(CheckFailed { stdout: "up/outside.go\n" })
```

### 3. Suite (run after the cases)

`cargo-serial.sh test --locked -p b10x-tool-toolchain --no-fail-fast`, exit 101:

| binary | result |
|---|---|
| lib | 48 ok |
| `adversary_contract` | 6 ok |
| `adversary_executor` | 7 ok, 1 failed (the case above) |
| `conformance` | 3 ok |
| `mutants` | 1 ok, five mutants |

### 4. Findings

**P1: the `go_fmt_check.paths` description promises a refusal the code does not make.** CONFIRMED, introduced, warning.
- Measured: `contract.rs:373` and `contract.yaml:76` say each path is "refused when it leaves the workspace". A path that leaves through a link is not refused: gofmt read the file outside the workspace and the call answered `CheckFailed`.
- `semantics.md:53-54` states the limit honestly ("a path the call only reads is checked by its spelling"); the field description is what an adopter reads per operation, and it says otherwise.
- What reaches it: any workspace with a link leading out (the correction's own `fixtures/go-links/up -> ..` is one), plus any `go_fmt_check` call through it.
- Fix (not applied), either of:
  - judge read paths with `lands` as well;
  - have `field()` derive "checked by spelling" for path parameters not listed in `writes`, then regenerate the document.

**P2: the go operation summaries still say "with network and checksum lookup disabled".** CONFIRMED (the wording mismatch only), pre-existing, note.
- Measured: `contract.yaml:35` and the other go summaries say network and checksum lookup are disabled.
- Against it: the runner applies none of the provider's environment (`GOSUMDB`, `GOENV`, `GOTOOLCHAIN`), and the same profile now declares `network`. That no lookup is disabled is inferred from the environment not being applied; I did not run it against the network.
- Harness 0.14.1 published the same text as its entry summary (`toolchain.rs:22`) over the same unconfined run (`local.rs:556-563`).
- Fix (not applied): reword the go builtin descriptions, or have the derivation qualify them. The descriptions are operator text, so this needs a decision.

### 5. Attacked and not broken

- **F1 (landing):** a link into `protected/`, a link out of the workspace, a dangling link, and `..` (refused before `lands` is reached) all hold. `NoLanding` is killed by a scenario.
- **F2/F3 (dash rule):** `-`, `--`, a `rest` with no separator, defaults and `enum` values all hold. A negative number is refused for every custom tool too; no built-in takes one.
- **F5 (deadline):** a zero deadline is refused with nothing started. On a partial plan the answer is the runs that started, a valid `List<toolchain.Run>`. A partial `toolchains` answer cannot be reached through the contract.
- **F4 and F6 (document against derivation):** the drift test holds, and my pass-1 cases for K2 and the drift gap pass.
- **Not attempted:**
  - a link changed between the judgement and the process start;
  - case and Unicode spellings, since this filesystem is case-sensitive.

### 6. Paths written outside the worktree

- `$HOME/.cache/tools-wave-w3/build-test/scratch/adversary/`: `pass1-adversary_executor.rs.orig`, `p2-*.log`, `p2-suite.log`, `pass2.log`. The run script is deleted.
- `$HOME/.cache/go-build`, written by the suite's go runs. Other sessions share it, so I don't know how many entries are mine.
- `$HOME/.cache/tools-wave-w3/cargo-serial.log`, appended by the build wrapper.

Lease released; none of my processes are left running.

### 7. Findings block

```findings
[
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 373, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "go_fmt_check.paths is described as refused when it leaves the workspace, but a read path is checked by spelling and up/outside.go through a link is read"},
  {"file": "contracts/build-test/v1alpha1/contract.yaml", "line": 35, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "go operation summaries say network and checksum lookup are disabled while the runner applies none of the provider's environment and the profile declares network"}
]
```

**Needs the coordinator:**
- P1: pick the fix, code (`lands` for read paths) or description.
- P2: the descriptions are operator text, so decide whether to reword them.
- The 67 the correction reported against the 64 this run executed before my additions.
