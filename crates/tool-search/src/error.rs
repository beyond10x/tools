//! [`SearchError`]: every refusal `search.grep` makes, each with harness's own wording as its
//! message where harness had one, and one contract error name under [`SearchError::name`].

use std::io;

use serde_json::{Value, json};
use tool_fs::FsError;

/// Why a search refused, or why a workspace did not open.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
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
    /// `pattern` is empty, which would match every line.
    #[error(
        "`pattern` is required and must not be empty: a search has to name what it is looking \
         for. Nothing was read."
    )]
    EmptyPattern,
    /// `regex` is set and `pattern` does not compile within the size limit.
    #[error("`{pattern}` is not a regular expression this build can compile: {source}")]
    InvalidPattern {
        /// The pattern as given.
        pattern: String,
        /// The compiler's own refusal.
        #[source]
        source: regex::Error,
    },
    /// `glob` is given and empty, which would name no file and answer no match.
    #[error(
        "`glob` must not be empty when it is given: an empty glob names no file, so the search \
         would answer no matches. `*.rs` is that name at any depth; `src/**/*.rs` is the whole \
         path."
    )]
    EmptyGlob,
    /// `glob` does not compile.
    #[error("{source}")]
    InvalidGlob {
        /// The glob as given.
        glob: String,
        /// The glob compiler's refusal, as `tool-fs` words it for `fs.glob`.
        #[source]
        source: FsError,
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
    /// The path lands outside the workspace, by how it is spelled or through a link.
    #[error("`{path}` resolves outside the workspace and was refused")]
    OutsideWorkspace {
        /// The path as given.
        path: String,
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

impl SearchError {
    /// The contract error this refusal is returned as.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Workspace { .. } | Self::WorkspaceNotADirectory { .. } | Self::Io { .. } => {
                "search.Io"
            }
            Self::EmptyPattern | Self::InvalidPattern { .. } => "search.InvalidPattern",
            Self::EmptyGlob | Self::InvalidGlob { .. } => "search.InvalidGlob",
            Self::NotFound { .. } => "search.NotFound",
            Self::OutsideWorkspace { .. } => "search.OutsideWorkspace",
        }
    }

    /// The contract error's fields, as JSON: `pattern`, `glob` or `path`, as the contract declares
    /// them.
    pub fn fields(&self) -> Value {
        match self {
            Self::Workspace { path, .. }
            | Self::WorkspaceNotADirectory { path }
            | Self::NotFound { path, .. }
            | Self::OutsideWorkspace { path }
            | Self::Io { path, .. } => json!({ "path": path }),
            Self::EmptyPattern => json!({ "pattern": "" }),
            Self::InvalidPattern { pattern, .. } => json!({ "pattern": pattern }),
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
