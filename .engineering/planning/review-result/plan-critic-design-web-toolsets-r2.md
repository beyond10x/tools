---
format: aep.planning-md/3
id: review-result:plan-critic-design-web-toolsets-r2
kind: review-result
status: active
title: 'Design critic, round 2, on the revised web, http, tool-meta and browser toolset stories: needs-revision, 6 findings'
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

Findings

story:profile-declares-ports — the rule `port-effects` checks an operation against "the port operations it requires", but `requires.ports` names only the port and no field maps an operation to the port operations it calls. Read over the whole port, the rule refuses the set's own contracts: egress.send carries secret and send-external at risk high, so http.get and web.fetch (`[network]`, risk low) and browser.goto fail, and web.ask lacks secret. Read per call, it has no referent. The body must define the mapping or the domain. — .engineering/planning/story/profile-declares-ports.md:55-56; .engineering/planning/story/toolset-http.md:51-53; .engineering/planning/story/toolset-web.md:50-51; .engineering/planning/story/toolset-browser.md:67-69; .engineering/planning/story/web-ask-through-summarize-port.md:49,56,86-87; docs/design/tool-contracts.md:82-84

story:toolset-http — egress offers one operation, egress.send, which is a whole request and response. The browser's forward proxy must carry HTTPS as CONNECT tunnels, and no operation in the set expresses that, so "every HTTP(S) request Chromium makes goes through egress" has no route for HTTPS. The port needs a tunnel operation, or the browser body must narrow its claim. Round-1 finding 1 is partly fixed: the goto route is now named, but the port's shape cannot carry it. — .engineering/planning/story/toolset-browser.md:23-24,39-40; .engineering/planning/story/toolset-http.md:47

story:profile-declares-ports — the fixture registry (Echo, Table, Refuse) has no egress-log fixture, yet web and browser acceptance read "the fixture egress log", and no item names what writes it. — .engineering/planning/story/toolset-web.md:88-89; .engineering/planning/story/toolset-browser.md:98-99; .engineering/planning/story/profile-declares-ports.md:61-63

story:conformance-runner-hosts-and-sequences — the `serve` route grammar has no request record, yet toolset-http reads what a served route "received" and counts POSTs "from its log". The runner must add a route record, or the http acceptance must drop those reads. — .engineering/planning/story/toolset-http.md:117-118,127; .engineering/planning/story/conformance-runner-hosts-and-sequences.md:40

story:profile-declares-ports — two items claim the same fixtures. This story's registry defines Echo and Table; web-ask's scope adds "the Summarize trait and its Echo fixture"; toolset-tool-meta adds "a Table fixture holding a list of contracts", where this story's Table answers from a host map. One item must own each fixture. — .engineering/planning/story/profile-declares-ports.md:61-62; .engineering/planning/story/web-ask-through-summarize-port.md:71; .engineering/planning/story/toolset-tool-meta.md:79-80

story:profile-declares-ports — each fixture appends "one line per call" to `.ports/<port>.jsonl`, but no item states what a line holds, while acceptance reads line content (summarize.jsonl "contains the prompt", catalogue.jsonl has an invoke line). The producer's output format is unstated. — .engineering/planning/story/profile-declares-ports.md:63,93; .engineering/planning/story/web-ask-through-summarize-port.md:79; .engineering/planning/story/toolset-tool-meta.md:92-93

What I read: 9 story artifacts, whole body, via `aep plan artifact show` (toolset-connector read though archived); review-result r1; docs/design/tool-contracts.md; contracts/profile/tool-contract-1.md. Also `relations`, `graph` (all edges), `validate` (valid, 49 artifacts), `waves`, and greps over the story files for CONNECT, tunnel, log and digest. Edges: 10 depends_on, 1 informed_by and 9 decomposes inside the set. No edge leaves the set except decomposes to epic:tool-contracts. No cycle. Longest chain conformance → profile → http → web → web-ask, so the set is not serialised. Acceptance reads traced: 15. Of these, 8 have a direct edge, 4 are covered only transitively, and 3 have no producer in the set (findings 3 and 4). Round 1: finding 2 (which crate owns port traits) is fixed at profile-declares-ports.md:60. Finding 3 (fixture producers) is fixed for the registry but not the logs (findings 3 and 4). Finding 1 is partly fixed (finding 2 here).

What I could not establish:
- That Chromium sends CONNECT for HTTPS through a configured proxy. This is HTTP proxy behaviour (RFC 9110), not run in this tree; finding 2 rests on it.
- Which reading the drafter meant by "the port operations it requires". The text supports both readings and each fails somewhere in the set; the choice is the drafter's.
- The catalogue port's operations have no profile (toolset-tool-meta.md:48), so tool.search, tool.describe and tool.activate cannot be checked under port-effects. This sits under finding 1.
- Unease, not a finding: `Toolset::offered(&self)` takes no bindings (profile-declares-ports.md:58) while the outcome says "given what is bound" (:33-34). Only toolset-http states a constructor convention (toolset-http.md:83).
- Unease: the connectors port's outbound call runs outside egress by design (connector-projection.md:62). This is an exception to the toolset-http Outcome's "web and browser reach the network only through egress".
- Unease: web.ask lands in web v1alpha1 "before any web implementation is released" (web-ask-through-summarize-port.md:26-27), and no graph state records that.
- Out of my lane: `waves` lists collisions for parallel safety (Cargo.lock across every new-crate story; contracts/profile/tool-contract-1.md and conformance.rs across conformance, profile and connector-projection; crates/tool-ports/src/lib.rs). web.crawl is in the design (docs/design/tool-contracts.md:57) and "not planned yet" (toolset-web.md:97), which is a scope matter.

```findings
- file: .engineering/planning/story/profile-declares-ports.md
  line: 55
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the rule port-effects checks an operation against the port operations it requires, but requires.ports names only the port and no field maps an operation to the port operations it calls; read over the whole port it refuses the set's own contracts (egress.send carries secret and send-external at risk high, so http.get and web.fetch at [network] risk low, browser.goto and web.ask without secret fail), and read per call it has no referent; the body must define the mapping or the domain"
- file: .engineering/planning/story/toolset-http.md
  line: 47
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "egress offers one operation, egress.send, a whole request and response, but the browser's forward proxy must carry HTTPS as CONNECT tunnels that no operation in the set expresses, so the claim that every HTTP(S) request goes through egress has no route for HTTPS; the port needs a tunnel operation or the browser body must narrow its claim"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "the fixture registry (Echo, Table, Refuse) has no egress-log fixture, yet the web and browser acceptance read the fixture egress log and no item names what writes it"
- file: .engineering/planning/story/conformance-runner-hosts-and-sequences.md
  line: 40
  category: design
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "the serve route grammar has no request record, yet toolset-http reads what a served route received and counts POSTs from its log, so the runner must add a route record or the http acceptance must drop those reads"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 61
  category: design
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "two items claim the same fixtures: this story's registry defines Echo and Table, web-ask's scope adds the Summarize trait and its Echo fixture, and toolset-tool-meta adds a Table fixture holding a list of contracts where this story's Table answers from a host map; one item must own each fixture"
- file: .engineering/planning/story/profile-declares-ports.md
  line: 63
  category: design
  severity: warning
  verdict: needs-revision
  origin: undecided
  message: "each fixture appends one line per call to .ports/<port>.jsonl, but no item states what a line holds, while acceptance reads line content (summarize.jsonl contains the prompt, catalogue.jsonl has an invoke line), so the producer's output format is unstated"
```
