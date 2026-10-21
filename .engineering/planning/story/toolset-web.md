---
format: aep.planning-md/3
id: story:toolset-web
kind: story
status: draft
title: The web toolset fetches a page as markdown and searches through a bound provider
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-http
- depends_on: story:profile-declares-ports
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/web/v1alpha1
- confidence: inferred
  path: contracts/websearch/v1alpha1
- confidence: inferred
  path: crates/tool-ports/src/websearch.rs
- confidence: inferred
  path: crates/tool-web
revision: 6
---
## Outcome

The `web` v1alpha1 toolset (`web.fetch`, `web.search`) and the `websearch` v1alpha1 port have
contracts and conformance suites, and `crates/tool-web` passes them. `web.fetch` turns a page into
readable markdown with no model involved, through the `egress` port; `web.search` asks whichever
search provider the host binds. `web.ask` joins the same toolset version in
`story:web-ask-through-summarize-port`, before any `web` implementation is released.

## Context

The scan of 2026-10-21 found every fetcher converting HTML with a library or a pure condenser, and
none summarizing. P1's condenser is pure and drops navigation, script and style before conversion;
it is read for behaviour. The `connectors` repository's Tavily adapter offers search
(`connectors/adapters/tavily/src/lib.rs:231`); a host that binds `websearch` to it bridges that
async call itself (design § Ports).

## Contracts

| op | input | output |
|---|---|---|
| `web.fetch` | `url`, `format: web.Format` (`markdown`, `text`, `raw`), `offset?`, `limit?` | `final_url`, `status`, `content_type`, `title?`, `text`, `lines: web.Lines{from, to, total}`, `truncated`, `sha256` of the whole converted text |
| `web.search` | `query`, `max_results?` (default 5, max 20) | `results: List<web.Result{title, url, snippet}>` |
| `websearch.query` (port) | `query`, `max_results` | `results` |

| property | `web.fetch` | `web.search` | `websearch.query` |
|---|---|---|---|
| effects | `[network]` | `[network, external]` | `[network, external]` |
| risk | low | high (the `external` floor) | high |
| idempotency | idempotent | idempotent | idempotent |
| determinism | nondeterministic | nondeterministic | nondeterministic |
| bounds | `timeout_ms {30000, 300000}`, `output_bytes {65536, 262144}` | `timeout_ms {15000, 60000}`, `output_bytes {65536, 262144}` | same as `web.search` |
| paging | `offset`, `limit`, `truncated`, `sha256` | — | — |
| subjects | `url` kind `url`, `web.fetch({url})` | — | — |
| requires | `ports: [egress.fetch/v1alpha1]` | `ports: [websearch.query/v1alpha1]` | — |
| placement, capability | local, `network.read` | local, `network.search` | local, `network.search` |
| output | untrusted, failure as result | untrusted, failure as result | — |

Outcomes of `web.fetch` beside `ok`: `web.AddressRefused`, `web.SchemeRefused` (the egress refusals
passed through), `web.UnsupportedType{content_type}` for a PDF, an image or any non-text type.

Converter: an HTML5 parser (`html5ever`, MIT OR Apache-2.0, `rust-version` 1.71, per the cargo
cache) with a condenser written here; the implementor records the crate and licence.

`format: raw` is not `http.get`: it answers the body as `egress.fetch` returned it, with
`web.fetch`'s paging and whole-text digest, for a page the conversion would damage.

## Scope

- `contracts/web/v1alpha1/`, `contracts/websearch/v1alpha1/` (new)
- `crates/tool-web/` (new; default egress from `crates/tool-http`)
- `crates/tool-ports/src/websearch.rs` (the `WebSearch` trait and its `Table` fixture, owned here)
- `Cargo.lock`: shared, merged serially.

## Acceptance

Fixtures: `crates/tool-web/tests/fixtures/article.html` and `article.md` (its expected markdown),
served with `serve`; the real `egress` from `crates/tool-http` with
`private_network: [127.0.0.1/32]`, wrapped in `Recorder` (`.ports/egress.jsonl`); the `websearch`
`Table` fixture.

- `web.fetch` of the article with `format: markdown` answers text whose sha256 equals
  `article.md`'s.
- The markdown has no text from the page's `<script>`, `<style>` or `<nav>` (`$contains` negated
  per element).
- The markdown keeps the page's `<h2>` heading, its one link and its code block.
- `format: raw` answers the served body byte for byte.
- `offset: 2, limit: 1` answers one line, `truncated: true` and the whole text's sha256 (paging).
- A served PDF answers `web.UnsupportedType`.
- A served page above `output_bytes` max answers `truncated: true`.
- `http://169.254.169.254/` answers `web.AddressRefused`, and `.ports/egress.jsonl` holds an
  `egress.fetch` line with outcome `http.AddressRefused` (the request went through the port).
- `file:///etc/hosts` answers `web.SchemeRefused`.
- Two `web.fetch` calls of one URL answer the same `sha256` (idempotency on a fixed body).
- `web.search` answers the `Table` fixture's three results for its query.
- `web.search` with `max_results: 21` is refused.
- With no `websearch` binding, `offered()` omits `web.search`.

## Not in this story

- `web.ask`: `story:web-ask-through-summarize-port`. `web.crawl`: not planned yet.
