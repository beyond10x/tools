//! The `code-editor` toolset: `code.write`, `code.edit`, `code.patch`, `code.move` and
//! `code.delete` over one workspace root, under the `tool-contract/1` profile.
//!
//! This crate is harness's `crates/harness-tools/src/local.rs` writer at `798325f` (harness 0.13.3)
//! moved here: its `open`, `resolve`, `display`, `resolve_new`, `write`, `edit` and `contained`,
//! and their tests, with the same refusals (a symlink target, a missing parent, a path that climbs
//! out, an edit that matches zero or several times, an empty `old`). Added here: the
//! `expected_sha256` precondition on every operation that replaces or removes a file, a unified
//! `diff` in the reply of `code.edit` and `code.patch`, `code.patch` (several edits resolved against
//! the original text, overlaps refused), `code.move` and `code.delete`, named errors in place of
//! strings, and the [`Toolset`] implementation the conformance runner drives. Reading, searching and
//! running stayed behind: the `fs` toolset and T3 own those.
//!
//! Two things differ from harness 0.13.3 on purpose. Occurrences of an edit's `old` are counted
//! overlapping, so `aa` fits at two places in `aaa` and is ambiguous, where harness counted one
//! (`str::matches`) and applied it. And a trailing `/` or `/.` on a path is stripped before any
//! check: on a link to a file it made `symlink_metadata` fail with "not a directory", which read as
//! "not there", and the walk stepped past the link and rebuilt it uncanonicalised, so an edit went
//! through a link to a file outside the workspace.
//!
//! Every path check here is this crate's own: `resolve` canonicalises and then checks containment.
//! That is real and it is not confinement: nothing stops the *process*, only these functions, and a
//! bug in them is the whole boundary. It is the right implementation for a run against the
//! operator's own tree on their own machine.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Toolset};
use tool_contract::digest::sha256_hex;

/// The contract document this crate implements, as text.
pub const CONTRACT_YAML: &str =
    include_str!("../../../contracts/code-editor/v1alpha1/contract.yaml");

/// The input of `code.write`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WriteInput {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The whole text the file is to hold.
    pub text: String,
    /// The digest the file must have before it is replaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
}

/// The output of `code.write`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WriteOutput {
    /// The file the bytes reached, workspace-relative, under its own name.
    pub path: String,
    /// How many bytes the file holds.
    pub bytes: u64,
    /// The digest of the file as written.
    pub sha256: String,
}

/// The input of `code.edit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditInput {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The text to replace; it must appear exactly once.
    pub old: String,
    /// The text that replaces it.
    pub new: String,
    /// The digest the file must have before it is changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
}

/// The output of `code.edit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EditOutput {
    /// The file, workspace-relative, under its own name.
    pub path: String,
    /// How many bytes the file holds now.
    pub bytes: u64,
    /// The digest of the file as written.
    pub sha256: String,
    /// A unified diff of the file before and after, for a reader: two header lines and no hunk
    /// mean the content did not change. Not a patch for a consumer to apply.
    pub diff: String,
}

/// One edit of a `code.patch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    /// The text to replace; it must appear exactly once in the original.
    pub old: String,
    /// The text that replaces it.
    pub new: String,
}

/// The input of `code.patch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchInput {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The edits, each resolved against the original text.
    pub edits: Vec<Edit>,
    /// The digest the file must have before it is changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
}

/// The output of `code.patch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PatchOutput {
    /// The file, workspace-relative, under its own name.
    pub path: String,
    /// How many bytes the file holds now.
    pub bytes: u64,
    /// The digest of the file as written.
    pub sha256: String,
    /// A unified diff of the file before and after, for a reader: two header lines and no hunk
    /// mean the content did not change; empty when no edit was applied. Not a patch to apply.
    pub diff: String,
    /// How many edits were applied.
    pub applied: u64,
}

/// The input of `code.move`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveInput {
    /// The file to move, relative to the workspace root.
    pub from: String,
    /// Where it goes, relative to the workspace root; nothing may be there yet.
    pub to: String,
}

/// The output of `code.move`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoveOutput {
    /// The file that was moved, workspace-relative, under its own name.
    pub from: String,
    /// Where it is now, workspace-relative.
    pub to: String,
}

/// The input of `code.delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteInput {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The digest the file must have before it is removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sha256: Option<String>,
}

/// The output of `code.delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeleteOutput {
    /// The file that was removed, workspace-relative, under its own name.
    pub path: String,
}

/// Why an operation was refused. Each variant is one error the contract declares; [`name`] and
/// [`fields`] give the contract's view of it.
///
/// [`name`]: EditError::name
/// [`fields`]: EditError::fields
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    /// `code.NotFound`: no file at the path.
    #[error("`{path}`: {detail}")]
    NotFound {
        /// The path as the caller named it.
        path: String,
        /// The operating system's own words.
        detail: String,
    },
    /// `code.NotAFile`: the path names something other than a regular file.
    #[error("`{path}` is not a regular file")]
    NotAFile {
        /// The path as the caller named it.
        path: String,
    },
    /// `code.OutsideWorkspace`: the path lands outside the workspace, or climbs out of it.
    #[error("{detail}")]
    OutsideWorkspace {
        /// The path as the caller named it.
        path: String,
        /// Why.
        detail: String,
    },
    /// `code.ParentMissing`: the file's parent directory does not exist and is never created.
    #[error(
        "`{path}` has a parent directory that does not exist; create the directory by an admitted \
         operation before writing the file"
    )]
    ParentMissing {
        /// The path as the caller named it.
        path: String,
    },
    /// `code.IsSymlink`: the path is a link this toolset will not write through or remove.
    #[error("{detail}")]
    IsSymlink {
        /// The path as the caller named it.
        path: String,
        /// Why.
        detail: String,
    },
    /// `code.EmptyOld`: an edit's `old` is empty.
    #[error("`old` must not be empty: an edit has to name where it lands (edit {index})")]
    EmptyOld {
        /// Which edit; `0` for `code.edit`.
        index: usize,
    },
    /// `code.NoMatch`: an edit's `old` appears nowhere in the file.
    #[error("that text appears nowhere in `{path}` (edit {index})")]
    NoMatch {
        /// The path as the caller named it.
        path: String,
        /// Which edit; `0` for `code.edit`.
        index: usize,
    },
    /// `code.Ambiguous`: an edit's `old` appears more than once.
    #[error(
        "that text appears {count} times in `{path}`; include enough surrounding text to name one \
         place (edit {index})"
    )]
    Ambiguous {
        /// The path as the caller named it.
        path: String,
        /// Which edit; `0` for `code.edit`.
        index: usize,
        /// How many times `old` appears.
        count: usize,
    },
    /// `code.Overlap`: two edits of a patch name ranges of the original that meet.
    #[error(
        "edits {first} and {second} of the patch overlap in `{path}`; every edit is resolved \
         against the original text, so no two may touch the same bytes"
    )]
    Overlap {
        /// The path as the caller named it.
        path: String,
        /// The lower index of the two edits.
        first: usize,
        /// The higher index.
        second: usize,
    },
    /// `code.PreconditionFailed`: the file's digest is not `expected_sha256`.
    #[error("`{path}` has digest {}, expected {expected}; nothing was changed", actual.as_deref().unwrap_or("none (no file)"))]
    PreconditionFailed {
        /// The path as the caller named it.
        path: String,
        /// The digest the caller expected.
        expected: String,
        /// The digest the file has; `None` when there is no file.
        actual: Option<String>,
    },
    /// `code.Exists`: a move's `to` is already there.
    #[error("`{path}` exists; a move does not replace what is there")]
    Exists {
        /// The path as the caller named it.
        path: String,
    },
    /// `code.Io`: the operating system refused.
    #[error("`{path}`: {detail}")]
    Io {
        /// The path as the caller named it.
        path: String,
        /// The operating system's own words.
        detail: String,
    },
    /// `code.NotText`: the file is not valid UTF-8 text, so there is no text to find `old` in.
    #[error("`{path}` is not UTF-8 text; an edit needs text to find `old` in")]
    NotText {
        /// The path as the caller named it.
        path: String,
    },
}

impl EditError {
    /// The contract error name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "code.NotFound",
            Self::NotAFile { .. } => "code.NotAFile",
            Self::OutsideWorkspace { .. } => "code.OutsideWorkspace",
            Self::ParentMissing { .. } => "code.ParentMissing",
            Self::IsSymlink { .. } => "code.IsSymlink",
            Self::EmptyOld { .. } => "code.EmptyOld",
            Self::NoMatch { .. } => "code.NoMatch",
            Self::Ambiguous { .. } => "code.Ambiguous",
            Self::Overlap { .. } => "code.Overlap",
            Self::PreconditionFailed { .. } => "code.PreconditionFailed",
            Self::Exists { .. } => "code.Exists",
            Self::Io { .. } => "code.Io",
            Self::NotText { .. } => "code.NotText",
        }
    }

    /// The error's fields, exactly as the contract declares them.
    pub fn fields(&self) -> Value {
        match self {
            Self::NotFound { path, .. }
            | Self::NotAFile { path }
            | Self::OutsideWorkspace { path, .. }
            | Self::ParentMissing { path }
            | Self::IsSymlink { path, .. }
            | Self::Exists { path }
            | Self::Io { path, .. }
            | Self::NotText { path } => json!({ "path": path }),
            Self::EmptyOld { index } | Self::NoMatch { index, .. } => json!({ "index": index }),
            Self::Ambiguous { index, count, .. } => json!({ "index": index, "count": count }),
            Self::Overlap { first, second, .. } => json!({ "first": first, "second": second }),
            Self::PreconditionFailed {
                expected, actual, ..
            } => json!({ "expected": expected, "actual": actual }),
        }
    }
}

/// One property switched off, for the conformance suite to catch.
#[cfg(any(test, feature = "mutants"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutant {
    /// `expected_sha256` is never checked.
    IgnorePrecondition,
    /// A patch applies its edits one after another against the evolving text.
    SequentialPatch,
    /// `diff` is always empty.
    NoDiff,
}

/// The code editor over one workspace root.
#[derive(Debug, Clone)]
pub struct Editor {
    root: PathBuf,
    #[cfg(any(test, feature = "mutants"))]
    mutant: Option<Mutant>,
}

impl Editor {
    /// Opens `root` as the only directory these operations can reach.
    ///
    /// # Errors
    ///
    /// `root` cannot be canonicalised, or is not a directory.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, EditError> {
        Ok(Self {
            root: open(root.as_ref())?,
            #[cfg(any(test, feature = "mutants"))]
            mutant: None,
        })
    }

    /// The same editor with one property switched off.
    ///
    /// # Errors
    ///
    /// As [`Editor::new`].
    #[cfg(any(test, feature = "mutants"))]
    pub fn with_mutant(root: impl AsRef<Path>, mutant: Mutant) -> Result<Self, EditError> {
        Ok(Self {
            root: open(root.as_ref())?,
            mutant: Some(mutant),
        })
    }

    /// The canonical workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where a path lands, workspace-relative: a link's target under the target's own name, a new
    /// path as itself. The same containment rule as a write, so a path a write would refuse is
    /// refused here.
    ///
    /// # Errors
    ///
    /// The path lands outside the workspace.
    pub fn lands(&self, relative: &str) -> Result<String, EditError> {
        self.resolve_new(relative).map(|path| self.display(&path))
    }

    /// `code.write`: the file holds `text`, whole.
    ///
    /// # Errors
    ///
    /// See [`EditError`].
    pub fn write(&self, input: &WriteInput) -> Result<WriteOutput, EditError> {
        let relative = input.path.as_str();
        let spelling = normalise(relative);
        let target = self.resolve_new(relative)?;
        refuse_link(&target, relative, "writes files")?;
        if spelling.as_directory && target.symlink_metadata().is_err() {
            return Err(EditError::NotAFile {
                path: relative.to_owned(),
            });
        }
        if target.parent().is_some_and(|parent| !parent.is_dir()) {
            return Err(EditError::ParentMissing {
                path: relative.to_owned(),
            });
        }
        if target.is_dir() {
            return Err(EditError::NotAFile {
                path: relative.to_owned(),
            });
        }
        if let Some(expected) = input.expected_sha256.as_deref() {
            let current = match fs::read(&target) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(io_error(relative, &error)),
            };
            self.check_digest(relative, Some(expected), current.as_deref())?;
        }
        fs::write(&target, &input.text).map_err(|error| io_error(relative, &error))?;
        Ok(WriteOutput {
            path: self.display(&target),
            bytes: len(input.text.len()),
            sha256: sha256_hex(input.text.as_bytes()),
        })
    }

    /// `code.edit`: exactly one occurrence of `old` becomes `new`.
    ///
    /// # Errors
    ///
    /// See [`EditError`].
    pub fn edit(&self, input: &EditInput) -> Result<EditOutput, EditError> {
        if input.old.is_empty() {
            return Err(EditError::EmptyOld { index: 0 });
        }
        let relative = input.path.as_str();
        let (target, before) = self.read_text(relative, input.expected_sha256.as_deref())?;
        // Exactly one place, or nothing happens. An edit that matched twice would silently change
        // the wrong one, and the caller has no way to find out which.
        let after = replace_once(&before, &input.old, &input.new, relative, 0)?;
        let written = self.replace(&target, relative, &before, &after, 1)?;
        Ok(EditOutput {
            path: written.path,
            bytes: written.bytes,
            sha256: written.sha256,
            diff: written.diff,
        })
    }

    /// `code.patch`: every edit resolved against the original text, then applied together.
    ///
    /// # Errors
    ///
    /// See [`EditError`].
    pub fn patch(&self, input: &PatchInput) -> Result<PatchOutput, EditError> {
        if let Some(index) = input.edits.iter().position(|edit| edit.old.is_empty()) {
            return Err(EditError::EmptyOld { index });
        }
        let relative = input.path.as_str();
        let (target, before) = self.read_text(relative, input.expected_sha256.as_deref())?;
        let after = if self.applies_sequentially() {
            let mut evolving = before.clone();
            for (index, edit) in input.edits.iter().enumerate() {
                evolving = replace_once(&evolving, &edit.old, &edit.new, relative, index)?;
            }
            evolving
        } else {
            apply_against_original(&before, &input.edits, relative)?
        };
        let written = self.replace(&target, relative, &before, &after, input.edits.len())?;
        Ok(PatchOutput {
            path: written.path,
            bytes: written.bytes,
            sha256: written.sha256,
            diff: written.diff,
            applied: len(input.edits.len()),
        })
    }

    /// `code.move`: the regular file at `from` is now at `to`, where nothing was.
    ///
    /// # Errors
    ///
    /// See [`EditError`].
    pub fn rename(&self, input: &MoveInput) -> Result<MoveOutput, EditError> {
        let from = self.resolve(&input.from)?;
        refuse_link(&from, &input.from, "moves files")?;
        if !from.is_file() {
            return Err(EditError::NotAFile {
                path: input.from.clone(),
            });
        }
        let to = self.resolve_destination(&input.to)?;
        if to.parent().is_some_and(|parent| !parent.is_dir()) {
            return Err(EditError::ParentMissing {
                path: input.to.clone(),
            });
        }
        fs::rename(&from, &to).map_err(|error| io_error(&input.from, &error))?;
        Ok(MoveOutput {
            from: self.display(&from),
            to: self.display(&to),
        })
    }

    /// `code.delete`: the regular file at `path` is removed. A link is refused by name, so a link
    /// is never followed to remove its target.
    ///
    /// # Errors
    ///
    /// See [`EditError`].
    pub fn delete(&self, input: &DeleteInput) -> Result<DeleteOutput, EditError> {
        let relative = input.path.as_str();
        let spelled = self.root.join(normalise(relative).path);
        // By the caller's own name first, however spelled and whatever it points at, before the
        // name is followed anywhere, so a link is never followed to remove its target. Only the
        // name's own directory is judged before that, so nothing is said about a link outside.
        if spelled
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            self.own_directory_inside(relative, &spelled)?;
            refuse_link(&spelled, relative, "removes files")?;
        }
        let target = self.resolve(relative)?;
        refuse_link(&target, relative, "removes files")?;
        if !target.is_file() {
            return Err(EditError::NotAFile {
                path: relative.to_owned(),
            });
        }
        if let Some(expected) = input.expected_sha256.as_deref() {
            let bytes = fs::read(&target).map_err(|error| io_error(relative, &error))?;
            self.check_digest(relative, Some(expected), Some(&bytes))?;
        }
        fs::remove_file(&target).map_err(|error| io_error(relative, &error))?;
        Ok(DeleteOutput {
            path: self.display(&target),
        })
    }

    /// Resolves a caller-supplied path to an existing entry inside the workspace.
    ///
    /// The whole candidate is canonicalised, as harness's `resolve` did, before the containment
    /// check, so a symlink or a `..` that leaves the workspace is refused by where it *lands*, not
    /// by how it is spelled. A candidate that is not there goes through
    /// [`resolve_new`](Self::resolve_new)'s containment check first, so a path that climbs out to
    /// a place that does not exist is refused as outside rather than as not found; only a
    /// contained path that is not there is `code.NotFound`.
    fn resolve(&self, relative: &str) -> Result<PathBuf, EditError> {
        let candidate = self.root.join(normalise(relative).path);
        if let Err(error) = candidate.symlink_metadata() {
            self.resolve_new(relative)?;
            return Err(EditError::NotFound {
                path: relative.to_owned(),
                detail: error.to_string(),
            });
        }
        let canonical = candidate
            .canonicalize()
            .map_err(|error| self.leads_nowhere(relative, &candidate, &error))?;
        if !canonical.starts_with(&self.root) {
            return Err(outside(relative));
        }
        Ok(canonical)
    }

    /// Resolves a move's `to`: containment first, by where its own directory lands, so nothing
    /// about the outside is answered; then nothing may be there, a link, dangling or not,
    /// included; then as a path to create, which a trailing `/` cannot name.
    fn resolve_destination(&self, relative: &str) -> Result<PathBuf, EditError> {
        let spelling = normalise(relative);
        let candidate = self.root.join(spelling.path);
        if candidate.symlink_metadata().is_ok() {
            self.own_directory_inside(relative, &candidate)?;
            return Err(EditError::Exists {
                path: relative.to_owned(),
            });
        }
        let to = self.resolve_new(relative)?;
        if to.symlink_metadata().is_ok() {
            return Err(EditError::Exists {
                path: relative.to_owned(),
            });
        }
        if spelling.as_directory {
            return Err(EditError::NotAFile {
                path: relative.to_owned(),
            });
        }
        Ok(to)
    }

    /// Refuses a spelled path whose own directory lands outside the workspace, before anything is
    /// said about what is there.
    fn own_directory_inside(&self, relative: &str, spelled: &Path) -> Result<(), EditError> {
        let dir = spelled.parent().unwrap_or(spelled);
        let landed = dir
            .canonicalize()
            .map_err(|error| io_error(relative, &error))?;
        if landed.starts_with(&self.root) {
            Ok(())
        } else {
            Err(outside(relative))
        }
    }

    fn leads_nowhere(&self, relative: &str, existing: &Path, error: &io::Error) -> EditError {
        EditError::IsSymlink {
            path: relative.to_owned(),
            detail: format!(
                "`{relative}`: `{}` is a link that leads nowhere this workspace can write ({error})",
                self.display(existing)
            ),
        }
    }

    /// Resolves a path the caller intends to *create*, which plain canonicalisation cannot:
    /// `canonicalize` on a path that is not there yet answers nothing at all.
    ///
    /// So it canonicalises the deepest ancestor that does exist, which is the part a symlink could
    /// have moved, checks containment there, and rebuilds. A `..` left in the unresolved tail is
    /// refused rather than appended, because `PathBuf::push` treats it as a name and the
    /// containment check would then pass on a path that escapes.
    ///
    /// Presence is `symlink_metadata`, never `exists`. `Path::exists` follows links, so a dangling
    /// link inside the workspace answered "not there" and the walk carried on up to the workspace
    /// root, whose containment check of course passes, leaving `fs::write` to follow the link and
    /// create the file wherever it pointed. Asking about the link itself stops the walk at it, and
    /// then `canonicalize` fails on a link that leads nowhere and refuses one that leads out.
    fn resolve_new(&self, relative: &str) -> Result<PathBuf, EditError> {
        let candidate = self.root.join(normalise(relative).path);
        let mut tail = Vec::new();
        let mut existing = candidate.as_path();
        while existing.symlink_metadata().is_err() {
            let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
                return Err(EditError::OutsideWorkspace {
                    path: relative.to_owned(),
                    detail: format!("`{relative}` names no path this workspace can write"),
                });
            };
            tail.push(name.to_owned());
            existing = parent;
        }

        let mut resolved = existing
            .canonicalize()
            .map_err(|error| self.leads_nowhere(relative, existing, &error))?;
        if !resolved.starts_with(&self.root) {
            return Err(outside(relative));
        }
        for name in tail.iter().rev() {
            if name == ".." {
                return Err(EditError::OutsideWorkspace {
                    path: relative.to_owned(),
                    detail: format!(
                        "`{relative}` climbs out of a directory that does not exist and was refused"
                    ),
                });
            }
            resolved.push(name);
        }
        Ok(resolved)
    }

    fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// The existing regular file at `relative` and its text, after the precondition held.
    fn read_text(
        &self,
        relative: &str,
        expected: Option<&str>,
    ) -> Result<(PathBuf, String), EditError> {
        let target = self.resolve(relative)?;
        refuse_link(&target, relative, "edits files")?;
        if !target.is_file() {
            return Err(EditError::NotAFile {
                path: relative.to_owned(),
            });
        }
        let bytes = fs::read(&target).map_err(|error| io_error(relative, &error))?;
        self.check_digest(relative, expected, Some(&bytes))?;
        let text = String::from_utf8(bytes).map_err(|_| EditError::NotText {
            path: relative.to_owned(),
        })?;
        Ok((target, text))
    }

    /// Compares `expected`, when given, with the digest of `current` (`None`: there is no file),
    /// before anything is written.
    fn check_digest(
        &self,
        relative: &str,
        expected: Option<&str>,
        current: Option<&[u8]>,
    ) -> Result<(), EditError> {
        let Some(expected) = expected else {
            return Ok(());
        };
        if !self.checks_precondition() {
            return Ok(());
        }
        let actual = current.map(sha256_hex);
        if actual.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(EditError::PreconditionFailed {
                path: relative.to_owned(),
                expected: expected.to_owned(),
                actual,
            })
        }
    }

    /// Writes `after` over the file, when it differs from `before`, and answers what the reply
    /// carries about it. `diff` is empty when no edit was `applied`; otherwise it is headed by the
    /// path twice, with hunks only where the text differs.
    fn replace(
        &self,
        target: &Path,
        relative: &str,
        before: &str,
        after: &str,
        applied: usize,
    ) -> Result<Written, EditError> {
        if after != before {
            fs::write(target, after).map_err(|error| io_error(relative, &error))?;
        }
        let path = self.display(target);
        let diff = if self.emits_diff() && applied > 0 {
            unified_diff(&path, before, after)
        } else {
            String::new()
        };
        Ok(Written {
            path,
            bytes: len(after.len()),
            sha256: sha256_hex(after.as_bytes()),
            diff,
        })
    }

    #[cfg_attr(not(any(test, feature = "mutants")), allow(clippy::unused_self))]
    fn checks_precondition(&self) -> bool {
        #[cfg(any(test, feature = "mutants"))]
        let on = self.mutant != Some(Mutant::IgnorePrecondition);
        #[cfg(not(any(test, feature = "mutants")))]
        let on = true;
        on
    }

    #[cfg_attr(not(any(test, feature = "mutants")), allow(clippy::unused_self))]
    fn applies_sequentially(&self) -> bool {
        #[cfg(any(test, feature = "mutants"))]
        let on = self.mutant == Some(Mutant::SequentialPatch);
        #[cfg(not(any(test, feature = "mutants")))]
        let on = false;
        on
    }

    #[cfg_attr(not(any(test, feature = "mutants")), allow(clippy::unused_self))]
    fn emits_diff(&self) -> bool {
        #[cfg(any(test, feature = "mutants"))]
        let on = self.mutant != Some(Mutant::NoDiff);
        #[cfg(not(any(test, feature = "mutants")))]
        let on = true;
        on
    }
}

/// What a write of new text answers with.
struct Written {
    path: String,
    bytes: u64,
    sha256: String,
    diff: String,
}

fn open(root: &Path) -> Result<PathBuf, EditError> {
    let shown = root.to_string_lossy().into_owned();
    let canonical = root.canonicalize().map_err(|error| EditError::Io {
        path: shown.clone(),
        detail: format!("workspace: {error}"),
    })?;
    if !canonical.is_dir() {
        return Err(EditError::NotAFile { path: shown });
    }
    Ok(canonical)
}

fn outside(relative: &str) -> EditError {
    EditError::OutsideWorkspace {
        path: relative.to_owned(),
        detail: format!("`{relative}` resolves outside the workspace and was refused"),
    }
}

fn io_error(relative: &str, error: &io::Error) -> EditError {
    EditError::Io {
        path: relative.to_owned(),
        detail: error.to_string(),
    }
}

/// How a caller spelled a path.
struct Spelling<'a> {
    /// The path without a trailing `/` or `/.`: `link/` and `link/.` name `link`.
    path: &'a str,
    /// Whether it was spelled as a directory, with a trailing `/` or `/.`. A path that is not
    /// there and is spelled so names a directory, which this toolset never creates.
    as_directory: bool,
}

/// Strips a trailing `/` or `/.`. On a link to a file the trailing `/` made `symlink_metadata`
/// fail with "not a directory", which read as "not there", and the walk in `resolve_new` stepped
/// past the link and rebuilt it uncanonicalised.
fn normalise(relative: &str) -> Spelling<'_> {
    let mut path = relative;
    loop {
        if let Some(rest) = path.strip_suffix("/.") {
            path = rest;
        } else if path.len() > 1 && path.ends_with('/') {
            path = &path[..path.len() - 1];
        } else {
            return Spelling {
                path,
                as_directory: path.len() < relative.len(),
            };
        }
    }
}

/// Belt and braces on top of a resolve: a link put at `target` between that check and the call
/// that follows would still be followed by `fs::read`, `fs::write` or `fs::rename`, and the bytes
/// would land wherever it points.
fn refuse_link(target: &Path, relative: &str, does: &str) -> Result<(), EditError> {
    if target
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(EditError::IsSymlink {
            path: relative.to_owned(),
            detail: format!(
                "`{relative}` is a symlink, and this workspace {does} rather than through links. \
                 Name the file itself."
            ),
        });
    }
    Ok(())
}

fn len(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A unified diff of `before` and `after`, headed by the workspace-relative path twice; the two
/// header lines alone when they are the same, which is how a diff says that nothing changed.
fn unified_diff(path: &str, before: &str, after: &str) -> String {
    if before == after {
        return format!("--- {path}\n+++ {path}\n");
    }
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .header(path, path)
        .to_string()
}

/// Where `old` starts in `text`, when it appears exactly once. Occurrences are counted
/// overlapping, advancing one character past each find, so `aa` fits at two places in `aaa`;
/// harness 0.13.3 counted non-overlapping occurrences (`str::matches`) and would have applied it.
fn locate(text: &str, old: &str, relative: &str, index: usize) -> Result<usize, EditError> {
    let step = old.chars().next().map_or(1, char::len_utf8);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text.get(from..).and_then(|rest| rest.find(old)) {
        found.push(from + at);
        from += at + step;
    }
    match found.as_slice() {
        [] => Err(EditError::NoMatch {
            path: relative.to_owned(),
            index,
        }),
        [start] => Ok(*start),
        many => Err(EditError::Ambiguous {
            path: relative.to_owned(),
            index,
            count: many.len(),
        }),
    }
}

fn replace_once(
    text: &str,
    old: &str,
    new: &str,
    relative: &str,
    index: usize,
) -> Result<String, EditError> {
    let start = locate(text, old, relative, index)?;
    let mut out = text.to_owned();
    out.replace_range(start..start + old.len(), new);
    Ok(out)
}

/// Resolves every edit's `old` against `text` as it is, refuses two that touch the same bytes,
/// then applies them from the end of the file backwards so that earlier offsets still hold.
fn apply_against_original(text: &str, edits: &[Edit], relative: &str) -> Result<String, EditError> {
    let mut ranges = Vec::with_capacity(edits.len());
    for (index, edit) in edits.iter().enumerate() {
        let start = locate(text, &edit.old, relative, index)?;
        ranges.push((start, start + edit.old.len(), index));
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        let ((_, end, a), (start, _, b)) = (pair[0], pair[1]);
        if start < end {
            return Err(EditError::Overlap {
                path: relative.to_owned(),
                first: a.min(b),
                second: a.max(b),
            });
        }
    }
    let mut out = text.to_owned();
    for &(start, end, index) in ranges.iter().rev() {
        out.replace_range(start..end, &edits[index].new);
    }
    Ok(out)
}

fn contract() -> &'static Contract {
    static CONTRACT: OnceLock<Contract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        Contract::from_yaml(CONTRACT_YAML).expect("the crate's own contract document parses")
    })
}

fn call<I: DeserializeOwned, O: Serialize>(
    input: &Value,
    operation: impl FnOnce(&I) -> Result<O, EditError>,
) -> Outcome {
    let input: I = match serde_json::from_value(input.clone()) {
        Ok(input) => input,
        Err(err) => {
            return Outcome::Error {
                name: "code.InvalidInput".to_string(),
                message: err.to_string(),
                fields: json!({}),
            };
        }
    };
    match operation(&input) {
        Ok(output) => match serde_json::to_value(output) {
            Ok(value) => Outcome::Ok(value),
            Err(err) => Outcome::Error {
                name: "code.Io".to_string(),
                message: err.to_string(),
                fields: json!({ "path": Value::Null }),
            },
        },
        Err(err) => Outcome::Error {
            name: err.name().to_string(),
            message: err.to_string(),
            fields: err.fields(),
        },
    }
}

impl Toolset for Editor {
    fn contract(&self) -> &Contract {
        contract()
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        vec![
            (
                "code.write".to_string(),
                schema_for!(WriteInput),
                schema_for!(WriteOutput),
            ),
            (
                "code.edit".to_string(),
                schema_for!(EditInput),
                schema_for!(EditOutput),
            ),
            (
                "code.patch".to_string(),
                schema_for!(PatchInput),
                schema_for!(PatchOutput),
            ),
            (
                "code.move".to_string(),
                schema_for!(MoveInput),
                schema_for!(MoveOutput),
            ),
            (
                "code.delete".to_string(),
                schema_for!(DeleteInput),
                schema_for!(DeleteOutput),
            ),
        ]
    }

    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        match operation {
            "code.write" => call(input, |i| self.write(i)),
            "code.edit" => call(input, |i| self.edit(i)),
            "code.patch" => call(input, |i| self.patch(i)),
            "code.move" => call(input, |i| self.rename(i)),
            "code.delete" => call(input, |i| self.delete(i)),
            other => Outcome::Error {
                name: "code.UnknownOperation".to_string(),
                message: format!("{other} is not an operation of the code-editor toolset"),
                fields: json!({}),
            },
        }
    }
}

#[cfg(test)]
mod tests;
