---
format: aep.planning-md/3
id: review-result:plan-critic-acceptance-web-toolsets-r2
kind: review-result
status: active
title: 'Acceptance critic, round 2, on the revised web, http, tool-meta and browser toolset stories: needs-revision, 25 findings'
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
story:conformance-runner-hosts-and-sequences — the acceptance has no check that a missing program or setting fails the run naming it, so the no-skipped-state rule can be unmet and the story still closes — .engineering/planning/story/conformance-runner-hosts-and-sequences.md:49
story:profile-declares-ports — the port-effects bullet checks only the effects branch (`external`), so a rule that ignores the consequences or risk branch still passes — .engineering/planning/story/profile-declares-ports.md:88
story:profile-declares-ports — "a test port" names no document, and Scope (lines 71-74) creates none, so the refusal bullet cannot be set up as written — .engineering/planning/story/profile-declares-ports.md:88
story:toolset-http — the secrets rule refuses a declared secret sent to a host outside its `hosts`, but the only secret bullet covers an undeclared reference, so a secret can reach an undeclared host and the story still closes — .engineering/planning/story/toolset-http.md:84
story:toolset-http — the private_network exemption is stated but no bullet checks an allowed range, so an implementation that ignores the allow-list and always refuses passes every bullet — .engineering/planning/story/toolset-http.md:78
story:toolset-http — the bullet says "12 classes", but the refused list names nine classes and fifteen forms, so the count a done-check must meet is undefined — .engineering/planning/story/toolset-http.md:107
story:toolset-http — the redirect bullet checks only the sixth redirect's refusal, so a guard that refuses every redirect passes and no bullet shows five redirects followed — .engineering/planning/story/toolset-http.md:115
story:toolset-http — the bullet joins idempotency and non-idempotency with a semicolon, two outcomes one can pass while the other fails, the same defect round 1 flagged on this artifact and only partly fixed — .engineering/planning/story/toolset-http.md:126
story:toolset-http — `http.Timeout{timeout_ms}` is a listed outcome and no acceptance bullet produces it, so a wrong Timeout answer can ship with the story closed — .engineering/planning/story/toolset-http.md:67
story:toolset-web — the loopback bullet relies on "the fixture egress log", and no fixture egress is defined in this set (profile-declares-ports defines Echo, Table and Refuse only) — .engineering/planning/story/toolset-web.md:88
story:toolset-web — `web.SchemeRefused` is a listed web.fetch outcome and no bullet answers it — .engineering/planning/story/toolset-web.md:61
story:web-ask-through-summarize-port — the truncation bullet's expected digest comes from `article-cut.md`, which no artifact in the set creates or scopes, so the bullet has no input — .engineering/planning/story/web-ask-through-summarize-port.md:81
story:web-ask-through-summarize-port — the refusal bullet names no outcome for the port's refusal, and the summarize.text row (line 44) lists none, so the answer has no expected value — .engineering/planning/story/web-ask-through-summarize-port.md:84
story:connector-projection — the Design says the host's `omit` value is sent at invoke, and no bullet checks the sent value, since the suite bullet checks only the refusal and the table answer — .engineering/planning/story/connector-projection.md:64
story:connector-projection — the union bullet names no field, the named descriptor.json has no oneOf or anyOf (grep count 0), and the grammar reads a null-typed list as Optional (profile lines 267, 274), so the refusal has no input — .engineering/planning/story/connector-projection.md:91
story:connector-projection — rule port-effects is stated to hold each projection to at least the port's effects, and no bullet refuses a projection below them — .engineering/planning/story/connector-projection.md:58
story:toolset-tool-meta — the grant's effects_within and consequences_within clauses have no bullet and the AboveGrant bullet checks risk only, so an invoke that ignores effects or consequences passes — .engineering/planning/story/toolset-tool-meta.md:92
story:toolset-tool-meta — `t.deferred` is not among the three fixture targets (lines 85-86) and the bullet names only the state after activation, so a target that was always searchable passes — .engineering/planning/story/toolset-tool-meta.md:96
story:toolset-tool-meta — the Outcome claims projected connector operations are invoked and no bullet invokes one, while connector-projection (line 99) hands that check to this story — .engineering/planning/story/toolset-tool-meta.md:28
story:toolset-browser — the Outcome names subresource requests and no bullet makes one, so a subresource that bypasses egress passes — .engineering/planning/story/toolset-browser.md:23
story:toolset-browser — the bullet relies on "the fixture egress log", which no artifact in the set defines — .engineering/planning/story/toolset-browser.md:98
story:toolset-browser — the cookie bullet checks only the second session, so it passes when no cookie was ever set and no step sets one — .engineering/planning/story/toolset-browser.md:102
story:toolset-browser — the close bullet joins idempotency with the unknown-session refusal by a semicolon, two outcomes that can split — .engineering/planning/story/toolset-browser.md:105
story:toolset-browser — the fifth open is "refused" with no outcome named, and the Outcomes line (78) lists no process-limit refusal — .engineering/planning/story/toolset-browser.md:107
story:toolset-browser — the Design says a machine without Chromium fails the suite naming it, and no bullet checks that, though the Context (line 32) records no Chromium on this host — .engineering/planning/story/toolset-browser.md:49

What I read: 8 of 8 ids in full via `aep plan artifact show`, plus `aep plan artifact kinds` and `lifecycle story`, the round-1 record, a grep of `.engineering/planning/` for the fixture names, the crate names in the runner's cargo command, and the profile grammar (contracts/profile/tool-contract-1.md:267, :274); line numbers from the store files under `.engineering/planning/story/`.

Round 1 (9 findings): 6 closed (profile last bullet, web markdown bullet, connector first bullet, connector hide and selector clauses removed from Design, browser skip bullet now backed by Design line 49, browser act bullet now at line 93); 1 partly (toolset-http seven-outcome bullet split at lines 105-127, semicolon pair at line 126 remains, finding 6 above); 2 out of this set (toolset-connector is archived).

Could not establish:
- I did not run the named cargo commands, so whether they pass today is open.
- Fixtures named in the acceptances are not in the tree yet (article.html, article.md, article-cut.md, form.html, projection.yaml, the descriptor copy under crates/tool-connector/tests/fixtures/); they are checkable only once written.
- The Tavily descriptor I grepped is the working copy at the connectors checkout; I did not confirm it is the commit the story cites.

Out of my lane, not setting the verdict:
- story:toolset-tool-meta lists `informed_by story:toolset-connector` and its Context (line 38) cites that archived story (relations lane).
- story:web-ask-through-summarize-port depends only on story:toolset-web but names the summarize port from story:profile-declares-ports and the egress rules (design lane).
- Connector, http, web and tool-meta stories all put Cargo.lock in scope (parallel-safety lane).

```findings
- file: .engineering/planning/story/conformance-runner-hosts-and-sequences.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance has no check that a missing program or setting fails the run naming it, so the no-skipped-state rule can be unmet and the story still closes"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 88
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the port-effects bullet checks only the effects branch (external), so a rule that ignores the consequences or risk branch still passes"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 88
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a test port names no document, and Scope creates none, so the refusal bullet cannot be set up as written"
- file: .engineering/planning/story/toolset-http.md
  line: 84
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the secrets rule refuses a declared secret sent to a host outside its hosts, but the only secret bullet covers an undeclared reference, so a secret can reach an undeclared host and the story still closes"
- file: .engineering/planning/story/toolset-http.md
  line: 78
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the private_network exemption is stated but no bullet checks an allowed range, so an implementation that ignores the allow-list and always refuses passes every bullet"
- file: .engineering/planning/story/toolset-http.md
  line: 107
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet says 12 classes, but the refused list names nine classes and fifteen forms, so the count a done-check must meet is undefined"
- file: .engineering/planning/story/toolset-http.md
  line: 115
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the redirect bullet checks only the sixth redirect's refusal, so a guard that refuses every redirect passes and no bullet shows five redirects followed"
- file: .engineering/planning/story/toolset-http.md
  line: 126
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet joins idempotency and non-idempotency with a semicolon, two outcomes one can pass while the other fails, the same defect round 1 flagged on this artifact and only partly fixed"
- file: .engineering/planning/story/toolset-http.md
  line: 67
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "http.Timeout is a listed outcome and no acceptance bullet produces it, so a wrong Timeout answer can ship with the story closed"
- file: .engineering/planning/story/toolset-web.md
  line: 88
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the loopback bullet relies on the fixture egress log, and no fixture egress is defined in this set (profile-declares-ports defines Echo, Table and Refuse only)"
- file: .engineering/planning/story/toolset-web.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "web.SchemeRefused is a listed web.fetch outcome and no bullet answers it"
- file: .engineering/planning/story/web-ask-through-summarize-port.md
  line: 81
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the truncation bullet's expected digest comes from article-cut.md, which no artifact in the set creates or scopes, so the bullet has no input"
- file: .engineering/planning/story/web-ask-through-summarize-port.md
  line: 84
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the refusal bullet names no outcome for the port's refusal, and the summarize.text row lists none, so the answer has no expected value"
- file: .engineering/planning/story/connector-projection.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design says the host's omit value is sent at invoke, and no bullet checks the sent value, since the suite bullet checks only the refusal and the table answer"
- file: .engineering/planning/story/connector-projection.md
  line: 91
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the union bullet names no field, the named descriptor.json has no oneOf or anyOf, and the grammar reads a null-typed list as Optional, so the refusal has no input"
- file: .engineering/planning/story/connector-projection.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "rule port-effects is stated to hold each projection to at least the port's effects, and no bullet refuses a projection below them"
- file: .engineering/planning/story/toolset-tool-meta.md
  line: 92
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the grant's effects_within and consequences_within clauses have no bullet and the AboveGrant bullet checks risk only, so an invoke that ignores effects or consequences passes"
- file: .engineering/planning/story/toolset-tool-meta.md
  line: 96
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "t.deferred is not among the three fixture targets and the bullet names only the state after activation, so a target that was always searchable passes"
- file: .engineering/planning/story/toolset-tool-meta.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome claims projected connector operations are invoked and no bullet invokes one, while connector-projection hands that check to this story"
- file: .engineering/planning/story/toolset-browser.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome names subresource requests and no bullet makes one, so a subresource that bypasses egress passes"
- file: .engineering/planning/story/toolset-browser.md
  line: 98
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet relies on the fixture egress log, which no artifact in the set defines"
- file: .engineering/planning/story/toolset-browser.md
  line: 102
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the cookie bullet checks only the second session, so it passes when no cookie was ever set and no step sets one"
- file: .engineering/planning/story/toolset-browser.md
  line: 105
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the close bullet joins idempotency with the unknown-session refusal by a semicolon, two outcomes that can split"
- file: .engineering/planning/story/toolset-browser.md
  line: 107
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the fifth open is refused with no outcome named, and the Outcomes line lists no process-limit refusal"
- file: .engineering/planning/story/toolset-browser.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design says a machine without Chromium fails the suite naming it, and no bullet checks that, though the Context records no Chromium on this host"
```
