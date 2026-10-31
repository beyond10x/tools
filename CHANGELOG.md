# Changelog

All notable changes to this repository are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions are workspace-wide and tags are
the bare version.

## [Unreleased]

## [0.6.0] - 2026-10-31

The `fs` containment check refuses a climb out of the workspace through a missing directory, and the
contract validator refuses directory names the build script cannot embed.

### Added

- Rule `directory` in the `tool-contract/1` profile and `tool_contract::validate::directories`: the
  `<toolset>` and `<version>` directory names of a document are each in `[a-z][a-z0-9-]*`, and
  `tool-contract validate` refuses another name, naming it. A document the validator passes can no
  longer stop the workspace from building.
- `tool_fs::Mutation::LexicalWalk`, a mutant that judges a missing path by its lexical ancestor, and
  the `fs` v1alpha2 scenario
  `a-climb-through-a-missing-directory-is-refused-as-outside-the-workspace` that kills it. Breaking
  for a caller that matches `Mutation` exhaustively: `Mutation::ALL` is now seven entries.

### Changed

- The design no longer plans a connector projection in tools (ADR 0103): Connectors publishes
  `tool-contract/1` documents for its operations, tools reads none of its descriptors, and the
  `connectors` port leaves the first set of ports. `docs/design/tool-contracts.md`
  § Connector contracts replaces § Connector projections. No code changes; none had been built.
- `tool_contract::documents::all` documents its order: the byte order of the directory names, not
  version precedence.

### Fixed

- `fs` refuses a path that climbs out of the workspace through a component that does not exist
  (`missing/../../x`) as `OutsideWorkspace`; the walk now takes each `..` off the path built so far
  instead of judging the lexical ancestor, which was the workspace root itself.

## [0.5.0] - 2026-10-23

Wave w4: the process-group kill that never reaches a reused group, the `fs` and `search` timeouts
enforced in v1alpha2, and the conformance runner's host keys, served fixtures and call sequences.
harness pins this tag.

### Added

- `fs` v1alpha2 and `search` v1alpha2 contracts: v1alpha1 plus an optional `timeout_ms` input
  (default 10000; above 60000 refused as `InvalidInput`, after every other input refusal) and a
  `deadline_reached` output on every operation. At the deadline an operation answers what it had
  done, with `truncated` set where present and `note` saying so.
- `tool_fs::deadline::Deadline` and `tool_fs::walk::walk_within`, a walk that stops at a deadline.
- `tool-conformance/1`, all keys optional so every released suite runs unchanged: a scenario's
  `host` map goes to the toolset factory and never to an operation; `fixture.serve` answers HTTP/1.1
  routes on loopback (status, headers, `body` or `body_file`, `redirect`, `delay_ms`) and logs each
  request to `.serve/<path>.jsonl`, refusing at parse two routes that would share a log and
  answering `501` to a request with `Transfer-Encoding`; `{{serve}}` and `{{workspace}}` in call
  input and `host`; `calls`, an ordered list on one toolset instance whose entries read earlier
  answers as `{{calls.<n>.output.<path>}}`; the file matcher `{$lines: n}`.
- `tool_contract::conformance::run_with`, whose factory receives a `ScenarioContext` (workspace,
  `host`, served address) and may report a `Missing` program or setting; the run then names it and
  counts no scenario as passed. The schema check builds with the `host` and workspace of the first
  scenario whose factory was called. `run` stays and calls it.

### Changed

- `b10x-tool-fs` and `b10x-tool-search` implement `fs` v1alpha2 and `search` v1alpha2 and enforce
  the declared `timeout_ms`; they no longer claim v1alpha1, whose documents stay unchanged in
  `contracts/`. Breaking: `FsError` and `SearchError` gain `TimeoutAboveMax`; the typed inputs gain
  `timeout_ms` and the outputs `deadline_reached`.

### Fixed

- `process.run` keeps the program's shell unreaped until the group kill at the bound, so the kill can
  never name a process group whose id was freed and reused; a program dead by its own signal is
  answered at once, and one that stops itself is killed at the bound.
- `fs.read`: a line cut by the 16 MiB counting bound ends on a whole character and is listed in
  `truncated_lines`; a CRLF line the bound stops at its line ending is answered whole and without
  its `\r`, as an LF line is.

## [0.4.0] - 2026-10-20

The build-test executor behind a runner seam, so harness runs local and confined toolchain plans
through one executor (atlas ADR 0097 point 3). harness pins this tag.

### Changed

- `tool_toolchain::executor::Executor` runs every planned argv through a `Runner` the caller
  supplies (`Executor::with_runner`), with `b10x-tool-process`'s `Process` as the default
  (`Executor::new`), so a local and a confined run share one executor. A runner receives each argv
  with the provider's name, its declared sandbox (placeholders unexpanded) and the time left, and
  answers a `RunOutput`; every check of the `build-test` contract stays the executor's and runs
  before the runner is called. The local runner does not apply the sandbox. Breaking:
  `Executor` takes a type parameter (defaulted), and `ResolvedProvider` gains a public `sandbox`
  field, so a struct literal of it names the new field.

## [0.3.0] - 2026-10-20

Wave w3: the build-test toolset with its executor on the process runner, and the search toolset.
harness pins this tag to move the rest of its tools here (atlas ADR 0097 point 3).

### Added

- `build-test` v1alpha1: a static contract for the 20 operations the five built-in toolchain
  providers (rust, go, taskfile, npm, yarn) derive, held equal to the derivation by a test, with a
  conformance suite of 44 scenarios over a Cargo workspace, a Go module, a Taskfile, an npm and a
  yarn project, all offline. `b10x-tool-toolchain` gains `executor`, harness 0.14.1's toolchain
  executor moved here over `b10x-tool-process`: the active providers' declared program set, each
  argv under the time its caller has left, a failed `empty_stdout` check answered as
  `build-test.CheckFailed`, and every path argument judged where it lands before anything starts.
  Six mutants, each failed by the suite. Every operation declares `placement: local`,
  `effects: [process, write, network, secret]`, every consequence and `risk: high`, because the
  runner confines nothing beyond the environment, the bounds and the paths.
- `search` v1alpha1 (`search.grep`) with its conformance suite (53 scenarios), and
  `b10x-tool-search`, harness 0.14.1's `grep` moved here: a literal substring by default or a
  regular expression, a glob filter by `fs.glob`'s rule, up to 5 numbered lines of context either
  side, at most 200 matches, lines cut at 400 characters with `line_truncated`, files over 1 MiB
  counted and named in `skipped_large_files` and `skipped_large_paths`, and `depth_bound_reached`.
  It declares `risk: low`, as `fs.read` does, and walks with `b10x-tool-fs`'s walk, not a copy.
- `tool_fs::walk` in `b10x-tool-fs` is public: `walk`, `Walked`, `contained`, `SKIPPED`,
  `MAX_WALK_DEPTH` and `PathGlob`. No `fs` behaviour changes.

### Changed

- Against harness 0.14.1, a build-test operation whose plan can start several processes
  (`rust_fmt`) always answers `runs`, a list, where harness answered an object when one process
  ran; the processes of one call share the time left instead of each getting all of it, and none
  starts once it is gone.

### Fixed

- Pre-existing in harness 0.14.1's toolchain executor: a path was judged by its spelling, so
  `go_fmt` on a link into a refused directory rewrote the file behind it; every path argument is
  now judged where it lands, and one that lands outside the workspace is refused. A value
  beginning with `-` reached the program as an option nobody declared (`-w` turned
  `go_fmt_check` into a rewrite); it is refused, except after a declared `--` separator.
- Against harness 0.14.1's `grep`, `search.grep` answers `truncated: false` for exactly
  `max_results` matches; refuses an empty `glob` and a `path` that climbs out of the workspace;
  passes over a named pipe rather than blocking on it; does not search an entry named like a
  skipped directory, a worktree's `.git` file among them; and treats a file holding a NUL byte as
  binary and does not search it.

## [0.2.0] - 2026-10-20

Wave w2: the process toolset, the contract documents readable without an implementation, the capped
glob walk, and every crate under Apache-2.0. metaharness pins this tag.

### Added

- `process` v1alpha1 (`process.run`) with its conformance suite (31 scenarios), and
  `b10x-tool-process`, harness 0.14.1's local process runner moved here: an argv and never a
  command line, a declared program set, an environment cleared to an allow-list, a caller's
  `timeout_ms` capped at 600000 and by what the run has left, the whole process group killed at the
  bound, and each stream kept at both ends with the omitted byte count. It declares
  `effects: [process, write, network, secret]` and every consequence the profile lists, because it
  confines nothing beyond the environment, time and output.
- `tool_contract::documents` in `b10x-tool-contract`: every `contracts/<toolset>/<version>/contract.yaml`
  embedded at build time (`all`, `get`, `Document::contract`, one constant per document), so an
  adopter reads command names and input schemas without depending on an implementation crate.
  `b10x-tool-fs`, `b10x-tool-code-editor` and `b10x-tool-process` take `CONTRACT_YAML` from it.

### Fixed

- Against harness 0.14.1's runner, `process.run` ends at its bound and kills the process group
  even after the program itself has exited, where a background descendant holding the output kept
  the call open with `timed_out: false`; a descendant that left the group is waited for at most
  500 ms after the kill; a cut moves only for a character it broke.

### Changed

- `b10x-tool-fs`, `b10x-tool-code-editor` and `b10x-tool-toolchain` are Apache-2.0, like every
  other crate in the repository; 0.1.0 released them as `LicenseRef-B10x-Proprietary`.
- `fs.glob` stops walking once `max_results` matching paths are answered (4 entries visited instead
  of 211 on a 1,110-entry tree with `max_results: 1`). `paths` and `truncated` are unchanged; a
  capped glob that stops before reaching a directory past the depth bound now answers
  `depth_bound_reached: false`, where 0.1.0 answered `true`.

## [0.1.0] - 2026-10-20

The first release: ADR 0095 phase T1, seeded from harness 0.13.3 under ADR 0097. harness and
metaharness pin this tag.

### Added

- The `tool-contract/1` profile (`contracts/profile/tool-contract-1.md`): effects, consequences,
  risk with a floor set by effects, idempotency, preconditions, determinism, bounds with a required
  maximum, paging, requires, subjects, placement, capability, output marks and presentation, plus
  the `tool-conformance/1` scenario format.
- `b10x-tool-contract`: the document parser, the validator (every refusal names the operation and
  the property), the ESS type grammar and the shape check against `schemars`-derived schemas, the
  conformance runner (symlink-faithful fixtures, coverage over the behavioural properties, outcome
  and error-field checks), the mutants helper and the `tool-contract validate` binary.
- `fs` v1alpha1 (`fs.read`, `fs.list`, `fs.glob`, `fs.stat`) with its conformance suite, and
  `b10x-tool-fs`, harness 0.13.3's local reader moved here with a whole-file digest on reads.
- `code-editor` v1alpha1 (`code.write`, `code.edit`, `code.patch`, `code.move`, `code.delete`) with
  its conformance suite, and `b10x-tool-code-editor`, harness 0.13.3's local writer moved here with
  `expected_sha256` preconditions, unified diffs and patches resolved against the original.
- `b10x-tool-toolchain`, harness 0.13.3's declarative toolchain registry moved here, deriving
  `build-test` operations under the profile.
