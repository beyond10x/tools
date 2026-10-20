---
format: aep.planning-md/3
id: review-result:adversary-toolset-toolchain-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:toolset-toolchain: 1 red case (no inverse from operation name to tool), 3 notes'
tags:
- adversary
relations:
- reviews: story:toolset-toolchain
revision: 1
---
Adversary pass 2 on story:toolset-toolchain, as returned by `aep:adversary` on 2026-10-20 after
correction round 1. The coordinator replaced the expanded home directory in paths with `$HOME`
(public repository rule) and changed nothing else.

```
unit: story:toolset-toolchain (pass 2)
verdict: red
cases: executed 28→29, red 1
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 2 paths (listed in part 6)
needs-coordinator: no
```

Findings cover the uncommitted working tree of `$HOME/.local/state/worktree/trees/b10x/tools/impl-toolset-toolchain` (branch `impl/toolset-toolchain`, base `796937c`; the crate does not exist at the base, so every origin is `introduced`). `lib.rs` is still 9 diff lines against the snapshot and the five builtins are byte-identical (`cmp`).

Pass-1 findings: F1, F3, F4, F5, J1, J3 resolved (cases 1, 3, 4, 5 green; `Confined` pinned; descriptions carry default and bounds); F2 resolved by the coordinator's decision (case 2 edited as asked, green); J2 carried (story Outcome unchanged); F6, N1, N2 new.

## 1. What I touched

```
$ git --no-pager diff --stat
 Cargo.lock | 51 ++++++  (the implementor's change, not mine)
$ git --no-pager status --short -uall   (my only path)
?? crates/tool-toolchain/tests/adversary_contract.rs   (304 lines; 227 after pass 1)
```

Edits to the file this pass: (a) case 2, line 88 and its docstring, as the coordinator asked — pins `build-test.build`, `build-test.t_9build`, `build-test.t_-build` and keeps the validate-accepts assertion; (b) one new case appended. No non-test path touched; no home path, person, employer or hostname in the file (grep: nothing found).

## 2. Case added (red now; run alone before the suite)

| # | Test | Asserts | Red output (alone, `adversary-2/case6-alone.log`, exit 101) |
|---|---|---|---|
| 6 | `every_derived_operation_name_routes_back_to_the_tool_that_plans_it` | custom provider with tools `Build`, `-`, `9`, `probe_plain` derives `[build-test.build, build-test.t_-, build-test.t_9, build-test.probe_plain]` and validates (green half); for every operation, `provider.plan(<segment>, {})` is `Some` | `assertion left == right failed: operations whose published name plans nothing  left: ["build-test.build", "build-test.t_-", "build-test.t_9"]  right: []` (line 299) |

## 3. Suite run (after the case existed)

```
$ cargo-serial.sh test -p b10x-tool-toolchain          (exit 101)
Running unittests src/lib.rs          running 23 tests ... test result: ok. 23 passed
Running tests/adversary_contract.rs   running 6 tests ... test result: FAILED. 5 passed; 1 failed
  FAILED: every_derived_operation_name_routes_back_to_the_tool_that_plans_it
```

`<before>` = 28 is this run with my pass-2 case excluded (23 lib tests + my 5 pass-1 cases, which were in the tree when the correction was made); the coordinator's message carried no `cases:` line.

## 4. Findings

| # | file:line | What was measured | What reaches it | Verdict / origin / severity |
|---|---|---|---|---|
| F6 contract-drift (new) | `crates/tool-toolchain/src/contract.rs:138` | `operation_name` is many-to-one and the crate has no inverse: `Operation` has no slot for the source tool, and `ResolvedProvider::plan` (lib.rs:645-650) matches `tool.name == name`; `plan("build")`, `plan("t_-")`, `plan("t_9")` are `None` for the tools that derived those operations | `Registry::load_file` + `resolve` for any custom tool name outside `[a-z][a-z0-9_-]*`; identity for every builtin, so builtins route | CONFIRMED / introduced / warning |
| N1 judgement (new) | `crates/tool-toolchain/src/contract.rs:208-212` | the harness answers a bare object for one process and a bare array for several (catalogue.rs:536-540), decided by the plan's argument cardinality at run time; the contract declares `runs: List<toolchain.Run>` as a struct field whatever the cardinality; the profile can express neither a bare array nor a cardinality-dependent output | `rust_fmt` with one path vs. two; the T2 executor must wrap, this crate cannot | INFEASIBLE / introduced / note |
| N2 judgement (new) | `crates/tool-toolchain/src/lib.rs:455-461` | `validate_collisions` compares tool names case-sensitively, so `load_file` admits `Build` beside `build`; the collapse surfaces only when a workspace resolving both is validated (`build-test.build: name: duplicate operation name`, pinned at contract.rs:610) | an operator loading two custom documents; lib.rs is the verbatim move, so the early refusal cannot live there | INFEASIBLE / introduced / note |
| J2 judgement (carried) | `.engineering/planning/story/toolset-toolchain.md` § Outcome | story promises `subjects` from `argv[0]`; a subject must be an input field (validate.rs:237-245); derivation and Acceptance omit it | story text vs implementation | INFEASIBLE / introduced / note |

Named fix, not applied (F6): a `ResolvedProvider::plan_operation(&self, operation: &str, arguments: &Value)` (or `tool_for(operation) -> Option<&ResolvedTool>`) in `contract.rs` matching `operation_name(&tool.name) == operation`; `renderings` is not the place ("generated names per harness; never the source of the name", document.rs:209).

## 5. Attacked, could not break

- Collapse to one name: `check_name` (validate.rs:178) refuses any identical operation name, including `9x` beside `t_9x` and a custom beside a builtin in one workspace; the implementor's test pins the literal violation.
- Dash-only and digit-only names: `build-test.t_-` and `build-test.t_9` derive and validate (green half of case 6).
- `toolchain.Run`: `Named` under the `[A-Za-z0-9._]` identifier alphabet (types.rs:70-73); struct of 8 primitive fields passes `check_types` and `Shape::of_fields` → `Struct{runs: List<Struct{8}>}`; declared only when a field names it, so the empty and plain documents carry no unused type.
- `check-failed`: outcome names unique, error declared only when named, `stdout: String` parses; conformance's `expect.outcome: ok` is a keyword, not a contract outcome name (conformance.rs:245-249), so the name has no grammar to fail.
- Eight fields vs `exec` (local.rs:625-637): names and types match one for one (`exit` Optional<Integer> for `status.code()`, `omitted_bytes`/`timeout_ms` Integer for usize/u64, `timed_out`/`truncated` Boolean, `argv` List<String>).
- J3 descriptions: own description, enum values, `default`, item bounds joined `; `-separated as the code states; `default ./...` and `1 to 128 items` pinned at contract.rs:702-727; `literal` renders a string bare and anything else as JSON.
- Tests that cannot fail: the two `== run_fields()` comparisons (contract.rs:745, 771) are self-referential but each follows a literal assertion of the same value; the `operation_name` test is all literals.

## 6. Paths written outside the worktree (this pass)

```
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary-2/case6-alone.log
$HOME/.cache/tools-wave-t1/toolset-toolchain/scratch/adversary-2/suite.log
```

(`cargo-serial.sh` appends to its own `$HOME/.cache/tools-wave-t1/cargo-serial.log`, which the wrapper owns.) Lease `adversary-toolset-toolchain` acquired, heartbeated twice, released.

## 7. Findings block

```findings
[
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 138, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "operation_name is many-to-one with no inverse in the crate: ResolvedProvider::plan matches tool.name exactly, so a mangled operation (build-test.build, t_-, t_9) cannot be planned by the name the contract publishes"},
  {"file": "crates/tool-toolchain/src/contract.rs", "line": 208, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the harness answers a bare object for one process and a bare array for several, by run-time cardinality; the profile can express neither, so runs: List<toolchain.Run> is a shape the executor must wrap into"},
  {"file": "crates/tool-toolchain/src/lib.rs", "line": 455, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "validate_collisions is case-sensitive, so load_file admits Build beside build and the collapse surfaces only as a duplicate-name violation when a workspace resolving both is validated; lib.rs is the verbatim move so the early refusal cannot live there"},
  {"file": ".engineering/planning/story/toolset-toolchain.md", "line": 1, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the story Outcome promises subjects from argv[0], which the profile cannot express since a subject must be an input field; the derivation and the Acceptance both omit it"}
]
```
