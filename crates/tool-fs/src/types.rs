//! The typed inputs and outputs of the four operations, with the field names of
//! `contracts/fs/v1alpha2/contract.yaml`. The schemas the toolset exposes are derived from these
//! with `schemars`, and the conformance run checks them against the contract's field lists.

use std::fs::FileType;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The input of `fs.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadInput {
    /// The file, relative to the workspace root.
    pub path: String,
    /// The first line to read, numbered from 1. Default 1; 0 names no line and is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// How many lines at most. Without it the window closes at the byte ceiling; 0 names no
    /// line and is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// The byte ceiling of the window. Default 65536; a larger ask is capped at 262144.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
    /// The wall-clock bound of the call in milliseconds. Default 10000; above 60000 is
    /// refused as `fs.InvalidInput`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl ReadInput {
    /// The whole file from line 1, under the default byte ceiling.
    pub fn whole(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            offset: None,
            limit: None,
            max_bytes: None,
            timeout_ms: None,
        }
    }

    /// `limit` lines from line `offset`, under the default byte ceiling.
    pub fn lines(path: impl Into<String>, offset: u64, limit: u64) -> Self {
        Self {
            offset: Some(offset),
            limit: Some(limit),
            ..Self::whole(path)
        }
    }
}

/// Which lines of the file a read's text holds, so a window is never mistaken for the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Window {
    /// The first line answered, numbered from 1.
    pub from: u64,
    /// The last line answered; one below `from` when nothing was, which is only an empty file
    /// read from line 1.
    pub to: u64,
    /// How many lines the file has; `None` when the count stopped at the counting bound.
    pub total: Option<u64>,
}

/// The output of `fs.read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReadOutput {
    /// The file's workspace-relative path after links were followed.
    pub path: String,
    /// The size of the whole file, from its metadata.
    pub bytes: u64,
    /// The window as numbered lines: a right-aligned line number, a tab, the line, a newline.
    pub text: String,
    /// Which lines the text holds.
    pub lines: Window,
    /// The window stopped before the file's last line, a line in it was cut, or the line count
    /// stopped at its bound.
    pub truncated: bool,
    /// The numbers of the lines the window cut; empty when none was.
    pub truncated_lines: Vec<u64>,
    /// The SHA-256 of the whole file as lowercase hex; `None` when the line count stopped at its
    /// bound.
    pub sha256: Option<String>,
    /// The last line the count reached when it stopped at its bound; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines_counted_to: Option<u64>,
    /// Why `lines.total` is `None`, when it is; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The call stopped at its deadline; the text holds the lines answered by then.
    pub deadline_reached: bool,
}

/// The input of `fs.list`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInput {
    /// The directory, relative to the workspace root. Default `.`, the root itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The wall-clock bound of the call in milliseconds. Default 10000; above 60000 is
    /// refused as `fs.InvalidInput`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// What an entry is by its own type: a regular file, a directory, a symbolic link (not followed,
/// whatever it points at), or anything else (a socket, a pipe, a device).
///
/// The variants carry no doc comments of their own: `schemars` renders a documented variant as
/// a `oneOf` branch with a `const`, and the contract's shape grammar reads an enum as a string
/// with an `enum` list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    File,
    Directory,
    Symlink,
    Other,
}

impl Kind {
    pub(crate) fn of(file_type: FileType) -> Self {
        if file_type.is_symlink() {
            Self::Symlink
        } else if file_type.is_dir() {
            Self::Directory
        } else if file_type.is_file() {
            Self::File
        } else {
            Self::Other
        }
    }
}

/// One entry of a listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Entry {
    /// The entry's own name.
    pub name: String,
    /// What it is, by its own type.
    pub kind: Kind,
    /// The size of a file; `None` for anything that is not one.
    pub bytes: Option<u64>,
}

/// The output of `fs.list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListOutput {
    /// The directory's workspace-relative path; empty for the root.
    pub path: String,
    /// The entries in name order, at most 500.
    pub entries: Vec<Entry>,
    /// The directory holds more entries than were answered, or the deadline was reached before
    /// the listing was whole.
    pub truncated: bool,
    /// The call stopped at its deadline; the entries are the ones read by then.
    pub deadline_reached: bool,
}

/// The input of `fs.glob`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GlobInput {
    /// Without a `/` it matches the name of a file anywhere under `path`; with one it matches the
    /// whole workspace-relative path, and `*` does not cross a separator.
    pub glob: String,
    /// Where to start, relative to the workspace root. Default `.`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The most paths to answer. Default 500; a larger ask is capped at 500.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u64>,
    /// The wall-clock bound of the call in milliseconds. Default 10000; above 60000 is
    /// refused as `fs.InvalidInput`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl GlobInput {
    /// `glob` from the workspace root, under the default result cap.
    pub fn new(glob: impl Into<String>) -> Self {
        Self {
            glob: glob.into(),
            path: None,
            max_results: None,
            timeout_ms: None,
        }
    }
}

/// The output of `fs.glob`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GlobOutput {
    /// The matching files' workspace-relative paths, in walk order.
    pub paths: Vec<String>,
    /// More paths matched than were answered, or the deadline was reached before the walk was
    /// done.
    pub truncated: bool,
    /// The walk stopped descending at its depth bound somewhere, so matches below it are not in
    /// `paths`.
    pub depth_bound_reached: bool,
    /// The call stopped at its deadline; `paths` holds the matches found by then.
    pub deadline_reached: bool,
}

/// The input of `fs.stat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StatInput {
    /// The entry, relative to the workspace root.
    pub path: String,
    /// The wall-clock bound of the call in milliseconds. Default 10000; above 60000 is
    /// refused as `fs.InvalidInput`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl StatInput {
    /// The entry at `path`, under the default deadline.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            timeout_ms: None,
        }
    }
}

/// The output of `fs.stat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StatOutput {
    /// The entry's workspace-relative path: the target's for a file or directory reached through
    /// links, the link's own for a symlink.
    pub path: String,
    /// The entry's own kind.
    pub kind: Kind,
    /// The size of a file; `None` for anything else.
    pub bytes: Option<u64>,
    /// The SHA-256 of a file of at most the counting bound (16 MiB) as lowercase hex; `None` for
    /// a larger file, for anything that is not a file, and when the deadline was reached.
    pub sha256: Option<String>,
    /// The call stopped at its deadline; `sha256` is `None`.
    pub deadline_reached: bool,
}
