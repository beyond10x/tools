---
format: aep.planning-md/3
id: review-result:adversary-toolset-toolchain-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:toolset-toolchain: 5 red cases, 8 findings in the derivation'
tags:
- adversary
relations:
- reviews: story:toolset-toolchain
revision: 1
---
Adversary pass 1 on story:toolset-toolchain, as returned by `aep:adversary` on 2026-10-20. The
coordinator replaced the expanded home directory in paths with `$HOME` (public repository rule) and
changed nothing else.

```
unit: story:toolset-toolchain
verdict: red
cases: executed 17→22, red 5
origin: introduced 8, pre-existing 0, undecided 0
wrote-outside-worktree: 6 paths (listed in part 6)
needs-coordinator: no
```

Findings cover the uncommitted working tree of `$HOME/.local/state/worktree/trees/b10x/tools/impl-toolset-toolchain` (branch `impl/toolset-toolchain`, base `796937c`). `crates/tool-toolchain` does not exist at the base (`git ls-tree` lists only `crates/tool-contract`), so every finding is `introduced`.

## 1. What I touched

```
$ git --no-pager diff --stat
 Cargo.lock | 51 ++++++  (the implementor's change, not mine)
$ git --no-pager status --short -uall   (my addition is the only path not in the implementor's report)
?? crates/tool-toolchain/tests/adversary_contract.rs   (new, 227 lines)
```

No non-test path touched. The file contains no home path, person, employer or hostname (grep for `/home/`, names, hosts: nothing found).

## 2. Cases added (all in `crates/tool-toolchain/tests/adversary_contract.rs`, all red now, each run alone before the suite)

| # | Test | Asserts | Red output (alone) |
|---|---|---|---|
| 1 | `a_workspace_matching_no_provider_derives_a_document_the_profile_accepts` | `Registry::builtins().resolve(empty_dir, None)` → 0 providers; `validate(&contract(&[]))` is Ok | `assertion left == right failed: ["document: errors: build-test.Refused is declared but no outcome names it"]` (line 49) |
| 2 | `every_tool_name_the_registry_admits_derives_an_operation_name_the_profile_accepts` | custom provider with tools `Build`, `9build`, `-build` loads via `load_file`, resolves, derives all three; `validate` is Ok | 3 violations: `build-test.Build: name: build-test.Build is not build-test.<op> with <op> in [a-z][a-z0-9_-]*` (and `9build`, `-build`) (line 92) |
| 3 | `the_output_names_every_field_the_harness_run_answers` | derived `output` names = harness `exec`'s answer keys | `left: ["exit","stderr","stdout","truncated"]  right: ["argv","exit","omitted_bytes","stderr","stdout","timed_out","timeout_ms","truncated"]` (line 119) |
| 4 | `a_for_each_tool_whose_plan_starts_several_processes_does_not_declare_a_single_runs_output` | `plan({"paths":["a.txt","b.txt"]}).argv == [["sh","a.txt"],["sh","b.txt"]]` (green); for-each op's `output != ` single-run op's `output` | `assertion left != right failed: a two-process plan and a one-process plan declare the same output: [stdout, stderr, exit, truncated]` (line 177) |
| 5 | `an_empty_stdout_check_derives_an_operation_that_differs_from_a_plain_run` | `(outcomes, output, guidance, profile)` of an `empty-stdout: true` tool `!=` the same tuple of a plain tool | `assertion left != right failed: apart from its name, the empty-stdout check derives the same operation as a plain run` (line 222) |

Logs: `case1-alone.log` … `case5-alone.log` in scratch (part 6). All five exit 101.

## 3. Suite run (after the cases existed)

```
$ cargo-serial.sh test -p b10x-tool-toolchain          (exit 101)
Running unittests src/lib.rs     running 17 tests ... test result: ok. 17 passed
Running tests/adversary_contract.rs   running 5 tests ... test result: FAILED. 0 passed; 5 failed
```

`<before>` = 17 is the implementor's reported `cases:` line, and matches the lib-test count in this run.

## 4. Findings

| # | file:line | What was measured | What reaches it | Verdict / origin / severity |
|---|---|---|---|---|
| F1 boundary | `crates/tool-toolchain/src/contract.rs:46` | `contract(&[])` always emits `errors: [refused]` with no operation naming it → `validate` refuses the document | `Registry::resolve(ws, None)` on any directory without project markers returns `Ok(vec![])` (lib.rs:500-525); also a Taskfile-only workspace whose tasks are all internal/dynamic: lib.rs:1143-1153 filters every tool, zero operations | CONFIRMED / introduced / warning |
| F2 acceptance | `crates/tool-toolchain/src/contract.rs:81` | `legal_name` (lib.rs:1001-1013) admits `[A-Za-z0-9_.-]+`; the profile's `<op>` is `[a-z][a-z0-9_-]*` (validate.rs:324-328); `Build`, `9build`, `-build` derive operations the validator refuses | `Registry::load_file` (the `--toolchain-spec FILE` operator path, lib.rs:379) + `resolve(ws, Some([..]))`; `sh` on PATH is the only machine need | CONFIRMED / introduced / warning |
| F3 contract-drift | `crates/tool-toolchain/src/contract.rs:202-235` | doc says "as the harness this crate came from answers it"; harness answers 8 keys, contract names 4; `first_difference` (types.rs:676-690) refuses a derived key absent from the contract; `timed_out` is lost and `exit: null` conflates timeout with any signal | harness `crates/harness-tools/src/local.rs:625-637` (`exec`), routed by `invoke_toolchain` catalogue.rs:525; no executor in this repo yet (`story:build-test-contract`) | CONFIRMED / introduced / warning |
| F4 contract-drift | `crates/tool-toolchain/src/contract.rs:110` | a `for-each` tool's plan has N argv (lib.rs:663-675); harness answers `Value::Array(outputs)` when N>1 (catalogue.rs:536-540) and one object when N=1; the derived output is the single-run struct regardless | `rust_fmt` (`builtins/rust.yaml:79`, `for-each: paths`) via `Registry::resolve` on any Cargo workspace with a rustup root | CONFIRMED / introduced / warning |
| F5 contract-drift | `crates/tool-toolchain/src/contract.rs:111-126,159-162` | `empty_stdout` is dropped: the op is byte-identical to a plain run; harness answers a `Refused` error for a program that ran and wrote stdout (catalogue.rs:525-533), matching neither outcome and contradicting `failure_as_result: true` | `go_fmt_check` (`builtins/go.yaml:64`) via `Registry::resolve` on any Go workspace | CONFIRMED / introduced / warning |
| J1 judgement | `crates/tool-toolchain/src/contract.rs:95-99` | `Placement::Local` is dead: every valid provider has ≥1 tool (lib.rs:714), ≥1 command (lib.rs:830), argv[0] ∈ `programs` (lib.rs:905-917); the unit's test at contract.rs:511-520 reaches it only by `programs.clear()` on a resolved provider | nothing found: no registry output has empty `programs` | INFEASIBLE / introduced / note |
| J2 judgement | `.engineering/planning/story/toolset-toolchain.md` § Outcome | story says `subjects` from `writes` parameters **and `argv[0]`**; contract.rs:86-94 derives from `writes` only; a subject must be an input field (validate.rs:237-245), so a literal argv[0] cannot be one; Acceptance does not ask for it | story text vs implementation disagree; `requires.binaries` carries the programs instead | INFEASIBLE / introduced / note |
| J3 judgement | `crates/tool-toolchain/src/contract.rs:169-199` | a parameter's `default`, `min-items`, `max-items` do not reach the field, though the tool's own `input_schema` carries them (lib.rs:1596-1604): `go_build.package` default `./...`, `rust_fmt.paths` 1..128 are invisible in the contract | every builtin with a default or item bound | CONFIRMED / introduced / note |

Named fixes, not applied: F1 — emit `errors` only when `operations` is non-empty, or make `contract()` refuse zero providers. F2 — make `operations()`/`contract()` fallible on a name outside `[a-z][a-z0-9_-]*`, or lower-case and sanitise the segment (tightening `legal_name` would break the verbatim-move claim on lib.rs). F3 — add `timed_out: Boolean`, `omitted_bytes: Integer`, `timeout_ms: Integer`, `argv: List<String>` to `output()`, or reword the doc as a deliberate subset and record it for the T2 executor. F4 — derive a `runs: List<Run>` output (a `struct` under `types`) for tools with a `for-each` command. F5 — add a third outcome (`failed` naming a `build-test.CheckFailed` error) for `empty_stdout` tools, or carry the rule in `guidance`.

## 5. Attacked, could not break

- Verbatim move: `lib.rs` differs from the snapshot only in the 3-line module doc and `pub mod contract;`; all five YAMLs byte-identical; the 8 harness tests unchanged.
- Duplicate operation names (two tools, one name, within or across providers): `validate_collisions` lib.rs:449-480 refuses before derivation.
- `requires.env` key shape: `validate_env` lib.rs:1046-1083 enforces `[A-Z][A-Z0-9_]*`.
- Summary limits: profile and validator have none; tool description is one line ≤512 B (lib.rs:1015).
- Parameter kinds: the match at contract.rs:170-174 is exhaustive; every emitted type text parses (`Optional<List<String>>`, `Optional<Integer>`).
- Risk floor: `[process]` and `[process, write]` both floor at `medium`; `Medium` declared.
- Enum with no resolved values (`"one of: "`): `resolve_provider` lib.rs:1143-1153 drops such a tool; only the test-only `resolve_offline` can reach contract.rs:187.
- `rust_fmt` `paths` subject: `writes` restricted to workspace-path kinds (lib.rs:874-893); pattern `build-test.rust_fmt({paths})` follows the fixture convention `notes.write({path})`.
- Tests that cannot fail: none of the nine; all pin literals or observed effects.

## 6. Paths written outside the worktree

```
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/case1-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/case2-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/case3-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/case4-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/case5-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary/suite.log
```

(`cargo-serial.sh` also appends to its own `$HOME/.cache/tools-wave-t1/cargo-serial.log`, which the wrapper owns.) Lease `adversary-toolset-toolchain` acquired, heartbeated twice, released.

## 7. Findings block

```findings
[
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 46, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "contract(&[]) for a workspace matching no provider declares build-test.Refused with no outcome naming it, so validate refuses the derived document"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 81, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a tool name the registry admits (Build, 9build, -build via load_file) derives an operation name the profile's [a-z][a-z0-9_-]* grammar refuses"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 202, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "output() claims to be what the harness run answers but omits argv, omitted_bytes, timed_out and timeout_ms, which the shape check would refuse and which lose the timeout-versus-signal distinction"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 110, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a for-each tool (rust_fmt) whose plan starts N processes declares the single-run output although the harness answers an array of runs for N>1"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 111, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "empty-stdout (go_fmt_check) is dropped: the operation is identical to a plain run although the harness answers an error for a program that ran and wrote stdout, contradicting failure_as_result"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 95, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "Placement::Local is unreachable because every valid provider has a non-empty programs list; the unit's test reaches it only by clearing programs on a resolved provider"},
  {"file": ".engineering/planning/story/toolset-toolchain.md", "line": 1, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the story Outcome promises subjects from argv[0], which the profile cannot express since a subject must be an input field; the derivation and the Acceptance both omit it"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 169, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "a parameter's default and min/max-items do not reach the derived field although the tool's own input_schema carries them"}
]
```
