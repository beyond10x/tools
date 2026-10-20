//! The bounded walk under one directory ([`walk`]), the containment check every entry it descends
//! into is re-checked against ([`contained`]), and the glob a `fs.glob` call matches with
//! ([`PathGlob`]).

use std::fs;
use std::path::Path;

use crate::error::FsError;

/// How deep under its starting directory a walk descends before it stops.
///
/// A bound rather than none: a deep or cyclic tree would otherwise hold the run open. The figure
/// is in the contract's guidance, because a bound the model cannot read is one it cannot work
/// around, and `fs.glob` answers `depth_bound_reached` when it bit.
pub(crate) const MAX_WALK_DEPTH: usize = 12;

/// Directories skipped while walking. Each is either machine output or another tool's private
/// state, and including them buries the answer the person asked for.
///
/// Named one by one in the contract's guidance. "Build output and version-control directories
/// are skipped" tells a model that something was left out and not *what*, so a model looking for
/// a file under `target/` reads a complete-looking empty answer and concludes the file is not
/// there.
pub(crate) const SKIPPED: &[&str] = &[".git", "target", "node_modules", ".venv", "__pycache__"];

#[cfg(test)]
thread_local! {
    /// How many entries [`walk`] has taken from a directory listing on this thread, each one a
    /// `canonicalize` in [`contained`]. Test builds only, so a test can see where a capped glob
    /// stopped; the production walk carries no counter.
    pub(crate) static VISITED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What one [`walk`] frame reports to the frame above it.
#[derive(Clone, Copy, Default)]
pub(crate) struct Walked {
    /// [`MAX_WALK_DEPTH`] stopped a descent somewhere in the part of the tree walked.
    pub(crate) bounded: bool,
    /// `visit` answered `false`: nothing after that entry was walked, in this frame or any above.
    pub(crate) stopped: bool,
}

/// Walks files under `directory`, calling `visit` until it returns `false`.
///
/// Every entry is re-checked against `boundary`. Checking only the path the caller supplied is
/// not enough: `is_dir` and `read_dir` both follow symlinks, so a link inside the workspace
/// pointing anywhere else would be walked and its contents returned under a workspace-relative
/// name.
///
/// Depth is bounded so a deep or cyclic tree cannot hold the run open.
///
/// **Answers whether that bound stopped a descent**, which is why it returns [`Walked::bounded`].
/// A walk that stopped at [`MAX_WALK_DEPTH`] visited a subset of the tree, and a glob that
/// answered `truncated: false` over it would be a bound nothing reported. The caller puts it in
/// the reply as `depth_bound_reached`.
///
/// **A stop reaches every frame.** [`Walked::stopped`] is how a frame tells its parent that
/// `visit` said stop, and the parent returns at once. Returning from the current frame only left
/// every parent iterating, a `canonicalize` per remaining entry, after a capped glob's answer was
/// already complete; and a bound met after the stop is not reported, because nothing past the
/// stop is walked.
///
/// [`SKIPPED`] directories are not reported the same way: they are named in the contract's
/// guidance, so a model reads *which* directories are not walked before it calls rather than
/// afterwards.
pub(crate) fn walk(
    directory: &Path,
    boundary: &Path,
    depth: usize,
    visit: &mut dyn FnMut(&Path) -> bool,
) -> Walked {
    if depth > MAX_WALK_DEPTH {
        return Walked {
            bounded: true,
            stopped: false,
        };
    }
    if !contained(directory, boundary) {
        return Walked::default();
    }
    if directory.is_file() {
        return Walked {
            bounded: false,
            stopped: !visit(directory),
        };
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return Walked::default();
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut walked = Walked::default();
    for entry in entries {
        #[cfg(test)]
        VISITED.with(|visited| visited.set(visited.get() + 1));
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !contained(&path, boundary) {
            continue;
        }
        if path.is_dir() {
            if SKIPPED.contains(&name.as_ref()) {
                continue;
            }
            let below = walk(&path, boundary, depth + 1, visit);
            walked.bounded |= below.bounded;
            walked.stopped = below.stopped;
        } else {
            walked.stopped = !visit(&path);
        }
        if walked.stopped {
            return walked;
        }
    }
    walked
}

/// Whether `path` really lives inside `boundary`, following every link on the way.
pub(crate) fn contained(path: &Path, boundary: &Path) -> bool {
    path.canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(boundary))
}

/// One glob, matched the way a person means it.
///
/// **A glob with no `/` in it matches the file's name anywhere in the tree**; one with a `/`
/// matches the whole workspace-relative path, and `*` does not cross a directory separator
/// there. That is ripgrep's rule, taken rather than invented: a model that asks for `*.rs` means
/// every Rust file, and answering it with the four in the workspace root would read as *there are
/// four*.
pub(crate) struct PathGlob {
    matcher: globset::GlobMatcher,
    by_name: bool,
}

impl PathGlob {
    /// Compiles `pattern`.
    ///
    /// # Errors
    ///
    /// The glob does not compile, with globset's own words: a pattern nobody can match is refused
    /// rather than quietly matching nothing.
    pub(crate) fn compile(pattern: &str) -> Result<Self, FsError> {
        let matcher = globset::GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|source| FsError::InvalidGlob {
                glob: pattern.to_owned(),
                source,
            })?
            .compile_matcher();
        Ok(Self {
            matcher,
            by_name: !pattern.contains('/'),
        })
    }

    pub(crate) fn matches(&self, relative: &str) -> bool {
        let candidate = if self.by_name {
            relative.rsplit('/').next().unwrap_or(relative)
        } else {
            relative
        };
        self.matcher.is_match(candidate)
    }
}
