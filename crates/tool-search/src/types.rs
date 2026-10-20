//! The typed input and output of `search.grep`, with the field names of
//! `contracts/search/v1alpha1/contract.yaml`. The schemas the toolset exposes are derived from
//! these with `schemars`, and the conformance run checks them against the contract's field lists.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The input of `search.grep`: harness's `pattern`, `path` and `SearchOptions` in one struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrepInput {
    /// What to find: a literal substring, or a regular expression when `regex` is true. Must not
    /// be empty.
    pub pattern: String,
    /// The directory or file to search, relative to the workspace root. Default `.`, the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Read `pattern` as a regular expression. Default false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<bool>,
    /// Only files matching this glob: a name when it has no `/`, a workspace-relative path when it
    /// has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    /// Lines either side of each match. Default 0; a larger ask than 5 is capped at 5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    /// The most matches to answer. Default 200; a larger ask is capped at 200.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u64>,
}

impl GrepInput {
    /// A literal search for `pattern` from the workspace root, with no glob, no context and the
    /// default result cap: harness's `SearchOptions::default()`.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            path: None,
            regex: None,
            glob: None,
            context: None,
            max_results: None,
        }
    }
}

/// One line of context around a match, under its own number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContextLine {
    /// The line's number in its file, numbered from 1.
    pub line: u64,
    /// The line, cut at 400 characters.
    pub text: String,
    /// The line is longer than 400 characters and `text` holds its first 400.
    pub line_truncated: bool,
}

/// One line that matched, and the lines around it when context was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    /// The file's workspace-relative path.
    pub path: String,
    /// The matching line's number in the file, numbered from 1.
    pub line: u64,
    /// The matching line, cut at 400 characters.
    pub text: String,
    /// The line is longer than 400 characters and `text` holds its first 400.
    pub line_truncated: bool,
    /// The lines before the match, at most `context` of them, in file order; absent when no
    /// context was asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Vec<ContextLine>>,
    /// The lines after the match, at most `context` of them, in file order; absent when no
    /// context was asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<ContextLine>>,
}

/// The output of `search.grep`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GrepOutput {
    /// The pattern as given.
    pub pattern: String,
    /// The matching lines, in walk order and then line order.
    pub matches: Vec<Match>,
    /// A match past the answered count exists, or a file was too large to search.
    pub truncated: bool,
    /// The walk stopped descending at its depth bound somewhere, so files below it were not
    /// searched.
    pub depth_bound_reached: bool,
    /// How many files over 1 MiB were met and not searched.
    pub skipped_large_files: u64,
    /// The workspace-relative paths of the first 20 files over 1 MiB that were not searched.
    pub skipped_large_paths: Vec<String>,
    /// `true` when `pattern` was read as a regular expression; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<bool>,
    /// The glob that narrowed the search, when one did; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    /// The context the matches carry, when the ask was over the cap of 5; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    /// Why `context` differs from the ask, when it does; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
