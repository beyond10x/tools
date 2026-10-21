---
format: aep.planning-md/3
id: review-result:plan-critic-design-web-toolsets-r1
kind: review-result
status: active
title: 'Design critic, round 1, on the web, http, connector and browser toolset stories: needs-revision, 3 findings'
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
story:toolset-browser — the body routes `browser.goto` through the egress guard, but egress/v1alpha1 offers only `egress.send`, which Chromium never calls, so redirects, subresource fetches and click-driven navigation have no named route through the guard, and no edge or acceptance covers that seam; the body must name the route or narrow the claim to the top-level URL — .engineering/planning/story/toolset-browser.md:39,41; .engineering/planning/story/toolset-http.md:24-26,49-50
story:profile-declares-ports — the rule says the port's Rust trait is "the crate's", but with tool-web taking the egress port and tool-http providing it, the rule puts the trait in tool-web, and tool-http would then have to depend on tool-web while tool-web depends on tool-http, a cycle; the rule must name the crate that owns a port trait — .engineering/planning/story/profile-declares-ports.md:54; .engineering/planning/story/toolset-web.md:49; .engineering/planning/story/web-ask-through-summarize-port.md:61
story:profile-declares-ports — the conformance binding names three runner fixtures (echo, table, refuse) and no served fixture, yet toolset-http reads a local server the runner starts, toolset-web reads a search fixture, toolset-connector reads a connectors fixture holding 2 instances, and web-ask reads an echo whose answer is a digest, and no item in the set produces any of them; the body must say whether port fixtures are a fixed registry or per-port documents and who writes each — .engineering/planning/story/profile-declares-ports.md:55-57; .engineering/planning/story/toolset-connector.md:57; contracts/profile/tool-contract-1.md:180

What I read: 7 artifacts, whole body, via `aep plan artifact show`, plus `relations`, `graph`, `validate` (valid, nothing restated) and `waves`, all in the worktree. The graph has 43 artifacts; I walked every edge touching the set: 7 depends_on inside the set, 7 decomposes to epic:tool-contracts, none from outside. The set is acyclic, the longest chain is 4 (profile, http, web, ask), and there are two roots, so nothing is serialised. I traced 11 cross-item acceptance reads: 6 have a direct edge, 2 are covered only transitively (web-ask via web), and 3 have no producer in the set (the three fixtures above). Design and profile docs were read by grep only.

What I could not establish:
- Whether Chromium's network can be routed through an egress client (CDP interception or proxy) at all; I did not verify this, and finding 1 rests on the body text.
- Whether the conformance runner can serve a local fixture; I did not read conformance.rs, and the profile's fixture keys are only `dir` and `files` (contracts/profile/tool-contract-1.md:180).
- The Tavily search binding in the connectors repository: outside this tree, not opened.
- Out of my lane (parallel-safety): contracts/profile/tool-contract-1.md is written by profile-declares-ports (§ Ports) and connector-projection (toolset grammar) with no edge between them; `aep plan artifact waves` lists this collision.
- Out of my lane (scope): no item owns the real search binding (toolset-web.md:35 calls it "a connector operation, not new code here"); the design lists crawl for `web` (docs/design/tool-contracts.md:56) and open/snapshot/act/close for `browser` (:60), and the set drops crawl and adds goto and screenshot.
- Unease, not a finding: profile-declares-ports.md:48-49 leaves the port document format to its implementor, and four port-authoring stories depend on that choice without saying which format they assume.
- Unease, not a finding: web-ask.md:52 makes `web.ask` land in v1alpha1 or v1alpha2 depending on whether toolset-web has released, a state the graph does not record.

```findings
- file: .engineering/planning/story/toolset-browser.md
  line: 39
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body routes browser.goto through the egress guard, but egress/v1alpha1 offers only egress.send, which Chromium never calls, so redirects, subresource fetches and click-driven navigation have no named route through the guard and no edge or acceptance covers that seam; the body must name the route or narrow the claim to the top-level URL"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 54
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the rule says the Rust trait is the crate's, but with tool-web taking the egress port and tool-http providing it, the rule puts the trait in tool-web, and tool-http would then have to depend on tool-web while tool-web depends on tool-http, a cycle; the rule must name the crate that owns a port trait"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 55
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the conformance binding names three runner fixtures (echo, table, refuse) and no served fixture, yet toolset-http reads a local server the runner starts, toolset-web reads a search fixture, toolset-connector reads a connectors fixture holding 2 instances, and web-ask reads an echo whose answer is a digest, and no item in the set produces any of them; the body must say whether port fixtures are a fixed registry or per-port documents and who writes each"
```
