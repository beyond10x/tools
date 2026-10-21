---
format: aep.planning-md/3
id: review-result:plan-critic-acceptance-web-toolsets-r1
kind: review-result
status: active
title: 'Acceptance critic, round 1, on the web, http, connector and browser toolset stories: needs-revision, 9 findings'
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

- story:profile-declares-ports — the last acceptance bullet joins two independent outcomes (a test port document validates; a toolset test double passes a suite run) with "and", and Scope names no path for the test double, so the second half cannot be located — .engineering/planning/story/profile-declares-ports.md:78
- story:toolset-http — the Scenarios bullet holds seven outcomes in one sentence joined by semicolons, so six can pass while the seventh fails and the bullet is neither done nor not done; it should be one bullet per scenario — .engineering/planning/story/toolset-http.md:61-63
- story:toolset-web — the markdown bullet joins two outcomes (script, style and nav dropped while headings, links and code kept; raw passed through) with a semicolon, and "a fixture page" names no fixture file, so the check has no named input — .engineering/planning/story/toolset-web.md:56-57
- story:connector-projection — the first bullet asserts three things in one sentence (validate accepts, 2 operations, "the renamed name"), and "the renamed name" names no value, so the rename cannot be checked from the text — .engineering/planning/story/connector-projection.md:56-57
- story:connector-projection — no acceptance checks that `hide` removes an operation, that the risk-at-most and effects-within selectors pick the right operations, or that an unreadable connector profile is refused (Design line 44), so a projection that ignores any of them can close — .engineering/planning/story/connector-projection.md:40
- story:toolset-connector — the Outcome promises the target operation's own profile is checked before the call and the semantics put that check on the call path, but no acceptance bullet fails if the check is removed, so the story can close without it — .engineering/planning/story/toolset-connector.md:25
- story:toolset-connector — "credential field" in the schema-check bullet is defined nowhere in the artifact, so two readers can disagree on whether a given field passes — .engineering/planning/story/toolset-connector.md:62
- story:toolset-browser — the first bullet passes when every browser scenario is skipped, and the Context records no Chromium on PATH on this host, so the bullet can close with no browser run — .engineering/planning/story/toolset-browser.md:55-56
- story:toolset-browser — the Outcome promises the model acts on a ref, but no bullet checks that browser.act acts on the element the named ref points to, that a snapshot answers refs, or anything about browser.act, the one operation the table marks non-idempotent and high-risk — .engineering/planning/story/toolset-browser.md:23

What I read: 7 of 7 artifacts in full via `aep plan artifact show`; plus `aep plan artifact kinds`, `aep plan artifact lifecycle story`, and tree checks (`Requires` at crates/tool-contract/src/document.rs:366-367 and profile line 110 confirmed; package `b10x-tool-contract` and `contracts/fs` exist; `contracts/ports` absent). story:web-ask-through-summarize-port reads clean on acceptance: all five bullets name an observable result.

Could not establish:
- No acceptance names the suite command. The `tool-contract` binary has only `validate` (crates/tool-contract/src/bin/tool-contract.rs:26-32); AGENTS.md:64 gives `cargo test -p <crate>` as the conformance run. I took that as convention, not a finding. I did not run cargo, since a build writes target/.
- The fixtures `echo`, `table` and `refuse` named in the profile and web-ask stories have no code in crates/tool-contract/src yet. That is a draft promise, not checkable now.
- The no-network check in toolset-http (line 66-67) names no command. I judged it observable by a network-less run; I did not confirm that.
- Whether the connector profile check lives in this crate or in the harness governor is unclear between the Outcome (line 25) and the semantics (line 44). Ask the drafter.
- Out of my lane, and not setting my verdict: toolset-web.md:60 ("Every address refusal ... not here") reads as scope rather than outcome; web-ask's scope leaves `contracts/web/<version>/` open; web-ask depends only on toolset-web but names the egress and profile rules.

```findings
- file: .engineering/planning/story/profile-declares-ports.md
  line: 78
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the last acceptance bullet joins two independent outcomes (a test port document validates; a toolset test double passes a suite run) with and, and Scope names no path for the test double, so the second half cannot be located"
- file: .engineering/planning/story/toolset-http.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Scenarios bullet holds seven outcomes in one sentence joined by semicolons, so six can pass while the seventh fails and the bullet is neither done nor not done; it should be one bullet per scenario"
- file: .engineering/planning/story/toolset-web.md
  line: 56
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the markdown bullet joins two outcomes (script, style and nav dropped while headings, links and code kept; raw passed through) with a semicolon, and a fixture page names no fixture file, so the check has no named input"
- file: .engineering/planning/story/connector-projection.md
  line: 56
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first bullet asserts three things in one sentence (validate accepts, 2 operations, the renamed name), and the renamed name names no value, so the rename cannot be checked from the text"
- file: .engineering/planning/story/connector-projection.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no acceptance checks that hide removes an operation, that the risk-at-most and effects-within selectors pick the right operations, or that an unreadable connector profile is refused (Design line 44), so a projection that ignores any of them can close"
- file: .engineering/planning/story/toolset-connector.md
  line: 25
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the target operation's own profile is checked before the call and the semantics put that check on the call path, but no acceptance bullet fails if the check is removed, so the story can close without it"
- file: .engineering/planning/story/toolset-connector.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "credential field in the schema-check bullet is defined nowhere in the artifact, so two readers can disagree on whether a given field passes"
- file: .engineering/planning/story/toolset-browser.md
  line: 55
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first bullet passes when every browser scenario is skipped, and the Context records no Chromium on PATH on this host, so the bullet can close with no browser run"
- file: .engineering/planning/story/toolset-browser.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises the model acts on a ref, but no bullet checks that browser.act acts on the element the named ref points to, that a snapshot answers refs, or anything about browser.act, the one operation the table marks non-idempotent and high-risk"
```
