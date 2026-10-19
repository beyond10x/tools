# Tool contracts

Status: draft, 2026-10-19. Decided by the owner on 2026-10-19: tool contracts and their default
implementations live in this repository (`contracts/`, `crates/tool-<toolset>`); atlas ADR 0095
records it.

## Problem

Each agent harness in the organization re-implements the tool layer. The inventory
([../inventory.md](../inventory.md)) counted, across the beyond10x repositories and earlier private
agent projects: 14 independent file readers, 15 shell or process runners, 13 delegation layers, 9
tool meta layers, 8 web fetchers, 6 git layers, 16 vocabularies for effects, risk and approval, and 5
hand-written MCP JSON-RPC servers. No contract file is consumed by two programs. The same operation
has five names (`file_read`, `file.read`, `Read`, `show`, `code_read`), and the properties a governor
needs (what a call may touch, whether it can be retried, how long it may run) are declared in some
layers, inferred in others and absent in most.

## Model

A **tool contract** is an ESS command plus a `tool-contract/1` profile.

The ESS command gives the operation's input, its output, its outcomes and its errors. The profile
gives what a harness, a governor and a reviewer need to know about the operation without running
it. Each property below is taken from the best existing form the inventory found:

| property | meaning | taken from |
|---|---|---|
| `effects` | read, write, process, network, secret, external, human-visible | P1 `Effect`; platform module-core |
| `consequences` | money, delete, send-external, irreversible, sensitive-data | P1 semantic effects; P2 semantics |
| `risk` | none, low, medium, high, critical; checked against `effects` by the profile validator | P1 tool-spec invariants |
| `idempotency` | idempotent; keyed by a caller `operation_id`, with a reconcile operation; non-idempotent | substrate; platform module-core |
| `preconditions` | `expected_sha256`, `expected_revision` | workspace; entity-runtime |
| `determinism` | deterministic or not, for replay and caching | P2 semantics |
| `bounds` | timeout default and maximum, output bytes, processes, cpu, memory; required and capped | substrate; P3 timeout middleware |
| `paging` | offset, limit, digest of the whole, `truncated` | substrate; epistemic-knowledge-runtime |
| `requires` | binaries, environment variables, settings, secrets | declared nowhere today |
| `subjects` | the input fields that name paths, URLs or argv, and the permission patterns they suggest | P4; P5; P1 |
| `placement` | local, confined, container | P1 placement; substrate |
| `capability` | the Canon capability the operation serves (ADR 0083) | platform provider driver; takt protocols |
| output | typed result blocks; canonical content separate from the model's view; `untrusted` marking; failure returned as a result | P1; P3; epistemic-knowledge-runtime; platform |
| presentation | guidance separate from description; tags; aliases; group activation | P3; P6; P1 |

Names are dotted family ids: `fs.read`, `fs.list`, `fs.glob`, `code.write`, `code.edit`,
`code.patch`, `search.grep`, `process.run`, `process.start`, `web.fetch`, `web.search`, `git.diff`,
`agent.spawn`, `tool.search`. Claude, Codex and MCP names are renderings generated from a table in
the contract, never the source.

## Toolsets

| toolset | operations (first version) |
|---|---|
| `fs` | read (paged, with whole-file digest), list, glob, stat |
| `code-editor` | write, edit with a preimage digest, patch (several edits resolved against the original), move, delete |
| `process` | run (argv only, bounded, no network unless granted), shell (gated), start, output, wait, signal |
| `search` | grep, code search |
| `web` | fetch (SSRF guard, body cap), search, crawl |
| `git` | status, diff, log, stage, commit, branch, worktree |
| `forge` | pull request open and read, CI read, merge |
| `build-test` | toolchain runs (cargo, go, npm, pytest, task) |
| `browser` | open, snapshot, act, close |
| `code-intel` | definitions, references, outline, diagnostics |
| `delegation` | spawn, await, message |
| `memory` | remember, recall, forget |
| `interaction` | ask the user, notify |
| `tool-meta` | search, describe, invoke, activate |
| `control` | turn done, plan and todo |

`fs` and `code-editor` come first: every harness has them, and they exercise most profile
properties (subjects, preconditions, paging, idempotency).

## Conformance

Each toolset version ships scenarios and fixtures. "Implements `fs` v1alpha1" means passing them.
Every profile property has at least one scenario that fails when an implementation ignores it (a
mutant that drops the bound, the precondition or the truncation flag fails a scenario). An
implementation derives its schemas from its types; the suite checks the derived schema equals the
contract's.

## Default implementations

`crates/tool-<toolset>`, Rust. Seeds, best first per toolset: the P1 tools (fs, code
editing, process, git, web), substrate (confined process and file operations), harness-tools. A
toolset digest over the sorted operations and a frozen-contract digest per published workflow make
drift visible.

## How harnesses adopt it

- A protocol action grants a Canon capability; a harness offers exactly the tools whose contracts
  serve granted capabilities ("offered" vs "admitted" kept apart). Commission binds and invokes
  (ADR 0082, 0083).
- Grants can select tools by declared facts (maximum risk, effects within a set) instead of names.
- First adopters: harness (fs and code-editor from this repository), metaharness (vendor renderings
  from the contracts), takt (`--allowedTools` derived from capabilities). Then loom, conductor,
  agentide, agent-platform, devcenter and substrate map to the contracts in their own waves.

## Phases

| phase | work |
|---|---|
| T0 | this repository, its store, atlas ADR 0095 and catalog entry |
| T1 | the `tool-contract/1` profile and its validator; `fs` and `code-editor` contracts with suites; `crates/tool-fs`, `crates/tool-code-editor` passing them |
| T2 | harness, metaharness and takt adopt T1 |
| T3 | the remaining toolsets in the order above; the other adopters |
