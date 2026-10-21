---
format: aep.planning-md/3
id: story:toolset-http
kind: story
status: draft
title: The http toolset and the guarded egress port have a contract, a conformance suite and a default implementation
relations:
- decomposes: epic:tool-contracts
- depends_on: story:profile-declares-ports
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/egress/v1alpha1
- confidence: inferred
  path: contracts/http/v1alpha1
- confidence: inferred
  path: crates/tool-http
- confidence: inferred
  path: crates/tool-ports/src/egress.rs
revision: 6
---
## Outcome

The `http` v1alpha1 toolset (`http.get`, `http.request`) and the `egress` v1alpha1 port
(`egress.fetch`, `egress.send`, `egress.connect`) have contracts and conformance suites, and
`crates/tool-http` passes them. `crates/tool-http` holds the default `egress` implementation: a
blocking client that judges the address it connects to, connects to that address, re-checks every
redirect, caps the decoded body and never sends a secret to a host it was not declared for.
`web` and `browser` reach the network only through `egress`; the connectors host's own client is
outside it (`story:connector-projection`).

## Context

A scan of 2026-10-21 over the inventory's projects counted 10 web fetchers (the inventory's
narrower scan of 2026-10-19 counted 8, `docs/inventory.md:25`); 2 guard the address, both in P1
and its Go predecessor. P1's guard resolves the name and connects to the resolved address, follows
at most 5 redirects re-checking each, caps the body at 256 KiB and lets a header carry a secret by
reference from an allow-list. The others check the scheme only, so a model reaches `127.0.0.1` and
`169.254.169.254`. P1 is read for behaviour; the crate is written here.

## Contracts

| op | input | output |
|---|---|---|
| `http.get` | `url`, `headers?`, `timeout_ms?`, `max_bytes?` | `status`, `headers`, `body`, `truncated`, `final_url`, `redirects` |
| `http.request` | `method`, `url`, `headers?`, `body?`, `secret_headers?` (header → secret reference), `timeout_ms?`, `max_bytes?` | the same |
| `egress.fetch` (port) | the `http.get` input | the same |
| `egress.send` (port) | the `http.request` input | the same |
| `egress.connect` (port) | `host`, `port`, `timeout_ms?` | `address`; in Rust, a connected stream |

| property | `http.get`, `egress.fetch` | `http.request`, `egress.send` | `egress.connect` |
|---|---|---|---|
| effects | `[network]` | `[network, secret]` | `[network]` |
| consequences | `[]` | `[send-external]` | `[]` |
| risk | low | high | low |
| idempotency | idempotent | non-idempotent | non-idempotent |
| determinism | nondeterministic | nondeterministic | nondeterministic |
| bounds | `timeout_ms {30000, 300000}`, `output_bytes {65536, 262144}` | same | `timeout_ms {30000, 120000}` |
| subjects | `url` kind `url`, `http.get({url})` | `url` kind `url`, `http.request({method} {url})` | — |
| requires (toolset ops) | `ports: [egress.fetch/v1alpha1]` | `ports: [egress.send/v1alpha1]`, `secrets` by reference | — |
| placement, capability | local, `network.read` | local, `network.send` | local, `network.read` |
| output | untrusted, failure as result | untrusted, failure as result | — |

`http.get` carries model-chosen URLs and headers but no body and no secret; it stays at risk low,
and a governor limits where it goes through `subjects`. `egress.connect` is the tunnel the browser's
forward proxy uses for HTTP and for HTTPS `CONNECT`; the address rules below apply to it.
Capability ids are proposals; the implementor checks them against Canon's vocabulary (ADR
0083) and records the result.

Outcomes beside `ok`: `http.AddressRefused{url, address}`, `http.SchemeRefused{url}`,
`http.TooManyRedirects{url}`, `http.SecretRefused{reference}`, `http.Timeout{timeout_ms}`.

## Guard rules (egress)

- Schemes `http` and `https` only, on the first URL and on every redirect.
- The guard judges the socket address the client connects to, after resolution, never the URL
  text; the connection goes to the address that was judged (no second lookup).
- Refused ranges (16): `127.0.0.0/8`, `::1/128`, `0.0.0.0/8`, `::/128`, `169.254.0.0/16`,
  `fe80::/10`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`, `100.64.0.0/10`,
  `::ffff:0:0/96` (IPv4-mapped), `64:ff9b::/96` (NAT64), `224.0.0.0/4`, `ff00::/8`,
  `255.255.255.255/32`.
- The host's `private_network` allow-list (CIDRs, a constructor argument, never an input field)
  exempts named ranges; a name resolving outside them is still refused.
- `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY` are ignored.
- Redirects: at most 5, each re-judged; a redirect to another origin drops every header that came
  from `secret_headers`.
- Secrets: the host passes `{reference → {value, hosts}}` to the constructor; a reference the host
  did not declare, or a request to a host outside its `hosts`, answers `http.SecretRefused` with no
  request sent; a secret value never appears in an answer, a header echoed back included.
- `max_bytes` counts the decoded body; a compressed body that expands past it is cut and answers
  `truncated: true`.
- Unknown input keys are refused (`deny_unknown_fields`).

Client: a blocking client with a pluggable resolver and redirects switched off, such as `ureq` 3;
the implementor records its licence and MSRV (workspace `rust-version = "1.98"`, `Cargo.toml:8`).
No async runtime enters the workspace.

This story owns the `Egress` trait (`crates/tool-ports/src/egress.rs`) and its `Table` fixture
(URL → response, answering the refusal outcomes for URLs the table marks refused), which `web` and
`browser` suites use wrapped in `Recorder`.

## Scope

- `contracts/http/v1alpha1/`, `contracts/egress/v1alpha1/` (new)
- `crates/tool-http/` (new), `crates/tool-ports/src/egress.rs`
- `Cargo.lock`: shared, merged serially (AGENTS.md § Branches and waves).

## Acceptance

Each bullet is one scenario with a mutant that fails it; the suite uses the runner's `serve` and
`host` keys (`story:conformance-runner-hosts-and-sequences`), `private_network: [127.0.0.1/32]`
unless a bullet says otherwise, and no outside network.

- `http.get` of `{{serve}}/page` answers the served body, status 200, `truncated: false`.
- With an empty `private_network`, `http.get` of `{{serve}}/page` answers `http.AddressRefused`.
- With `private_network: [127.0.0.1/32]`, the same call answers 200 (the allow-list exempts it).
- Each of the 16 refused ranges, written as one literal address in it, answers
  `http.AddressRefused` (16 scenarios).
- `http://2130706433/`, `http://0x7f.1/` and `http://127.1/` answer `http.AddressRefused` with an
  empty `private_network`.
- A name the fixture resolver answers first with an allowed and then with a refused address is
  connected at the first address (rebinding).
- `file:///etc/hosts` and `gopher://127.0.0.1/` answer `http.SchemeRefused`.
- A served redirect to `file:///etc/passwd` answers `http.SchemeRefused`.
- A served redirect to `169.254.169.254` answers `http.AddressRefused`.
- A chain of 5 served redirects answers the final route's body, `redirects: 5`.
- A chain of 6 answers `http.TooManyRedirects`.
- A redirect from `{{serve}}` to a second served origin (`127.0.0.2`, allowed) leaves no
  `secret_headers` header in the second route's `.serve` record.
- An undeclared secret reference answers `http.SecretRefused`, and `.serve/_page.jsonl` is absent.
- A declared reference whose `hosts` does not name the served host answers `http.SecretRefused`,
  and `.serve/_page.jsonl` is absent.
- A route that echoes the secret header answers a body without the secret value.
- A 400 KiB body with `max_bytes: 1024` answers 1024 bytes and `truncated: true`.
- A gzip body that decodes past `max_bytes` answers `truncated: true`.
- A route with `delay_ms: 2000` called with `timeout_ms: 500` answers `http.Timeout`.
- `timeout_ms: 400000` is refused as above the 300000 ceiling.
- `HTTP_PROXY` naming a served route is not used for a request: that route's `.serve` record is
  absent.
- An input key `private_network` is refused.
- Two identical `http.get` calls (`calls`) answer the same body.
- Two identical `http.request` POSTs (`calls`) leave two lines in the route's `.serve` record.
- `egress.connect` to `169.254.169.254:80` answers `http.AddressRefused`; to the served address it
  answers the address (a crate test, since the answer is a stream).

## Not in this story

- Markdown conversion: `story:toolset-web`. Cookies, sessions, retries.
