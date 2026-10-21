---
format: aep.planning-md/3
id: review-result:plan-critic-parallel-safety-web-toolsets-r1
kind: review-result
status: active
title: 'Parallel-safety critic, round 1, on the web, http, connector and browser toolset stories: needs-revision, 5 findings'
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

Findings, one line each (all `origin: introduced`, all `severity: warning`):

story:connector-projection — its Scope lists `Cargo.lock` and story:toolset-http's Scope lists it too; neither body names the other and no depends_on edge orders them, so one wave can edit the same lock file unsequenced — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-http` (## Scope)

story:connector-projection — its Scope lists `Cargo.lock` and story:toolset-web's Scope lists it too; neither body names the other and no depends_on edge orders them — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-web` (## Scope)

story:connector-projection — its Scope lists `Cargo.lock` and story:toolset-browser's Scope lists it too; neither body names the other and no depends_on edge orders them — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:toolset-browser` (## Scope)

story:toolset-browser — its Scope lists `Cargo.lock` and story:toolset-web's Scope lists it too; neither body names the other and no depends_on edge orders them — `aep plan artifact show story:toolset-browser` (## Scope), `aep plan artifact show story:toolset-web` (## Scope)

story:connector-projection — its Scope and story:profile-declares-ports' Scope both edit `contracts/profile/tool-contract-1.md` (a new § on toolset grammar versus a `requires.ports` row and § Ports); neither body names the other and no depends_on edge orders them — `aep plan artifact show story:connector-projection` (## Scope), `aep plan artifact show story:profile-declares-ports` (## Scope)

What I read: 7 drafted stories in full via `aep plan artifact show`, plus `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph` (in-set edges) and `aep plan artifact relations`. On disk, 8 cited paths exist (`Cargo.lock`, `contracts/profile/tool-contract-1.md`, `crates/tool-contract/src/{document,validate,conformance}.rs`, `crates/tool-toolchain/src/executor.rs`). New surfaces are absent as expected.

Counts: surfaces established cited 7, inferred 0, unplaceable 0. The CLI tags every scope line "inferred", but each body names its paths in ## Scope, so I counted them cited. Acceptance reads traced to an in-set producer: 8, of which 6 have a direct depends_on edge and 2 are transitive only. Those 2 are story:web-ask-through-summarize-port reading the `echo` fixture and the offering rule from story:profile-declares-ports via story:toolset-web. Ordering holds through the chain, so I did not count them as findings.

Could not establish:
- story:toolset-connector has no Cargo.lock in its Scope but builds in `crates/tool-connector`. Whether it changes the lock file is not stated.
- The Rust crates (tool-http, tool-web, tool-browser, tool-connector) emit tool-contract/1 documents, yet no body names a `crates/tool-contract` dependency. Rebuild coupling to profile-declares-ports' edits to `document.rs` is not assessed.
- Out of my lane: the port document format is left to the implementor ("top-level `port: true` (or a sibling format)", story:profile-declares-ports ## Design). That is for plan-critic-design.
- Out of my lane: whether "`validate contracts/` passes" is checkable is for plan-critic-acceptance.
- Collisions involving items outside this set were ignored.

```findings
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:connector-projection and story:toolset-http both list Cargo.lock in ## Scope, neither body names the other and no depends_on edge orders them, so the two land on one lock file unsequenced"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:connector-projection and story:toolset-web both list Cargo.lock in ## Scope, neither body names the other and no depends_on edge orders them, so the two land on one lock file unsequenced"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:connector-projection and story:toolset-browser both list Cargo.lock in ## Scope, neither body names the other and no depends_on edge orders them, so the two land on one lock file unsequenced"
- file: "aep plan artifact show story:toolset-browser"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:toolset-browser and story:toolset-web both list Cargo.lock in ## Scope, neither body names the other and no depends_on edge orders them, so the two land on one lock file unsequenced"
- file: "aep plan artifact show story:connector-projection"
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:connector-projection and story:profile-declares-ports both edit contracts/profile/tool-contract-1.md in ## Scope, neither body names the other and no depends_on edge orders them, so both sections of that document are written unsequenced"
```
