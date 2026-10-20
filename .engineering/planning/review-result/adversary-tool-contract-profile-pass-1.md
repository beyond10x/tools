---
format: aep.planning-md/3
id: review-result:adversary-tool-contract-profile-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:tool-contract-profile: 5 findings, 2 confirmed in the runner'
tags:
- adversary
relations:
- reviews: story:tool-contract-profile
revision: 1
---
Adversary pass 1 on story:tool-contract-profile, as returned by `aep:adversary` on 2026-10-20. The
coordinator replaced the expanded home directory in paths with `$HOME` (public repository rule) and
changed nothing else.

```
unit: story:tool-contract-profile, working tree $HOME/.local/state/worktree/trees/b10x/tools/impl-tool-contract-profile at base 9b97da8a6e1bcd621647468c7f77ec92a19e4d0d (uncommitted)
verdict: red
cases: executed 31→68, red 5
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 10 paths, all under $HOME/.cache/tools-wave-t1/tool-contract-profile/scratch/adversary/ (listed in part 6)
needs-coordinator: no
```

## 1. What I touched

`git --no-pager diff --stat` (tracked files; both are the implementor's, not mine):

```
 .github/workflows/check.yml | 14 ++++++++++++++
 Taskfile.yml                |  7 ++++++-
 2 files changed, 20 insertions(+), 1 deletion(-)
```

The crate is untracked, so the diff cannot show my bound. Proof instead: `sha256sum -c` of all 19 non-test files (`crates/tool-contract/src/**`, both `Cargo.toml`, `Cargo.lock`, `contracts/**`, `docs/**`, `Taskfile.yml`, `.github/**`) against the baseline taken before I wrote anything: `all 19 non-test files unchanged`, checked after the last cargo run. My additions, all under `crates/tool-contract/tests/`:

| file | cases | red now |
|---|---|---|
| `crates/tool-contract/tests/adversary_conformance.rs` | 5 | 3 |
| `crates/tool-contract/tests/adversary_shapes.rs` | 12 | 2 |
| `crates/tool-contract/tests/adversary_validate.rs` | 18 | 0 |
| `crates/tool-contract/tests/adversary_cli.rs` | 2 | 0 |

No existing case was changed. Forbidden-literal scan of the four files: clean. `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0. `cargo fmt --all --check`: exit 1 on my four files only; I ran `rustfmt --edition 2024` on them (whitespace only, panic line numbers unchanged), after which exit 0.

## 2. The red cases, each run alone before the suite

**F1/F2** `adversary_conformance.rs:110` — `an_expected_error_that_is_an_outcome_of_another_operation_only_is_not_evidence` and `an_expected_error_the_contract_never_declares_is_not_evidence`. A suite scenario on `notes.read` with `expect: {error: notes.PreconditionFailed}` (an outcome of `notes.write` only), and one with `expect: {error: notes.Undeclared}`; a toolset that returns exactly that error. Asserts the runner refuses the suite or fails the scenario. Red output, verbatim (F1; F2 identical with `notes.Undeclared`):

```
notes.PreconditionFailed is not an outcome of notes.read, yet the runner took the scenario as evidence:
ok   read
ok   write
schema: notes.read: schemas: the toolset derives none for it
schema: notes.write: schemas: the toolset derives none for it
2 of 2 scenarios passed, 2 schema mismatches
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
EXIT=101
```

**F3** `adversary_conformance.rs:161` — `a_symlink_in_a_fixture_dir_is_laid_out_as_a_symlink`. A `fixture.dir` holding `a.txt` and `link -> a.txt`; asserts the workspace's `link` is a symlink after `run`. Red:

```
fixture dir copied `link -> a.txt` as FileType { is_file: true, is_dir: false, is_symlink: false, .. }, not as a symlink:
ok   read
ok   write
...
EXIT=101
```

**F4** `adversary_shapes.rs:44` — `an_operation_with_no_input_fields_matches_an_empty_derived_struct`. Contract operation with no `input`; `struct NoInput {}` derived schema. Red:

```
probe.op: input: (root): the derived schema has no shape: schema at #: an object with neither properties nor a value schema
derived schema:
{ "$schema": "https://json-schema.org/draft/2020-12/schema", "title": "NoInput", "type": "object" }
EXIT=101
```

**F5** `adversary_shapes.rs:44` — `an_optional_inline_enum_matches`. Contract `Optional<probe.Kind>` with variants `[file, directory]`; `Option<InlineKind>` with `#[schemars(inline)]`. Red:

```
probe.op: input: k: contract Enum[directory, file], derived Enum[directory, file, null]
derived schema: ... "k": { "type": ["string", "null"], "enum": ["file", "directory", null] }
EXIT=101
```

Logs of each alone-run: `red-*.log` and `probe-*.log` in the scratch directory (part 6).

## 3. The suite, after the cases existed

`cargo test --workspace --locked --no-fail-fast` via the wave's `cargo-serial.sh`, exit 101. Per binary: lib 2 ok, bin 0, adversary_cli 2 ok, adversary_conformance 2 ok / **3 FAILED**, adversary_shapes 10 ok / **2 FAILED**, adversary_validate 18 ok, cli 6 ok, conformance 9 ok, shapes 6 ok, validate 8 ok. Totals: passed 63, failed 5, executed 68. `error: 2 targets failed`. Log: `suite-after-fmt.log` (the pre-format run `suite.log` has identical counts).

## 4. Findings

| # | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|
| F1/F2 | `src/conformance.rs:496` `coverage` and `run_scenario` never compare `expect.error` or the returned error name with the called operation's `outcomes`; the profile doc line 92 says "Every error an operation can return is one of its outcomes". A scenario expecting an error that is an outcome of another operation, or of none, passes `ok`. | Any toolset crate whose implementation returns an error from the wrong operation (e.g. `fs.read` returning `code.PreconditionFailed`) and whose scenario author wrote the same name; the validator refuses "declared and never named" but nothing refuses "expected and never declared". | CONFIRMED | introduced |
| F3 | `src/conformance.rs:654` `fs::copy` follows symlinks, so a `fixture.dir` symlink arrives as a regular file; `walk` (:672) skips symlinks, so `unchanged` cannot name one either. Profile doc line 180: "`dir`: a directory to copy into the workspace". | The design doc line 128 declares `fs.Kind` variant `symlink` and line 250 `fs.stat` output `kind`; neither `fixture.files` (text only) nor `fixture.dir` can put a symlink in a workspace, so no conformance scenario can ever exercise that variant. | CONFIRMED | introduced |
| F4 | `src/types.rs:559` refuses `{"type":"object"}` with neither `properties` nor `additionalProperties`; schemars 1.2.3 emits exactly that for an empty struct. Profile doc lines 63-64 make `input`/`output` optional, line 262 maps `Struct` only from `type: object` with `properties`. An input-less or output-less operation can never pass the shape check. | Nothing found: every T1 operation in the design doc table (lines 245-256) has both input and output fields. Built by me. | INFEASIBLE | introduced |
| F5 | `src/types.rs:480` folds every `enum` member into the variants, including the `null` schemars adds when making an inline enum nullable; profile doc line 263 says variants are compared as a set of the enum's variants. `Optional<Enum>` through `$ref` (the fixture's path) is fine. | Nothing found: only `#[schemars(inline)]` enums take this path; no T1 crate exists yet to show one. Built by me. | INFEASIBLE | introduced |
| J1 | `docs/design/tool-contracts.md:236` gives the runner API as `run(suite, \|root\| make_toolset(root))`; the implemented and profile-documented signature is `run(suite, contract, scratch, make)`. | The brief names § T1 as "the runner API the toolset crates will build on"; three sibling units read it. | CONFIRMED | introduced (the API is new; the doc line predates the unit and is outside its scope list) |

Fixes I did not apply. F1/F2: in `coverage`, refuse a scenario whose `expect.error` is not the `error` of an outcome of `call.operation` (one line `<scenario>: expect: <error> is not an outcome of <operation>`), and in `run_scenario` fail a returned `Outcome::Error` whose name is not such an outcome. F3: `copy_tree` reads `symlink_metadata` and recreates links with `std::os::unix::fs::symlink`, and `walk` records a link's target as its "content", or the profile doc states that fixtures cannot carry symlinks. F4: either treat `type: object` without `properties` as `Struct{}`, or the profile doc states that an input-less operation must derive `properties: {}`. F5: `string_shape` drops `null` from `enum` (and `declared_type` already reports the nullability). J1: update the design doc line or have the profile doc declare itself the authority for the API.

## 5. Attacked and not broken (one line each)

- 18 validator rules with no prior test (rendering reuse, default>max, missing `timeout_ms`, unnamed error, undeclared error, duplicate operation, paging wrong side, empty paging, unknown precondition, trailing-dot capability, non-String Map key, undeclared type, unparsable type text, duplicate field, empty enum, duplicate outcome, unknown profile key, risk message naming floor and effect): all refused as the profile doc states.
- CLI: one accepted + one refused document exits 1 and still prints `ok` for the accepted one; wrong `format` exits 1 naming path, found and expected formats.
- Runner: one scenario name across two documents refused; suite version mismatch refused naming both versions.
- Shapes: `List<Optional>`, `Optional<List>`, `Map<String, Optional>`, `Bytes` and `Optional<Bytes>`, single-variant enum (`const`), `Optional<Enum>` and `Optional<Struct>` through `$ref`, newtype, i32/u8/usize/i64, list-of-struct difference reported at `ws[].to`: all match.
- Residue not made into a case: `holds` may name a property the called operation does not declare (e.g. `paging` on `notes.write`); the doc permits it literally (line 179), so a note only. `fixture.files` keys and `fixture.dir` are not confined to the workspace (`../` accepted); not a stated rule.

## 6. Paths written outside the worktree

All under `$HOME/.cache/tools-wave-t1/tool-contract-profile/scratch/adversary/`: `baseline-non-test.sha256`, `red-an_expected_error_that_is_an_outcome_of_another_operation_only_is_not_evidence.log`, `red-an_expected_error_the_contract_never_declares_is_not_evidence.log`, `red-a_symlink_in_a_fixture_dir_is_laid_out_as_a_symlink.log`, `probe-an_operation_with_no_input_fields_matches_an_empty_derived_struct.log`, `probe-an_optional_inline_enum_matches.log`, `suite.log`, `suite-after-fmt.log`, `fmt.log`, `fmt-after.log`, `clippy.log`. The wrapper also appends to its own `$HOME/.cache/tools-wave-t1/cargo-serial.log`. Build output went to the worktree's `target/` only; the session lease was released with `session-end`.

```findings
[
  {"file": "crates/tool-contract/src/conformance.rs", "line": 496, "category": "acceptance", "severity": "blocker", "verdict": "CONFIRMED", "origin": "introduced", "message": "a scenario whose expect.error is not an outcome of the called operation (declared elsewhere or nowhere) passes when the implementation returns that error, so the suite is evidence for an error the contract says the operation cannot return"},
  {"file": "crates/tool-contract/src/conformance.rs", "line": 654, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "copy_tree follows symlinks and walk skips them, so no fixture can lay out a symlink and the fs.Kind symlink variant the design declares can never be exercised by a scenario"},
  {"file": "crates/tool-contract/src/types.rs", "line": 559, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "schemars derives {type: object} for an empty struct and the check refuses it, so an operation without input or output fields, which the profile document allows, can never pass the shape check; no T1 operation is input-less"},
  {"file": "crates/tool-contract/src/types.rs", "line": 480, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the null schemars adds to an inline optional enum's enum list is folded into the variants, so Optional<Enum> mismatches unless the enum goes through $ref; only #[schemars(inline)] enums reach it"},
  {"file": "docs/design/tool-contracts.md", "line": 236, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the design document gives the runner API as run(suite, make) while the implementation and the profile document give run(suite, contract, scratch, make), and the sibling toolset units are told to build on the design document"}
]
```
