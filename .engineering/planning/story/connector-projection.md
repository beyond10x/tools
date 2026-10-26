---
format: aep.planning-md/3
id: story:connector-projection
kind: story
status: archived
title: A connectors descriptor projects to tool-contract operations, selected and renamed
relations:
- decomposes: epic:tool-contracts
- depends_on: story:profile-declares-ports
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/connectors/v1alpha1
- confidence: inferred
  path: contracts/profile/tool-contract-1.md
- confidence: inferred
  path: crates/tool-connector
- confidence: inferred
  path: crates/tool-ports/src/connectors.rs
revision: 8
transitions:
- {from: "draft", to: "archived", at: "2026-10-26T15:35:59Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":7}}}
---
## Outcome

Archived on 2026-10-26, before any work: the story moved to the Connectors repository
(https://github.com/beyond10x/connectors), where it is `story:connector-projection` under
`epic:tool-contract-publication`. ADR 0103 decision 5: Connectors emits a `tool-contract/1` document
per admitted operation and passes `tool-conformance/1`; the profile and the input fields the
connection fills (this story's `omit`) move into Connectors' operation definition; which operations a
run sees stays the harness's grant. Decision 3: tools takes no dependency on the connectors
descriptor format, and `crates/tool-connector`, `contracts/connectors/v1alpha1`, the `Connectors`
port and the `tool-projection/1` section are not built here. Recorded in
`approval-record:connector-projection-moves-to-connectors`.

The outcome as drafted, kept with the sections below as the record of the plan this replaces:

> `crates/tool-connector` turns a connectors `Descriptor` and a host-authored `tool-projection/1`
> document into an in-memory `tool-contract/1` contract, one operation per selected connector
> operation, each with the full profile the projection gives it, and runs those operations through
> the `connectors` v1alpha1 port. A host offers connector operations as tools from this one function
> instead of compiling them itself.

## Context

Three beyond10x repositories compile connector operations to tools today: devcenter
(`devcenter/crates/devcenter-mcp/src/lib.rs:31,57`), agent-platform
(`agent-platform/crates/agent-platform-connectors/src/lib.rs:191`) and platform
(`platform/runtime/agent/crates/agent-connector-tool/src/lib.rs:150`); `docs/inventory.md` names
all three. A connectors `Operation` is `{id, description, contract, profile, input_schema,
output_schema}`, and `profile` is a provider profile name, `"tavily/2026-10"`
(`connectors/crates/connectors-core/src/lib.rs:83-92`,
`connectors/adapters/tavily/generated/descriptor.json`): it carries no effects or risk, so the
projection supplies them. P1's connector pack shows the selector shape (by service, path, method,
risk) and hides most of a large catalogue from the model.

## Design

- Input: descriptor JSON, parsed by this crate's own serde types (no dependency on the connectors
  crates, so no async runtime and no git dependency), and a projection document.
- `tool-projection/1` (defined in the profile document): `instance`; `select` (connector operation
  id globs); per selected operation `name` (the `<op>` segment), `summary`, `profile` (a full
  `tool-contract/1` profile), `omit` (input fields the host fills, with their values from host
  configuration). An operation selected with no `profile` is refused, never defaulted.
- Output: `toolset` and `family` `connector-<instance>`; an instance name outside
  `[a-z][a-z0-9-]*` is refused. Operation `connector-<instance>.<name>`.
- Types: the JSON Schema subset the grammar has (string, integer, boolean, array, object with
  `properties`, `enum` as an enum type, a `["T", "null"]` type list as `Optional<T>`) converts to
  field lists; any other shape (`oneOf`, `anyOf`, `number`, an object without `properties`) is
  refused naming the operation and the schema path, unless the projection marks that field
  `as: json`, which types it `Json` (`story:profile-declares-ports`).
- Every projected operation's `requires.ports` names `connectors.invoke/v1alpha1`; rule
  `port-effects` then holds the projection to at least the port's effects.
- Port `connectors.invoke {instance, operation, input: Json} → {result: Json}`: effects
  `[network, secret]`, risk medium, non-idempotent, nondeterministic, `timeout_ms {30000, 300000}`,
  `output_bytes {65536, 262144}`, placement local, capability `connector.invoke`. The connectors
  host makes the outbound call with its own HTTP client, outside `egress`; its address policy is the
  connectors host's.
- At invoke: an input key named in `omit` is refused (`connector.FieldNotAllowed`); the host's value
  is sent.
- A projected contract never lands under `contracts/`; its digest is computed like a released
  contract's.
- This story owns the `Connectors` trait and its `Table` fixture
  (`crates/tool-ports/src/connectors.rs`).

## Scope

- `crates/tool-connector/` (new), `crates/tool-ports/src/connectors.rs`
- `contracts/connectors/v1alpha1/` (new, the port)
- `contracts/profile/tool-contract-1.md` (§ `tool-projection/1`)
- `Cargo.lock`: shared, merged serially.

## Acceptance

Fixtures under `crates/tool-connector/tests/fixtures/`: `descriptor.json` (the three Tavily
operations from `connectors/adapters/tavily/generated/descriptor.json`), `descriptor-union.json`
(the same with `websearch.search`'s `topic` field written as `oneOf`), and `projection.yaml`
(selects `websearch.search` and `websearch.fetch`, names them `search` and `fetch`, gives each
effects `[network, secret, external]`, risk high, and omits `country` on `search` with the host
value `"de"`).

- The projected contract passes `validate_set` with the port documents.
- It has 2 operations, named `connector-tavily.search` and `connector-tavily.fetch`.
- `websearch.crawl`, not selected, is absent.
- Each operation's profile equals the projection's, field by field (effects, consequences, risk,
  idempotency, determinism, bounds, subjects, requires, placement, capability, output); a mutant
  per field that defaults it fails.
- A selected operation with no `profile` is refused, naming it.
- A projection whose `search` effects omit `secret` is refused by rule `port-effects`.
- An instance named `Tavily.Prod` is refused.
- `descriptor-union.json` is refused, naming `connector-tavily.search` and `/properties/topic`.
- The same with `topic` marked `as: json` is accepted and types `topic` `Json`.
- `country` is absent from the projected input schema of `search`.
- A conformance suite in `crates/tool-connector/tests/` over the projected contract, with the
  `connectors` `Table` fixture in `Recorder`, passes:
  - `search` answers the table's result.
  - The `.ports/connectors.jsonl` line for that call contains `"country":"de"`.
  - An input carrying `country` answers `connector.FieldNotAllowed`, and no new line is written.

## Not in this story

- Invoking a projected operation by name from a model: `story:toolset-tool-meta`.
- Which projection a run sees: the harness's grant.
