# `process` v1alpha1: what each operation guarantees

The contract is [`contract.yaml`](contract.yaml); this document states, in prose that cites its
field names, what a conforming implementation guarantees. The suite under [`scenarios/`](scenarios)
holds each guarantee to at least one scenario.

## `process.run`

**An argv, never a command line.** `argv` is the program and then its arguments, one list item
each, and it is the operation's subject (`subjects`, kind `argv`; the program is `argv[0]`). The
items reach the program exactly as given: nothing is split on spaces, quoted, globbed, expanded,
substituted or redirected, so `argv: [printf, "%s", "a; touch x"]` prints `a; touch x` and creates
nothing. A caller that wants a pipe or a redirect names a declared shell as the program and hands
it the script as an argument (`argv: [sh, -c, "…"]`); what that shell then does is the shell's.
An `argv` that is not a list of strings is `process.InvalidInput`, an empty one is
`process.EmptyArgv`, and in both cases nothing was started.

**The declared program set.** The implementation is configured with a set of programs it may
start (`requires.settings: [programs]`), possibly empty. `argv[0]` is compared with each declared
program **as text**, before any search path is consulted: when `sh` is declared, `/bin/sh` is not.
A program outside the set is refused as `process.ProgramNotDeclared`, whose fields name the
`program` that was asked for and the whole `declared` set in the order it was declared, and nothing
was started: no process, and no effect of one. A declared program that cannot be started, because
it is not on the search path the child gets or is not executable, is `process.SpawnFailed` naming
the `program`. A workspace root that cannot be opened, can no longer be entered (its search
permission was taken away) or is no longer a directory is `process.WorkspaceUnavailable` naming its
`path`. It is checked before a start, and asked again when a start fails, so a removed or closed
workspace does not read as a missing program.

**Where and with what it runs.** The program runs with the workspace root as its working directory
and with no standard input: a read of it is at end of file at once, so a program that waits for
input does not hold the call open. Its environment is cleared and then given, by name, only the
variables on an allow-list that are set in the caller's own environment: `PATH`, `HOME`, `LANG`,
`LC_ALL`, `TERM`, `TMPDIR`, `CARGO_HOME`, `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN`, `CARGO_TARGET_DIR`,
`SSL_CERT_FILE` and `SSL_CERT_DIR`, and any further names the implementation was configured to let
through. Each is a path or a locale and none carries a credential; a proxy URL can carry one, so the
proxy variables are not on the list. A credential held in the caller's environment therefore does
not reach a program whose arguments somebody else chose. A shell started this way may add its own
bookkeeping variables (`PWD`, `OLDPWD`, `SHLVL`, `_`) for the programs it starts in turn.

**What is not confined.** The runner clears the environment and bounds the run's time and output;
it confines nothing else. The program runs as the caller's own user with the caller's own access,
inside the workspace root and outside it: it may write and delete files, reach the network, and
read credentials from files such as those under `HOME`. The operation therefore declares every
effect that allows (`effects: [process, write, network, secret]`, `risk: high`), and every
consequence the profile names (`consequences: [money, delete, send-external, irreversible,
sensitive-data]`): the lists are the upper bound of an unconfined runner, not a claim that any
one run has them. What a declared
program actually does is the program's; choosing the declared set is choosing it. An
implementation that confines the program further is a different contract version or a narrower
toolset, not this operation.

**The bound and the group kill.** `timeout_ms` bounds the run's wall-clock time
(`bounds.timeout_ms`): a call that names none gets 600000, a larger ask is capped at 600000, and an
implementation that knows how much of its caller's run is left caps it further at that. The output
field `timeout_ms` answers the bound the run was actually given, after every cap.

The run lasts until the program has ended **and** both of its streams have ended, that is until
every process holding the write end of `stdout` or `stderr` has closed it; a shell that started
`sleep` in the background and exited is still running while that `sleep` holds its output. The
program is started in a process group of its own. When the bound is reached first, the **whole
group** is killed, whether or not the program itself has already ended, so a descendant in the
group, such as `sleep` started in the background by a shell, does not outlive the call or hold it
open. The run then waits briefly, at most half a second, for the streams to end and the group to
be gone, and answers `timed_out: true` with whatever the streams held by then. `exit` is `null`
when the program itself was killed, and the program's own code when it had already ended by itself
and only its descendants were killed. A run that finishes inside the bound answers
`timed_out: false`.

A process that left the program's group (a shell's job control, `set -m`, starts each job in a
group of its own, and `setsid` starts a session) is **not** killed at the bound: the runner kills
one group and nothing else. It is no longer waited for either: when the half second after the
kill is over, the run answers with what was read so far, and that process may live on and keep
writing into a stream nobody reads.

Every outcome is decided by the program and the machine and not by the input
alone (`determinism: nondeterministic`), and running the same `argv` twice runs the program twice,
with whatever effects it has twice (`idempotency: non-idempotent`).

**Both ends kept, and the omitted count.** Each stream, `stdout` and `stderr`, answers at most
65536 bytes of what the program wrote (`bounds.output_bytes`): its first 32768 bytes and its last
32768, with a marker between them, `\n… N bytes omitted here …\n`, naming how many bytes fell in
between. The end of a stream is kept because that is where a test suite's verdict is. A stream of
at most 65536 bytes is answered whole, with no marker. Both cuts land between UTF-8 characters,
so neither end opens or closes on a broken character, and the bytes a cut moves to stay whole are
counted in `N` too. Only a character the cut itself broke moves it, and whether it broke one is
decided by the bytes on both sides of the cut: the end's last (or first) bytes and the stream's
bytes just past the cut must together make one valid character. Bytes that are not UTF-8,
anywhere in an end and at its cut, are the stream's own and are answered, not dropped.
`omitted_bytes` is the sum of both streams' `N`, and `truncated` is `true`
exactly when it is above 0. The stream is read to its end whatever is kept, so a program is never
blocked on a full pipe by the cap. Bytes that are not UTF-8 are answered with the replacement
character.

**A failed program is a result.** A program that exits non-zero answers `outcome: ok` with its
`exit` code and both streams (`output.failure_as_result`); the error outcomes are reserved for a
run that did not start, or a started program the system then refused to report on
(`process.WaitFailed`, after its process group was killed). Every output is content the caller did
not write (`output.untrusted`).

The suite runs against an implementation whose declared set is exactly `sh`, `printf` and
`b10x-conformance-absent-program`, in that order, with nothing added to the allow-list, on a
machine where `sh` (with `kill` and `wait`), `printf`, `sleep` (taking a fraction of a second),
`env` and `touch` are on the search path and
`b10x-conformance-absent-program` is not; and in a caller's environment that holds at least one
variable off the allow-list, as every test process a build tool starts does.
