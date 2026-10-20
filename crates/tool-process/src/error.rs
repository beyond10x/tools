//! [`ProcessError`]: every refusal a run makes, each with the runner's own wording as its message
//! and one contract error name under [`ProcessError::name`]. Harness answered these as `Refused`,
//! a sentence with an optional `Refusal` beside it; here each case is its own variant.

use std::io;

use serde_json::{Value, json};

/// Why a run refused, failed to start, or why a workspace did not open.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// The input is not `process.run`'s input.
    #[error("the input is not process.run's input: {0}")]
    InvalidInput(#[source] serde_json::Error),
    /// `argv` is empty, so it names no program.
    #[error("`argv` must name a program")]
    EmptyArgv,
    /// `argv[0]` is not in the declared set, compared as text.
    #[error(
        "`{program}` is not a program this run may start; the declared programs are: {}. Nothing \
         was started.",
        if declared.is_empty() { "none".to_owned() } else { declared.join(", ") }
    )]
    ProgramNotDeclared {
        /// `argv[0]` as given.
        program: String,
        /// The declared set, in the order it was declared.
        declared: Vec<String>,
    },
    /// The workspace root cannot be canonicalised.
    #[error("workspace `{path}`: {source}")]
    Workspace {
        /// The root as given.
        path: String,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The workspace root is not a directory, or no longer is one.
    #[error("workspace `{path}` is not a directory; nothing was started")]
    WorkspaceNotADirectory {
        /// The root.
        path: String,
    },
    /// A declared program could not be started.
    #[error("`{program}`: {source}")]
    SpawnFailed {
        /// `argv[0]` as given.
        program: String,
        /// The system's refusal.
        #[source]
        source: io::Error,
    },
    /// The program started and the system then refused to report on it; its group was killed.
    #[error("`{program}`: {source}")]
    WaitFailed {
        /// `argv[0]` as given.
        program: String,
        /// The system's refusal.
        #[source]
        source: io::Error,
    },
}

impl ProcessError {
    /// Every contract error name a [`ProcessError`] can be returned as.
    pub const NAMES: [&'static str; 6] = [
        "process.InvalidInput",
        "process.EmptyArgv",
        "process.ProgramNotDeclared",
        "process.WorkspaceUnavailable",
        "process.SpawnFailed",
        "process.WaitFailed",
    ];

    /// The contract error this refusal is returned as.
    pub fn name(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "process.InvalidInput",
            Self::EmptyArgv => "process.EmptyArgv",
            Self::ProgramNotDeclared { .. } => "process.ProgramNotDeclared",
            Self::Workspace { .. } | Self::WorkspaceNotADirectory { .. } => {
                "process.WorkspaceUnavailable"
            }
            Self::SpawnFailed { .. } => "process.SpawnFailed",
            Self::WaitFailed { .. } => "process.WaitFailed",
        }
    }

    /// The contract error's fields, as JSON, as the contract declares them.
    pub fn fields(&self) -> Value {
        match self {
            Self::InvalidInput(_) | Self::EmptyArgv => json!({}),
            Self::ProgramNotDeclared { program, declared } => {
                json!({ "program": program, "declared": declared })
            }
            Self::Workspace { path, .. } | Self::WorkspaceNotADirectory { path } => {
                json!({ "path": path })
            }
            Self::SpawnFailed { program, .. } | Self::WaitFailed { program, .. } => {
                json!({ "program": program })
            }
        }
    }
}
