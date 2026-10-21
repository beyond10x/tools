---
format: aep.planning-md/3
id: story:toolset-connector
kind: story
status: archived
title: The connector toolset lists, describes and invokes any configured connector operation through a port
relations:
- decomposes: epic:tool-contracts
- depends_on: story:connector-projection
- depends_on: story:profile-declares-ports
scope:
- confidence: inferred
  path: contracts/connector/v1alpha1
- confidence: inferred
  path: contracts/ports/connectors/v1alpha1
- confidence: inferred
  path: crates/tool-connector
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-10-20T23:31:10Z", actor: "human:timo", revision: 4}
---
## Outcome

Archived on 2026-10-21, before any work: replaced by `story:toolset-tool-meta`.

## Why

Round 1 of the plan review (`review-result:plan-critic-scope-web-toolsets-r1`,
`review-result:plan-critic-design-web-toolsets-r1` and five independent reviewers) found that
`connector.list`, `connector.describe` and `connector.invoke` are the design's `tool-meta` operations
under a second name (`docs/design/tool-contracts.md` § Toolsets), and that a connectors
`Operation.profile` is a provider profile name carrying no effects or risk
(`connectors/adapters/tavily/generated/descriptor.json`). With connector operations projected into
contracts (`story:connector-projection`), calling any configured connector is `tool-meta` over the
projected contracts, and the grant check moves into `tool.invoke`.
