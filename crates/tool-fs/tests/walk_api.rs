//! The walk `tool-fs` shares with the other toolsets that read the tree (wave w3, D2: the search
//! toolset reuses it rather than a copy), used from outside the crate the way a dependent uses it:
//! [`walk`] and what it reports ([`Walked`]), the containment check ([`contained`]), the skipped
//! directories ([`SKIPPED`]), the depth bound ([`MAX_WALK_DEPTH`]) and the glob ([`PathGlob`]).

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use tool_fs::walk::{MAX_WALK_DEPTH, PathGlob, SKIPPED, Walked, contained, walk};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the directories are made");
    fs::write(path, "x").expect("the file is written");
}

fn canonical(dir: &tempfile::TempDir) -> PathBuf {
    dir.path()
        .canonicalize()
        .expect("the scratch directory resolves")
}

/// Every file the walk visits under `root`, workspace-relative, in visit order, and what it said.
fn visited(root: &Path, stop_after: Option<usize>) -> (Vec<String>, Walked) {
    let mut seen = Vec::new();
    let walked = walk(root, root, 0, &mut |file| {
        seen.push(
            file.strip_prefix(root)
                .expect("a file under the root")
                .to_string_lossy()
                .into_owned(),
        );
        stop_after.is_none_or(|most| seen.len() < most)
    });
    (seen, walked)
}

#[test]
fn the_walk_visits_files_depth_first_in_name_order_and_skips_every_skipped_directory() {
    let dir = scratch();
    let root = canonical(&dir);
    touch(&root.join("b.txt"));
    touch(&root.join("a/x.txt"));
    for skipped in SKIPPED {
        touch(&root.join(skipped).join("hidden.txt"));
    }
    let (seen, walked) = visited(&root, None);
    assert_eq!(seen, ["a/x.txt", "b.txt"]);
    assert_eq!(
        walked,
        Walked {
            bounded: false,
            stopped: false
        }
    );
}

#[test]
fn a_visit_that_says_stop_stops_every_frame_and_the_walk_says_it_stopped() {
    let dir = scratch();
    let root = canonical(&dir);
    touch(&root.join("d/e/one.txt"));
    touch(&root.join("d/e/two.txt"));
    touch(&root.join("d/later.txt"));
    touch(&root.join("z.txt"));
    let (seen, walked) = visited(&root, Some(1));
    assert_eq!(seen, ["d/e/one.txt"], "nothing after the stop is visited");
    assert!(walked.stopped);
    assert!(!walked.bounded);
}

#[test]
fn the_depth_bound_is_the_published_figure_and_the_walk_reports_where_it_bit() {
    let dir = scratch();
    let root = canonical(&dir);
    let chain = |levels: usize| {
        (0..levels)
            .map(|level| format!("d{level}"))
            .collect::<Vec<_>>()
            .join("/")
    };
    touch(&root.join(chain(MAX_WALK_DEPTH)).join("last.txt"));
    touch(&root.join(chain(MAX_WALK_DEPTH + 1)).join("past.txt"));
    let (seen, walked) = visited(&root, None);
    assert_eq!(
        seen,
        [format!("{}/last.txt", chain(MAX_WALK_DEPTH))],
        "a file {MAX_WALK_DEPTH} directories down is visited, one further is not"
    );
    assert!(walked.bounded);
    assert_eq!(
        MAX_WALK_DEPTH, 12,
        "the figure the contracts' guidance names"
    );
}

#[test]
fn the_skipped_directories_are_the_five_the_contracts_name() {
    assert_eq!(
        SKIPPED,
        [".git", "target", "node_modules", ".venv", "__pycache__"]
    );
}

#[test]
fn contained_answers_where_a_path_lands_with_every_link_followed() {
    let outside = scratch();
    let inside = scratch();
    let root = canonical(&inside);
    touch(&root.join("own.txt"));
    touch(&outside.path().join("loose.txt"));
    symlink(root.join("own.txt"), root.join("inner")).expect("a link inside");
    symlink(outside.path(), root.join("out")).expect("a link to a directory outside");
    symlink(outside.path().join("loose.txt"), root.join("out.txt")).expect("a link to a file");

    assert!(contained(&root.join("own.txt"), &root));
    assert!(contained(&root.join("inner"), &root));
    assert!(contained(&root, &root), "the root is inside itself");
    assert!(!contained(&root.join("out"), &root));
    assert!(!contained(&root.join("out/loose.txt"), &root));
    assert!(!contained(&root.join("out.txt"), &root));
    assert!(!contained(&root.join(".."), &root));
    assert!(
        !contained(&root.join("missing.txt"), &root),
        "a path that names nothing lands nowhere"
    );

    let (seen, walked) = visited(&root, None);
    assert_eq!(
        seen,
        ["inner", "own.txt"],
        "the walk re-checks every entry: neither the linked directory nor the linked file"
    );
    assert!(!walked.bounded);
}

#[test]
fn a_walk_started_on_a_file_visits_that_file_alone() {
    let dir = scratch();
    let root = canonical(&dir);
    touch(&root.join("a/one.txt"));
    touch(&root.join("a/two.txt"));
    let file = root.join("a/one.txt");
    let mut seen = Vec::new();
    let walked = walk(&file, &root, 0, &mut |path| {
        seen.push(path.to_path_buf());
        true
    });
    assert_eq!(seen, [file]);
    assert_eq!(walked, Walked::default());
}

#[test]
fn a_glob_without_a_separator_matches_the_name_and_one_with_a_separator_the_whole_path() {
    let by_name = PathGlob::compile("*.rs").expect("the glob compiles");
    assert!(by_name.matches("crates/x/src/lib.rs"));
    assert!(by_name.matches("top.rs"));
    assert!(!by_name.matches("notes.md"));

    let by_path = PathGlob::compile("src/*.rs").expect("the glob compiles");
    assert!(by_path.matches("src/one.rs"));
    assert!(
        !by_path.matches("src/inner/one.rs"),
        "`*` does not cross a separator"
    );
    assert!(!by_path.matches("one.rs"));

    let refusal = PathGlob::compile("crates/[").expect_err("the glob does not compile");
    assert_eq!(refusal.name(), "fs.InvalidGlob");
    assert!(refusal.to_string().contains("not a glob"), "{refusal}");
}
