---
format: aep.planning-md/3
id: story:toolset-tool-meta
kind: story
status: draft
title: The tool-meta toolset searches, describes, invokes and activates any offered operation within the host's grant
relations:
- decomposes: epic:tool-contracts
- depends_on: story:profile-declares-ports
- informed_by: story:toolset-connector
- depends_on: story:connector-projection
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/catalogue/v1alpha1
- confidence: inferred
  path: contracts/tool-meta/v1alpha1
- confidence: inferred
  path: crates/tool-meta
- confidence: inferred
  path: crates/tool-ports/src/catalogue.rs
revision: 4
---
## Outcome

The `tool-meta` v1alpha1 toolset (`tool.search`, `tool.describe`, `tool.invoke`, `tool.activate`)
and the `catalogue` v1alpha1 port have contracts and conformance suites, and `crates/tool-meta`
passes them. A model finds and calls any operation the host offers, projected connector operations
included, and `tool.invoke` itself refuses a target outside the grant the host set.

## Context

The request (operator, 2026-10-21): a tool able to call any configured connector, or projections of
it. The design lists `tool-meta` (search, describe, invoke, activate,
`docs/design/tool-contracts.md` § Toolsets) and the inventory counts 9 tool meta layers
(`docs/inventory.md:24`); with connector operations projected into contracts
(`story:connector-projection`), calling any connector is this toolset over the projected contracts,
not a second meta layer. Round 1 of the plan review archived the earlier draft of a separate
connector toolset for that reason.

## Contracts

| op | input | output |
|---|---|---|
| `tool.search` | `query?`, `max_risk?`, `effects_within?` | `tools: List<tool.Entry{name, summary, risk, effects}>` |
| `tool.describe` | `name` | `name`, `summary`, `guidance`, `input_schema: Json`, `profile: Json`, `profile_digest` |
| `tool.invoke` | `name`, `input: Json`, `profile_digest` | `result: Json`, `truncated` |
| `tool.activate` | `names` | `offered` |
| `catalogue.list` (port) | — | the offered contracts with their profiles and digests |
| `catalogue.invoke` (port) | `name`, `input: Json` | `result: Json` |

| property | `tool.search`, `tool.describe`, `tool.activate`, `catalogue.list` | `tool.invoke`, `catalogue.invoke` |
|---|---|---|
| effects | `[read]` | every effect |
| consequences | `[]` | every consequence |
| risk | none | critical |
| idempotency | idempotent | non-idempotent |
| determinism | deterministic | nondeterministic |
| bounds | `timeout_ms {5000, 30000}`, `output_bytes {65536, 262144}` | `timeout_ms {60000, 600000}`, `output_bytes {65536, 262144}` |
| requires (toolset ops) | `ports: [catalogue.list/v1alpha1]` | `ports: [catalogue.list/v1alpha1, catalogue.invoke/v1alpha1]` |
| placement, capability | local, `tool.read` | local, `tool.invoke` |
| output | untrusted (descriptions are vendor-authored) | untrusted, failure as result |

`tool.invoke` declares the upper bound of what it can reach. The host passes a grant to the
constructor: `{max_risk, effects_within, consequences_within}`. Before calling, `tool.invoke` reads
the target's own profile from the catalogue and refuses:

- `tool.NotOffered{name}`: the target is not in the catalogue's offered set (hidden, not granted,
  or not activated);
- `tool.AboveGrant{name, property}`: the target's risk, an effect or a consequence is outside the
  grant;
- `tool.ProfileChanged{name}`: the target's profile digest differs from the `profile_digest` the
  caller read through `tool.describe`.

`tool.search` and `tool.describe` answer only offered targets. Approval, the human prompt, stays
with the harness. This story owns the `Catalogue` trait and its `Table` fixture, a list of
contracts with an offered flag each (`crates/tool-ports/src/catalogue.rs`).

## Scope

- `contracts/tool-meta/v1alpha1/`, `contracts/catalogue/v1alpha1/` (new)
- `crates/tool-meta/` (new), `crates/tool-ports/src/catalogue.rs`
- `Cargo.lock`: shared, merged serially.

## Acceptance

Fixture catalogue (in `Recorder`): `t.low` (risk low, `[read]`), `t.high` (risk high,
`[network, external]`), `t.ext` (`[network]`, consequences `[send-external]`, risk high), `t.hidden` (not offered), `t.deferred` (offered only after
activation), and `connector-tavily.search` projected from `story:connector-projection`'s fixtures
with the `connectors` `Table` fixture behind it. Grant: `{max_risk: high, effects_within: [read,
network, secret, external], consequences_within: []}` unless a bullet says otherwise.

- `tool.search` answers `t.low`, `t.high`, `t.ext` and `connector-tavily.search`, not `t.hidden`
  or `t.deferred`.
- `tool.search` with `max_risk: low` answers `t.low` only.
- `tool.describe t.hidden` answers `tool.NotOffered`.
- `tool.invoke t.low` with the digest from `tool.describe` (`calls`) answers the fixture result.
- With grant `max_risk: medium`, `tool.invoke t.high` answers `tool.AboveGrant` naming `risk`, and
  `.ports/catalogue.jsonl` has no `catalogue.invoke` line.
- With grant `effects_within: [read]`, `tool.invoke t.high` answers `tool.AboveGrant` naming
  `effects`.
- `tool.invoke t.ext` answers `tool.AboveGrant` naming `consequences`.
- `tool.invoke t.hidden` answers `tool.NotOffered`.
- `tool.invoke t.low` with a stale digest answers `tool.ProfileChanged`.
- `tool.search`, then `tool.activate [t.deferred]`, then `tool.search` (`calls`): the first answer
  lacks `t.deferred`, the third has it.
- `tool.invoke connector-tavily.search` with its digest answers the `connectors` table result.
- A result above `output_bytes` answers `truncated: true`.
- Two identical `tool.invoke t.low` calls leave two `catalogue.invoke` lines.

## Not in this story

- Building the catalogue from harness grants: the harness's.
