//! The `search` toolset: `search.grep` over one workspace root, under the `tool-contract/1`
//! contract in `contracts/search/v1alpha2`: v1alpha1 with the declared `timeout_ms` enforced
//! ([`tool_fs::deadline`]) and `deadline_reached` in every answer.
//!
//! This crate is harness's `crates/harness-tools/src/local.rs` search at harness 0.14.1
//! (`f109612e`) moved here: the constants `MAX_GREP_RESULTS`, `MAX_GREP_FILE_BYTES`,
//! `MAX_SEARCH_CONTEXT` and `MAX_MATCH_CHARS`, `grep` (now [`Search::grep`]) with its
//! `SearchOptions` (now the fields of [`GrepInput`]), `search_expression`, `cut_to` and `around`,
//! with the same reply fields, bounds and refusals, and its tests for cut matches, large files at
//! both edges of the bound, regular expressions and their size limit, globs and numbered context,
//! the depth bound, links out of the workspace and the capped context. The walk is not copied: it
//! is `tool-fs`'s ([`tool_fs::walk`]), the one `fs.glob` walks with, so the skipped directories,
//! the depth bound, the containment re-check and the glob rule are that crate's, and a fix to the
//! walk reaches both. Added here: typed input and output ([`types`]) whose `schemars` schemas the
//! conformance run checks against the contract; [`SearchError`] in place of harness's message
//! strings, each variant named by one contract error; the [`tool_contract::Toolset`]
//! implementation; and the mutants the suite is checked against.
//!
//! # Where it differs from harness 0.14.1
//!
//! Each pinned by a scenario or a test. `truncated` on the result cap is set only when a match is
//! found past the cap, and the search stops there: harness tested the cap before every file and
//! every line, so exactly `max_results` matches followed by anything at all answered `true`
//! (`fs.glob` made the same correction in wave w2). An empty `glob` is refused as
//! `search.InvalidGlob`, as `fs.glob` refuses one, where harness compiled it into a filter that
//! matched no file and answered an empty list. A `path` is resolved by `fs`'s rule, so one that
//! climbs out to a place that does not exist is `search.OutsideWorkspace` rather than a
//! not-found. Only regular files are read, so a named pipe in the tree is passed over where
//! harness opened it and blocked. A file whose name is one of the skipped names (a worktree's
//! `.git` file) is not searched, where harness skipped those names on directories only and the
//! search's own description said `.git` is skipped. A file that holds a NUL byte is binary and is
//! passed over like one that is not UTF-8, where harness searched it and answered its NUL bytes.
//! And the walk stops in every frame once the search stops (`tool-fs`'s walk since wave w2),
//! where harness's kept iterating in the frames above.
//!
//! # The boundary, and what it is not
//!
//! Every path check here is this crate's and `tool-fs`'s own: [`Search::new`] canonicalises the
//! root, a search canonicalises its path and checks containment, and the walk re-checks every
//! entry it descends into. That is real and it is not confinement: nothing stops the process,
//! only these functions, and a bug in them is the whole boundary. It is the right search for a
//! run against an operator's own tree; a run that wants the effects confined puts a confined root
//! under it.

mod error;
mod toolset;
pub mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use regex::{Regex, RegexBuilder};
use tool_fs::deadline::{DEFAULT_TIMEOUT_MS, Deadline, MAX_TIMEOUT_MS};
use tool_fs::walk::{PathGlob, SKIPPED, contained, walk_within};

pub use error::SearchError;
pub use toolset::{CONTRACT_YAML, contract};
pub use types::{ContextLine, GrepInput, GrepOutput, Match};

/// The most matches one search answers, and the cap on a caller's `max_results`.
pub const MAX_GREP_RESULTS: usize = 200;
/// The largest file a search reads, in bytes by its metadata: a larger one is counted and named
/// in the reply, not read.
pub const MAX_GREP_FILE_BYTES: u64 = 1024 * 1024;
/// The most lines of context a search may be asked for either side of a match.
///
/// Five is enough to see a function signature over a match and small enough that two hundred
/// matches with context is still a reply and not a file dump.
pub const MAX_SEARCH_CONTEXT: u64 = 5;
/// How much of a matched line, and of a line of context, a search answers with. One minified
/// file on a single line would otherwise bury every other result, so the line is cut, and the
/// match says that it was.
pub const MAX_MATCH_CHARS: usize = 400;
/// How many of the files too large to search are named by path; all of them are counted.
pub const MAX_SKIPPED_PATHS: usize = 20;
/// The size limit of a regular expression's compiled program and of its lazy DFA. The crate's
/// default would let one caller-supplied pattern take ten megabytes of this process's memory.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// One property switched off, for the mutant check the conformance suite is held to.
///
/// Only a mutant build (`--features mutants`, or the crate's own tests) can set one, through
/// `Search::mutated`; outside it every [`Search`] is intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    /// The 200-match ceiling is not applied: with no `max_results` every match is answered, and a
    /// larger ask is not capped.
    NoResultCap,
    /// `truncated` is always `false`.
    NoTruncatedFlag,
    /// A file over 1 MiB is searched like any other.
    NoLargeFileBound,
    /// The boundary is the workspace's parent rather than the workspace: a path, or an entry the
    /// walk reaches, that lands beside the workspace is accepted. Not the filesystem root, so a
    /// mutant run never walks the machine.
    NoContainment,
    /// A matched line and a line of context are answered whole.
    NoMatchCut,
    /// A `context` above 5 is not capped.
    NoContextCap,
    /// A file whose name is one of the skipped names is searched; only directories so named are
    /// skipped, by the walk, as harness had it.
    NoSkippedFileNames,
    /// A file that is valid UTF-8 and holds a NUL byte is searched, as harness had it.
    NulIsText,
    /// The deadline never passes: `timeout_ms` is still refused above its maximum, and then
    /// never checked.
    IgnoreDeadline,
}

impl Mutation {
    /// Every mutation, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::NoResultCap,
        Self::NoTruncatedFlag,
        Self::NoLargeFileBound,
        Self::NoContainment,
        Self::NoMatchCut,
        Self::NoContextCap,
        Self::NoSkippedFileNames,
        Self::NulIsText,
        Self::IgnoreDeadline,
    ];

    /// The mutation's name in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoResultCap => "no-result-cap",
            Self::NoTruncatedFlag => "no-truncated-flag",
            Self::NoLargeFileBound => "no-large-file-bound",
            Self::NoContainment => "no-containment",
            Self::NoMatchCut => "no-match-cut",
            Self::NoContextCap => "no-context-cap",
            Self::NoSkippedFileNames => "no-skipped-file-names",
            Self::NulIsText => "nul-is-text",
            Self::IgnoreDeadline => "ignore-deadline",
        }
    }
}

/// A bounded, read-only search of one directory.
#[derive(Debug, Clone)]
pub struct Search {
    root: PathBuf,
    #[cfg(any(test, feature = "mutants"))]
    mutation: Option<Mutation>,
}

impl Search {
    /// Opens `root` as the only directory these searches can see.
    ///
    /// # Errors
    ///
    /// `root` cannot be canonicalised, or is not a directory.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, SearchError> {
        Ok(Self {
            root: Self::open(root)?,
            #[cfg(any(test, feature = "mutants"))]
            mutation: None,
        })
    }

    /// The same search with one property switched off, for the mutant check.
    ///
    /// # Errors
    ///
    /// As [`Search::new`].
    #[cfg(any(test, feature = "mutants"))]
    pub fn mutated(root: impl AsRef<Path>, mutation: Mutation) -> Result<Self, SearchError> {
        Ok(Self {
            root: Self::open(root)?,
            mutation: Some(mutation),
        })
    }

    #[cfg(any(test, feature = "mutants"))]
    fn mutated_by(&self, mutation: Mutation) -> bool {
        self.mutation == Some(mutation)
    }

    #[cfg(not(any(test, feature = "mutants")))]
    #[allow(clippy::unused_self)]
    fn mutated_by(&self, _mutation: Mutation) -> bool {
        false
    }

    fn open(root: impl AsRef<Path>) -> Result<PathBuf, SearchError> {
        let given = root.as_ref().display().to_string();
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|source| SearchError::Workspace {
                path: given.clone(),
                source,
            })?;
        if !root.is_dir() {
            return Err(SearchError::WorkspaceNotADirectory { path: given });
        }
        Ok(root)
    }

    /// The canonical workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What every path a search touches must land inside: the workspace root.
    fn boundary(&self) -> &Path {
        if self.mutated_by(Mutation::NoContainment) {
            self.root.parent().unwrap_or(&self.root)
        } else {
            &self.root
        }
    }

    /// Resolves a caller-supplied path inside the workspace, by the rule `fs` resolves with.
    ///
    /// Canonicalisation happens before the containment check, so a symlink or a `..` that leaves
    /// the workspace is refused by where it *lands*, not by how it is spelled. A path that does
    /// not exist is refused by where it *would* land: its deepest existing ancestor is checked,
    /// and one outside the workspace is `OutsideWorkspace` rather than `NotFound`, so `../x` is
    /// refused for climbing out whether or not there is an `x`.
    fn resolve(&self, relative: &str) -> Result<PathBuf, SearchError> {
        let candidate = self.root.join(relative);
        let outside = || SearchError::OutsideWorkspace {
            path: relative.to_owned(),
        };
        match candidate.canonicalize() {
            Ok(canonical) if contained(&canonical, self.boundary()) => Ok(canonical),
            Ok(_) => Err(outside()),
            Err(error) => {
                let lands_inside = candidate
                    .ancestors()
                    .skip(1)
                    .find(|ancestor| ancestor.canonicalize().is_ok())
                    .is_some_and(|ancestor| contained(ancestor, self.boundary()));
                if lands_inside {
                    Err(SearchError::io(relative, error))
                } else {
                    Err(outside())
                }
            }
        }
    }

    fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// How many matches this search answers: `max_results`, capped at [`MAX_GREP_RESULTS`].
    fn limit(&self, asked: Option<u64>) -> usize {
        let asked = asked.map(|asked| usize::try_from(asked).unwrap_or(usize::MAX));
        if self.mutated_by(Mutation::NoResultCap) {
            asked.unwrap_or(usize::MAX)
        } else {
            asked.unwrap_or(MAX_GREP_RESULTS).min(MAX_GREP_RESULTS)
        }
    }

    /// `search.grep`: the lines of the files under `path` that contain `pattern`, or match it as
    /// a regular expression, with the lines around each when `context` asks for them.
    ///
    /// # Errors
    ///
    /// The pattern is empty or, as a regular expression, does not compile within the size limit;
    /// the glob is empty or does not compile; the path is absent or lands outside the workspace;
    /// `timeout_ms` is above [`MAX_TIMEOUT_MS`].
    ///
    /// # The deadline
    ///
    /// Taken first, so the bound is the call's whole wall clock. Above the maximum it is refused
    /// rather than capped, as `fs` refuses it, and only once the pattern, the glob and the path
    /// have been judged (wave w4 D3): those are the refusals a governor reads, so a call that
    /// would be refused for them is refused for them whatever its `timeout_ms`. The walk checks
    /// the deadline at every directory and before every entry ([`walk_within`]), and the search
    /// checks it before every line it matches; where it passed the matches found by then are
    /// answered, `truncated` is set because a match may lie past the stop, and `note` says so.
    pub fn grep(&self, input: &GrepInput) -> Result<GrepOutput, SearchError> {
        let deadline = if self.mutated_by(Mutation::IgnoreDeadline) {
            Deadline::never()
        } else {
            Deadline::after_ms(
                input
                    .timeout_ms
                    .unwrap_or(DEFAULT_TIMEOUT_MS)
                    .min(MAX_TIMEOUT_MS),
            )
        };
        self.grep_within(input, &deadline)
    }

    /// [`Search::grep`] under a deadline the caller made, so a test can stop it at a known line.
    pub(crate) fn grep_within(
        &self,
        input: &GrepInput,
        deadline: &Deadline,
    ) -> Result<GrepOutput, SearchError> {
        let pattern = input.pattern.as_str();
        if pattern.is_empty() {
            return Err(SearchError::EmptyPattern);
        }
        // Capped rather than refused: a caller who asked for forty lines of context wanted to see
        // around the match, and five is seeing around it. The reply echoes the figure that was
        // actually used when it differs from the one that was asked for, because a cap the reader
        // cannot see is a bound they will not work around.
        let asked_context = input.context.unwrap_or(0);
        let context = if self.mutated_by(Mutation::NoContextCap) {
            asked_context
        } else {
            asked_context.min(MAX_SEARCH_CONTEXT)
        };
        let regex = input.regex.unwrap_or(false);
        let expression = search_expression(pattern, regex)?;
        let filter = input.glob.as_deref().map(compile_glob).transpose()?;
        let start = self.resolve(input.path.as_deref().unwrap_or("."))?;
        // After the pattern, the glob and the path, before anything is read (wave w4 D3).
        if let Some(timeout_ms) = input.timeout_ms.filter(|asked| *asked > MAX_TIMEOUT_MS) {
            return Err(SearchError::TimeoutAboveMax { timeout_ms });
        }

        let mut grep = Grep {
            search: self,
            start: &start,
            pattern,
            expression: expression.as_ref(),
            filter: filter.as_ref(),
            limit: self.limit(input.max_results),
            context: usize::try_from(context).unwrap_or(usize::MAX),
            cut: !self.mutated_by(Mutation::NoMatchCut),
            bounded: !self.mutated_by(Mutation::NoLargeFileBound),
            deadline,
            matches: Vec::new(),
            truncated: false,
            skipped_large_files: 0,
            skipped_large_paths: Vec::new(),
        };
        let walked = walk_within(&start, self.boundary(), 0, deadline, &mut |file| {
            grep.visit(file)
        });
        let deadline_reached = deadline.reached();

        // Only when the cap bit. A caller who asked for forty lines got five, and a reply that
        // did not say so leaves them reading five as though it were forty.
        let capped = asked_context > MAX_SEARCH_CONTEXT;
        let capped_note = capped.then(|| {
            format!(
                "`context` was {asked_context} and this build answers at most \
                 {MAX_SEARCH_CONTEXT} lines either side of a match, so {MAX_SEARCH_CONTEXT} is \
                 what these matches carry."
            )
        });
        // Only when the deadline stopped the search: the matches are the ones found by then, and
        // a reply that did not say why it stopped would read as the whole answer.
        let deadline_note = deadline_reached.then(|| {
            "the call's deadline (`timeout_ms`) was reached before the search was done, so \
             these are the matches found by then and more may exist. Ask again with a longer \
             `timeout_ms`, a narrower `path` or a `glob`."
                .to_owned()
        });
        let note = match (capped_note, deadline_note) {
            (Some(capped), Some(deadline)) => Some(format!("{capped} And {deadline}")),
            (capped, deadline) => capped.or(deadline),
        };
        Ok(GrepOutput {
            pattern: pattern.to_owned(),
            matches: grep.matches,
            truncated: (grep.truncated || deadline_reached)
                && !self.mutated_by(Mutation::NoTruncatedFlag),
            // Always present, like `truncated`: a walk that stopped descending at the depth bound
            // searched a subset of the tree, and a reply that said nothing about it reads exactly
            // like one that searched everything.
            depth_bound_reached: walked.bounded,
            skipped_large_files: grep.skipped_large_files,
            skipped_large_paths: grep.skipped_large_paths,
            // Echoed only when they were asked for, so a reader of the record can tell a literal
            // search from a regular expression without holding the call beside the answer.
            regex: regex.then_some(true),
            glob: input.glob.clone(),
            context: capped.then_some(MAX_SEARCH_CONTEXT),
            note,
            deadline_reached,
        })
    }
}

/// One search's walk: what it was asked, what it has answered so far and what it passed over.
struct Grep<'a> {
    search: &'a Search,
    /// Where the walk started, as resolved: a file the caller named as `path`.
    start: &'a Path,
    pattern: &'a str,
    expression: Option<&'a Regex>,
    filter: Option<&'a PathGlob>,
    limit: usize,
    context: usize,
    /// Lines are cut at [`MAX_MATCH_CHARS`].
    cut: bool,
    /// Files over [`MAX_GREP_FILE_BYTES`] are passed over.
    bounded: bool,
    /// Checked before every line; the walk checks it before every entry.
    deadline: &'a Deadline,
    matches: Vec<Match>,
    truncated: bool,
    skipped_large_files: u64,
    skipped_large_paths: Vec<String>,
}

impl Grep<'_> {
    /// Searches one file the walk reached. `false`, which stops the walk, once a match past the
    /// cap is found.
    fn visit(&mut self, file: &Path) -> bool {
        // The skipped names are skipped whatever the entry is. The walk skips a directory so
        // named; a file so named (a worktree's `.git` file) reaches here, and is not searched
        // either, unless the caller named it as `path`. `tool-fs`'s walk is not changed for it:
        // `fs` v1alpha1 says it skips the *directories* of those names.
        if file != self.start
            && !self.search.mutated_by(Mutation::NoSkippedFileNames)
            && file
                .file_name()
                .is_some_and(|name| SKIPPED.contains(&name.to_string_lossy().as_ref()))
        {
            return true;
        }
        let shown = self.search.display(file);
        if self.filter.is_some_and(|glob| !glob.matches(&shown)) {
            return true;
        }
        let Ok(metadata) = file.metadata() else {
            return true;
        };
        // Only regular files are opened: opening a named pipe for reading blocks until a writer
        // comes, and none will.
        if !metadata.is_file() {
            return true;
        }
        if self.bounded && metadata.len() > MAX_GREP_FILE_BYTES {
            self.skipped_large_files = self.skipped_large_files.saturating_add(1);
            if self.skipped_large_paths.len() < MAX_SKIPPED_PATHS {
                self.skipped_large_paths.push(shown);
            }
            // A match may be in it, so this is not the whole answer.
            self.truncated = true;
            return true;
        }
        // Binary or unreadable. Skipping is right; reporting each one is noise. Binary is not
        // UTF-8, or valid UTF-8 that holds a NUL byte: text does not, and a match answered from
        // such a file would carry its NUL bytes.
        let Ok(text) = fs::read_to_string(file) else {
            return true;
        };
        if text.as_bytes().contains(&0) && !self.search.mutated_by(Mutation::NulIsText) {
            return true;
        }
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            // A file of 1 MiB of short lines is the long part of a search, so the deadline is
            // checked here and not only between files.
            if self.deadline.passed() {
                return false;
            }
            if !self.hits(line) {
                continue;
            }
            // The cap is tested on matches, not on lines or files: exactly `max_results` matches
            // followed by lines that do not match is a complete answer, and `truncated` says *a
            // match past the cap exists*. harness 0.14.1 tested it before every line and file.
            if self.matches.len() >= self.limit {
                self.truncated = true;
                return false;
            }
            let mut found = Match {
                path: shown.clone(),
                line: number(index),
                text: self.cut_to(line),
                // Always present, so the shape is stable and a reader who checks the flag never
                // has to wonder whether its absence means "whole".
                line_truncated: over(line),
                before: None,
                after: None,
            };
            // Only when context was asked for. Two empty lists on every match of every search
            // would be bytes replayed on every later turn to say *nothing here*.
            if self.context > 0 {
                let after_end = index
                    .saturating_add(1)
                    .saturating_add(self.context)
                    .min(lines.len());
                found.before = Some(self.around(&lines, index.saturating_sub(self.context)..index));
                found.after = Some(self.around(&lines, index + 1..after_end));
            }
            self.matches.push(found);
        }
        true
    }

    fn hits(&self, line: &str) -> bool {
        match self.expression {
            Some(expression) => expression.is_match(line),
            None => line.contains(self.pattern),
        }
    }

    /// A line, cut at [`MAX_MATCH_CHARS`] on a character boundary.
    fn cut_to(&self, line: &str) -> String {
        if self.cut {
            line.chars().take(MAX_MATCH_CHARS).collect()
        } else {
            line.to_owned()
        }
    }

    /// The lines of `range`, each under its own number, for the context around a match.
    fn around(&self, lines: &[&str], range: Range<usize>) -> Vec<ContextLine> {
        lines[range.clone()]
            .iter()
            .zip(range)
            .map(|(line, index)| ContextLine {
                line: number(index),
                text: self.cut_to(line),
                line_truncated: over(line),
            })
            .collect()
    }
}

/// Whether `line` is longer than [`MAX_MATCH_CHARS`] characters, so its answer is cut.
fn over(line: &str) -> bool {
    line.chars().nth(MAX_MATCH_CHARS).is_some()
}

/// A line's number, counted from 1, from its index.
fn number(index: usize) -> u64 {
    u64::try_from(index).map_or(u64::MAX, |index| index.saturating_add(1))
}

/// Compiles a caller-supplied regular expression once, before anything is walked.
///
/// Both program limits are explicit: the crate defaults would let one pattern choose far more
/// memory. A compile failure is named instead of being misreported as an empty search result.
fn search_expression(pattern: &str, enabled: bool) -> Result<Option<Regex>, SearchError> {
    if !enabled {
        return Ok(None);
    }
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map(Some)
        .map_err(|source| SearchError::InvalidPattern {
            pattern: pattern.to_owned(),
            source,
        })
}

/// Compiles `glob` by `fs.glob`'s rule. An empty one is refused: it names no file, and a search
/// narrowed by it would answer no match, which reads as *the text is not in the tree*.
fn compile_glob(glob: &str) -> Result<PathGlob, SearchError> {
    if glob.is_empty() {
        return Err(SearchError::EmptyGlob);
    }
    PathGlob::compile(glob).map_err(|source| SearchError::InvalidGlob {
        glob: glob.to_owned(),
        source,
    })
}
