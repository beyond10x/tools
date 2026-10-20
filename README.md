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

Status: started 2026-10-19. The design is [docs/design/tool-contracts.md](docs/design/tool-contracts.md).
The releases carry the profile, the `fs`, `code-editor`, `process`, `search` and `build-test`
contracts with their suites,
and the crates below; the rest of the toolsets are planned in the AEP store under
`.engineering/planning`. Changes are in [CHANGELOG.md](CHANGELOG.md).

## Layout

| path | what |
|---|---|
| `contracts/profile/tool-contract-1.md` | the `tool-contract/1` profile and the `tool-conformance/1` suite format |
| `contracts/<toolset>/<version>/` | `contract.yaml`, `semantics.md`, `scenarios/`, `fixtures/` |
| `crates/` | the crates below |
| `docs/design/` | the design |
| `docs/inventory.md` | the inventory of existing tool layers the design starts from |
| `.engineering/planning` | the AEP planning store |

## Crates

| crate | what | from | licence |
|---|---|---|---|
| `b10x-tool-contract` | profile types, validator, conformance runner, `tool-contract validate`, every contract document embedded (`tool_contract::documents`) | written here | Apache-2.0 |
| `b10x-tool-fs` | `fs` v1alpha1: read, list, glob, stat over one workspace root | harness 0.13.3 `harness-tools/src/local.rs` | Apache-2.0 |
| `b10x-tool-code-editor` | `code-editor` v1alpha1: write, edit, patch, move, delete | harness 0.13.3 `harness-tools/src/local.rs` | Apache-2.0 |
| `b10x-tool-toolchain` | declarative toolchain registry deriving `build-test` operations, and their executor over `b10x-tool-process`, passing `build-test` v1alpha1 | harness 0.13.3 `harness-toolchain`; harness 0.14.1 `harness-tools` `toolchain.rs`, `catalogue.rs` `invoke_toolchain` | Apache-2.0 |
| `b10x-tool-process` | `process` v1alpha1: run one declared program by argv in one workspace root | harness 0.14.1 `harness-tools/src/local.rs` | Apache-2.0 |
| `b10x-tool-search` | `search` v1alpha1: grep, a literal or regular-expression search over one workspace root, walking with `b10x-tool-fs`'s walk | harness 0.14.1 `harness-tools/src/local.rs` | Apache-2.0 |

Run `task check` for the gate, or `cargo test -p <crate>` for one crate's unit tests, its conformance
run and its mutant tests.

## License

The repository and every crate in it are Apache-2.0, see [LICENSE](LICENSE), including the crates
moved from harness. Each crate declares its licence in its own `Cargo.toml`, named in the table
above.
