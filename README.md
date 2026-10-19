# tools

Contracts for the tools an AI agent calls, and the b10x default implementations of them.

Every agent harness ends up writing its own file reader, editor, shell runner and web fetcher, each
with its own names, its own idea of side effects and its own limits. An inventory across our agent
systems counted 14 independent file readers, 15 shell runners and 16 vocabularies for effects, risk
and approval, and no contract file shared by any two programs ([docs/inventory.md](docs/inventory.md)).

This repository separates the contract from the implementation:

- `contracts/` holds language-neutral tool contracts: a toolset's operations, their inputs and
  outputs, and the properties a harness needs to govern them (effects, risk, idempotency, bounds,
  required binaries and environment, the paths or URLs a call touches, the capability it serves).
  Each toolset is versioned and ships a conformance suite.
- `crates/tool-<toolset>` holds the b10x default implementations in Rust. An implementation
  "implements `fs` v1alpha1" when it passes that toolset's conformance suite; any harness can link
  these crates or bring its own implementation and pass the same suite.

Status: started 2026-10-19. The design is [docs/design/tool-contracts.md](docs/design/tool-contracts.md);
the first contracts (`fs`, `code-editor`) and their crates are planned in the AEP store under
`.engineering/planning`.

## Layout

| path | what |
|---|---|
| `contracts/` | the `tool-contract/1` profile and one directory per toolset and version |
| `crates/` | default implementations, `tool-<toolset>` (none yet) |
| `docs/design/` | the design |
| `docs/inventory.md` | the inventory of existing tool layers the design starts from |
| `.engineering/planning` | the AEP planning store |

## License

Apache-2.0, see [LICENSE](LICENSE).
