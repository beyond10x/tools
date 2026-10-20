---
format: aep.planning-md/3
id: story:docs-site
kind: story
status: draft
title: tools has a documentation site that README and AGENTS.md agree with
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: .github/workflows
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: website
revision: 2
---
## Outcome

tools has a documentation site at https://beyond10x.github.io/tools/, built from `website/docs`,
and `README.md` and `AGENTS.md` agree with it.

## Context

- On 2026-10-20 the repository has no `website/` directory, and the GitHub Pages API answers 404
  for `beyond10x/tools`.
- Other components carry one (`aep/website`, `canon/website`, `connectors/website`, among others).
- The `docs` skill does this end to end for a component.

## Scope

- `website/` (new)
- `.github/workflows/` (the Pages job)
- `README.md`, `AGENTS.md`

## Acceptance

- https://beyond10x.github.io/tools/ answers 200 and names the `tool-contract/1` profile and the
  `fs`, `code-editor` and `build-test` toolsets released at the time it is built.
- The README crate table and the site name the same crates with the same licences.
