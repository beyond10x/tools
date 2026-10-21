---
format: aep.planning-md/3
id: story:web-ask-through-summarize-port
kind: story
status: draft
title: web.ask answers a page against a prompt through a summarize port the harness binds
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-web
scope:
- confidence: inferred
  path: contracts/summarize/v1alpha1
- confidence: inferred
  path: contracts/web/v1alpha1
- confidence: inferred
  path: crates/tool-ports/src/summarize.rs
- confidence: inferred
  path: crates/tool-web
revision: 6
---
## Outcome

`web.ask {url, prompt}` fetches a page as `web.fetch` does and answers what a summarizer the host
binds says about it against the prompt, beside the digest of the full markdown. The summarizer is
the `summarize` v1alpha1 port: the contract declares it, the harness supplies it, and the suite runs
it with the `Echo` fixture. `web.ask` is part of `web` v1alpha1 and lands before any `web`
implementation is released.

## Context

The request (operator, 2026-10-21): fetch turns any result into readable markdown with a fast
summary, the summarizer a contract the harness fulfils and injects. Scan of the same day: no tool
in the inventory's projects does this. `web.ask` is its own operation, because a profile is static
per operation and `web.ask` differs from `web.fetch` in effects, risk and ports.

A harness binds `summarize` with an adapter that owns its model client: a `Box<dyn Toolset>` is
`'static` (`crates/tool-contract/src/conformance.rs:574`), so a fork borrowed from a parent model
(`harness/crates/harness-wire/src/port.rs:126`) cannot be held; the adapter takes an owned client.

## Contracts

| op | input | output |
|---|---|---|
| `summarize.text` (port) | `text`, `prompt`, `max_output_tokens?` | `summary`, `model?` |
| `web.ask` | `url`, `prompt` | `final_url`, `summary`, `sha256` of the full markdown, `truncated` |

| property | `summarize.text` | `web.ask` |
|---|---|---|
| effects | `[external]` | `[network, external]` (rule `port-effects`) |
| consequences | `[send-external]` | `[send-external]` |
| risk | high | high |
| idempotency | idempotent | idempotent |
| determinism | nondeterministic | nondeterministic |
| bounds | `timeout_ms {60000, 300000}`, `output_bytes {16384, 65536}` | same |
| subjects | — | `url` kind `url`, `web.ask({url})` |
| requires | — | `ports: [egress.fetch/v1alpha1, summarize.text/v1alpha1]` |
| placement, capability | local, `model.summarize` | local, `network.read` |
| output | untrusted | untrusted, failure as result |

Semantics of `summarize.text`, written into the port's `semantics.md`: the call carries no tools
and no conversation, only `text` and `prompt`; its answer is untrusted text. A harness-side test
that its adapter sends an empty tool list belongs to the harness repository.

`truncated` means the markdown was cut at `egress`'s `output_bytes` before it reached the
summarizer.

Outcomes: `summarize.text` answers `summarize.Refused{reason}` when the model declines or the
harness budget refuses the call; `web.ask` passes it through as `web.SummarizerRefused{reason}`,
beside `web.AddressRefused` and `web.SchemeRefused` from `web.fetch`.

This story owns the `Summarize` trait and its `Echo` and `Refuse` fixtures
(`crates/tool-ports/src/summarize.rs`).

## Scope

- `contracts/summarize/v1alpha1/` (new)
- `contracts/web/v1alpha1/` (the `web.ask` operation and its scenarios)
- `crates/tool-web/` (`web.ask`; new fixtures `tests/fixtures/long.html` and `long-cut.md`, the
  markdown of `long.html` cut at `output_bytes` max)
- `crates/tool-ports/src/summarize.rs`

## Acceptance

With `Echo` bound, which answers `summary` = the sha256 hex of the `text` it received:

- `web.ask` of the served `article.html` answers `summary` equal to `$sha256_of: article.md`, so
  the summarizer received the converted markdown; a mutant that passes the raw HTML fails it.
- The `.ports/summarize.jsonl` line for that call contains the prompt.
- The served `long.html` answers `truncated: true`.
- For `long.html`, `summary` equals `$sha256_of: long-cut.md` (the summarizer received the cut
  text).
- `http://169.254.169.254/` answers `web.AddressRefused` and `.ports/summarize.jsonl` is absent
  (nothing reached the summarizer).
- With the `Refuse` fixture bound, `web.ask` answers `web.SummarizerRefused`.
- With no `summarize` binding, `offered()` omits `web.ask`.
- `tool-contract validate contracts/` refuses `web.ask` declared with effects `[network]` only
  (rule `port-effects`).

## Not in this story

- A default summarizer here: the port is the harness's to fulfil. Page caching.
