# `code-editor` v1alpha1: semantics

What each operation of [`contract.yaml`](contract.yaml) guarantees, in prose that cites the
contract's field names. The conformance suite under [`scenarios/`](scenarios/) is the evidence;
this document is what the scenarios are evidence *for*.

## Common to every operation

**Paths.** Every `path`, `from` and `to` is relative to the workspace root. A path is judged by
where it *lands* after links are followed, not by how it is spelled: a path that resolves outside
the workspace, whether through `..`, an absolute path or a link that points out, is refused as
`code.OutsideWorkspace`. For a path that does not exist yet, the deepest existing ancestor is
resolved and checked; a `..` left in the part that does not exist is refused rather than appended,
and a link in that ancestry that leads nowhere is `code.IsSymlink`. A link that lands *inside* the
workspace is followed: the operation acts on the target, and the reply's `path` is the target's own
name, workspace-relative. A trailing `/` or `/.` on a path names the same entry as the path without
it (`link/` is `link`), so a link so spelled is still followed to where it lands or refused by name;
the `path` an error carries is the caller's own spelling. A trailing `/` or `/.` on a path that is
**not there** names a directory, which this toolset never creates: `code.write` and a move's `to`
refuse it as `code.NotAFile`, once containment is settled, and create nothing.

**Never created.** A parent directory that does not exist is `code.ParentMissing`; no operation
creates directories.

**Never followed.** A write to a link is a write to its target only when the target lands inside
the workspace; otherwise the link is refused. `code.delete` refuses a link by name, so a link is
never followed to remove what it points at.

**Regular files only.** A path that names a directory is `code.NotAFile` for every operation that
takes one.

**The precondition.** Where the input carries `expected_sha256`, it is the lowercase hex SHA-256
the file must have *before* anything is written. It is compared first; a mismatch is
`code.PreconditionFailed {expected, actual}` with the file unchanged, where `actual` is the digest
the file has, or absent when there is no file. A `code.write` to a path that does not exist with
an `expected_sha256` given is such a mismatch. Without `expected_sha256` the file is not checked.

**Digests in replies.** `sha256` in an output is the digest of the whole file as written.
`bytes` is its length in bytes.

**Nothing half done.** A refusal leaves the file exactly as it was. The only effect an operation
has is the one its success outcome names.

**Input.** An input that does not fit the operation's input fields (a missing field, an unknown
field, a wrong type) is `code.InvalidInput`; the message says which. A file that is not valid
UTF-8 text is `code.NotText {path}` for the operations that look for text in it, with the file
unchanged. An operating-system failure not covered by a named refusal is `code.Io {path}`, with
the system's words in the message.

## `code.write`

`{path, text, expected_sha256?}` → `{path, bytes, sha256}`. The file at `path` holds exactly
`text`, created when absent and replaced when present. `path` in the output is where the bytes
landed. Writing the same `text` twice leaves the same file and answers the same `sha256`
(`idempotency: idempotent`).

## `code.edit`

`{path, old, new, expected_sha256?}` → `{path, bytes, sha256, diff}`. `old` must be non-empty
(`code.EmptyOld`) and appear in the file **exactly once**: nowhere is `code.NoMatch`, more than
once is `code.Ambiguous {index, count}` with `count` the number of occurrences; in both cases the
file is unchanged. Occurrences are counted **overlapping**: `aa` fits at two places in `aaa` and is
ambiguous. The one occurrence becomes `new`. `index` is `0` for `code.edit`; it names the edit for
`code.patch`, which shares these errors. A file that is not UTF-8 text is `code.NotText`.

`diff` is a unified diff of the file before and after: a `---` header and a `+++` header, each
naming the workspace-relative `path`, then hunks of `-` and `+` lines with three lines of context.
`code.edit` always applies its one replacement, so its `diff` always carries the two header lines
and is never empty; when `old` equals `new` the text is the same before and after, and the `diff`
is the two header lines with no hunk, which is how it says that nothing changed. `diff` is for a
reader, a record of what the call did; it is not a patch a consumer applies, and a header-only
`diff` is to be read as "the content did not change", not fed to a patch tool.

The same edit applied a second time finds no `old` and is refused (`idempotency: non-idempotent`).

## `code.patch`

`{path, edits: [{old, new}], expected_sha256?}` → `{path, bytes, sha256, diff, applied}`. Every
edit's `old` is resolved against the **original** text, the file as it is before the patch, and
each must appear there exactly once, under `code.edit`'s rules and errors with `index` naming the
edit. An `old` that would only appear after another edit was applied appears nowhere in the
original and is `code.NoMatch`.

Two edits **overlap** when the ranges of the original their `old`s occupy share at least one byte;
two edits with the same `old` occupy the same range and overlap. Overlap is `code.Overlap {first,
second}` naming the two edits by index, lower first, with the file unchanged. Adjacent ranges that
share no byte do not overlap.

When every edit resolves and none overlap, all are applied from the end of the file backwards, so
the offsets found in the original hold; `applied` is their number. Either every edit applies or
nothing changes. `diff` is as for `code.edit`: headed by the path twice whenever an edit was
applied, with hunks only where the text differs, so a patch whose edits replace text with itself
answers the header lines alone, which means the content did not change; as for `code.edit`, `diff`
is for a reader and not a patch to apply. `diff` is empty exactly when no edit was applied: with
`edits: []` the file is untouched, `applied` is `0` and `diff` is empty.

## `code.move`

`{from, to}` → `{from, to}`. The regular file at `from` is now at `to` and no longer at `from`.
`from` must exist (`code.NotFound`) and be a regular file (`code.NotAFile`); it is resolved as an
existing path, so a link inside the workspace moves its target. `to` is judged for containment
first, by where its own directory lands (`code.OutsideWorkspace`); then nothing may be there, not
a file, a directory or a link, dangling or not (`code.Exists`); then, as a new path, it must not be
spelled as a directory (`new/` is `code.NotAFile`) and its parent directory must exist
(`code.ParentMissing`). A move never replaces what is at `to`. Both paths are
subjects. The same move a second time finds no `from` (`idempotency: non-idempotent`).

## `code.delete`

`{path, expected_sha256?}` → `{path}`. The regular file at `path` is removed. A directory is
`code.NotAFile`; a link is `code.IsSymlink`, refused by the name the caller gave however it is
spelled (`link/` included), whatever it points at, inside or outside or nowhere, before the name is
followed anywhere. The one thing judged first is that the link's own directory lands inside the
workspace: a name whose directory lands outside is `code.OutsideWorkspace`, and nothing is said
about what is there. Deleting a path that is already gone is `code.NotFound` and removes nothing else; the
tree after a repeated delete is the tree after the first (`idempotency: idempotent`).
