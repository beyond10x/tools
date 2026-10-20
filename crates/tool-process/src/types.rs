//! The typed input and output of `process.run`, with the field names of
//! `contracts/process/v1alpha1/contract.yaml`. The schemas the toolset exposes are derived from
//! these with `schemars`, and the conformance run checks them against the contract's field lists.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The input of `process.run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunInput {
    /// The program, then its arguments, one item each. Never a command line.
    pub argv: Vec<String>,
    /// The wall-clock bound in milliseconds. Default 600000; a larger ask is capped at 600000,
    /// and the time the caller's run has left caps it further.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl RunInput {
    /// `argv`, under the default bound.
    pub fn argv(argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            timeout_ms: None,
        }
    }

    /// The same run, bounded at `timeout_ms` milliseconds before the caps.
    #[must_use]
    pub fn within(self, timeout_ms: u64) -> Self {
        Self {
            timeout_ms: Some(timeout_ms),
            ..self
        }
    }
}

/// The output of `process.run`: what the program did, and what the run did to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RunOutput {
    /// The argv that was run, as given.
    pub argv: Vec<String>,
    /// The program's exit code; `None` when it was ended by a signal, the kill at the bound
    /// included.
    pub exit: Option<i32>,
    /// The standard output, both ends kept; a cut in the middle is a marker naming the bytes it
    /// replaced.
    pub stdout: String,
    /// The standard error, kept the same way.
    pub stderr: String,
    /// Either stream was cut; `omitted_bytes` is then above 0.
    pub truncated: bool,
    /// The bytes dropped from the middle of the two streams together; each stream's marker names
    /// its own share.
    pub omitted_bytes: u64,
    /// The bound was reached and the process group was killed; `exit` then says nothing about the
    /// program's own outcome.
    pub timed_out: bool,
    /// The bound the run was given, in milliseconds, after the caps, so a kill at the bound is not
    /// read as the program's own slowness.
    pub timeout_ms: u64,
}
