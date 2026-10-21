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

Released toolsets: `fs`, `code-editor`, `process`, `search` and `build-test`, each with its
conformance suite (`CHANGELOG.md` names the release of each). Planned, in order: `web`, `git`,
`forge`, `browser`, `code-intel`, `delegation`, `memory`, `interaction`, `tool-meta`, `control`.
The design is `docs/design/tool-contracts.md`.
