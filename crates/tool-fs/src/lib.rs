//! The `fs` toolset: `fs.read`, `fs.list`, `fs.glob` and `fs.stat` over one workspace root, under
//! the `tool-contract/1` contract in `contracts/fs/v1alpha1`.
//!
//! This crate is harness's `crates/harness-tools/src/local.rs` reader at `798325f` (harness
//! 0.13.3) moved here: its constants, `open`, `resolve`, `display`, `list`, `open_read`, `read`,
//! `find_paths` (now [`Fs::glob`]), `PathGlob`, `walk`, `Line`, `next_line`, `whole_prefix` and
//! `contained`, with the same reply fields, bounds and refusals, and its tests for reads,
//! listings, globs, links and line handling. Added here: the whole-file `sha256` a read carries,
//! computed in the one pass that counts the lines; [`Fs::stat`]; typed inputs and outputs
//! ([`types`]) whose `schemars` schemas the conformance run checks against the contract;
//! [`FsError`], each variant named by one contract error; the [`tool_contract::Toolset`]
//! implementation; and the mutants the suite is checked against. Left in harness: `write`, `edit`
//! and `resolve_new` (the code-editor toolset), `grep`, `exec` and the `Operations` trait.
//!
//! The walk is shared, not copied: [`walk`](mod@walk) is public so a toolset that reads the tree
//! (the `search` toolset's `search.grep`) walks it exactly as `fs.glob` does, with the same
//! skipped directories, depth bound, containment re-check and glob rule, and a fix to the walk
//! reaches both.
//!
//! # Where it differs from harness 0.13.3
//!
//! Four corrections, each pinned by a scenario or a test. A glob's `truncated` is set only when
//! a *matching* path is found past `max_results`; harness tested the cap on every visited file,
//! so exactly `max_results` matches followed by a file the glob did not name answered `true`.
//! The line count charges a separator only where the file has one and stops once *more than*
//! 16 MiB have been scanned, so a file of at most 16 MiB is counted and digested whole; and the
//! scan is bounded in **bytes read**, not in lines seen, so a single line longer than the bound
//! stops it too (harness tested the bound only between lines, so a file over it with no newline
//! was scanned whole). `limit: 0` is refused as `fs.InvalidWindow` rather than answered as an
//! empty window, and the path is judged before the window, so a bad window on a path outside the
//! workspace is `fs.OutsideWorkspace`. A path through a file (`a.txt/x`) is `fs.NotFound`, not
//! an I/O error. The first line's cut at the byte ceiling leaves room for a separator only where
//! the line has one.
//!
//! # The boundary, and what it is not
//!
//! Every path check here is this crate's own: [`Fs::new`] canonicalises the root, each operation
//! canonicalises its path and then checks containment, and the walk re-checks every entry it
//! descends into. That is real and it is not confinement: nothing stops the process, only these
//! functions, and a bug in them is the whole boundary. It is the right reader for a run against
//! an operator's own tree; a run that wants the effects confined puts a confined root under it.

mod error;
mod line;
mod toolset;
pub mod types;
pub mod walk;

#[cfg(test)]
mod tests;

use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufReader, Read as _};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub use error::FsError;
pub use toolset::{CONTRACT_YAML, contract};
pub use types::{
    Entry, GlobInput, GlobOutput, Kind, ListInput, ListOutput, ReadInput, ReadOutput, StatInput,
    StatOutput, Window,
};

use line::{Line, next_line, whole_prefix};
use walk::{PathGlob, walk};

/// The most entries one listing answers.
const MAX_LIST_ENTRIES: usize = 500;
/// The byte ceiling of a read window when the caller names none.
const MAX_READ_BYTES: u64 = 64 * 1024;
/// The most a caller may raise the byte ceiling to.
const MAX_READ_BYTES_CEILING: u64 = 256 * 1024;
/// The most paths one glob answers with.
///
/// Larger than a listing's page would suggest, because a glob is already the filter: a glob of
/// `**/*.rs` in a real workspace is the answer, not a page of it. Above this the reply says it
/// was cut and the caller narrows the glob.
const MAX_FIND_RESULTS: usize = 500;
/// How much of one line a read answers with.
///
/// A minified bundle is one line of two megabytes, and a window of ten lines that included it
/// would be the whole read. The line is cut and its number is reported, so a model that quoted
/// it back to an edit is refused for not matching rather than left to wonder.
const MAX_READ_LINE_CHARS: usize = 2_000;
/// How many bytes of one line are held while it is being read.
///
/// Four bytes per character is the widest UTF-8 gets, so this holds at least
/// [`MAX_READ_LINE_CHARS`] characters whatever the encoding, and a file with no newline in it at
/// all cannot make a read allocate its whole length.
const MAX_READ_LINE_BYTES: usize = MAX_READ_LINE_CHARS * 4;
/// How far a read scans while counting the lines of a file, inclusive, and the largest file
/// `fs.stat` digests: a file of at most this many bytes is counted and digested whole.
///
/// `lines.total` is what stops a window being mistaken for a whole file, and counting lines means
/// reading to the end. On a multi-gigabyte artefact that is a full sequential scan on every
/// read, bounded by nothing: the window itself is over after 256 KiB at the most.
///
/// So the scan stops once it has consumed more than this many bytes, whatever line it is in (a
/// single line longer than this is cut by it too), and the reply says so rather than answering
/// a number it did not finish: `lines.total` and `sha256` are `None` and `lines_counted_to`
/// names the last line the scan reached. Sixteen mebibytes is far past any source file and far short of a build
/// artefact, and a `None` there is absence kept as absence rather than a count that would be
/// wrong.
const MAX_LINE_COUNT_BYTES: u64 = 16 * 1024 * 1024;

/// One property switched off, for the mutant check the conformance suite is held to.
///
/// Only a mutant build (`--features mutants`, or the crate's own tests) can set one, through
/// `Fs::mutated`; outside it every [`Fs`] is intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    /// The byte ceiling of a read is not applied.
    NoReadCap,
    /// `truncated` is always `false`.
    NoTruncatedFlag,
    /// A read's `sha256` is the digest of the answered text instead of the whole file.
    PageDigest,
    /// A path that lands outside the workspace is accepted.
    NoContainment,
    /// A caller's `max_bytes` above the 262,144 ceiling is not capped.
    NoCeilingCap,
}

impl Mutation {
    /// Every mutation, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::NoReadCap,
        Self::NoTruncatedFlag,
        Self::PageDigest,
        Self::NoContainment,
        Self::NoCeilingCap,
    ];

    /// The mutation's name in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoReadCap => "no-read-cap",
            Self::NoTruncatedFlag => "no-truncated-flag",
            Self::PageDigest => "page-digest",
            Self::NoContainment => "no-containment",
            Self::NoCeilingCap => "no-ceiling-cap",
        }
    }
}

/// A bounded, read-only view of one directory.
#[derive(Debug, Clone)]
pub struct Fs {
    root: PathBuf,
    #[cfg(any(test, feature = "mutants"))]
    mutation: Option<Mutation>,
}

impl Fs {
    /// Opens `root` as the only directory these operations can see.
    ///
    /// # Errors
    ///
    /// `root` cannot be canonicalised, or is not a directory.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, FsError> {
        Ok(Self {
            root: Self::open(root)?,
            #[cfg(any(test, feature = "mutants"))]
            mutation: None,
        })
    }

    /// The same view with one property switched off, for the mutant check.
    ///
    /// # Errors
    ///
    /// As [`Fs::new`].
    #[cfg(any(test, feature = "mutants"))]
    pub fn mutated(root: impl AsRef<Path>, mutation: Mutation) -> Result<Self, FsError> {
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

    fn open(root: impl AsRef<Path>) -> Result<PathBuf, FsError> {
        let given = root.as_ref().display().to_string();
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|source| FsError::Workspace {
                path: given.clone(),
                source,
            })?;
        if !root.is_dir() {
            return Err(FsError::WorkspaceNotADirectory { path: given });
        }
        Ok(root)
    }

    /// The canonical workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a caller-supplied path inside the workspace.
    ///
    /// Canonicalisation happens before the containment check, so a symlink or a `..` that leaves
    /// the workspace is refused by where it *lands*, not by how it is spelled. A path that does
    /// not exist is refused by where it *would* land: its deepest existing ancestor is
    /// canonicalised, and one outside the workspace is `OutsideWorkspace` rather than `NotFound`,
    /// so `../x` is refused for climbing out whether or not there is an `x`.
    fn resolve(&self, relative: &str) -> Result<PathBuf, FsError> {
        let candidate = self.root.join(relative);
        let outside = |resolved: &Path| {
            !self.mutated_by(Mutation::NoContainment) && !resolved.starts_with(&self.root)
        };
        match candidate.canonicalize() {
            Ok(canonical) => {
                if outside(&canonical) {
                    return Err(FsError::OutsideWorkspace {
                        path: relative.to_owned(),
                    });
                }
                Ok(canonical)
            }
            Err(error) => {
                let mut ancestor = candidate.as_path();
                let lands = loop {
                    if let Ok(resolved) = ancestor.canonicalize() {
                        break Some(resolved);
                    }
                    match ancestor.parent() {
                        Some(parent) => ancestor = parent,
                        None => break None,
                    }
                };
                if lands.as_deref().is_none_or(outside) {
                    return Err(FsError::OutsideWorkspace {
                        path: relative.to_owned(),
                    });
                }
                Err(FsError::io(relative, error))
            }
        }
    }

    fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// Opens one resolved file and returns the metadata every read reply carries.
    fn open_read(&self, relative: &str) -> Result<(PathBuf, u64, BufReader<fs::File>), FsError> {
        let target = self.resolve(relative)?;
        if !target.is_file() {
            return Err(FsError::NotAFile {
                path: relative.to_owned(),
            });
        }
        let bytes = fs::metadata(&target)
            .map_err(|source| FsError::io(relative, source))?
            .len();
        let file = fs::File::open(&target).map_err(|source| FsError::io(relative, source))?;
        Ok((target, bytes, BufReader::new(file)))
    }

    /// `fs.read`: one window of one file, as numbered lines, with the digest of the whole file.
    ///
    /// # Errors
    ///
    /// The path is not a file, is absent, or lands outside the workspace; `offset` is 0 or past
    /// the end; the file cannot be read.
    ///
    /// # Why the reply is `cat -n` and not the file's own bytes
    ///
    /// What a model does with a read is quote part of it back to an edit, and an edit lands
    /// where the quoted text is. Numbering every line gives it a way to say *which* occurrence
    /// it means and gives a person reading the record a way to check. The number is a prefix
    /// the model strips, and the contract's guidance says so.
    ///
    /// # Why the file is walked past the window, and how far
    ///
    /// `lines.total` is the thing that keeps a window from being mistaken for a file: without
    /// it a model that read lines 1..64 of a 4,000-line file has no way to know there is more.
    /// Counting them means reading past the window. It is sequential I/O and **one line is in
    /// memory at a time**, so a multi-gigabyte artefact is not pulled into memory; the digest
    /// of the whole file is taken from the same pass, so it costs no second read.
    ///
    /// It is still a scan, though, and on that artefact it is a scan of the whole thing on
    /// every read with nothing to stop it. So it stops once more than [`MAX_LINE_COUNT_BYTES`]
    /// have been scanned: past there
    /// `lines.total` and `sha256` are `None` and `lines_counted_to` says which line the scan
    /// reached. `bytes` is the file's own size from its metadata either way, so the size of the
    /// thing is never in doubt, only how many lines it holds.
    pub fn read(&self, input: &ReadInput) -> Result<ReadOutput, FsError> {
        let relative = input.path.as_str();
        let ceiling = if self.mutated_by(Mutation::NoReadCap) {
            u64::MAX
        } else if self.mutated_by(Mutation::NoCeilingCap) {
            input.max_bytes.unwrap_or(MAX_READ_BYTES)
        } else {
            input
                .max_bytes
                .unwrap_or(MAX_READ_BYTES)
                .min(MAX_READ_BYTES_CEILING)
        };
        // The path is judged before the window: a bad window on a path outside the workspace is
        // refused for the path, which is the refusal a governor reads.
        let (target, bytes, mut reader) = self.open_read(relative)?;
        let offset = input.offset.unwrap_or(1);
        if offset == 0 {
            return Err(FsError::OffsetZero {
                path: relative.to_owned(),
            });
        }
        // A `limit` of 0 would answer `to` one below `from` on any file, the shape the contract
        // reserves for an empty file read from line 1; it names no line, so it is refused.
        if input.limit == Some(0) {
            return Err(FsError::LimitZero {
                path: relative.to_owned(),
            });
        }

        let mut digest = Sha256::new();
        let mut scan = Scan::default();
        while let Some(line) =
            next_line(&mut reader, MAX_READ_LINE_BYTES, &mut digest, scan.budget())
                .map_err(|source| FsError::io(relative, source))?
        {
            if !scan.take(&line, offset, input.limit, ceiling) {
                break;
            }
        }
        let Scan {
            text,
            cut,
            total,
            kept,
            last,
            counting_bounded,
            byte_ceiling_cut,
            ..
        } = scan;

        // A window that starts past the end is a refusal and not an empty answer: the model
        // asked for something that is not there, and *no lines* would read as *the file is
        // empty*. Line 1 of an empty file is not that case: it is the file, answered whole.
        if offset > 1 && offset > total {
            let path = relative.to_owned();
            return Err(if counting_bounded {
                FsError::PastScan {
                    path,
                    offset,
                    total,
                }
            } else {
                FsError::PastEnd {
                    path,
                    offset,
                    total,
                }
            });
        }
        // The digest is of every byte the scan consumed, which is the whole file exactly when
        // the scan reached the end. A digest of the part it reached would read as the whole
        // file's, so like `lines.total` it is absent then.
        let sha256 = if counting_bounded {
            None
        } else if self.mutated_by(Mutation::PageDigest) {
            Some(hex(&Sha256::digest(text.as_bytes())))
        } else {
            Some(hex(&digest.finalize()))
        };
        // **The window stops before the file's last line, or a line in it was cut.** Not "the
        // model has seen all of this file": a window with an `offset` in the middle that ends
        // on the last line answers `false` and the lines before `offset` were never in it. A
        // one-line minified bundle whose only line was cut answers `true`, because reaching the
        // last line is not reading it. `true` as well when the line count was bounded: the scan
        // stopped before the end, so whether the window reached the last line is not known.
        let truncated = !self.mutated_by(Mutation::NoTruncatedFlag)
            && (counting_bounded || byte_ceiling_cut || last < total || !cut.is_empty());
        // Only where counting stopped: a field that says *nothing happened* on every ordinary
        // read is bytes replayed on every later turn to say nothing.
        let (lines_counted_to, note) = if counting_bounded {
            let note = format!(
                "line counting stopped after {MAX_LINE_COUNT_BYTES} bytes of `{}`, at line \
                 {total}, so how many lines it has is not known. `bytes` is still the whole \
                 file's size, and a window past line {total} is refused rather than guessed at.",
                self.display(&target)
            );
            (Some(total), Some(note))
        } else {
            (None, None)
        };
        Ok(ReadOutput {
            path: self.display(&target),
            bytes,
            text,
            lines: Window {
                from: offset,
                // One below `from` when the window answered nothing at all, which is only an
                // empty file read from line 1.
                to: if kept == 0 {
                    offset.saturating_sub(1)
                } else {
                    last
                },
                // `None` when the scan stopped: how many lines the file has is then unknown,
                // and a number that counted part of it would be read as the whole count.
                total: (!counting_bounded).then_some(total),
            },
            truncated,
            // Every line this reply cut, by number. Always present, so an empty list means
            // *nothing was cut*. No marker is put in the text itself: the text is what gets
            // quoted back to an edit, and a marker inside it would be quoted too.
            truncated_lines: cut,
            sha256,
            lines_counted_to,
            note,
        })
    }

    /// `fs.list`: the entries of one directory in name order, each with its own kind.
    ///
    /// # Errors
    ///
    /// The path is not a directory, is absent, or lands outside the workspace; the directory
    /// cannot be read.
    pub fn list(&self, input: &ListInput) -> Result<ListOutput, FsError> {
        let relative = input.path.as_deref().unwrap_or(".");
        let target = self.resolve(relative)?;
        if !target.is_dir() {
            return Err(FsError::NotADirectory {
                path: relative.to_owned(),
            });
        }
        let mut reader = fs::read_dir(&target)
            .map_err(|source| FsError::io(relative, source))?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        reader.sort_by_key(fs::DirEntry::file_name);
        let mut entries = Vec::new();
        let mut truncated = false;
        for entry in reader {
            if entries.len() >= MAX_LIST_ENTRIES {
                truncated = true;
                break;
            }
            // A link's own type is not its target's, so a link to a directory used to be listed
            // as a file with the length of the link text. Naming it a link says what it is
            // without this listing having to follow it, which is a decision for whoever opens
            // it.
            let kind = entry.file_type().map_or(Kind::Other, Kind::of);
            let bytes = if kind == Kind::File {
                entry.metadata().ok().map(|meta| meta.len())
            } else {
                None
            };
            entries.push(Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
                bytes,
            });
        }
        Ok(ListOutput {
            path: self.display(&target),
            entries,
            // Named, so the model knows the listing is partial rather than assuming it is whole.
            truncated: truncated && !self.mutated_by(Mutation::NoTruncatedFlag),
        })
    }

    /// `fs.glob`: the workspace-relative paths of the files under `path` that match `glob`.
    ///
    /// # Errors
    ///
    /// The glob is empty or does not compile; the path is absent or lands outside the workspace.
    pub fn glob(&self, input: &GlobInput) -> Result<GlobOutput, FsError> {
        // An empty glob matches no path, so a glob with one answered an empty list, which reads
        // as *there are no such files* rather than as *you named no pattern*.
        if input.glob.is_empty() {
            return Err(FsError::EmptyGlob);
        }
        let limit = input.max_results.map_or(MAX_FIND_RESULTS, |asked| {
            usize::try_from(asked)
                .unwrap_or(usize::MAX)
                .min(MAX_FIND_RESULTS)
        });
        let filter = PathGlob::compile(&input.glob)?;
        let root = self.resolve(input.path.as_deref().unwrap_or("."))?;

        let mut paths = Vec::new();
        let mut truncated = false;
        let walked = walk(&root, &self.root, 0, &mut |file| {
            let shown = self.display(file);
            if !filter.matches(&shown) {
                return true;
            }
            // The cap is tested on matches, not on visits: exactly `max_results` matches
            // followed by a file the glob does not name is a complete answer, and `truncated`
            // says *more matched than were answered*. harness 0.13.3 tested it on every visit.
            if paths.len() >= limit {
                truncated = true;
                return false;
            }
            paths.push(shown);
            true
        });
        Ok(GlobOutput {
            paths,
            truncated: truncated && !self.mutated_by(Mutation::NoTruncatedFlag),
            // Always present, like `truncated`. A glob says it lists every match under the
            // workspace; where the walk stopped descending, it listed the matches above a depth
            // instead, and nothing else in the reply would say so.
            depth_bound_reached: walked.bounded,
        })
    }

    /// `fs.stat`: the kind, size and digest of one entry.
    ///
    /// # Errors
    ///
    /// The path is absent or lands outside the workspace; its metadata cannot be read.
    pub fn stat(&self, input: &StatInput) -> Result<StatOutput, FsError> {
        let relative = input.path.as_str();
        // Where the path lands decides whether it is answered at all: a link out of the
        // workspace is refused here, before its own kind is looked at.
        let target = self.resolve(relative)?;
        let own = self.root.join(relative);
        let meta = own
            .symlink_metadata()
            .map_err(|source| FsError::io(relative, source))?;
        let kind = Kind::of(meta.file_type());
        if kind == Kind::Symlink {
            // The link's own place: its parent resolved, its own name kept, so a link is
            // answered as a link under the name the caller can list it by.
            let place = match (own.parent(), own.file_name()) {
                (Some(parent), Some(name)) => parent
                    .canonicalize()
                    .map_err(|source| FsError::io(relative, source))?
                    .join(name),
                _ => target,
            };
            return Ok(StatOutput {
                path: self.display(&place),
                kind,
                bytes: None,
                sha256: None,
            });
        }
        let bytes = (kind == Kind::File).then_some(meta.len());
        // At most the counting bound, for the reason a read's digest is: a file a read would
        // not finish scanning is not one this answers a digest for either.
        let sha256 = if kind == Kind::File && meta.len() <= MAX_LINE_COUNT_BYTES {
            Some(digest_file(&target).map_err(|source| FsError::io(relative, source))?)
        } else {
            None
        };
        Ok(StatOutput {
            path: self.display(&target),
            kind,
            bytes,
            sha256,
        })
    }
}

/// One read's pass over its file: what the window holds so far and where the scan is.
#[derive(Default)]
struct Scan {
    /// The numbered lines answered so far.
    text: String,
    /// The numbers of the lines cut so far.
    cut: Vec<u64>,
    /// Lines counted so far; the last one reached.
    total: u64,
    /// Lines answered so far.
    kept: u64,
    /// What the byte ceiling has been charged so far.
    answered_bytes: u64,
    /// Bytes of the file scanned so far, separators included.
    scanned: u64,
    /// The number of the last line answered.
    last: u64,
    /// The window has closed; lines are only counted now.
    closed: bool,
    /// The scan consumed more than [`MAX_LINE_COUNT_BYTES`] and stopped before the end of the
    /// file.
    counting_bounded: bool,
    /// The first line alone was over the ceiling and was cut to fit.
    byte_ceiling_cut: bool,
}

impl Scan {
    /// Counts `line` and answers it when it is in the window and under the bounds. `false` when
    /// the scan stops at the counting bound.
    fn take(&mut self, line: &Line, offset: u64, limit: Option<u64>, ceiling: u64) -> bool {
        self.total += 1;
        // A separator is charged only where the file has one: a last line without a newline is
        // a line, but there is no byte after it to count.
        self.scanned += line.length + u64::from(line.terminated);
        if self.total >= offset && !self.closed {
            self.admit(line, limit, ceiling);
        }
        // The scan is the whole cost of this call on a large file, so it is the thing that is
        // bounded, and it is bounded in bytes: `next_line` consumed no more than `budget`
        // allowed, and the check is made after every line whatever branch handled it, so a
        // bound-crossing line is counted (and shown, when it is in the window) and then the
        // scan stops. Checked after the line is counted, so `total` names a line that was
        // actually reached; `>` so that a file of exactly the bound is counted whole.
        if self.scanned > MAX_LINE_COUNT_BYTES {
            self.counting_bounded = true;
            return false;
        }
        true
    }

    /// How many more bytes the scan may consume before it is over the bound: one past it, so
    /// that the crossing is seen.
    fn budget(&self) -> u64 {
        (MAX_LINE_COUNT_BYTES + 1).saturating_sub(self.scanned)
    }

    /// Answers `line` when the window's bounds allow it, else closes the window.
    fn admit(&mut self, line: &Line, limit: Option<u64>, ceiling: u64) {
        // The window closes on whichever bound arrives first: the line count the caller asked
        // for, or the byte ceiling. The first line is always answered even when it is over the
        // ceiling on its own, because a read that answered nothing would look like an empty
        // file. The separator counts towards the ceiling where the line has one: it is a byte
        // of the file, and ignoring it would answer more than the ceiling allowed for a file of
        // short lines.
        let separator = u64::from(line.terminated);
        let weight = line.length + separator;
        let within_limit = limit.is_none_or(|count| self.kept < count);
        let within_ceiling = self.kept == 0 || self.answered_bytes + weight <= ceiling;
        if !(within_limit && within_ceiling) {
            self.closed = true;
            return;
        }
        let shown_bytes = if self.kept == 0 && weight > ceiling {
            self.byte_ceiling_cut = true;
            // The cut leaves room for the separator only where the line has one.
            let allowed = usize::try_from(ceiling.saturating_sub(separator))
                .unwrap_or(usize::MAX)
                .min(line.kept.len());
            &line.kept[..whole_prefix(&line.kept[..allowed])]
        } else {
            &line.kept
        };
        let content = String::from_utf8_lossy(shown_bytes);
        let shown: String = content.chars().take(MAX_READ_LINE_CHARS).collect();
        if self.byte_ceiling_cut || !line.whole || content.chars().count() > MAX_READ_LINE_CHARS {
            self.cut.push(self.total);
        }
        let _ = writeln!(self.text, "{:>6}\t{shown}", self.total);
        self.kept += 1;
        self.answered_bytes = self.answered_bytes.saturating_add(weight.min(ceiling));
        self.last = self.total;
    }
}

/// The SHA-256 of the file at `path` as lowercase hex, read in chunks.
fn digest_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex(&digest.finalize()))
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
