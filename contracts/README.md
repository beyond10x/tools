# contracts

One directory per toolset and version:

```
contracts/
  profile/tool-contract-1.md        the tool-contract/1 profile: every property and its meaning
  <toolset>/v1alpha1/
    contract.yaml                   the operations: ESS commands plus the tool-contract/1 profile
    semantics.md                    what each operation guarantees, in prose that cites the contract
    scenarios/                      the conformance suite
    fixtures/                       inputs the scenarios use
```

Planned toolsets, in order: `fs`, `code-editor`, `process`, `search`, `web`, `git`, `forge`,
`build-test`, `browser`, `code-intel`, `delegation`, `memory`, `interaction`, `tool-meta`, `control`.
The design is `docs/design/tool-contracts.md`. Nothing here is released yet.
