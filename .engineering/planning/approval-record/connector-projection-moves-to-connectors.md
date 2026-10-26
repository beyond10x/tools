---
format: aep.planning-md/3
id: approval-record:connector-projection-moves-to-connectors
kind: approval-record
status: approved
title: story:connector-projection moves to Connectors (ADR 0103); tools archives its copy
summary: 'Decided by the orchestrator on 2026-10-26: archive the tools story, amend story:toolset-tool-meta per ADR 0103 decision 6'
relations:
- decides: story:connector-projection
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-10-26T15:35:42Z", actor: "human:timo", revision: 2}
---
## Decision

`story:connector-projection` is archived in this store and its outcome moves to the Connectors
repository (https://github.com/beyond10x/connectors), where it is `story:connector-projection`
under `epic:tool-contract-publication`. ADR 0103 (accepted 2026-10-21) decides it: Connectors
publishes a `tool-contract/1` document for each admitted operation and passes
`tool-conformance/1` (decision 1); tools takes no dependency on the connectors descriptor format
and builds no connectors-specific port, so `crates/tool-connector`, `contracts/connectors/v1alpha1`
and the `tool-projection/1` section are not built here (decision 3); tools archives its copy of the
story with a pointer to the Connectors story (decision 5).

`story:toolset-tool-meta` stays here and is amended per decision 6: it drops
`depends_on: story:connector-projection`, and its `connector-tavily.search` fixture becomes a
contract document of the shape Connectors publishes, served by the `catalogue` `Table` fixture.

## Instruction

Decided by `b10x-chef-orchestrator` on 2026-10-26 under the owner's standing grant, quoted
verbatim:

> In ~/beyond10x/tools: the story that archives the tools projection (`story:connector-projection`
> or the story the tools store names for the projection connectors now owns) moves through the
> store and is implemented: the projection code and docs leave tools with a pointer to connectors,
> gate, bot PR, merge, release per tools' AGENTS.md if applicable.

## What it changes

- `story:connector-projection`: draft to archived, with a body pointing to the Connectors story.
  No code left: the story was never implemented (`crates/tool-connector`,
  `contracts/connectors/` and `crates/tool-ports/` do not exist).
- `story:toolset-tool-meta`: the edge and the fixture, as above.
- `story:profile-declares-ports`: `connectors` leaves its first set of ports.
- `story:toolset-http`: its pointer for the connector client outside `egress` names Connectors.
- `docs/design/tool-contracts.md` § Connector contracts (was § Connector projections) and the
  port list; `docs/inventory.md` is unchanged (its connectors rows describe existing layers, not a
  planned projection).
- `story:toolset-connector` is archived and keeps its edge as written.
