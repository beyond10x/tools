# `search` v1alpha2: what each operation guarantees

The contract is [`contract.yaml`](contract.yaml); this document states, in prose that cites its
field names, what a conforming implementation guarantees. The suite under [`scenarios/`](scenarios)
holds each guarantee to at least one scenario.

## Paths and the boundary

`search.grep` takes a `path` relative to one workspace root (default the root itself), and `path`
is its subject (`subjects`, kind `path`). A path is resolved by where it lands, not by how it is
spelled, under the rule the `fs` toolset states: links are followed and `..` is applied before the
result is checked against the root. A path that lands outside the root is refused as
`search.OutsideWorkspace` with nothing read, whether it climbs out (`..`, `../x`), starts at the
filesystem root (`/`), or goes through a link whose target is outside; a path that climbs out is
refused that way even when nothing exists where it points. A path that lands inside the root and
does not exist is `search.NotFound`, and so is a path through a file (`a.txt/x`). `path` may name a
directory, which is walked, or a file, which is searched alone and answered under its
workspace-relative name after links were followed.

The operation has `effects: [read]` and writes nothing, so the tree after a call is the tree before
it; it is `idempotent` and `deterministic`: the same call over the same tree answers the same
output, because the walk visits each directory's entries in name order. An input that is not the
operation's input (a missing `pattern`, a value of the wrong type, an unknown field) is
`search.InvalidInput`; the suite holds the missing field and the wrong type, and the unknown field
is held by the crate's own tests, because the `tool-conformance/1` runner refuses a `call.input`
key the operation does not declare before any scenario runs. Every failure is a result, never a
transport error (`output.failure_as_result`), and every output is content the caller did not write
(`output.untrusted`).

## The walk

The files searched are the ones the `fs` toolset's `fs.glob` walks, depth first, each directory's
entries in name order, less the files named below. An entry named `.git`, `target`,
`node_modules`, `.venv` or `__pycache__` is skipped, a directory or a file: a directory so named
is not descended into and nothing under it is searched, as `fs.glob` skips it, and a file so named
(a worktree's `.git` file, a link of that name) is not searched either, where `fs.glob` would
answer it. A `path` that names such an entry is searched, because it was asked for by name: a
search with `path: target` searches what is under `target`. Every entry is re-checked against the
root: a link
that leads out of the workspace is not descended into, and a linked file outside is not read; a
link that lands inside is followed and its matches are answered under the link's own path, and a
link that leads nowhere is passed over. The walk stops descending at depth 12, and
`depth_bound_reached: true` says it did somewhere; files below that depth are not searched. A
`path` that names a file is searched alone.

## The deadline

v1alpha2 is v1alpha1 with this section added. `search.grep` takes `timeout_ms`, the wall-clock
bound of the call in milliseconds (`bounds.timeout_ms`): a call that names none gets 10000, and one
above 60000 is refused as `search.InvalidInput` with nothing read. The pattern, the glob and the
path are judged first, so a call that would be refused is refused whatever its deadline. The
deadline is then checked when the walk starts, before every entry the walk takes and before every
line of a file being searched, so `timeout_ms: 0` always stops before any file is read and a slow
search stops near its bound. At the deadline the search answers the matches it had found and
`deadline_reached` is `true`, `truncated` is `true` because a match may lie past where it stopped,
and `note` says the deadline was reached (after the context cap's words when both apply);
`skipped_large_files` and `skipped_large_paths` count the files met by then. On every other answer
`deadline_reached` is `false`.

## `search.grep`

**The pattern.** `pattern` is a literal substring unless `regex` is `true`, so `a.b` finds the
text `a.b` and not `axb`, and a pattern that would be a broken regular expression (`fn (alpha`) is
searched for as text. With `regex: true` it is a regular expression, compiled once before anything
is read, within a size limit of 1 MiB for the compiled program and for its lazy DFA; one that does
not compile, or does not fit the limit, is refused as `search.InvalidPattern` naming the
`pattern`, with the compiler's own words in the message, rather than answered with no matches. An
empty `pattern` is refused the same way. The reply echoes `pattern`, and carries `regex: true`
when the pattern was read as a regular expression; `regex` is absent from a literal search.

**The glob.** `glob` narrows which files are read, by the rule `fs.glob` matches with: a `glob`
with no `/` matches the **name** of a file anywhere under `path`, so `*.rs` is every Rust file; a
`glob` with a `/` matches the whole workspace-relative path, and `*` does not cross a separator, so
`src/**` is every file under `src/`. An empty `glob` is refused as `search.InvalidGlob`, as is one
that does not compile, with the compiler's words in the message. The reply echoes `glob` when one
narrowed the search and leaves it absent otherwise.

**Lines and matches.** A file is searched line by line; a line ends at a newline, and a `\r`
before the newline is not part of the line. Each line that contains the pattern (or matches the
expression) is one entry of `matches`, in walk order and then in line order: its `path`, its
`line` number counted from 1, its `text`, and `line_truncated`. `text` is the line cut at 400
characters on a character boundary, and `line_truncated` is `true` exactly when the line is longer
than that; it is always present, so its absence never has to be read. A binary file of at most
1 MiB is not searched and is not reported, whether the walk reaches it or `path` names it: a file
is binary when it is not UTF-8, or when it holds a NUL byte anywhere, even as valid UTF-8; other
control characters do not make it binary. A binary file over 1 MiB is counted and named as too
large, like any file over 1 MiB, because its size is checked before it is read. Neither is an entry that is not a regular file, such as a named
pipe, which is never opened.

**Context.** `context` asks for that many lines either side of each match (default 0). When it is
more than 0, every match carries `before` and `after`: the lines before and after it, at most
`context` of each, in file order, each a `search.Line` with its own `line` number, its `text` cut
at 400 characters and its own `line_truncated`; at the start or end of a file the list holds the
lines there are, and may be empty. When `context` is 0 or absent, `before` and `after` are absent.
`context` is capped at 5 (`bounds`): a larger ask is answered with 5 lines either side, and the
reply then carries `context: 5` and a `note` that names the figure that was asked for. A `context`
of at most 5 is not echoed, and `note` is absent.

**The result cap.** At most 200 matches are answered, fewer when `max_results` says so; a larger
`max_results` is capped at 200 (`bounds`, `paging.limit`). `truncated: true` says a match past that
count exists: the search stops at the first match it cannot answer. Exactly `max_results` matches
is a complete answer with `truncated: false`, whatever lines and files that do not match follow
them. `max_results: 0` answers no match, and `truncated` says whether there was one to answer.

**Files too large to search.** A file of more than 1 MiB (1,048,576 bytes, by its metadata) is
not read (`bounds`). It is counted in `skipped_large_files`, the first 20 of them are named by
workspace-relative path in `skipped_large_paths`, and `truncated` is `true`, because a match may be
in it. A file of exactly 1 MiB is searched. Both fields are always present: `0` and `[]` when no
file was too large. Files met after the search stopped at the result cap are not counted.
