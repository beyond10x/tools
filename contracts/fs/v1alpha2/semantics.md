# `fs` v1alpha2: what each operation guarantees

The contract is [`contract.yaml`](contract.yaml); this document states, in prose that cites its
field names, what a conforming implementation guarantees. The suite under [`scenarios/`](scenarios)
holds each guarantee to at least one scenario.

## Paths and the boundary

Every operation takes a `path` relative to one workspace root, and every `path` is a subject
(`subjects`, kind `path`). A path is resolved by where it lands, not by how it is spelled: links
are followed and `..` is applied before the result is checked against the root. A path that lands
outside the root is refused as `fs.OutsideWorkspace` with nothing read, whether it climbs out
(`../x`), starts at the filesystem root (`/etc/x`), or goes through a link whose target is outside;
a path that climbs out is refused that way even when nothing exists where it points. A path that
lands inside the root and does not exist is `fs.NotFound`; so is a path through a file
(`a.txt/x`), and so is a link that leads nowhere. A link that lands inside the root is followed, and a file or directory reached
through it is answered under the target's own workspace-relative name.

The workspace root itself is the empty relative path: `fs.list` of the root answers `path: ""`.

Every operation has `effects: [read]` and writes nothing, so the tree after a call is the tree
before it; every operation is `idempotent` and `deterministic`: the same call over the same tree
answers the same output. An input that is not the operation's input (a missing field, an unknown
one, a value of the wrong type) is `fs.InvalidInput`; the suite holds the missing field and the
wrong type, and the unknown field is held by the crate's own tests, because the
`tool-conformance/1` runner refuses a `call.input` key the operation does not declare before any
scenario runs. Every failure is a result, never a transport
error (`output.failure_as_result`), and every output is content the caller did not write
(`output.untrusted`).

## The deadline

v1alpha2 is v1alpha1 with this section added. Every operation takes `timeout_ms`, the wall-clock
bound of the call in milliseconds (`bounds.timeout_ms`): a call that names none gets 10000, and one
above 60000 is refused as `fs.InvalidInput` with nothing read. The deadline is checked once before
the operation reads anything (a directory's entries, a file's bytes, the walk's first entry) and
again in every loop that reads: per line of a read's scan, per entry of a listing and of the walk,
per chunk of `fs.stat`'s digest; so `timeout_ms: 0` always stops before any work, and a slow call
stops near its bound. Path resolution and every refusal come first, so a call that would be refused
is refused whatever its deadline. At the deadline the operation answers what it had done and
`deadline_reached` is `true`; on every other answer it is `false`. `fs.read` answers the lines it had
read, a line it was in the middle of cut and listed in `truncated_lines`; `lines.total` and `sha256`
are `null`, `lines_counted_to` names the last line the scan reached, `note` says the deadline was
reached, `truncated` is `true`, and an `offset` past the lines reached answers an empty window rather
than a refusal, because the end of the file was never found. `fs.list` answers the entries it had
read, in name order but not necessarily the first in name order, with `truncated: true`. `fs.glob`
answers the matches it had found, with `truncated: true`. `fs.stat` answers `kind` and `bytes` and
leaves `sha256` `null`.

## `fs.read`

Answers one window of one regular file as numbered lines. `text` is the window in the form of
`cat -n`: a right-aligned line number, a tab, the line, a newline, so a line can be quoted back to
an editing operation by stripping its number and a reader can say *which* occurrence is meant. A
`\r` before a newline is not part of the line and is dropped.

`lines` says which lines `text` holds: `from` is `offset` (default 1), `to` the last line
answered, `total` how many lines the file has. A window is therefore never mistaken for the file.
An empty file read from line 1 answers `text: ""` and `lines: {from: 1, to: 0, total: 0}`.

The window closes on whichever bound arrives first: `limit` lines, or the byte ceiling `max_bytes`
(default 65536, capped at 262144; `bounds.output_bytes`). The first line is always answered even
when it alone is over the ceiling, cut to fit. A line over 2000 characters is cut on a character
boundary and its number is listed in `truncated_lines`, which is always present and empty when
nothing was cut; no marker is put in `text`.

`truncated` is `true` when the window stopped before the file's last line, when a line in it was
cut, or when the line count stopped at its bound. It is `false` for a window that reached the last
line whole, whatever `offset` was: a window in the middle that ends on the last line is
`truncated: false` even though the lines before `offset` were never in it.

`sha256` is the SHA-256 of the **whole file**, not of the window (`paging.digest`), as 64 lowercase
hex digits, computed in the same pass that counts the lines. A window in the middle of a file
carries the same `sha256` as a read of the whole file.

Counting lines means reading past the window to the end. The scan is bounded in bytes read, not
in lines: it stops once more than 16 MiB (16,777,216 bytes) of the file have been read, whatever
line it is in, so a file of at most 16 MiB is counted and digested whole and a file over 16 MiB
is never counted whole, a single-line one included; past the bound `lines.total` and `sha256` are
`null`, `lines_counted_to` names the last line the scan reached,
`note` says why, and `truncated` is `true`. On every other read `lines_counted_to` and `note` are
absent. `bytes` is the file's size from its metadata and is answered whatever the scan did.

`offset: 0` names no line and is refused as `fs.InvalidWindow`; so is `limit: 0`, which would
otherwise answer `to` one below `from` on any file; so is an `offset` past the last
line (or past `lines_counted_to` when the count stopped), with nothing read. The path is judged
before the window: a bad window on a path that is absent, not a file or outside the workspace is
refused for the path. A directory is `fs.NotAFile`.

## `fs.list`

Answers the entries of one directory (`path`, default the root) in name order, at most 500
(`truncated: true` when the directory holds more). Each entry carries its own `name`, its `kind`
by its own type, and `bytes` for a file (`null` for anything else).

A symlink is listed as `kind: symlink` and is never followed: a link to a directory is neither a
file nor a directory here, and a link whose target is outside the workspace is still just an entry
of this listing. `fs.list` of a path that lands outside the workspace is refused, and `fs.list` of
a file is `fs.NotADirectory`.

## `fs.glob`

Answers the workspace-relative paths of the files under `path` (default the root) that match
`glob`, in walk order (entries in name order, depth first). A `glob` with no `/` matches the
**name** of a file anywhere under `path`: `*.rs` is every Rust file. A `glob` with a `/` matches
the whole workspace-relative path, and `*` does not cross a separator: `crates/**/*.rs`. An empty
`glob` is refused as `fs.InvalidGlob`, as is one that does not compile, with the compiler's words
in the message.

The walk skips the directories `.git`, `target`, `node_modules`, `.venv` and `__pycache__`, and
re-checks every entry against the root: a link that leads out of the workspace is not descended
into and a linked file outside is not answered, while a link that lands inside is answered under
the link's own path. The walk stops descending at depth 12; `depth_bound_reached: true` says it
did somewhere, and a match below that depth is not in `paths`. At most 500 paths are answered,
fewer when `max_results` says so (`paging.limit`); `truncated: true` says a matching path was
found past that count. Exactly `max_results` matches is a complete answer with
`truncated: false`, whatever files the glob did not name follow them.

## `fs.stat`

Answers one entry's `kind` by its own type, `bytes` for a file (`null` otherwise) and `sha256`
for a file of at most 16 MiB (16,777,216 bytes; `null` for a larger file and for anything that is
not a file), without
reading it as text. A symlink is `kind: symlink`, answered under its own `path`, whatever it points
at; a link that leads out of the workspace is `fs.OutsideWorkspace`. A file or directory reached
through a link that lands inside is answered under the target's `path`. The root is a directory
with `path: ""`.
