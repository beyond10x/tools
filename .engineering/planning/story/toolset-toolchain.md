---
format: aep.planning-md/3
id: story:toolset-toolchain
kind: story
status: active
title: The toolchain registry moves from harness and derives build-test operations under the profile
relations:
- decomposes: epic:tool-contracts
- depends_on: story:tool-contract-profile
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/tool-toolchain
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T01:52:30Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-20T01:54:15Z", actor: "human:timo", revision: 4}
---
## Outcome

`crates/tool-toolchain` is harness's declarative toolchain registry moved here:
`crates/harness-toolchain/src/lib.rs` and `builtins/{rust,go,taskfile,npm,yarn}.yaml` at harness
0.13.3 (`798325f`), with its tests. Every `ResolvedTool` derives one `build-test.<tool>` operation
under the `tool-contract/1` profile (effects `process`, plus `write` when the tool declares
`workspace-writes`; `requires.binaries` from `sandbox.programs`, `requires.env` from `sandbox.env`;
`subjects` from `writes` parameters and `argv[0]`; `placement` from the sandbox), and the five
built-in providers' derived operations pass `tool_contract::validate`.

## Context

Owner decision 2026-10-20 (atlas ADR 0097): `harness-toolchain` becomes a tools crate. The design
(`docs/design/tool-contracts.md` § Toolsets) lists `build-test` as the toolset for toolchain runs.
The registry fits the profile: a tool's programs, environment, roots and written paths are already
declared in its YAML, which is what `requires` and `subjects` ask for; what the registry does not
hold is a static contract document with a conformance suite, because its operations are resolved
per workspace (which providers match, which task names exist). That static contract is
`story:build-test-contract`.

## Scope

- `crates/tool-toolchain/` (new): `src/lib.rs` and `builtins/` from harness, `src/contract.rs`
  (the derivation), tests
- `Cargo.lock` (shared with the other crates of the wave; regenerated at merge)

## Acceptance

- harness-toolchain's eight tests pass in `crates/tool-toolchain` unchanged in substance.
- `Registry::builtins()` resolves each built-in provider against a fixture workspace and every
  derived operation passes `tool_contract::validate`, in a test.
- A tool with `workspace-writes: true` derives `effects: [process, write]` and `risk: medium`; one
  without derives `effects: [process]`; a tool whose parameter is `workspace-path-list` and is named
  in `writes` derives a `path` subject on that field.
