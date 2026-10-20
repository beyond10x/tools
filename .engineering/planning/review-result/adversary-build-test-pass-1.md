---
format: aep.planning-md/3
id: review-result:adversary-build-test-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:build-test-contract: 8 red, written paths through a link and option-shaped arguments bypass the write scope, 7 findings'
relations:
- reviews: story:build-test-contract
revision: 1
---
## Adversary report, pass 1

```
unit: story:build-test-contract (uncommitted tree on impl/build-test-contract, base 1813c93)
verdict: red
cases: executed 54→60, red 8
origin: introduced 4, pre-existing 3, undecided 0
wrote-outside-worktree: $HOME/.cache/tools-wave-w3/build-test/scratch/adversary/ (logs); $HOME/.cache/go-build and $HOME/.npm/_logs (written by the suite's own toolchain runs); $HOME/.cache/tools-wave-w3/cargo-serial.log (wrapper)
needs-coordinator: yes
```

### 1. Diff stat

All my files are untracked, so `git --no-pager diff --stat` still shows only the implementor's 4 tracked files (104 insertions, 11 deletions), unchanged by me. My additions are all test, scenario or fixture paths:

| path | lines |
|---|---|
| `crates/tool-toolchain/tests/adversary_executor.rs` | 249 |
| `contracts/build-test/v1alpha1/scenarios/adversary-links.yaml` | 15 |
| `contracts/build-test/v1alpha1/scenarios/adversary-arguments.yaml` | 37 |
| `contracts/build-test/v1alpha1/fixtures/adversary-symlink/{go.mod, protected/kept.go, alias.go → protected/kept.go}` | 3 + 5 + link |

### 2. Cases added (each run alone first, all red now)

| case in `adversary_executor.rs` | what it asserts | red output when run alone |
|---|---|---|
| `the_link_scenario_passes_alone` (runs `adversary-links.yaml`) | `go_fmt` on a link into `protected/` is refused and `protected/kept.go` is unchanged | `expected error build-test.Refused, got ok: {"argv":["gofmt","-w","alias.go"],"exit":0,…}` |
| `the_argument_scenario_passes_alone` (runs `adversary-arguments.yaml`) | `go_build {package: "-n"}` is refused; `go_fmt_check {paths: [-w, main.go]}` is refused and writes nothing | `expected error build-test.Refused, got ok: {"argv":["go","build","-n"],"exit":0,…}` and `expected error build-test.Refused, got error build-test.CheckFailed: formatting check failed: "main.go\n"` |
| `a_check_declared_without_write_leaves_the_named_file_as_it_was` | `go_fmt_check` does not rewrite `main.go` | `:190` `go_fmt_check (effects [Process]) rewrote main.go under a scope that refuses every write`; the file is now gofmt-formatted (`\tprintln`) |
| `an_operation_declared_confined_does_not_run_with_the_callers_home` | a `confined` operation's program does not see the caller's `HOME` | `:103` `build-test.taskfile_run declares placement confined, and its program read the caller's own HOME` |
| `no_argv_starts_after_the_calls_deadline_has_passed` | no argv is started with 0 ms left | `:155` second run `{"argv":["sh","-c","printf two"],…,"timed_out":true,"timeout_ms":0}` |
| `a_provider_that_stops_containing_a_path_derives_a_different_operation` | a `go_fmt_check.paths` changed to `string-list` changes the derived operation | `:209` `assertion left != right failed: … leaves its operation equal to the frozen one` |

The first run of the two scenario cases failed only because the runner refuses a one-file suite on coverage. That was a mistake in my test, not a finding. I cut the contract to the called operations and reran; the outputs above are from that rerun.

### 3. Suite (run after the cases existed)

Command: `cargo-serial.sh test --locked -p b10x-tool-toolchain --no-fail-fast`, exit 101.

| binary | result |
|---|---|
| lib | 44 ok |
| `adversary_contract` | 6 ok |
| `adversary_executor` | 0 of 6 pass |
| `conformance` | 2 ok, `the_suite_passes_against_the_executor` FAILED: 35 of 38 scenarios passed, the 3 failures are my adversary scenarios |
| `mutants` | FAILED at "the intact executor must pass", caused by the same 3 scenarios |

That makes 60 tests executed and 8 failing: my 6, plus conformance and mutants, which my scenarios turn red.

### 4. Findings

**F1 (K1): written paths are judged by spelling, so a link bypasses the write scope.** CONFIRMED, pre-existing, blocker.
- Measured: `executor.rs:449` asks the scope about `alias.go`, gofmt then writes through the link to `protected/kept.go`.
- What reaches it: a workspace with a checked-in link, plus any `go_fmt` or `rust_fmt` call naming it.
- Harness had it: `catalogue.rs:524-527` asks the scope about the spelled path, even though harness's own `operations.rs:294-296` describes this exact bypass for `file.write`.
- Fix (not applied): ask the scope about where the bytes land. Canonicalise each written path as harness's `lands` does (`local.rs:1188-1192`), or refuse a written path that has a link component.

**F2: a path item beginning with `-` turns the read-only `go_fmt_check` into a write.** CONFIRMED, pre-existing, blocker.
- Measured: `lib.rs:1702` `validate_relative` accepts `-w` as a relative path. `go_fmt_check` is declared `effects: [process]` with no subjects, so the scope is never asked, and `main.go` was rewritten.
- What reaches it: any caller of `build-test.go_fmt_check`. A harness that admits operations by declared effects grants it as read-only.
- Base `lib.rs:1694` is the same code; harness ran it through `toolchain.rs:128` and `catalogue.rs:530`.
- Fix (not applied): in `validate_arguments` (`lib.rs:1650`), refuse a scalar or list item beginning with `-` for the kinds `string`, `workspace-path`, `workspace-path-list` and `enum`, naming the argument. Items after a `rest` separator are exempt.

**F3: a `package` value becomes an undeclared flag**, against `semantics.md:16` ("nothing a caller passes becomes … a flag that was not declared"). CONFIRMED, pre-existing, warning.
- Measured: `lib.rs:1666` accepts any string, so the argv is `[go, build, -n]`.
- What reaches it: the `package` input of `go_build`, `go_test`, `go_vet`, `go_bench`, `go_run` and the `rust_*` tools, and `bin` of `rust_run`. Some `go build` options start other programs.
- Base `lib.rs:1658` is the same code. Fix: the same as F2.

**F4 (K2): the profile says `placement: confined`, but the run is unconfined.** CONFIRMED, introduced, blocker.
- Measured: `contract.rs:324` and `contract.yaml:54` declare `confined`; the program sees the caller's `HOME`. The stderr of the `go build -n` run shows the caller's `$HOME/.cache/go-build` in use.
- What reaches it: every call through the executor, including the conformance suite itself.
- Harness published no placement and also ran unconfined (`local.rs:556-563`).
- Fix (per the coordinator's decision): declare `placement: local`, and add effects the way wave-w2 D6 did for `process.run` (`network`, `secret`). Align `semantics.md:53-59`.

**F5: an argv is started after the call's deadline has passed.** CONFIRMED, introduced, warning.
- Measured: at `executor.rs:459-464`, once `left` is 0 the next argv is still spawned with a 0 ms bound and then killed.
- What reaches it: a multi-argv plan, such as `rust_fmt` with 2 or more paths, under `Executor::run(…, remaining)` or `within`. No caller in this repo passes a remaining time yet.
- Fix (not applied): when `left` is zero, stop. Answer the runs so far, or refuse naming the deadline.

**F6: the drift test cannot see a path parameter lose its containment.** CONFIRMED, introduced, warning.
- Measured: `contract.rs:339` maps `string`, `workspace-path` and `enum` all to `String`, and the descriptions do not name the kind.
- What reaches it: an edit to `builtins/*.yaml`, which is exactly what `semantics.md:19-21` says the drift test catches.
- Fix (not applied): have `field()` name the kind for `workspace-path(-list)` and regenerate `contract.yaml`. Add a `go_fmt_check` scenario with a `../` path.

**F7: the acceptance wording differs from the implementation.** Introduced, note. `build-test-contract.md:45-46` says the failed check "answers a refusal". The executor answers a separate error, `build-test.CheckFailed`, which is a deliberate decision. The coordinator should either amend the acceptance statement or accept it as is.

### 5. Attacked and not broken

- **Whitespace-only stdout:** passes the check, with the same `trim` as harness (`catalogue.rs:534`).
- **A failed check:** stops the plan before its next argv.
- **Answer shapes:** `runs` matches the contract's output types (schema check plus the `rust_fmt` scenarios). `toolchains` cannot be reached through `Toolset`.
- **Document against derivation:** equal in names, inputs, outputs, outcomes, profiles, errors and types; F6 is the only gap.
- **Mutants:** each of the four is killed by its named scenario. No conformance scenario holds the shared deadline across several argv; only `executor/tests.rs:289` does.
- **Undeclared `argv[0]`:** refused by the runner and by `validate_provider`.
- **Fixtures:** none needs the network. The machine dependence on caller caches is part of F4.

### 6. Paths written outside the worktree

- `$HOME/.cache/tools-wave-w3/build-test/scratch/adversary/`: `build-red.log`, `red-*.log`, `red2-*.log`, `red3-*.log`, `red4-*.log`, `suite.log`, `suite2.log`, `pass1b.log`. Probe scripts and the probe directory are deleted.
- `$HOME/.cache/go-build` (439 entries newer than my start) and `$HOME/.npm/_logs` (7), written by the suite's go and npm runs. Other sessions share both, so I don't know how many entries are mine.
- `$HOME/.cache/tools-wave-w3/cargo-serial.log`, appended by the build wrapper.

Lease released with `session-end`; none of my processes are left running.

### 7. Findings block

```findings
[
  {"file": "crates/tool-toolchain/src/executor.rs", "line": 449, "category": "contract-drift", "severity": "blocker", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "the write scope judges a planned path by its spelling, so a link to a refused path is written through by gofmt -w"},
  {"file": "crates/tool-toolchain/src/lib.rs", "line": 1702, "category": "boundary", "severity": "blocker", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "a path item -w passes the containment check and makes go_fmt_check, declared effects [process], rewrite files without asking the write scope"},
  {"file": "crates/tool-toolchain/src/lib.rs", "line": 1666, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "a string parameter beginning with - reaches the program as an undeclared option, against semantics.md:16"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 324, "category": "contract-drift", "severity": "blocker", "verdict": "CONFIRMED", "origin": "introduced", "message": "every operation declares placement confined while the executor runs the program unconfined with the caller's HOME and caches"},
  {"file": "crates/tool-toolchain/src/executor.rs", "line": 459, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "once the call's deadline has passed, each remaining argv is still started with a 0 ms bound"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 339, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a builtin path parameter changed to a plain string derives an identical operation, so the drift test cannot see path containment dropped"},
  {"file": ".engineering/planning/story/build-test-contract.md", "line": 45, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the acceptance statement asks for a refusal on a failed formatting check, and the executor answers the separate error build-test.CheckFailed"}
]
```
