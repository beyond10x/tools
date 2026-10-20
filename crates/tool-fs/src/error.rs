//! [`FsError`]: every refusal the operations make, each with the reader's own wording as its
//! message and one contract error name under [`FsError::name`].

use std::io;

use serde_json::{Value, json};

use crate::MAX_LINE_COUNT_BYTES;

/// Why an operation refused, or why a workspace did not open.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// The workspace root cannot be canonicalised.
    #[error("workspace `{path}`: {source}")]
    Workspace {
        /// The root as given.
        path: String,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The workspace root is not a directory.
    #[error("workspace `{path}` is not a directory")]
    WorkspaceNotADirectory {
        /// The root as given.
        path: String,
    },
    /// Nothing exists at the path: it is absent, a component of it is a file where a directory
    /// would have to be, or a link there leads nowhere.
    #[error("`{path}`: {source}")]
    NotFound {
        /// The path as given.
        path: String,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The path names something that is not a regular file.
    #[error("`{path}` is not a file")]
    NotAFile {
        /// The path as given.
        path: String,
    },
    /// The path names something that is not a directory.
    #[error("`{path}` is not a directory")]
    NotADirectory {
        /// The path as given.
        path: String,
    },
    /// The path lands outside the workspace, by how it is spelled or through a link.
    #[error("`{path}` resolves outside the workspace and was refused")]
    OutsideWorkspace {
        /// The path as given.
        path: String,
    },
    /// `offset` is 0, which names no line.
    #[error(
        "`offset` is the first line to read and lines are numbered from 1, so 0 names no line. \
         `{path}` was not read."
    )]
    OffsetZero {
        /// The path as given.
        path: String,
    },
    /// `limit` is 0, which names no line.
    #[error("`limit` is how many lines to read and 0 names none. `{path}` was not read.")]
    LimitZero {
        /// The path as given.
        path: String,
    },
    /// `offset` names a line past the end of the file.
    #[error(
        "`{path}` has {total} lines and `offset` names line {offset}, which is past the end. \
         Nothing was read."
    )]
    PastEnd {
        /// The path as given.
        path: String,
        /// The line asked for.
        offset: u64,
        /// How many lines the file has.
        total: u64,
    },
    /// `offset` names a line past where the bounded line count reached.
    #[error(
        "`{path}` was scanned to line {total} — this build stops counting lines after {bound} \
         bytes — and `offset` names line {offset}, past where the scan reached. Nothing was read.",
        bound = MAX_LINE_COUNT_BYTES
    )]
    PastScan {
        /// The path as given.
        path: String,
        /// The line asked for.
        offset: u64,
        /// The last line the count reached.
        total: u64,
    },
    /// `glob` is empty, which would match nothing and read as *there are no such files*.
    #[error(
        "`glob` is required and must not be empty: a glob has to name what it is looking for. \
         `*.rs` is that name at any depth; `crates/**/*.rs` is the whole path."
    )]
    EmptyGlob,
    /// `glob` does not compile.
    #[error("`{glob}` is not a glob this build can compile: {source}")]
    InvalidGlob {
        /// The glob as given.
        glob: String,
        /// The compiler's own refusal.
        #[source]
        source: globset::Error,
    },
    /// The filesystem refused for a reason other than the path's absence.
    #[error("`{path}`: {source}")]
    Io {
        /// The path as given.
        path: String,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
}

impl FsError {
    /// The contract error this refusal is returned as.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Workspace { .. } | Self::Io { .. } => "fs.Io",
            Self::WorkspaceNotADirectory { .. } | Self::NotADirectory { .. } => "fs.NotADirectory",
            Self::NotFound { .. } => "fs.NotFound",
            Self::NotAFile { .. } => "fs.NotAFile",
            Self::OutsideWorkspace { .. } => "fs.OutsideWorkspace",
            Self::OffsetZero { .. }
            | Self::LimitZero { .. }
            | Self::PastEnd { .. }
            | Self::PastScan { .. } => "fs.InvalidWindow",
            Self::EmptyGlob | Self::InvalidGlob { .. } => "fs.InvalidGlob",
        }
    }

    /// The contract error's fields, as JSON: `path` or `glob`, as the contract declares them.
    pub fn fields(&self) -> Value {
        match self {
            Self::Workspace { path, .. }
            | Self::WorkspaceNotADirectory { path }
            | Self::NotFound { path, .. }
            | Self::NotAFile { path }
            | Self::NotADirectory { path }
            | Self::OutsideWorkspace { path }
            | Self::OffsetZero { path }
            | Self::LimitZero { path }
            | Self::PastEnd { path, .. }
            | Self::PastScan { path, .. }
            | Self::Io { path, .. } => json!({ "path": path }),
            Self::EmptyGlob => json!({ "glob": "" }),
            Self::InvalidGlob { glob, .. } => json!({ "glob": glob }),
        }
    }

    /// `NotFound` for a path that names nothing: absent, or with a file where a directory would
    /// have to be (`a.txt/x`). `Io` for any other failure.
    pub(crate) fn io(path: &str, source: io::Error) -> Self {
        let path = path.to_owned();
        if matches!(
            source.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
        ) {
            Self::NotFound { path, source }
        } else {
            Self::Io { path, source }
        }
    }
}
