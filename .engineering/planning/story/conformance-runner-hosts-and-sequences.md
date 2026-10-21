---
format: aep.planning-md/3
id: story:conformance-runner-hosts-and-sequences
kind: story
status: active
title: The conformance runner serves HTTP fixtures, passes host configuration and runs several calls on one toolset
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: inferred
  path: contracts/profile/tool-contract-1.md
- confidence: inferred
  path: crates/tool-contract/src/conformance.rs
- confidence: inferred
  path: crates/tool-contract/tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-21T00:33:23Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

The `tool-conformance/1` runner can run a suite for a toolset that needs its host: a scenario
carries host configuration the model never sees, the runner can serve HTTP fixtures on loopback, a
scenario can make several calls on one toolset instance, and a fixture port's call log is
checkable. Every key is optional, so every released suite runs unchanged.

## Context

Review of 2026-10-21 (5 reviewers over the web, http, browser and tool-meta drafts): the runner
lays out files, makes one call on a fresh toolset and matches its output
(`crates/tool-contract/src/conformance.rs:274-310`, `:611-612`); `Fixture` holds only `files` and
`dir` (`:292-299`); `call.input` is literal JSON (`:309`); `run` takes
`make: impl Fn(&Path) -> Box<dyn Toolset>` (`:570-575`). None of the planned network suites can be
written in that grammar: no served endpoint, no host allow-list, no second call for a stale ref or
a repeated request.

## Design

| key | where | meaning |
|---|---|---|
| `host` | scenario | a YAML map handed to the toolset factory, never to the operation: allow-lists, secret values, a grant ceiling, port fixture tables |
| `serve` | scenario fixture | routes the runner serves on `127.0.0.1:<ephemeral>` over plain HTTP/1.1 (`std::net`, no new dependency): `path → {status, headers, body \| body_file, redirect, delay_ms}`; each request a route receives is appended to `.serve/<path with / as _>.jsonl` in the workspace: `{"method", "path", "headers", "body"}` |
| `{{serve}}`, `{{workspace}}` | any string in `call.input` or `host` | replaced with the served base URL and the scenario's workspace path |
| `calls` | scenario, instead of `call` | an ordered list of `{operation, input, expect}` on one toolset instance; `{{calls.<n>.output.<path>}}` reads an earlier answer |
| `.ports/<port>.jsonl` | workspace | where a fixture port writes one line per call; checked with `expect.files` and the new matcher `{$lines: n}` |

API: a new `conformance::run_with(suite, contract, scratch, make)` whose `make` takes a
`ScenarioContext { workspace, host, served }`; `run` stays and calls it, so no released crate's
tests change. A suite with `calls` counts each entry's `holds` toward coverage.

A missing program or setting the suite needs fails the run, naming it, as the build-test suite does
(`crates/tool-toolchain/tests/conformance.rs:6-8`); there is no skipped state.

## Scope

- `crates/tool-contract/src/conformance.rs`
- `contracts/profile/tool-contract-1.md` (§ Conformance: the keys above)

## Acceptance

- Every released suite passes unchanged: `cargo test -p b10x-tool-fs -p b10x-tool-code-editor
  -p b10x-tool-process -p b10x-tool-search -p b10x-tool-toolchain` exits 0.
- A test toolset in `crates/tool-contract/tests/` that echoes its `host` map passes a scenario
  whose `expect.output` names a `host` value, and fails when the runner drops `host` (a mutant).
- A scenario with `serve: {/a: {status: 200, body: "x"}}` and a test toolset that GETs
  `{{serve}}/a` answers `x`.
- A `calls` scenario of two calls passes only when the second call reads the first call's output
  through `{{calls.0.output.id}}`.
- `{$lines: 2}` passes on a two-line `.ports/fixture.jsonl` and fails on a three-line one.
- A served route is refused at suite parse when its path does not start with `/`.
- A request to `{{serve}}/a` with header `X-T: 1` leaves one line in `.serve/_a.jsonl` containing
  `X-T`.
- A route with `delay_ms: 2000` answers after at least 2000 ms.
- A test factory that reports a missing program `probe-bin` fails the run with a line naming
  `probe-bin`, and the summary counts no scenario as passed.

## Not in this story

- Port documents and the offering rule: `story:profile-declares-ports`.
- A clock or time steps: session idle limits stay crate unit tests.
