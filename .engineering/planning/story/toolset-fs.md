---
format: aep.planning-md/3
id: story:toolset-fs
kind: story
status: implemented
title: The fs toolset has a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: contracts/fs/v1alpha1
- confidence: cited
  path: crates/tool-fs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T01:52:29Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-20T01:54:15Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-20T05:03:56Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}}
---
## Outcome

The `fs` v1alpha1 toolset (`fs.read`, `fs.list`, `fs.glob`, `fs.stat`) has a contract, semantics and a
conformance suite, and `crates/tool-fs` passes the suite. The crate is harness's local file reader
moved here: `crates/harness-tools/src/local.rs` at harness 0.13.3 (`798325f`), its `read`, `list`,
`find_paths`, `walk`, `PathGlob`, `next_line` and `contained` functions and their tests, with the
same reply fields, bounds and refusals, plus a whole-file `sha256` on reads and the new `fs.stat`.

## Context

The inventory's 14 file readers; the best forms: paged reads with a whole-file digest (substrate), a
truncation contract (epistemic-knowledge-runtime), read caps (P1). harness's reader already has the
truncation contract (`truncated`, `truncated_lines`, `lines.total` null past the 16 MiB counting
bound, `depth_bound_reached`) and the caps (64 KiB default, 256 KiB ceiling, 500 entries, 500 paths,
depth 12); `docs/design/tool-contracts.md` § Operation sets fixes the field names. Owner decision
2026-10-20 (ADR 0097): harness's tools move here and harness consumes them.

## Scope

- `contracts/fs/v1alpha1/` (new, cited): `contract.yaml`, `semantics.md`, `scenarios/` (9 files, 70
  scenarios, 4 of them `adversary-*`), `fixtures/` (`linked/`, `large/`, `over-ceiling/`,
  `adversary-links/`, `adversary-chain/`, `adversary-long-line/`, with committed symlinks)
- `crates/tool-fs/` (new, cited): `src/{lib,line,walk,types,error,toolset,tests}.rs`,
  `tests/{conformance,mutants,adversary}.rs`
- `Cargo.lock` (cited; shared with the other crates of the wave, regenerated at merge)

Confirmed by the implementor on 2026-10-20: every cited path exists as named; no inferred line.

## Acceptance

- The suite passes against `crates/tool-fs` (`cargo test -p b10x-tool-fs`).
- Each of these mutants of `crates/tool-fs` fails at least one scenario: no read cap, no `truncated`
  flag, a digest of the page instead of the whole file, a path outside the subject root accepted.
- `crates/tool-fs` derives its schemas from its types, and the suite checks they equal the contract's.
- harness's `local.rs` tests for read, list, find, links and line handling run here unchanged in
  substance (renamed where the API moved) and pass.
