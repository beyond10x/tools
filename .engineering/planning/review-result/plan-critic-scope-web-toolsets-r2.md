---
format: aep.planning-md/3
id: review-result:plan-critic-scope-web-toolsets-r2
kind: review-result
status: active
title: 'Scope critic, round 2, on the revised web, http, tool-meta and browser toolset stories: needs-revision, 2 findings'
relations:
- reviews: story:conformance-runner-hosts-and-sequences
- reviews: story:profile-declares-ports
- reviews: story:toolset-http
- reviews: story:toolset-web
- reviews: story:web-ask-through-summarize-port
- reviews: story:connector-projection
- reviews: story:toolset-tool-meta
- reviews: story:toolset-browser
revision: 1
---
needs-revision
epic:... (none on approve) — see findings below
story:toolset-web — web.fetch format raw ("format: raw answers the served body byte for byte") claims the outcome story:toolset-http's http.get already claims (toolset-http.md:45), so one of the two rows should drop it or delegate to the other — .engineering/planning/story/toolset-web.md:84
story:profile-declares-ports — the design says "A port operation is never offered to a model", and no item claims it: the offered rule (line 58) and its acceptance (line 92) do not keep port operations out of offered(), so the body should say so and an acceptance should test it — docs/design/tool-contracts.md:81

Read: epic:tool-contracts body first (28 lines, via `aep plan artifact show` and the file); the 8 set items via `aep plan artifact show`; archived story:toolset-connector; `aep plan artifact graph`, `kinds`, `relations`; review-result:plan-critic-scope-web-toolsets-r1; working-tree docs/design/tool-contracts.md §§ Toolsets, Ports, Connector projections, Conformance.
Promises: 13 extracted from the epic body (7 categories in its Outcome, the defaults-pass-suites sentence, the location sentence, 4 acceptance bullets). 3 traced to an item in this set (web category, defaults pass suites for web/http/browser/tool-meta, location). 6 claimed by graph items outside the set. 4 claimed by no item (git, delegation, harness adoption, metaharness).

Could not establish:
- Round 1 findings on toolset-http, toolset-connector, connector-projection and toolset-browser: resolved, since the working-tree design now names http, browser goto/screenshot, tool-meta and connector projections. Round 1 accepted the design as the place. The epic (revision 1) still names none of them. toolset-connector is archived and out of the set.
- The design revision is uncommitted (`git status`: M docs/design/tool-contracts.md). I read the working-tree version.
- web.crawl: design:57 lists crawl in the first-version web row; toolset-web.md:97 says "not planned yet" with no reason. Named, so not counted; the operator should confirm.
- "fast summary" in the operator request (web-ask-through-summarize-port.md:31) is claimed by no item and excluded by none. Not a parent promise, so not counted.
- Desktop computer use: recorded as not planned in toolset-browser.md (Context, "Not in this story"). Named, not counted.
- Round 1 finding on toolset-web is quoted against the current line 84; the round 1 quote no longer appears in the story. Substance unchanged.
- Out of lane: (a) design:243-245 lists `output` as behavioural with a scenario, while profile-declares-ports.md:66 and :74 make `output.untrusted` declarative (plan-critic-design). (b) Cargo.lock and contracts/profile/tool-contract-1.md are shared by three stories (parallel-safety). (c) The connectors host's HTTP client is outside egress (connector-projection.md, toolset-http.md Context), a security trade-off. (d) "platform" paths at connector-projection.md:35-36: the forbidden_literals gate is not in this worktree, so not judged.

```findings
- file: .engineering/planning/story/toolset-web.md
  line: 84
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "web.fetch format raw (\"format: raw answers the served body byte for byte\") claims the outcome story:toolset-http's http.get already claims (toolset-http.md:45), so one of the two rows should drop it or delegate to the other"
- file: docs/design/tool-contracts.md
  line: 81
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the design says \"A port operation is never offered to a model\" and no item claims it: story:profile-declares-ports does not keep port operations out of offered() (line 58) and its acceptance (line 92) does not test it"
```
