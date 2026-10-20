# `build-test` v1alpha1: what each operation guarantees

The contract is [`contract.yaml`](contract.yaml); this document states, in prose that cites its
field names, what a conforming implementation guarantees. The suite under [`scenarios/`](scenarios)
holds each guarantee to at least one scenario, against the fixture workspaces under
[`fixtures/`](fixtures): a Cargo workspace, a Go module, a Taskfile, an npm project and a yarn
project.

## The operations and where they come from

Each operation is one tool of one of five toolchain providers: `rust`, `go`, `taskfile`, `npm`
and `yarn`. A provider is a declaration of the programs it may start (`requires.binaries`), the
environment and installation roots a confined run of it is given (`requires.env`,
`requires.settings`), the files that mark a workspace as its project, and its tools. A tool
compiles its typed `input` into one or more fixed argv, each starting with one of its provider's
programs; nothing a caller passes becomes a program, a flag that was not declared, or text a
shell takes apart. The operation's name is `build-test.<tool>`.

The document is the operations the five providers derive, frozen: an implementation crate holds
the two equal, operation for operation, so that a provider changed without a new contract version
fails its tests. The one difference is an enum parameter (`taskfile_run.task`, `npm_run.script`,
`yarn_run.script`): its values are the workspace's own (the Taskfile's public tasks, the
`package.json` scripts), read when the workspace is resolved, so the document declares the
`String` they are drawn from and says where they come from.

**Which operations a workspace offers.** A provider is active in a workspace whose project
markers it matches (`Cargo.toml`; `go.mod` or `go.work`; a Taskfile; `package.json` without
`yarn.lock`; `package.json` with `yarn.lock`), and an implementation offers the operations of the
active providers. A call to an operation of a provider the workspace does not activate is
`build-test.Refused` and starts nothing.

## Arguments

An argument outside the operation's `input` is refused, as is a required one that is missing,
one of the wrong type, a list with more items than its description allows, an enum value the
workspace does not declare, and a path parameter that is absolute or climbs out of the workspace
(`../x`). A path parameter says so in its description (``"paths relative to the workspace root,
each refused when it leaves the workspace"``), so a provider whose path became a plain string
derives a different operation.

**No argument becomes an option.** A string, a path, an enum value or a list item that begins
with `-` is refused, whatever position it would take in the argv, because the program would read
it as an option nobody declared: `-w` as a `go_fmt_check` path would turn a listing into a
rewrite, `-n` as a `go_build` package into a dry run, a Taskfile task named `--dry` into
`task --dry`. The one exception is a list the tool places only after a declared separator
(`args`, after `--`), where a leading `-` is the program's own argument: `taskfile_run` with
`args: ["-x"]` runs `task … echo-args -- -x`.

Each refusal is `build-test.Refused`, whose one field, `message`, is a sentence naming the
argument and the tool (``"`bin` is required by `rust_run`"``, ``"`task` is not valid for
`taskfile_run`"``, ``"`../outside.rs` is not a contained relative path"``, ``"`package` is
`--frozen`, which begins with `-` and would reach `rust_check`'s program as an option"``), and
nothing was started. Every path argument, read or written, is also judged where it lands before
anything starts, and refused when it lands outside the workspace or through a link that leads
nowhere (below); the write scope is asked only about the paths a call writes.

## Running

Each argv starts with its provider's program in the workspace root, with no standard input and
an environment cleared to the process runner's allow-list (`PATH`, `HOME`, `LANG`, `LC_ALL`,
`TERM`, `TMPDIR`, `CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, `CARGO_TARGET_DIR`,
`SSL_CERT_FILE`, `SSL_CERT_DIR`), in a process group of its own. The program is compared, as
text, with the declared program set: every program the active providers declare. A provider's
command can name no other program (a provider that does is not loaded), so the set is the
runner's own second check; an implementation that hands its runner none starts nothing.

**What is not confined.** The program runs on the caller's own machine as the caller's own
user (`placement: local`). The runner clears the environment to the allow-list above, bounds the
run's time and output, and judges the paths a call names; it confines nothing else.
The program reads the caller's `HOME` and toolchain configuration (`~/.cargo`, `~/.config/go`,
`~/.npmrc`), may write anywhere that user can (build caches under `HOME` among them), may reach
the network and may read credentials from files. So every operation declares every effect such
a runner allows (`effects: [process, write, network, secret]`, `risk: high`) and every
consequence the profile names (`consequences: [money, delete, send-external, irreversible,
sensitive-data]`): the upper bound of an unconfined runner, as for `process.run`, not a claim
that any one tool has them. The provider's sandbox environment and roots (`requires.env`,
`requires.settings`) are what a confined implementation, a narrower contract, would give the
run; this one does not apply them, and the fixtures carry what keeps their runs offline
themselves (a `--locked --offline` Cargo build with a `Cargo.lock`, a Go module with no
dependencies, an `.npmrc` with `offline=true`).

**Paths, where they land, and the write scope.** A tool that writes named paths declares
them: `go_fmt` and `rust_fmt` write exactly the files `paths` names (`subjects`, kind `path`), and
leave every other file as it was. Before any process of a call starts, every value of a path
parameter, read (`go_fmt_check.paths`) or written, is resolved to where it lands: the deepest
part of it that exists, a link included, is followed. A path that lands outside the workspace
(``"`up/escape.go` lands outside the workspace; nothing was started"``, and the same for
`go_fmt_check` on `up/outside.go`), or goes through a link that leads nowhere, refuses the whole
call as `build-test.Refused`, with or without a write scope. An implementation that is given a
write scope asks it about each written path as spelled and, when it lands elsewhere, where it
lands; a path the scope refuses either way refuses the whole call, with the scope's own sentence
(prefixed ``"`alias.go` leads to `protected/kept.go`, and "`` for a landing). Nothing was
started, so no file changed. With no scope, every path that lands inside the workspace may be
written.

**The bound, and the time left.** Each process runs under the wall-clock bound
(`bounds.timeout_ms`, 600000) and the output bound (`bounds.output_bytes`, 65536 per stream), as
the `process` toolset's `process.run` does: the whole process group is killed at the bound, the
answer says `timed_out: true` with `exit: null` when the program itself was killed, and each
stream keeps its first and last 32768 bytes with a marker, `\n… N bytes omitted here …\n`, naming
the bytes dropped between them, `omitted_bytes` their sum and `truncated: true`. An
implementation that knows how much of its caller's run is left takes a deadline when the call
starts and gives each process what is left of it, so a call that starts several processes ends
by the deadline as a whole; `timeout_ms` in each answer is the bound that process was given. No
process starts once nothing is left: a call whose deadline has passed before its first process
is `build-test.Refused` and starts nothing, and a call whose deadline passes partway answers the
runs that started, the last of them with `timed_out: true`, and starts no other.

## The answer

**One process answers one run.** An operation whose plan starts exactly one process answers the
eight fields of `toolchain.Run`: the `argv` that was started, the `exit` status, `stdout`,
`stderr`, `truncated`, `omitted_bytes`, `timed_out` and `timeout_ms`.

**An operation whose plan can start several processes answers a list.** `rust_fmt` starts one
`rustfmt` per named path, and answers `runs`, one `toolchain.Run` per process in the order they
ran, **including when it started one**. The answer's shape is decided by the operation, never by
how many processes one call happened to start. (`go_fmt` and `go_fmt_check` pass every path to
one `gofmt`, so they start one process and answer one run.) An answer that is an object for one
process and an array for several, decided at run time, cannot be declared in a field list; an
adopter that wants that shape unwraps a `runs` list of one.

**A failed program is a result.** A build that fails, a test that fails, a format check that
finds a difference: each answers `outcome: ok` with the program's own `exit` and output
(`output.failure_as_result`); `cargo fmt --check` exits 1 with the difference on standard output,
and that is a run. Every output is content the caller did not write (`output.untrusted`).

**The formatting check.** `go_fmt_check` lists the named files `gofmt` would change and exits 0
either way, so its result is read from standard output: when the program writes anything but
whitespace there, the call is `build-test.CheckFailed`, whose `stdout` field is what it wrote,
and whose message is `formatting check failed: ` followed by that output as a JSON string. No
further process of the call starts after it.

**Neither idempotent nor deterministic.** A second call runs the programs again, with whatever
effects they have (`idempotency: non-idempotent`), and what a program answers is decided by the
workspace, the toolchain and the machine when it runs (`determinism: nondeterministic`). What a
tool itself writes is its own: build products and caches, formatted files; `go_fmt_check` and
`rust_fmt_check` are meant to write nothing, but run unconfined they declare what every other
operation does.

## The suite's configuration

The suite runs against an implementation over the five providers as this machine resolves them
in each scenario's workspace, with a write scope that refuses every path whose first component is
`protected`, with the sentence ``"`<path>` is outside what this run may change"``, and with no
time left to share except in `a-run-gets-only-the-time-its-caller-has-left`, whose caller has
1000 milliseconds left. It needs, on the search path, every program a provider declares: `cargo`,
`rustc` and `rustfmt` from a rustup `stable` toolchain with `clippy` (whose installation root the
`rust` provider resolves), `go` and `gofmt`, `task`, `npm` with `node`, and `yarn` (classic), and
`sh`, `sleep`, `echo`, `yes` and `head` for the Taskfile's tasks. It reaches no network: no
fixture has a dependency to fetch. `CARGO_TARGET_DIR` must be unset, so a Cargo build lands in the
scenario's own workspace. Two fixtures carry a symbolic link (`fixtures/go-links/up` to `..`,
`fixtures/adversary-symlink/alias.go` into `protected/`), which the runner lays out as a link.
