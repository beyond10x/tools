# tools

Agent tool contracts (`contracts/`) and their b10x default implementations (`crates/tool-*`). This
file is for agents working on this repository; `README.md` is for people.

## Start here

| read | for |
|---|---|
| `docs/design/tool-contracts.md` | the contract model, the toolsets, conformance, how harnesses adopt it |
| `docs/inventory.md` | the existing tool layers each contract property was taken from |
| `aep plan artifact list` | the planned work: epic `tool-contracts` and its stories |

## Rules

- A contract is a specification. It names no implementation, no vendor tool name and no harness.
  Vendor names (Claude, Codex, MCP) are generated renderings, never the source.
- A contract change is versioned: a released `v1alpha1` is never edited in place; a change that a
  conforming implementation could fail is a new version beside it.
- "Implements" means "passes the toolset's conformance suite". Every profile property has at least
  one scenario that fails when an implementation ignores it.
- Implementations derive their schemas from their types; the conformance suite checks the derived
  schema equals the contract's. No hand-written schema in a crate.
- Capabilities are Canon's (atlas ADR 0083): a contract names the capability it serves; it does not
  define a new authority vocabulary. Commission binds and invokes (ADR 0082).
- Public repository: no person's name, employer, customer, private repository name or private
  hostname in any committed text, commit message or release note.
- All Git writes (commits, merges, pushes, tags) go through the bot route (`b10x-gates bot -- …`).
- The planning store changes only through `aep plan artifact` verbs.

## Commands

```
task check                      # store validation; contract and crate checks join as they land
aep plan artifact validate
aep plan artifact waves --kind story --status draft
```
