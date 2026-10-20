//! The adversary's cases for `story:fs-glob-stops-at-the-cap` (wave w2), each written against a
//! sentence of `contracts/fs/v1alpha1/semantics.md`, section `fs.glob`, before anything was run.
//!
//! The walk now returns as soon as the match past the cap is found. These cases hold the answer at
//! the edges of that stop: the 500-path ceiling, a stop inside the deepest directory the walk
//! enters, the depth bound met before, between and after it, skipped directories and links on
//! either side of it; and a property over seeded random trees that compares every capped answer
//! with one computed here from an uncapped walk of the same tree.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use tool_fs::{Fs, GlobInput, GlobOutput};

/// The depth the contract names: a file twelve directories down is answered, a thirteenth
/// directory is not entered.
const DEPTH_BOUND: usize = 12;
/// The most paths one glob answers when `max_results` is absent.
const CEILING: usize = 500;
/// The directories the contract names as skipped.
const SKIPPED: [&str; 5] = [".git", "target", "node_modules", ".venv", "__pycache__"];

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the directories are made");
    fs::write(path, "x").expect("the file is written");
}

fn chain(levels: std::ops::Range<usize>) -> String {
    levels
        .map(|level| format!("d{level}"))
        .collect::<Vec<_>>()
        .join("/")
}

fn glob(root: &Path, max_results: Option<u64>) -> GlobOutput {
    Fs::new(root)
        .expect("the workspace opens")
        .glob(&GlobInput {
            max_results,
            ..GlobInput::new("*.txt")
        })
        .expect("the glob answers")
}

fn answer(output: GlobOutput) -> (Vec<String>, bool, bool) {
    (output.paths, output.truncated, output.depth_bound_reached)
}

fn named(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

// semantics.md, fs.glob: "At most 500 paths are answered, fewer when max_results says so;
// truncated: true says a matching path was found past that count." The suite's scenario holds
// `$min_len: 500` only. Here the 501st match is in a second directory, so the stop crosses a
// frame, and a directory past the depth bound comes after it.
#[test]
fn w2_with_no_max_results_the_501st_match_stops_the_walk_at_exactly_500_paths() {
    let inside = scratch();
    for n in 0..300 {
        touch(&inside.path().join(format!("a/{n:03}.txt")));
    }
    for n in 0..201 {
        touch(&inside.path().join(format!("b/{n:03}.txt")));
    }
    touch(&inside.path().join("c").join(chain(0..13)).join("deep.txt"));
    let mut first_500: Vec<String> = (0..300).map(|n| format!("a/{n:03}.txt")).collect();
    first_500.extend((0..200).map(|n| format!("b/{n:03}.txt")));

    assert_eq!(
        answer(glob(inside.path(), None)),
        (first_500.clone(), true, false),
        "501 matches: the first 500 in walk order, cut, and c/ after the stop is never walked"
    );
    assert_eq!(
        answer(glob(inside.path(), Some(600))),
        (first_500.clone(), true, false),
        "a larger ask is capped at 500"
    );

    fs::remove_file(inside.path().join("b/200.txt")).expect("the 501st match is removed");
    assert_eq!(
        answer(glob(inside.path(), None)),
        (first_500, false, true),
        "exactly 500 matches is a complete answer, and with no stop c/ is walked to the bound"
    );
}

// semantics.md, fs.glob: "The walk stops descending at depth 12; depth_bound_reached: true says it
// did somewhere". A stop inside the twelfth directory, the deepest one whose files are answered,
// with its thirteenth directory after the stop, before it, and between the last answered path and
// the match that stops the walk.
#[test]
fn w2_a_stop_inside_the_deepest_directory_the_walk_enters() {
    let deepest = chain(1..13);
    let at = |name: &str| format!("{deepest}/{name}");

    let after = scratch();
    for name in ["a.txt", "b.txt", "z/c.txt"] {
        touch(&after.path().join(at(name)));
    }
    assert_eq!(
        answer(glob(after.path(), Some(1))),
        (vec![at("a.txt")], true, false),
        "the thirteenth directory comes after the stop"
    );
    assert_eq!(
        answer(glob(after.path(), Some(2))),
        (vec![at("a.txt"), at("b.txt")], false, true),
        "exactly two matches: no stop, so the thirteenth directory is reached and reported"
    );

    let before = scratch();
    for name in ["0/c.txt", "a.txt", "b.txt"] {
        touch(&before.path().join(at(name)));
    }
    assert_eq!(
        answer(glob(before.path(), Some(1))),
        (vec![at("a.txt")], true, true),
        "the thirteenth directory comes before the stop"
    );

    let between = scratch();
    for name in ["a.txt", "m/c.txt", "z.txt"] {
        touch(&between.path().join(at(name)));
    }
    assert_eq!(
        answer(glob(between.path(), Some(1))),
        (vec![at("a.txt")], true, true),
        "the thirteenth directory comes after the last answered path and before the stop"
    );
}

// semantics.md, fs.glob: "The walk skips the directories .git, target, node_modules, .venv and
// __pycache__, and re-checks every entry against the root: a link that leads out of the workspace
// is not descended into and a linked file outside is not answered, while a link that lands inside
// is answered under the link's own path." Each of those on either side of the stop: none of the
// skipped or outside ones counts towards the cap or reports the bound, and a link inside counts
// like the file it reaches.
#[test]
fn w2_skipped_directories_and_links_on_either_side_of_the_stop() {
    let outside = scratch();
    touch(&outside.path().join("p.txt"));
    touch(&outside.path().join("q.txt"));
    touch(&outside.path().join("loose.txt"));

    let inside = scratch();
    let root = inside.path();
    touch(&root.join(".git").join(chain(0..14)).join("x.txt"));
    touch(&root.join(".git/y.txt"));
    symlink(outside.path(), root.join("0out")).expect("a link to a directory outside");
    symlink(outside.path().join("loose.txt"), root.join("1loose.txt"))
        .expect("a link to a file outside");
    symlink(root.join("missing"), root.join("2nowhere.txt")).expect("a link that leads nowhere");
    touch(&root.join("a.txt"));
    touch(&root.join("real/keep.txt"));
    symlink(root.join("real"), root.join("alias")).expect("a link to a directory inside");
    touch(&root.join("node_modules/n.txt"));
    touch(&root.join("target").join(chain(0..14)).join("t.txt"));
    touch(&root.join("z").join(chain(0..14)).join("deep.txt"));

    assert_eq!(
        answer(glob(root, Some(1))),
        (named(&["a.txt"]), true, false),
        "the link inside is the match past the cap; nothing before a.txt counted"
    );
    assert_eq!(
        answer(glob(root, Some(2))),
        (named(&["a.txt", "alias/keep.txt"]), true, false),
        "real/keep.txt is the match past the cap"
    );
    let whole = (
        named(&["a.txt", "alias/keep.txt", "real/keep.txt"]),
        false,
        true,
    );
    assert_eq!(
        answer(glob(root, Some(3))),
        whole,
        "three matches, no stop: target/ is skipped and z/ is walked to the bound"
    );
    assert_eq!(answer(glob(root, None)), whole);
}

/// What an uncapped walk meets, in walk order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Met {
    /// A file the glob names, by its workspace-relative path.
    Match(String),
    /// A directory past the depth bound, not entered.
    Bound,
}

/// The walk the semantics describe, never stopped: entries in name order, depth first; the named
/// directories skipped; an entry that does not land inside the root not followed; a directory past
/// depth 12 not entered.
fn meet(root: &Path, directory: &Path, depth: usize, met: &mut Vec<Met>) {
    if depth > DEPTH_BOUND {
        met.push(Met::Bound);
        return;
    }
    let mut entries = fs::read_dir(directory)
        .expect("the directory is listed")
        .map(|entry| entry.expect("an entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let Ok(lands) = path.canonicalize() else {
            continue;
        };
        if !lands.starts_with(root) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if lands.is_dir() {
            if !SKIPPED.contains(&name.as_str()) {
                meet(root, &path, depth + 1, met);
            }
        } else if Path::new(&name).extension().is_some_and(|ext| ext == "txt") {
            let shown = path.strip_prefix(root).expect("under the root");
            met.push(Met::Match(shown.to_string_lossy().into_owned()));
        }
    }
}

/// The answer a glob capped at `cap` owes over what the uncapped walk met: the first `cap`
/// matches; `truncated` when one more was met; the bound reported where it was met before the
/// walk stopped, which is anywhere when it never stopped.
fn owed(met: &[Met], cap: usize) -> (Vec<String>, bool, bool) {
    let mut paths = Vec::new();
    let mut bound = false;
    for event in met {
        match event {
            Met::Bound => bound = true,
            Met::Match(path) if paths.len() < cap => paths.push(path.clone()),
            Met::Match(_) => return (paths, true, bound),
        }
    }
    (paths, false, bound)
}

/// xorshift64, so the trees are the same on every run and a failure names the tree that broke.
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        let bound = u64::try_from(bound).expect("the bound fits");
        usize::try_from(x % bound).expect("the draw fits")
    }

    fn pick(&mut self, from: &[&'static str]) -> &'static str {
        from[self.below(from.len())]
    }
}

/// Grows one random tree. Links inside point only at spine directories and their files, none of
/// which holds a link, so the only cycle a tree can have is the `loop` link to the root itself.
struct Grower {
    rng: Rng,
    outside: PathBuf,
    targets: Vec<PathBuf>,
    deferred: Vec<(PathBuf, String)>,
}

impl Grower {
    fn grow(&mut self, dir: &Path, level: usize) {
        fs::create_dir_all(dir).expect("the directory is made");
        let count = 1 + self.rng.below(4) + if level == 0 { 3 } else { 0 };
        for index in 0..count {
            let stem = format!("{}{index}", self.rng.pick(&["a", "b", "m", "z", "0", "_"]));
            let roll = self.rng.below(100);
            if roll < 35 {
                touch(&dir.join(format!("{stem}.txt")));
            } else if roll < 48 {
                touch(&dir.join(format!("{stem}.md")));
            } else if roll < 70 && level < 3 {
                self.grow(&dir.join(&stem), level + 1);
            } else if roll < 80 {
                self.spine(&dir.join(&stem));
            } else if roll < 84 {
                let skipped = dir.join(self.rng.pick(&SKIPPED));
                touch(&skipped.join(chain(0..14)).join("buried.txt"));
                touch(&skipped.join("near.txt"));
            } else if roll < 88 {
                symlink(&self.outside, dir.join(format!("{stem}out"))).expect("a link out");
            } else if roll < 91 {
                symlink(
                    self.outside.join("o1.txt"),
                    dir.join(format!("{stem}o.txt")),
                )
                .expect("a link to a file out");
            } else if roll < 94 {
                symlink(dir.join("missing"), dir.join(format!("{stem}gone.txt")))
                    .expect("a link that leads nowhere");
            } else {
                self.deferred.push((dir.to_path_buf(), stem));
            }
        }
    }

    fn spine(&mut self, top: &Path) {
        let length = 10 + self.rng.below(6);
        let bottom = top.join(chain(0..length));
        touch(&bottom.join("leaf.txt"));
        let middle = top.join(chain(0..length / 2));
        if self.rng.below(2) == 0 {
            touch(&middle.join("mid.txt"));
        }
        self.targets.push(middle);
        self.targets.push(bottom.join("leaf.txt"));
    }

    fn link_inside(&mut self) {
        for (dir, stem) in std::mem::take(&mut self.deferred) {
            if self.targets.is_empty() {
                return;
            }
            let target = self.targets[self.rng.below(self.targets.len())].clone();
            let name = if target.is_file() {
                format!("{stem}in.txt")
            } else {
                format!("{stem}in")
            };
            symlink(&target, dir.join(name)).expect("a link inside");
        }
    }
}

fn caps(matches: usize, rng: &mut Rng) -> Vec<usize> {
    if matches <= 60 {
        return (0..=matches + 1).collect();
    }
    let mut caps: Vec<usize> = (0..=5).chain(matches - 1..=matches + 1).collect();
    caps.extend((0..10).map(|_| rng.below(matches)));
    caps
}

// semantics.md, fs.glob, the whole section, as a property: over 150 seeded trees mixing plain
// files, directories, spines past the depth bound, skipped directories, links out, links in, links
// that lead nowhere and a link back to the root, every cap from 0 to one past the number of matches
// answers the first `cap` matches of the uncapped walk, `truncated` exactly when there is one more,
// and `depth_bound_reached` where the bound was met before the stop.
#[test]
fn w2_every_cap_over_seeded_random_trees_answers_what_the_uncapped_walk_owes() {
    let mut checked = 0;
    for tree in 0..150_u64 {
        let outside = scratch();
        touch(&outside.path().join("o1.txt"));
        touch(&outside.path().join("sub/o2.txt"));
        let inside = scratch();
        let root = inside.path().canonicalize().expect("the root resolves");
        let mut grower = Grower {
            rng: Rng(0x9E37_79B9_7F4A_7C15 ^ (tree + 1).wrapping_mul(0x2545_F491_4F6C_DD1D)),
            outside: outside.path().to_path_buf(),
            targets: Vec::new(),
            deferred: Vec::new(),
        };
        grower.grow(&root, 0);
        grower.link_inside();
        if grower.rng.below(6) == 0 {
            symlink(&root, root.join("loop")).expect("a link back to the root");
        }

        let mut met = Vec::new();
        meet(&root, &root, 0, &mut met);
        let matches = met.iter().filter(|event| **event != Met::Bound).count();
        if matches >= 450 {
            continue;
        }
        checked += 1;
        assert_eq!(
            answer(glob(&root, None)),
            owed(&met, CEILING),
            "tree {tree}, no max_results: {met:?}"
        );
        for cap in caps(matches, &mut grower.rng) {
            let asked = u64::try_from(cap).expect("the cap fits");
            assert_eq!(
                answer(glob(&root, Some(asked))),
                owed(&met, cap),
                "tree {tree}, max_results {cap}: {met:?}"
            );
        }
    }
    println!("{checked} of 150 trees checked");
    assert!(
        checked >= 100,
        "only {checked} of 150 trees were small enough to check"
    );
}
