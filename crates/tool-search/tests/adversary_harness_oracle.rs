//! The adversary's property for `story:toolset-search` (wave w3): over seeded random trees and
//! calls, `search.grep` answers what harness 0.14.1's `grep` answers.
//!
//! The acceptance statement says a literal pattern, a regular expression, a glob filter and
//! context lines answer as harness's `search` does. The oracle below is harness's `grep`, `walk`,
//! `contained`, `resolve`, `display`, `search_expression`, `cut_to` and `around` from
//! `crates/harness-tools/src/local.rs` at 0.14.1, copied with their logic unchanged; only its
//! `PathGlob` is `tool-fs`'s, which is the same code and the only glob this crate can reach.
//!
//! The documented changes are kept out of the comparison rather than out of the trees: no empty
//! glob, no named pipe and no file holding a NUL byte (binary since correction round 1, F2) is
//! generated, and where the oracle's answer reached the result cap (where
//! harness's `truncated`, its stop and what it met after the cap differ by design) only the
//! matches and the echoed fields are compared. Everywhere else the two answers must be equal as
//! JSON, field for field. Trees carry links in and out, a link to the root, a dangling link,
//! skipped directories, a chain past the depth bound, files at both edges of 1 MiB, lines of 400
//! and 401 characters of two and four bytes, CRLF and lone `\r`, non-UTF-8 bytes, and a socket.

use std::fs;
use std::ops::Range;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tool_fs::walk::PathGlob;
use tool_search::{GrepInput, Search};

const MAX_GREP_RESULTS: usize = 200;
const MAX_GREP_FILE_BYTES: u64 = 1024 * 1024;
const MAX_GREP_DEPTH: usize = 12;
const MAX_SEARCH_CONTEXT: u64 = 5;
const MAX_MATCH_CHARS: usize = 400;
const SKIPPED: &[&str] = &[".git", "target", "node_modules", ".venv", "__pycache__"];

const SEED: u64 = 0x5EA2_C0DE_0014_0001;
const TREES: usize = 80;
const CALLS_PER_TREE: usize = 12;

/// harness's `SearchOptions`.
struct Options {
    regex: bool,
    glob: Option<String>,
    context: Option<u64>,
    max_results: Option<usize>,
}

/// harness's `LocalOperations`, as far as `grep` reaches.
struct Harness {
    root: PathBuf,
}

impl Harness {
    fn resolve(&self, relative: &str) -> Result<PathBuf, String> {
        let candidate = self.root.join(relative);
        let canonical = candidate
            .canonicalize()
            .map_err(|error| format!("`{relative}`: {error}"))?;
        if !canonical.starts_with(&self.root) {
            return Err(format!(
                "`{relative}` resolves outside the workspace and was refused"
            ));
        }
        Ok(canonical)
    }

    fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    #[allow(clippy::too_many_lines)]
    fn grep(&self, pattern: &str, relative: &str, options: &Options) -> Result<Value, String> {
        if pattern.is_empty() {
            return Err("`pattern` is required and must not be empty".to_owned());
        }
        let limit = options
            .max_results
            .unwrap_or(MAX_GREP_RESULTS)
            .min(MAX_GREP_RESULTS);
        let asked_context = options.context.unwrap_or(0);
        let context = usize::try_from(asked_context.min(MAX_SEARCH_CONTEXT)).unwrap_or(usize::MAX);
        let expression = search_expression(pattern, options.regex)?;
        let filter = options
            .glob
            .as_deref()
            .map(|glob| PathGlob::compile(glob).map_err(|error| error.to_string()))
            .transpose()?;
        let root = self.resolve(relative)?;

        let mut matches = Vec::new();
        let mut truncated = false;
        let mut skipped_large_files = 0_u64;
        let mut skipped_large_paths = Vec::new();
        let boundary = self.root.clone();
        let depth_bound_reached = walk(&root, &boundary, 0, &mut |file| {
            if matches.len() >= limit {
                truncated = true;
                return false;
            }
            let shown = self.display(file);
            if filter.as_ref().is_some_and(|glob| !glob.matches(&shown)) {
                return true;
            }
            let Ok(metadata) = file.metadata() else {
                return true;
            };
            if metadata.len() > MAX_GREP_FILE_BYTES {
                skipped_large_files = skipped_large_files.saturating_add(1);
                if skipped_large_paths.len() < 20 {
                    skipped_large_paths.push(shown);
                }
                truncated = true;
                return true;
            }
            let Ok(text) = fs::read_to_string(file) else {
                return true;
            };
            let lines: Vec<&str> = text.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if matches.len() >= limit {
                    truncated = true;
                    return false;
                }
                let hit = match &expression {
                    Some(expression) => expression.is_match(line),
                    None => line.contains(pattern),
                };
                if !hit {
                    continue;
                }
                let mut hit = json!({
                    "path": shown,
                    "line": index + 1,
                    "text": cut_to(line),
                    "line_truncated": line.chars().nth(MAX_MATCH_CHARS).is_some(),
                });
                if context > 0 {
                    hit["before"] = around(&lines, index.saturating_sub(context)..index);
                    hit["after"] =
                        around(&lines, index + 1..(index + 1 + context).min(lines.len()));
                }
                matches.push(hit);
            }
            true
        });

        let mut answer = json!({
            "pattern": pattern,
            "matches": matches,
            "truncated": truncated,
            "depth_bound_reached": depth_bound_reached,
            "skipped_large_files": skipped_large_files,
            "skipped_large_paths": skipped_large_paths,
        });
        if options.regex {
            answer["regex"] = json!(true);
        }
        if let Some(glob) = &options.glob {
            answer["glob"] = json!(glob);
        }
        if asked_context > MAX_SEARCH_CONTEXT {
            answer["context"] = json!(MAX_SEARCH_CONTEXT);
            answer["note"] = json!(format!(
                "`context` was {asked_context} and this build answers at most \
                 {MAX_SEARCH_CONTEXT} lines either side of a match, so {MAX_SEARCH_CONTEXT} is \
                 what these matches carry."
            ));
        }
        Ok(answer)
    }
}

fn search_expression(pattern: &str, enabled: bool) -> Result<Option<regex::Regex>, String> {
    if !enabled {
        return Ok(None);
    }
    regex::RegexBuilder::new(pattern)
        .size_limit(1 << 20)
        .dfa_size_limit(1 << 20)
        .build()
        .map(Some)
        .map_err(|error| {
            format!("`{pattern}` is not a regular expression this build can compile: {error}")
        })
}

fn cut_to(line: &str) -> String {
    line.chars().take(MAX_MATCH_CHARS).collect()
}

fn around(lines: &[&str], range: Range<usize>) -> Value {
    Value::Array(
        lines[range.clone()]
            .iter()
            .zip(range)
            .map(|(line, index)| {
                json!({
                    "line": index + 1,
                    "text": cut_to(line),
                    "line_truncated": line.chars().nth(MAX_MATCH_CHARS).is_some(),
                })
            })
            .collect(),
    )
}

fn contained(path: &Path, boundary: &Path) -> bool {
    path.canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(boundary))
}

fn walk(
    directory: &Path,
    boundary: &Path,
    depth: usize,
    visit: &mut dyn FnMut(&Path) -> bool,
) -> bool {
    if depth > MAX_GREP_DEPTH {
        return true;
    }
    if !contained(directory, boundary) {
        return false;
    }
    if directory.is_file() {
        visit(directory);
        return false;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut bounded = false;
    for entry in entries {
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
            bounded |= walk(&path, boundary, depth + 1, visit);
        } else if !visit(&path) {
            return bounded;
        }
    }
    bounded
}

/// xorshift64*: the trees and calls are the same on every machine and every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a small bound");
        usize::try_from(self.next() % bound).expect("below a small bound")
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

const DIRS: &[&str] = &[
    "",
    "src",
    "src/inner",
    "a",
    "a/b",
    "target/debug",
    ".git",
    "node_modules/m",
    "docs",
    "__pycache__",
];
const NAMES: &[&str] = &[
    "one.rs",
    "two.txt",
    "notes.md",
    "x.json",
    "Y.RS",
    ".hidden.rs",
    "needle.txt",
];
const LITERALS: &[&str] = &[
    "needle",
    "needle",
    "needle",
    "needle",
    "needle",
    "a.b",
    "fn (alpha",
    "\u{e9}",
    "NEEDLE",
    "needle needle",
    "x",
    "\u{1F600}",
];
const EXPRESSIONS: &[&str] = &[
    r"fn\s+(alpha|gamma)",
    "^needle$",
    "a.b",
    "(?i)needle",
    "\u{e9}+needle",
    r"needle\s*$",
    "^$",
    "fn (alpha",
    "(a{1000}){1000}",
];
const GLOBS: &[Option<&str>] = &[
    None,
    None,
    None,
    None,
    Some("*.rs"),
    Some("*.txt"),
    Some("src/*.rs"),
    Some("src/**"),
    Some("**/*.md"),
    Some("*"),
    Some("a/**"),
    Some("["),
];
const CONTEXTS: &[Option<u64>] = &[None, None, Some(0), Some(1), Some(2), Some(5), Some(9)];
const MAXES: &[Option<u64>] = &[None, None, Some(0), Some(1), Some(3), Some(250)];
const PATHS: &[Option<&str>] = &[
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("."),
    Some("src"),
    Some("a"),
    Some("a/b/.."),
    Some("src/one.rs"),
    Some("l.txt"),
    Some("ld"),
    Some("missing"),
    Some(".."),
    Some("/"),
    Some("d0/d1"),
    Some("out"),
    Some("out.txt"),
    Some("self/src"),
];

fn line(rng: &mut Rng) -> String {
    match rng.below(16) {
        0 => "needle".to_owned(),
        1 => "a.b".to_owned(),
        2 => "axb".to_owned(),
        3 => "fn alpha() {}".to_owned(),
        4 => "fn beta() {}".to_owned(),
        5 => "NEEDLE".to_owned(),
        6 => String::new(),
        7 => "needle needle".to_owned(),
        8 => "  let needle = 1;".to_owned(),
        9 => "\u{e9} needle \u{e9}".to_owned(),
        10 => "fn (alpha".to_owned(),
        11 => format!("{}needle", "\u{e9}".repeat(394)),
        12 => format!("needle{}", "\u{e9}".repeat(395)),
        13 => format!("{}needle{}", "a".repeat(500), "b".repeat(500)),
        14 => format!("needle{}", "\u{1F600}".repeat(395)),
        _ => "x".to_owned(),
    }
}

fn text(rng: &mut Rng) -> String {
    let ending = if rng.chance(25) { "\r\n" } else { "\n" };
    let count = 1 + rng.below(7);
    let mut text = (0..count)
        .map(|_| line(rng))
        .collect::<Vec<_>>()
        .join(ending);
    if rng.chance(80) {
        text.push_str(ending);
    } else if rng.chance(30) {
        text.push('\r');
    }
    text
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the directories are made");
    fs::write(path, bytes).expect("the file is written");
}

fn build(rng: &mut Rng, root: &Path, outside: &Path) {
    for _ in 0..3 + rng.below(10) {
        let dir = rng.pick(DIRS);
        let name = rng.pick(NAMES);
        let content = text(rng);
        write(&root.join(dir).join(name), content);
    }
    if rng.chance(25) {
        write(&root.join("src/many.txt"), "needle\n".repeat(30));
    }
    if rng.chance(30) {
        write(&root.join("bin/odd.dat"), b"needle\xff\xfe\n");
    }
    // A file holding a NUL byte is binary since correction round 1 (F2) and is not searched,
    // where harness searched it: a documented change, so none is generated. The draw stays, so
    // every tree after it is the one this property was first run on.
    let _ = rng.chance(30);
    if rng.chance(20) {
        // A socket on the walk: not a regular file, and harness's open of it fails at once.
        UnixListener::bind(root.join("sock")).expect("the socket is bound");
    }
    if rng.chance(20) {
        let mut deep = root.to_path_buf();
        for level in 0..14 {
            deep = deep.join(format!("d{level}"));
            if level >= 10 {
                write(&deep.join("deep.txt"), "needle\n");
            }
        }
    }
    if rng.chance(6) {
        let bound = usize::try_from(MAX_GREP_FILE_BYTES).expect("the bound fits");
        write(
            &root.join("big/exact.txt"),
            format!("needle {}", "x".repeat(bound - 7)),
        );
        write(
            &root.join("big/over.txt"),
            format!("needle {}", "x".repeat(bound - 6)),
        );
    }
    let links: [(u64, &str, PathBuf); 6] = [
        (40, "l.txt", PathBuf::from("src/one.rs")),
        (40, "ld", PathBuf::from("src")),
        (30, "out", outside.to_path_buf()),
        (30, "out.txt", outside.join("loose.txt")),
        (20, "gone", PathBuf::from("missing")),
        (8, "self", PathBuf::from(".")),
    ];
    for (percent, name, target) in links {
        if rng.chance(percent) {
            symlink(target, root.join(name)).expect("the link is made");
        }
    }
}

fn call(rng: &mut Rng) -> (GrepInput, Options, String) {
    let regex = rng.chance(35);
    let pattern = if regex {
        rng.pick(EXPRESSIONS)
    } else {
        rng.pick(LITERALS)
    };
    let glob = rng.pick(GLOBS).map(str::to_owned);
    let context = *rng.pick(CONTEXTS);
    let max_results = *rng.pick(MAXES);
    let path = rng.pick(PATHS).map(str::to_owned);
    let relative = path.clone().unwrap_or_else(|| ".".to_owned());
    let options = Options {
        regex,
        glob: glob.clone(),
        context,
        max_results: max_results.map(|most| usize::try_from(most).expect("a small figure")),
    };
    let input = GrepInput {
        path,
        regex: regex.then_some(true),
        glob,
        context,
        max_results,
        ..GrepInput::new(*pattern)
    };
    (input, options, relative)
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

/// The answers agree, or the call is one of the documented changes and the part they share agrees.
fn compare(case: &str, input: &GrepInput, harness: Result<Value, String>, ours: Option<Value>) {
    let (harness, mut ours) = match (harness, ours) {
        (Ok(harness), Some(ours)) => (harness, ours),
        (Err(_), None) => return,
        (harness, ours) => panic!("{case}: {input:?}: harness {harness:?}, search {ours:?}"),
    };
    // `deadline_reached` is v1alpha2's, which harness never answered: no seeded call comes near
    // the default deadline, so it is false here, and then the rest is compared as before.
    let reached = ours
        .as_object_mut()
        .and_then(|fields| fields.remove("deadline_reached"));
    assert_eq!(
        reached,
        Some(Value::Bool(false)),
        "{case}: {input:?}: deadline_reached"
    );
    let limit = input
        .max_results
        .map_or(MAX_GREP_RESULTS, |most| {
            usize::try_from(most).expect("a small figure")
        })
        .min(MAX_GREP_RESULTS);
    let answered = harness["matches"].as_array().map_or(0, Vec::len);
    if answered < limit {
        assert_eq!(
            ours,
            harness,
            "{case}: {input:?}\nsearch:  {}\nharness: {}",
            serde_json::to_string_pretty(&ours).expect("prints"),
            serde_json::to_string_pretty(&harness).expect("prints"),
        );
    } else {
        for field in ["pattern", "matches", "regex", "glob", "context", "note"] {
            assert_eq!(
                ours.get(field),
                harness.get(field),
                "{case}: {input:?}: {field} differs at the cap"
            );
        }
    }
}

#[test]
fn search_grep_answers_what_harness_grep_answers_over_seeded_trees() {
    let mut rng = Rng(SEED);
    let mut compared = 0_usize;
    let mut answered = 0_usize;
    let mut with_matches = 0_usize;
    for tree in 0..TREES {
        let outside = scratch();
        write(&outside.path().join("loose.txt"), "needle\n");
        write(&outside.path().join("private/secret.txt"), "needle\n");
        let dir = scratch();
        let root = dir.path().canonicalize().expect("the scratch resolves");
        build(&mut rng, &root, outside.path());
        let harness = Harness { root: root.clone() };
        let search = Search::new(&root).expect("the workspace opens");
        for index in 0..CALLS_PER_TREE {
            let (input, options, relative) = call(&mut rng);
            let theirs = harness.grep(&input.pattern, &relative, &options);
            let ours = search
                .grep(&input)
                .ok()
                .map(|output| serde_json::to_value(output).expect("the output serialises"));
            if let Some(value) = &ours {
                answered += 1;
                if value["matches"].as_array().is_some_and(|m| !m.is_empty()) {
                    with_matches += 1;
                }
            }
            compare(&format!("tree {tree} call {index}"), &input, theirs, ours);
            compared += 1;
        }
    }
    assert_eq!(compared, TREES * CALLS_PER_TREE);
    println!("{compared} calls, {answered} answered, {with_matches} with matches");
    assert!(
        answered > compared / 3 && with_matches > answered / 3,
        "the calls answer and find something often enough to be evidence: \
         {compared} calls, {answered} answered, {with_matches} with matches"
    );
}
