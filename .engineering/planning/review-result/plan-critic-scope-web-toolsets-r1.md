---
format: aep.planning-md/3
id: review-result:plan-critic-scope-web-toolsets-r1
kind: review-result
status: active
title: 'Scope critic, round 1, on the web, http, connector and browser toolset stories: needs-revision, 5 findings'
relations:
- reviews: story:profile-declares-ports
- reviews: story:toolset-http
- reviews: story:toolset-web
- reviews: story:web-ask-through-summarize-port
- reviews: story:connector-projection
- reviews: story:toolset-connector
- reviews: story:toolset-browser
revision: 1
---
needs-revision

- B/.engineering/planning/story/toolset-http.md:43 — the http toolset and its http.request (effects network and secret, consequence send-external) are named by no sentence of the parent. The epic Outcome lists seven categories without http (B/.engineering/planning/epic/tool-contracts.md:11-12), and the design toolset table has no http row (B/docs/design/tool-contracts.md:48-64). The epic or the design must name it, or the story leaves this decomposition.
- B/.engineering/planning/story/toolset-connector.md:22 — the connector toolset, whose connector.invoke reaches any configured connector operation at risk high, is named by no sentence of the parent. The epic Outcome and the design toolset table both omit it (B/.engineering/planning/epic/tool-contracts.md:11-12; B/docs/design/tool-contracts.md:48-64). The epic or the design must name it, or the story leaves this decomposition.
- B/.engineering/planning/story/connector-projection.md:22 — the projection compiler is traced only to story:toolset-connector, which is itself untraced. Its selection by declared facts traces to B/docs/design/tool-contracts.md:91, but no sentence names connector operations as tools. It follows the same epic or design revision as story:toolset-connector.
- B/.engineering/planning/story/toolset-browser.md:39 — browser.goto and browser.screenshot are not in the design's browser row, which lists open, snapshot, act and close only (B/docs/design/tool-contracts.md:60). No cited sentence reaches the two added operations.
- B/.engineering/planning/story/toolset-web.md:57 — web.fetch format raw ("answers the body unchanged" under the egress guard) claims the outcome that story:toolset-http's http.get already claims (B/.engineering/planning/story/toolset-http.md:43). One of the two rows should drop it or delegate to the other.

Read: the parent body (B/.engineering/planning/epic/tool-contracts.md, 28 lines), all seven drafted bodies via `aep plan artifact show`, `aep plan artifact graph`, `kinds` and `relations`, and the design's §Toolsets (48-64) and §How harnesses adopt (86-91).

Promises: 14 extracted from the parent. 4 traced to an item in this set (contract set, web category, default implementations pass suites, location in contracts/ and crates/tool-<toolset>). 6 claimed by stories outside this set (file, edit, process, search, fs and code-editor release and pass). 4 claimed by no story in the graph (git, delegation, harness adopting file and edit tools, metaharness rendering vendor names).

Could not establish:
- git and delegation have no claiming item in the graph. This set is web-only, so I did not count them against it.
- The operator's "any result" is narrowed to HTML and text, with PDF refused (B/.engineering/planning/story/toolset-web.md:59). It is named, so not a finding; the operator should accept it.
- "Computer use evaluated" rests only on the "not planned" line (B/.engineering/planning/story/toolset-browser.md:65). It is a named decision, not a gap.
- The web.fetch SSRF guard (design line 56) is tested only in toolset-http (B/.engineering/planning/story/toolset-web.md:60). Whether web's own suite must carry it is for plan-critic-design.
- Confidentiality: the forbidden_literals gate is not in this repository. A literal scan of the seven stories for the organisation's forbidden literals found nothing [redacted by the coordinator: the critic listed the literals]. I did not judge "P1", "platform" or "agent-platform" against the gate list.
- Out of my lane (parallel-safety): connector-projection and profile-declares-ports both edit contracts/profile/tool-contract-1.md, and toolset-http and toolset-web both list Cargo.lock.

```findings
- file: .engineering/planning/story/toolset-http.md
  line: 43
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the http toolset and its http.request (effects network and secret, consequence send-external) are named by no sentence of the parent; the epic Outcome lists seven categories without http (.engineering/planning/epic/tool-contracts.md:11-12) and the design toolset table has no http row (docs/design/tool-contracts.md:48-64), so the epic or the design must name it, or the story leaves this decomposition"
- file: .engineering/planning/story/toolset-connector.md
  line: 22
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the connector toolset, whose connector.invoke reaches any configured connector operation at risk high, is named by no sentence of the parent; the epic Outcome (.engineering/planning/epic/tool-contracts.md:11-12) and the design toolset table (docs/design/tool-contracts.md:48-64) both omit it, so the epic or the design must name it, or the story leaves this decomposition"
- file: .engineering/planning/story/connector-projection.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the projection compiler is traced only to story:toolset-connector, which is itself untraced; its selection by declared facts traces to docs/design/tool-contracts.md:91, but no sentence names connector operations as tools, so it follows the same epic or design revision as story:toolset-connector"
- file: .engineering/planning/story/toolset-browser.md
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "browser.goto and browser.screenshot are not in the design browser row, which lists open, snapshot, act and close only (docs/design/tool-contracts.md:60), so no cited sentence reaches the two added operations"
- file: .engineering/planning/story/toolset-web.md
  line: 57
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "web.fetch format raw (answers the body unchanged under the egress guard) claims the outcome that story:toolset-http http.get already claims (.engineering/planning/story/toolset-http.md:43); one of the two rows should drop it or delegate to the other"
```
