---
format: aep.planning-md/3
id: story:oracle-socket-fits-the-path-limit
kind: story
status: draft
title: The grep oracle test binds its socket on any checkout path
summary: the seeded-tree socket binds under a long checkout path instead of failing on SUN_LEN
relations:
- decomposes: epic:tool-contracts
revision: 3
---
## Outcome

`search_grep_answers_what_harness_grep_answers_over_seeded_trees` passes on a checkout whose path
is long, as it does on a short one: the socket it puts on a seeded tree binds whatever the length of
the checkout's path.

## Context

The test seeds trees under `CARGO_TARGET_TMPDIR` (`scratch()` in
`crates/tool-search/tests/adversary_harness_oracle.rs`) and, on some trees, binds a Unix socket at
`<tree>/sock` (`build`). A Unix socket path is limited to `SUN_LEN` (108 bytes on Linux, 104 on
macOS). On a checkout under a long directory, the scratch path under `target/tmp` passes that limit
and `UnixListener::bind` fails with "path must be shorter than SUN_LEN", so the test fails before it
compares anything. It was seen on a fresh clone in a long scratch directory: the repository's tests
were red at the clone's base for this reason alone, and only a shorter clone path made them pass.

## Scope

- `crates/tool-search/tests/adversary_harness_oracle.rs` (`scratch`, `build`)

## Acceptance

- With the repository cloned under a directory whose path is at least 120 bytes long, `cargo test
  -p tool-search --test adversary_harness_oracle` passes.
- The seeded trees, and the socket among them, are the ones the property was first run on: the
  random draws and their order do not change.
