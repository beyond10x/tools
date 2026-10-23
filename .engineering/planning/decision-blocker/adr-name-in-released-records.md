---
format: aep.planning-md/3
id: decision-blocker:adr-name-in-released-records
kind: decision-blocker
status: cleared
title: Whether released records keep the private repository name in their ADR citations
summary: Released changelog entries, wave pages and release notes keep their ADR citations as written
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-23T11:41:11Z", actor: "human:timo", revision: 3}
---
## Question

`story:private-adr-citations` dropped the repository that holds an ADR from every citation in
maintained text, and `AGENTS.md` § Rules exempts the records that stay as written: the released
`CHANGELOG.md` entries, the wave pages in `docs/plans/` and the release notes of 0.4.0 and earlier.
Should those released records be rewritten too?

## The two answers on the table

| option | what it means for this repository |
|---|---|
| A | the released records stay as written; new text cites `ADR <n>` only |
| B | the released changelog entries, wave pages and GitHub release notes are rewritten, which edits published history |

## Who answers

The owner's orchestrating session, `b10x-chef-orchestrator`, under the owner's standing grant.

## Answer

Option A, decided by `b10x-chef-orchestrator` on 2026-10-23: unit 4 exempts these records as
records that stay as written, and rewriting published release notes edits history, which the grant
does not cover. New text must not repeat the repository name; `AGENTS.md` § Rules already says so.
