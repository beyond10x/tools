---
format: aep.planning-md/3
id: review-result:plan-critic-parallel-safety-web-toolsets-r2
kind: review-result
status: active
title: 'Parallel-safety critic, round 2, on the revised web, http, tool-meta and browser toolset stories: needs-revision, 7 findings'
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

story:connector-projection — the body does not say its `crates/tool-ports/src/` (Connectors trait) is shared with story:toolset-http's (Egress trait), and no depends_on orders the two, so both edit one crate unsequenced — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-http` (## Scope)
story:connector-projection — the body does not say its `crates/tool-ports/src/` is shared with story:toolset-web's (WebSearch trait), and no depends_on orders the two — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-web` (## Scope)
story:connector-projection — the body does not say its `crates/tool-ports/src/` is shared with story:web-ask-through-summarize-port's (Summarize trait), and no depends_on orders the two — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:web-ask-through-summarize-port` (## Scope)
story:connector-projection — the body does not say its `crates/tool-ports/src/` is shared with story:toolset-tool-meta's (Catalogue trait), and no depends_on orders the two — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-tool-meta` (## Scope)
story:toolset-http — the body does not say its `crates/tool-ports/src/` is shared with story:toolset-tool-meta's (Catalogue trait), and no depends_on orders the two — `aep plan artifact show story:toolset-http` (## Scope), `aep plan artifact show story:toolset-tool-meta` (## Scope)
story:toolset-tool-meta — the body does not say its `crates/tool-ports/src/` is shared with story:toolset-web's (WebSearch trait), and no depends_on orders the two — `aep plan artifact show story:toolset-tool-meta` (## Scope), `aep plan artifact show story:toolset-web` (## Scope)
story:toolset-tool-meta — the body does not say its `crates/tool-ports/src/` is shared with story:web-ask-through-summarize-port's (Summarize trait), and no depends_on orders the two — `aep plan artifact show story:toolset-tool-meta` (## Scope), `aep plan artifact show story:web-ask-through-summarize-port` (## Scope)

What I read: the 8 stories in full via `aep plan artifact show`, the round-1 record, `waves --kind story --status draft`, `graph`, `relations`, `AGENTS.md` (§ Branches and waves, which exists), and the root `Cargo.toml` (`members = ["crates/*"]`, so new crates need no workspace edit).
Surfaces: 8 cited, 0 inferred, 0 unplaced. The store labels every scope inferred; each body names its paths under ## Scope, so I counted them cited.
Collisions among the 8: 31 printed by waves (5 more involve out-of-set items). 16 are ordered by depends_on (transitive chains count, as in round 1). 8 are unordered on Cargo.lock but admitted ("shared, merged serially" in each body). 7 are unordered and unadmitted on `crates/tool-ports/src/`, which are the findings above.
Acceptance reads of in-set state: 11 reader-producer pairs, 6 with a direct depends_on edge and 5 through a chain only.
Round 1: all 5 findings no longer hold. The four Cargo.lock pairs are admitted in the bodies, and the contract-1.md pair is ordered by story:connector-projection depends_on story:profile-declares-ports.
Waves already puts the 8 items in 8 different waves, by its own scope order. No edge records that order, so nothing runs concurrently today; the defect is the bodies' silence. The same-file claim inside `src/` is inferred (the CLI names `lib.rs`).

What I could not establish:
- Whether any item edits `crates/tool-ports/Cargo.toml` (the `fixtures` feature). No body names it.
- Out of set, not assessed: story:build-test-applies-provider-environment, story:contract-directory-names-validated, story:process-kill-never-reaches-a-reused-group.
- Out of lane (acceptance): the "fixture egress log" read by story:toolset-web and story:toolset-browser has no in-set producer. PP's fixtures are Echo, Table and Refuse.
- Out of lane (acceptance): story:toolset-http reads a served-route request log that story:conformance-runner-hosts-and-sequences' `serve` does not define.
- Out of lane (acceptance): story:web-ask-through-summarize-port reads `article-cut.md`, which no in-set artifact creates.
- Out of lane (design): two `Table` fixtures in tool-ports, one from story:profile-declares-ports (host map) and one from story:toolset-tool-meta (contract list). Both are ordered by depends_on.

```findings
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:toolset-http (Egress trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:toolset-web (WebSearch trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:web-ask-through-summarize-port (Summarize trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:toolset-tool-meta (Catalogue trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:toolset-http"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:toolset-tool-meta (Catalogue trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:toolset-tool-meta"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:toolset-web (WebSearch trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
- file: "aep plan artifact show story:toolset-tool-meta"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say its crates/tool-ports/src/ is shared with story:web-ask-through-summarize-port (Summarize trait), and no depends_on orders the two; surface cited as the directory, same file inferred"
```
