---
format: aep.planning-md/3
id: story:private-adr-citations
kind: story
status: implemented
title: No committed text names a private repository in an ADR citation
relations:
- decomposes: epic:tool-contracts
scope:
- confidence: cited
  path: .engineering/planning/epic/tool-contracts.md
- confidence: cited
  path: .engineering/planning/story
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: docs/design/tool-contracts.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T11:50:42Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-21T11:50:47Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-21T11:52:39Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Outcome

No text in this repository that is still edited names a private repository: every ADR citation reads
`ADR <n>` without the repository that holds it, and `AGENTS.md` § Rules says so and names the records
that stay as written.

## Context

Found by the 2026-10-21 cross-repository review (finding 3, `low`, `suspected`): `AGENTS.md:35`
cited ADR 0083 with the name of the repository that holds it, while `AGENTS.md:37` bars private
repository names in committed text; that repository is private (`gh repo view`, 2026-10-21). A
case-insensitive whole-word search for the name on `main` listed 17 lines in 14 files: `AGENTS.md`,
`docs/design/tool-contracts.md`, the epic, 7 stories, the released 0.3.0 and 0.4.0 `CHANGELOG.md`
entries and the wave pages t1, w2 and w3; the 0.2.0, 0.3.0 and 0.4.0 release notes name it once each.

Decided 2026-10-21 (option A): the citations still edited drop the name; released `CHANGELOG.md`
entries, wave pages and release notes are records, stay as written and are named as exempt. History
is not rewritten.

## Scope

- `AGENTS.md` (§ Rules: the citation at line 35 and the rule at line 37)
- `docs/design/tool-contracts.md`
- `.engineering/planning/epic/tool-contracts.md` and the stories `ess-domains-for-contracts`,
  `toolset-code-editor`, `toolset-fs`, `toolset-http`, `toolset-process`, `toolset-search`,
  `toolset-toolchain` and this one

## Acceptance

- The whole-word search for the name lists only lines in released `CHANGELOG.md` entries and in
  `docs/plans/wave-*.md`.
- `AGENTS.md` § Rules states the `ADR <n>` form and names those records as exempt.
- `aep plan artifact validate` answers valid.
