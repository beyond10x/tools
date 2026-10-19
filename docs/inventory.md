# Inventory of agent tool layers, 2026-10-19

Read-only scans of every beyond10x repository and of earlier private agent projects, made before
this repository's contracts were designed. Private projects are not named; they are P1 to P6:

| label | what it is |
|---|---|
| P1 | a Rust agent runtime with a plugin protocol, a connector catalogue and an exchange for published workflows |
| P2 | two Go predecessors of P1, one archived, one a prototype |
| P3 | a Go agent SDK with per-call intents, a risk gate and a command risk assessor, and three related copies |
| P4 | two Go coding agents |
| P5 | a fork of an open-source coding agent |
| P6 | smaller Go, Rust and TypeScript agent experiments |

Counts are counted from the scan tables; each row was read from the code at file and line.

## Independent implementations per family

| family | implementations | where |
|---|---|---|
| shell / process | 15 | harness, agentide, substrate, P1, P2 (2), P3 (4), P4 (2), P5, P6 (2) |
| file read / list / glob | 14 | harness, conductor, workspace, substrate, P1, P2 (2), P3 (4), P4 (2), P5 |
| delegation / subagents / messaging | 13 | harness, conductor, agentide, P1, P2, P3 (4), P4, P5, P6 (2) |
| tool meta (search, describe, invoke, activate) | 9 | harness, metaharness, platform, devcenter, P1, P3 (4) |
| web fetch / search | 8 | P1, P2, P3 (4), P4, P5 |
| git | 6 | conductor, workspace, P1, P2, P3 (2) |
| memory / knowledge | 6 | epistemic-knowledge-runtime, platform, P2, P3 (3) |
| effect / risk / approval vocabularies | 16 | harness, agentide, connectors, takt, devcenter, platform (5 crates), service-sdk, P1, P2, P3, P4, P5 |
| hand-written MCP JSON-RPC servers | 5 | epistemic-knowledge-runtime, devcenter, entity-runtime, substrate, cortex |

No contract file is consumed by two programs. Shared vocabularies exist only inside one family
(P1's tool spec within P1; P3's interface, copied twice).

## beyond10x repositories with a tool layer

| repository | contract site |
|---|---|
| harness | `harness-wire` `ToolSpec`, `Envelope{effects, risk, idempotency, access}`, `ToolPort`; `harness-tools` catalogue; toolchain YAML |
| metaharness | neutral `Operation` enum with Claude and Codex name tables |
| loom | a copy of harness wire and loop at 798325f |
| conductor | an ESS domain for its tool broker with role grants; a guard over vendor tool names |
| agentide, workspace | an intent profile over ESS commands (effect, risk, approval, subjects); `code_edit` with an `expected_sha256` precondition |
| agent-platform, platform, zwirn | connector operations projected to tools; operation descriptors with effect, risk floor and keyed idempotency; meta tools with byte caps |
| devcenter | MCP tools compiled from connector operations; effect and approval posture; toolset and input digests |
| substrate | MCP tools for confined workspaces and processes; caller operation ids; required bounded limits; leases; paged reads with digests |
| entity-runtime | generated MCP tools for entities; `expected_revision` |
| epistemic-knowledge-runtime, cortex | MCP knowledge tools marked untrusted, with a truncation contract; an MCP client and proxy with redaction |
| service-sdk, connectors | service operations; adapter specifications; versioned contracts under `contracts/` |
| takt | vendor tool allow-lists plus protocol actions with `effect` and `requires: capability` |
| intake, llm, uilab, loom-coder | one forced tool used as structured output |

## Contract properties found

| property | best existing form |
|---|---|
| effects | P1, platform module-core |
| semantic consequences | P1, P2 |
| risk, checked against effects | P1 |
| idempotency keyed by a caller operation id, with reconcile | substrate, platform module-core |
| preconditions | workspace, entity-runtime |
| determinism | P2 |
| bounds, required and capped | substrate, P3 |
| paging and truncation | substrate, epistemic-knowledge-runtime |
| required binaries and environment | none: discovered at run time everywhere |
| permission subjects and suggested patterns | P4, P5, P1 |
| per-call intent | P3, P1 |
| approval posture; allow / requires approval / reject / error | devcenter, P3, P2 |
| capability served | platform, takt |
| output schema; typed result blocks; content vs model view | P1, P3, P6 |
| untrusted marking; redaction | epistemic-knowledge-runtime, cortex, P1 |
| guidance, tags, aliases | P3, P6 |
| failure returned as a result | platform |
| streaming progress | P1, P4, P6 |
| dry run | P1 |
| toolset digest; frozen contract digest | devcenter, P1 |
| grants by declared facts | P1 |
| offered vs admitted | swarm |
| group activation; deferred loading | P1, P3, entity-runtime |
