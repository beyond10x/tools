---
format: aep.planning-md/3
id: epic:tool-contracts
kind: epic
status: draft
title: Agent tools behind one set of contracts, with b10x default implementations
revision: 1
---
## Outcome

Every agent harness in the organization calls file, edit, process, search, web, git and delegation
tools through one set of versioned, language-neutral contracts, and the b10x default
implementations of those contracts pass each toolset's conformance suite.

## Context

`docs/inventory.md`: 14 file readers, 15 process runners and 16 effect, risk and approval
vocabularies, no contract shared by two programs. `docs/design/tool-contracts.md`: the model
(ESS command plus the `tool-contract/1` profile), the toolsets, conformance and the phases T0 to T3.
Owner decision 2026-10-19: this repository holds the contracts (`contracts/`) and the default
implementations (`crates/tool-<toolset>`); ADR 0095.

## Acceptance

- `fs` and `code-editor` v1alpha1 contracts are released with conformance suites, and
  `crates/tool-fs` and `crates/tool-code-editor` pass them.
- harness offers its file and edit tools from these crates, and metaharness renders their vendor
  names from the contracts.
