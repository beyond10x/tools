---
format: aep.planning-md/3
id: story:toolset-browser
kind: story
status: draft
title: The browser toolset drives headless Chromium through a ref snapshot
relations:
- decomposes: epic:tool-contracts
- depends_on: story:toolset-http
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: contracts/browser/v1alpha1
- confidence: inferred
  path: crates/tool-browser
revision: 6
---
## Outcome

The `browser` v1alpha1 toolset has a contract, semantics and a conformance suite, and
`crates/tool-browser` passes it: a model drives a headless Chromium through an accessibility-tree
snapshot whose elements carry refs, and acts on a ref. Every HTTP(S) request Chromium makes, from
navigation, a click, a redirect, a subresource or page script, goes through the `egress` guard.

## Context

The scan of 2026-10-21 found three browser tools in the inventory's projects. P1's is Rust over raw
CDP with five operations and an accessibility digest with `e<N>` refs; the other two drive Chromium
by CSS selector, and one guards the URL it navigates to. No checkout of browser-use exists;
browser-use is a Python agent loop, and its indexed-element model matches the ref snapshot. Host
facts (command output, 2026-10-21): no Chromium on PATH; the Playwright browser cache holds
chromium-1228 and chromium-1243; X11 with `xdotool` and `scrot`. Desktop computer use is not
planned: no harness in the scan has a use case.

## Design

- Chromium starts per session with a fresh user-data directory in the session's scratch directory
  (no host profile, cookies or logins), downloads denied, `--proxy-server` pointing at a forward
  proxy in this crate that opens every upstream connection, plain HTTP and HTTPS `CONNECT` alike,
  through `egress.connect`, `--proxy-bypass-list=<-loopback>`
  so loopback is proxied too, and WebRTC limited to proxied traffic
  (`--force-webrtc-ip-handling-policy=disable_non_proxied_udp`).
- Navigation schemes: `browser.open` and `browser.goto` refuse anything but `http` and `https`;
  navigations a page requests (`Page.frameRequestedNavigation`) to another scheme are refused.
- CDP over a synchronous websocket client (`tungstenite`); no async runtime. The implementor records
  the crate and licence.
- The Chromium executable is a host setting (`requires.settings: [browser.executable]`) and
  `requires.binaries: [chromium]`, as build-test names programs
  (`contracts/build-test/v1alpha1/contract.yaml:157`). A machine without it fails the suite, naming
  it.
- Session count is the `processes` bound; the idle limit is a host setting, tested by a crate unit
  test.

## Contracts

| op | input | output |
|---|---|---|
| `browser.open` | `url?` | `session` |
| `browser.goto` | `session`, `url` | `final_url`, `title` |
| `browser.snapshot` | `session`, `view: browser.View` (`actions`, `content`, `full`) | `text` with `e<N>` refs, `truncated` |
| `browser.act` | `session`, `action: browser.Action` (`click`, `type`, `fill`, `press`, `scroll`, `select`), `ref?`, `value?` | the `actions` snapshot after the action |
| `browser.screenshot` | `session`, `ref?`, `full_page?` | `png: Bytes`, `width`, `height` |
| `browser.close` | `session` | `closed` |

| property | open | goto | snapshot | act | screenshot | close |
|---|---|---|---|---|---|---|
| effects | `[process, network]` | `[network]` | `[read]` | `[network]` | `[read]` | `[process]` |
| consequences | `[]` | `[]` | `[]` | `[send-external]` | `[]` | `[]` |
| risk | medium | low | none | high | none | medium |
| idempotency | non-idempotent | non-idempotent | idempotent | non-idempotent | idempotent | idempotent |
| determinism | nondeterministic | nondeterministic | nondeterministic | nondeterministic | nondeterministic | deterministic |
| bounds | `timeout_ms {30000, 120000}`, `processes {1, 4}` | `timeout_ms {30000, 120000}` | `timeout_ms {10000, 60000}`, `output_bytes {65536, 262144}` | as snapshot | `timeout_ms {10000, 60000}`, `output_bytes {1048576, 4194304}` | `timeout_ms {5000, 30000}` |
| subjects | `url` kind `url` | `url` kind `url` | — | — | — | — |
| requires | `ports: [egress.connect/v1alpha1]`, binary, setting | `ports: [egress.connect/v1alpha1]` | — | `ports: [egress.connect/v1alpha1]` | — | — |
| output | failure as result | failure as result | untrusted | untrusted | untrusted | — |

Placement local, capability `browser.drive` for all six (a proposal, checked against Canon by the
implementor). Outcomes: `browser.AddressRefused`, `browser.SchemeRefused`, `browser.StaleRef{ref}`,
`browser.UnknownSession{session}`, `browser.SessionLimit{max}`.

## Scope

- `contracts/browser/v1alpha1/` (new)
- `crates/tool-browser/` (new)
- `Cargo.lock`: shared, merged serially.

## Acceptance

The suite binds the real `egress` from `crates/tool-http` with `private_network: [127.0.0.1/32]`,
wrapped in `Recorder` (`.ports/egress.jsonl`). Fixture site served with `serve`: `form.html` (a
button `Go` that sets the heading to `Done`, a link to `/next`, a link to `file:///etc/hosts`, an
`<img src="http://169.254.169.254/x.png">`, a script that fetches `http://169.254.169.254/`),
`/next`, `/r` (redirects to `http://169.254.169.254/`), `/set-cookie` (sets `k=v`) and
`/show-cookie` (echoes the request's `Cookie` header into the page).

- `browser.open {{serve}}/form.html` then `browser.snapshot` (`calls`) answers a ref for the `Go`
  button.
- `browser.act click` on that ref answers a snapshot whose heading is `Done`.
- After `browser.goto /next`, acting on the old ref answers `browser.StaleRef`.
- `browser.goto http://169.254.169.254/` answers `browser.AddressRefused`.
- `browser.open http://169.254.169.254/` answers `browser.AddressRefused`.
- Clicking the `file:` link answers `browser.SchemeRefused`.
- After `form.html` loads, `.ports/egress.jsonl` holds an `egress.connect` line to
  `169.254.169.254:80` with outcome `http.AddressRefused` (the image and the script fetch were
  refused, not sent).
- `browser.goto {{serve}}/r` answers `browser.AddressRefused`.
- In session 1, `goto /set-cookie` then `goto /show-cookie` answers a snapshot containing `k=v`.
- In session 2, `goto /show-cookie` answers a snapshot without `k=v`.
- `browser.screenshot` answers a PNG with `width` and `height` above 0.
- A snapshot above `output_bytes` answers `truncated: true`.
- `browser.close` twice answers `closed` both times.
- `browser.snapshot` on a closed session answers `browser.UnknownSession`.
- A fifth `browser.open` while four are open answers `browser.SessionLimit{max: 4}`.
- With `browser.executable` set to a path that does not exist, the suite fails with a line naming
  `chromium` and counts no scenario as passed.

## Not in this story

- Desktop computer use (screen capture, X11 input).
