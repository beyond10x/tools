# tools

Agent tool contracts (`contracts/`) and their b10x default implementations (`crates/tool-*`). This
file is for agents working on this repository; `README.md` is for people.

## Serves

- **O1 — governed reach.** A contract declares each operation's effects, consequences, risk,
  bounds and the Canon capability it serves before any run, so whatever binds and invokes the tool
  checks a call against declared properties instead of inferred ones.
- **O2 — decisions as data, with evidence.** A contract is an ESS command plus the
  `tool-contract/1` profile, and "implements" is a passing conformance run, not a claim.
- **O3 — any harness, observed and compared.** Harnesses that adopt a toolset call the same
  operations under the same names and properties, so their tool calls are recorded in one
  vocabulary and can be compared on the same work.

## Start here

| read | for |
|---|---|
| `docs/design/tool-contracts.md` | the contract model, the toolsets, conformance, how harnesses adopt it |
| `docs/inventory.md` | the existing tool layers each contract property was taken from |
| `aep plan artifact list` | the planned work: epic `tool-contracts` and its stories |

## Rules

- A contract is a specification. It names no implementation, no vendor tool name and no harness.
  Vendor names (Claude, Codex, MCP) are generated renderings, never the source.
- A contract change is versioned: a released `v1alpha1` is never edited in place; a change that a
  conforming implementation could fail is a new version beside it.
- "Implements" means "passes the toolset's conformance suite". Every profile property has at least
  one scenario that fails when an implementation ignores it.
- Implementations derive their schemas from their types; the conformance suite checks the derived
  schema equals the contract's. No hand-written schema in a crate.
- Capabilities are Canon's (atlas ADR 0083): a contract names the capability it serves; it does not
  define a new authority vocabulary. Commission binds and invokes (ADR 0082).
- Public repository: no person's name, employer, customer, private repository name or private
  hostname in any committed text, commit message or release note.
- Licence per crate, never at the workspace: every crate is Apache-2.0, a crate moved from
  harness included (owner's decision of 2026-10-20); `README.md`'s crate table names each. A
  licence other than Apache-2.0 is the owner's decision, not a wave's.
- All Git writes (commits, merges, pushes, tags) go through the bot route (`b10x-gates bot -- …`).
- The planning store changes only through `aep plan artifact` verbs.
- A review record (`review-result`) is immutable: before `aep plan artifact new --from`, make every
  path in the reviewer's text repository-relative. The gate refuses absolute home-directory paths
  (rule `personal-paths`), and no verb can correct a record afterwards.

## Layout

| path | what |
|---|---|
| `contracts/profile/tool-contract-1.md` | the profile: every property, its values and the validator's rules |
| `contracts/<toolset>/<version>/` | `contract.yaml`, `semantics.md`, `scenarios/`, `fixtures/` |
| `crates/tool-contract` | profile types, document parser, validator, `tool-conformance/1` runner, schema shape check, the `tool-contract` binary |
| `crates/tool-<toolset>` | one default implementation per toolset; each records the harness file it came from |
| `docs/design/tool-contracts.md` | the design; § T1 in detail has the document grammar and the runner API |
| `docs/plans/` | wave pages |
| `.engineering/planning` | the AEP store |

## Commands

```
task check                                   # the gate: store, cargo test, fmt, clippy, msrv check, contract validation
aep plan artifact validate
aep plan artifact waves --kind story --status draft
cargo run -p b10x-tool-contract -- validate contracts/
cargo test -p b10x-tool-fs                   # one crate's unit tests, its conformance run and its mutant tests
```

## Builds

- Each worktree builds into its own `target/` inside the tree. Never set `CARGO_TARGET_DIR` to a
  shared directory; never point two trees at one build directory.
- One `cargo` at a time on a shared machine, with a disk floor: a wave serialises builds through a
  lock and stops a build under the floor. Check `df -h /` before a build.
- `cargo fmt -p <crate>` in a unit; `cargo fmt --all --check` only in the gate.
- The workspace is `members = ["crates/*"]`; adding a crate touches no manifest but `Cargo.lock`.

## Branches and waves

- Integration branch `wave/<name>`, one unit branch `impl/<story-slug>` per story, every unit forked
  from the wave branch; merged serially into it; the gate runs once on the merge; the wave branch
  merges into `main`. The wave page is `docs/plans/wave-<name>.md` and is kept current.
- Implementors write source, contracts and tests only. The coordinator owns the store,
  `AGENTS.md`, `README.md`, `Taskfile.yml`, `.github/`, the design and the wave page.
- Managed worktrees only (`worktree create`), each finished with
  `worktree finish --discard-cache --archive`.

## Releases

- Versions are workspace-wide (`[workspace.package] version`); tags are the bare version (`0.1.0`),
  annotated, on a gated `main` commit. The workspace is `publish = false`: a release is a source
  release harness and metaharness pin by tag.
- `CHANGELOG.md` in Keep a Changelog form; a change enters `Unreleased` with the change that makes it.
- Steps: a commit `release: tools <version>` that sets the version, `Cargo.lock` and the changelog
  date; the full gate; `b10x-gates bot -- tag -a <version> -m "tools <version>"`;
  `b10x-gates bot -- push origin main <version>`; `b10x-gates gh -- release create <version>`.
- A release is a separate step after a wave closes, never a step inside one.
