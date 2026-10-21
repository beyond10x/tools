---
format: aep.planning-md/3
id: story:profile-declares-ports
kind: story
status: draft
title: An operation declares the ports its host must bind, and a port is a versioned contract
relations:
- decomposes: epic:tool-contracts
- depends_on: story:conformance-runner-hosts-and-sequences
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/profile/tool-contract-1.md
- confidence: inferred
  path: crates/tool-contract/src/bin/tool-contract.rs
- confidence: inferred
  path: crates/tool-contract/src/conformance.rs
- confidence: inferred
  path: crates/tool-contract/src/document.rs
- confidence: inferred
  path: crates/tool-contract/src/types.rs
- confidence: inferred
  path: crates/tool-contract/src/validate.rs
- confidence: inferred
  path: crates/tool-contract/tests/fixtures/ports
- confidence: inferred
  path: crates/tool-ports
revision: 6
---
## Outcome

An operation declares the ports its host must bind, a port is a versioned `kind: port` contract,
the validator reads the contracts as a set and refuses an operation that hides a port's effects,
the Rust port traits live in one crate with their fixtures, and a toolset answers which of its
operations are offered given what is bound. `docs/design/tool-contracts.md` § Ports is the design
(recorded 2026-10-21).

## Context

The operator asked (2026-10-21) for a fetch whose summarizer is modelled as a contract the harness
fulfils and injects. `requires` today is `{binaries, env, settings, secrets}`
(`contracts/profile/tool-contract-1.md:110`); `Requires` and `Contract` refuse unknown keys
(`crates/tool-contract/src/document.rs:70-71`, `:366-367`); `validate` takes one document
(`crates/tool-contract/src/validate.rs:35`) and the binary validates file by file
(`crates/tool-contract/src/bin/tool-contract.rs:95-96`); the build script embeds
`contracts/<toolset>/<version>/contract.yaml` at that depth only (`crates/tool-contract/build.rs:56-65`),
which is why ports sit at the same depth. The seam pattern exists: `Executor<R: Runner>` takes a
synchronous caller-supplied runner (`crates/tool-toolchain/src/executor.rs:396`, `:412`).

## Design

- `Contract` gains `kind: toolset | port`, default `toolset`. A port document sits at
  `contracts/<port>/<version>/`; a port name may not equal a toolset name.
- `Requires` gains `ports: [<port operation>/<version>]`, naming the port operations the
  operation calls, such as `egress.fetch/v1alpha1`.
- `validate_set(&[Contract])`: rule `ports` refuses a named port operation no `kind: port`
  document declares; rule `port-effects` refuses an operation whose effects or consequences omit
  one of a named port operation's, or whose risk is under the highest of theirs. The binary
  validates `contracts/` as one set. `validate(&Contract)` keeps its signature and its rules.
- `Toolset` gains `fn offered(&self) -> Vec<String>`, a default method answering every operation,
  so no released implementation changes. A toolset built without a port binding answers its
  operations minus those that name the port. A port document has no `Toolset`, so a port operation
  is never offered.
- `crates/tool-ports` (new): `src/lib.rs` with one `mod` line per port in the first set
  (`egress`, `websearch`, `summarize`, `connectors`, `catalogue`) and an empty `src/<port>.rs`
  each, so a port story edits only its own file; the `fixtures` feature in `Cargo.toml`; and
  `Recorder<P>`, which wraps any port implementation and appends one JSON line per call to
  `.ports/<port>.jsonl`: `{"operation": "<port op>", "input": {…}, "outcome": "ok" | "<error
  name>"}`. Each port's trait and its fixtures (`Echo`, `Table`, `Refuse`, as that port needs)
  land with the story that adds the port.
- The profile document defines `external`: a third party carries out the call on the caller's
  behalf, may keep its input and may spend credit. `output.untrusted` becomes declarative
  (validator-only): the harness applies it from the contract, so no scenario observes it.
- A `Json` primitive joins the type grammar: any JSON value, schema shape `Any`, for inputs and
  answers whose shape is another contract's (`tool.invoke`, projected connector operations).

## Scope

- `contracts/profile/tool-contract-1.md` (`kind`, `requires.ports`, rules `ports` and
  `port-effects`, `external`, `Json`, `output.untrusted` declarative)
- `crates/tool-contract/src/document.rs`, `validate.rs`, `types.rs`, `conformance.rs`
  (`offered`), `bin/tool-contract.rs`
- `crates/tool-contract/tests/fixtures/ports/` (new)
- `crates/tool-ports/` (new: `Cargo.toml`, `src/lib.rs`, `src/recorder.rs`, the five empty
  `src/<port>.rs`)
- `Cargo.lock`: shared with every new-crate story below; units merge serially into the wave branch
  (AGENTS.md § Branches and waves) and the later one regenerates it.

## Acceptance

- `cargo run -p b10x-tool-contract -- validate contracts/` exits 0 over every released contract,
  unchanged.
Test documents live in `crates/tool-contract/tests/fixtures/ports/`: a port `probe` v1alpha1 with
one operation `probe.call` (effects `[external]`, consequences `[money]`, risk high), and a toolset
`user` whose operation `user.go` names `requires.ports: [probe.call/v1alpha1]`.

- `cargo run -p b10x-tool-contract -- validate contracts/` exits 0 over every released contract,
  unchanged.
- `user.go` naming `requires.ports: [nope.call/v1alpha1]` is refused with one line naming
  `user.go`, `ports` and `nope.call/v1alpha1`.
- `user.go` naming an operation of a `kind: toolset` document is refused by rule `ports`.
- `user.go` without `external` in its effects is refused by rule `port-effects`, naming `external`.
- `user.go` without `money` in its consequences is refused by rule `port-effects`, naming `money`.
- `user.go` at risk medium is refused by rule `port-effects`, naming the risk.
- `user.go` with `[external]`, `[money]` and risk high is accepted.
- A port document under `contracts/` is returned by `documents::all()`.
- A field typed `Json` passes the schema check against a `serde_json::Value` field.
- `offered()` on every released toolset answers all its operations.
- A test toolset built without its `probe` binding answers `offered()` without `user.go`.
- `Recorder` over a test port writes one line per call, with the operation name and outcome,
  checked with `{$lines: n}` and `$contains`.

## Not in this story

- Any real port document: `egress` (`story:toolset-http`), `websearch` (`story:toolset-web`),
  `summarize` (`story:web-ask-through-summarize-port`), `connectors`
  (`story:connector-projection`), `catalogue` (`story:toolset-tool-meta`).
- Harness binding: harness and metaharness adopt in their own repositories.
