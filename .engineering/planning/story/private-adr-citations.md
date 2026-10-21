---
format: aep.planning-md/3
id: story:private-adr-citations
kind: story
status: draft
title: No committed text names a private repository in an ADR citation
relations:
- decomposes: epic:tool-contracts
revision: 1
---
## Outcome

No committed text in this repository names a private repository: every ADR citation reads `ADR <n>`
without the repository that holds it, or `AGENTS.md` § Rules says which private names a citation may
carry, and `git grep -i -w atlas` agrees with whichever rule stands.

## Context

Found by the 2026-10-21 cross-repository review (finding 3, `low`, `suspected`): `AGENTS.md:35`
cites "atlas ADR 0083" while `AGENTS.md:37` bars private repository names in committed text. The
repository it names is private (`gh repo view` answered `"visibility":"PRIVATE"` on 2026-10-21).
`git grep -i -w atlas` on `wave/w4` lists 17 lines: `AGENTS.md`, `CHANGELOG.md` (the released 0.3.0
and 0.4.0 entries), `docs/design/tool-contracts.md`, three wave pages and six stories plus the epic.
Released CHANGELOG entries and wave pages are records; whether they are edited or the rule names
them as exempt is the first decision of this story.

## Scope

- `AGENTS.md` (§ Rules, the citation at line 35 and the rule at line 37)
- `docs/design/tool-contracts.md`
- `CHANGELOG.md` (released entries: edit or exempt)
- `.engineering/planning/` stories and the epic that cite the ADRs
