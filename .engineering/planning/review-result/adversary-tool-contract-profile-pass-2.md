---
format: aep.planning-md/3
id: review-result:adversary-tool-contract-profile-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:tool-contract-profile: no failing case, 3 judgement findings'
tags:
- adversary
relations:
- reviews: story:tool-contract-profile
revision: 1
---
Adversary pass 2 on story:tool-contract-profile, as returned by `aep:adversary` on 2026-10-20 after
correction round 1. The coordinator replaced the expanded home directory in paths with `$HOME`
(public repository rule) and changed nothing else.

```
unit: story:tool-contract-profile, pass 2, working tree $HOME/.local/state/worktree/trees/b10x/tools/impl-tool-contract-profile at base 9b97da8a6e1bcd621647468c7f77ec92a19e4d0d (uncommitted, after the implementor's correction)
verdict: CONFIRMED (judgement only; no failing case)
cases: executed 68→81, red 0
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 paths, all under $HOME/.cache/tools-wave-t1/tool-contract-profile/scratch/adversary-2/ (part 6)
needs-coordinator: none
```

Pass-1 standing: resolved 4 (F1, F2, F3, F4, F5 counted as 5 red cases, now all green), carried 1 (J1), new 2 (J2, J3).

## 1. What I touched

`git --no-pager diff --stat` is unchanged from pass 1 (the implementor's `check.yml` and `Taskfile.yml`; the crate is untracked). Bound: `sha256sum -c` of the 19 non-test files against a baseline taken at the start of pass 2, checked after the last cargo run: `all 19 non-test files unchanged`. Pass-1 files untouched (digests match the coordinator's). New files, both test-only:

| file | cases | red now |
|---|---|---|
| `crates/tool-contract/tests/adversary2_conformance.rs` | 9 | 0 |
| `crates/tool-contract/tests/adversary2_shapes.rs` | 4 | 0 |

Forbidden-literal scan: clean. `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.

## 2. The cases, each binary run alone before the suite

All green on first execution (`alone-adversary2_conformance.log`, `alone-adversary2_shapes.log`), so no red output exists to quote. What each asserts:

| case | asserts |
|---|---|
| `a_call_input_key_that_is_not_an_input_field_is_refused_naming_it` | `input: {path, lmit}` on `notes.read` → refused line starts `read:`, names `lmit` and `notes.read`; nothing runs |
| `a_call_omitting_an_optional_input_field_is_not_refused` | `input: {path}` only → no refusal, scenario passes |
| `a_returned_error_that_is_a_declared_outcome_passes` | toolset returns `notes.NotFound{path}` from `notes.read`, scenario expects it → passes |
| `a_returned_error_that_is_not_an_outcome_fails_the_scenario_even_when_ok_was_expected` | toolset returns `notes.PreconditionFailed` from `notes.read`, scenario expects `ok` → detail contains `not an outcome of notes.read` (the new message; a plain "expected ok, got error" would fail this case) |
| `a_link_whose_target_changes_is_a_change` | fixture `link -> a.txt`, toolset re-points to `b.txt` → `unchanged` fails naming `link` |
| `a_link_replaced_by_a_file_holding_its_targets_content_is_a_change` | toolset replaces the link with a regular file of identical bytes → `unchanged` fails |
| `equals_follows_an_untouched_link_and_the_files_around_it_are_unchanged` | `{link: {equals: "one\n"}, a.txt: unchanged, b.txt: unchanged}` → passes |
| `an_untouched_link_and_a_dangling_link_are_unchanged` | `{link: unchanged, dangling: unchanged}` → passes |
| `a_dangling_link_is_not_absent` | `{dangling: absent}` → fails with `exists` |
| `an_empty_derived_struct_against_an_input_list_names_the_missing_field` | contract `[path]`, derived `{}` → `field == "path"`, "absent from the derived schema" |
| `a_derived_field_against_an_empty_input_names_the_extra_field` | contract no input, derived `{path}` → `field == "path"`, "absent from the contract" |
| `additional_properties_true_is_refused_naming_the_values` | `{type: object, additionalProperties: true}` → `(root)`, "no declared shape" |
| `a_closed_empty_struct_is_the_empty_struct` | `#[schemars(deny_unknown_fields)] struct Closed {}` → accepted against no input |

## 3. The suite, after the cases existed

`cargo test --workspace --locked --no-fail-fast` via the wrapper, exit 0. Per binary: lib 2, bin 0, adversary2_conformance 9, adversary2_shapes 4, adversary_cli 2, adversary_conformance 5, adversary_shapes 12, adversary_validate 18, cli 6, conformance 9, shapes 6, validate 8; totals `passed=81 failed=0 executed=81`. Log: `suite.log`.

## 4. Judgement findings (no case could be made of them)

| # | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|
| J1 (carried) | `docs/design/tool-contracts.md:236` still reads `run(suite, \|root\| make_toolset(root))`; implemented and profile-documented (`contracts/profile/tool-contract-1.md:225`) is `run(suite, contract, scratch, make)`. Unchanged by the correction (`docs/` digest identical). | The brief names § T1 as the API the sibling toolset units build on. | CONFIRMED | introduced |
| J2 (new) | `contracts/profile/tool-contract-1.md:266` makes `type: object` with neither `properties` nor an `additionalProperties` schema the empty `Struct`; lines 270-271 of the same document say such an object "has no shape and is refused". The code follows line 266 (`src/types.rs:546-563`). | Any reader of the shape table. | CONFIRMED | introduced (pass-2 correction) |
| J3 (new) | Declared error `fields` (`contract.yaml` `errors[].fields`) are checked against nothing: `Toolset::schemas()` carries input/output only (`src/conformance.rs:394`), and `run_scenario` compares the returned error by name alone (`src/conformance.rs:613-620`). An implementation returning `notes.NotFound` with `fields: {}` passes any scenario that does not name `path` under `expect.output`. | Every error outcome of every toolset; the acceptance statement scopes the shape check to the field list of input and output, so this is a gap in the contract's reach, not in the acceptance. | CONFIRMED | introduced |

Fixes I did not apply. J1: one line in the design doc, or a sentence in the profile doc declaring itself the authority for the API. J2: drop "an object with neither `properties` nor `additionalProperties`" from the "outside this table" sentence (and name `additionalProperties: true` there instead). J3: either a third schema per error in `Toolset::schemas()` checked with `check_shapes` against `errors[].fields`, or a runtime check that the returned `fields` object's keys are exactly the declared field names.

## 5. Attacked and not broken

- Every new check named in the correction is reachable from `run` and from the CLI's `validate` (coverage), and behaves as the profile document now states (the 13 cases above).
- A symlink to a directory is recreated as a link and not recursed into on either side (read of `copy_tree` :689 and `walk` :747-749; not separately cased).
- The returned-error check in `run_scenario` (:613) is unreachable as a pass/fail gate once coverage refuses an `expect.error` outside the outcomes: any returned error either equals the expected valid one or already fails on name mismatch; only its message is new. Harmless redundancy, not a finding.
- Residue carried from pass 1: `holds` may still name a property the called operation does not declare (permitted by line 179); `fixture.files` keys and `fixture.dir` are not confined to the workspace. Neither is a stated rule.
- `call.input` that is not a JSON object (a list, a scalar) is not key-checked (`as_object()` None at :525); the document calls `input` "the operation's input as JSON", so a non-object is undefined rather than refused. Boundary, not cased.

## 6. Paths written outside the worktree

All under `$HOME/.cache/tools-wave-t1/tool-contract-profile/scratch/adversary-2/`: `baseline-non-test.sha256`, `alone-adversary2_conformance.log`, `alone-adversary2_shapes.log`, `suite.log`, `fmt.log`, `clippy.log`; the wrapper appends to `$HOME/.cache/tools-wave-t1/cargo-serial.log`. Build output in the worktree's `target/` only. Both pass-2 lease hooks (`session-start`, `heartbeat`, `session-end`) ran under session `adversary2-tool-contract-profile`.

```findings
[
  {"file": "docs/design/tool-contracts.md", "line": 236, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the design document gives the runner API as run(suite, make) while the implementation and the profile document give run(suite, contract, scratch, make), and the sibling toolset units are told to build on the design document"},
  {"file": "contracts/profile/tool-contract-1.md", "line": 266, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "line 266 maps an object with neither properties nor an additionalProperties schema to the empty Struct while lines 270-271 of the same document say it has no shape and is refused; the code follows line 266"},
  {"file": "crates/tool-contract/src/conformance.rs", "line": 613, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "declared error fields are checked against nothing: schemas() carries input and output only and a returned error is compared by name alone, so an implementation may return any fields for a declared error and still conform"}
]
```
