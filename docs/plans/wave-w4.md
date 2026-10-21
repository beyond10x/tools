# Wave w4: the process-group kill, declared timeouts, the conformance runner's host keys

Run 2026-10-21 by the `tools` session under `aep:implementing` 0.23.2 (wave mode). Proposed by the
`tools` session and approved by `b10x-chef-orchestrator` under exception 6: "Wave approved under
exception 6: units 1 → 2 → 3, serial, one sub-agent at a time." Unit 1 first because harness pins
its fix.

## Units

| unit | story | branch | scope confidence |
|---|---|---|---|
| 1 | `story:process-kill-never-reaches-a-reused-group` | `impl/process-kill-never-reaches-a-reused-group` | cited |
| 2 | `story:declared-timeouts-are-enforced` | `impl/declared-timeouts-are-enforced` | cited |
| 3 | `story:conformance-runner-hosts-and-sequences` | `impl/conformance-runner-hosts-and-sequences` | inferred |

Serial (N = 1 at a time, the approver's rule): each unit forks from `wave/w4` after the previous
unit merged.

Not in this wave: `story:build-test-applies-provider-environment` (same file as unit 1,
`crates/tool-process/src/lib.rs`), `story:contract-directory-names-validated` (same file as unit 3,
`contracts/profile/tool-contract-1.md`), `story:ess-domains-for-contracts`, `story:docs-site`, and
the web-toolsets stories after unit 3 (`story:connector-projection` and `story:toolset-tool-meta`
wait on an ADR question with the operator).

## What `aep plan artifact waves --kind story --status draft` printed (2026-10-21, before the moves)

```
wave 1
  story:build-test-applies-provider-environment
  story:conformance-runner-hosts-and-sequences (inferred)
wave 2
wave 3
  story:profile-declares-ports (inferred)
...
```

(full output in the `tools-plan-web-toolsets` session report; the three units share no surface:
unit 1 `crates/tool-process/src/lib.rs`; unit 2 `contracts/fs`, `contracts/search`,
`crates/tool-fs/src/{lib,walk}.rs`, `crates/tool-search/src/lib.rs`; unit 3
`contracts/profile/tool-contract-1.md`, `crates/tool-contract/src/conformance.rs`,
`crates/tool-contract/tests`.)

## Triples

Wave root: `$HOME/.cache/tools-wave-w4`. Managed trees under
`$HOME/.local/state/worktree/trees/b10x/tools/`, each building into its own `target/`.
`<root>/bin/cargo-serial.sh` runs one `cargo` at a time through `b10x-build-slot`, with `sccache`.

| unit | managed id | worktree | build dir | scratch |
|---|---|---|---|---|
| coordinator | `wave-w4` | `<trees>/wave-w4` (branch `wave/w4`) | `<tree>/target` | `<root>/coordinator/scratch` |
| 1 | `impl-process-kill` | `<trees>/impl-process-kill` | `<tree>/target` | `<root>/process-kill/scratch` |
| 2 | `impl-declared-timeouts` | `<trees>/impl-declared-timeouts` | `<tree>/target` | `<root>/declared-timeouts/scratch` |
| 3 | `impl-conformance-runner` | `<trees>/impl-conformance-runner` | `<tree>/target` | `<root>/conformance-runner/scratch` |

## Pre-flight (2026-10-21)

| check | value |
|---|---|
| base | `main` at `2363756` (plan commit after tag `0.4.0`) |
| free disk | 51G on `/` (`df -h /`), build-slot floor 20G |
| other tools trees | `tools-plan-web-toolsets` (merged plan branch, kept until this wave closes) |
| dispatch | `aep:implementor`, `aep:adversary` (plugin agents), one at a time |

## Commits this approval covers

1. The opening store commit on `wave/w4` (moves to `active`, this page, the `AGENTS.md` line).
2. One commit per unit on its `impl/*` branch, by the coordinator through the bot route.
3. The merge of each green unit into `wave/w4`.
4. The closing store commit.
5. The merge of `wave/w4` into `main` and its push.

The release after the wave is approved separately by the same message ("After the wave, release on
the newest ess").

## Decisions taken by the coordinator

| id | decision | reason |
|---|---|---|

## Stage log

| when (UTC) | what |
|---|---|
| 2026-10-21 | integration tree `wave-w4` at `2363756`, branch `wave/w4`; the three stories moved `draft → proposed → active` |
