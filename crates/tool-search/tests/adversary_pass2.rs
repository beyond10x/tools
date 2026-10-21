//! The adversary's second pass on `story:toolset-search` (wave w3), against correction round 1:
//! the skipped names are now the rule for files as well as directories (F1), and a file that
//! holds a NUL byte is binary (F2).
//!
//! The correction's own cases put every skipped entry at the workspace root and the NUL byte at
//! byte 5007, inside the first 8000 bytes that git's own binary check reads. These hold the
//! sentences of `semantics.md` past those edges: a `.git` file below the root (a submodule's), a
//! NUL byte as the very last byte of the largest file a search reads, and the two kinds of binary
//! file answered alike on both sides of 1 MiB.

use std::fs;
use std::path::Path;

use tool_contract::conformance::{Outcome, Suite, Toolset, run};
use tool_search::{GrepInput, GrepOutput, MAX_GREP_FILE_BYTES, Search, contract};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn bound() -> usize {
    usize::try_from(MAX_GREP_FILE_BYTES).expect("the search bound fits usize")
}

fn grep(root: &Path, path: Option<&str>) -> GrepOutput {
    Search::new(root)
        .expect("the workspace opens")
        .grep(&GrepInput {
            path: path.map(str::to_owned),
            ..GrepInput::new("needle")
        })
        .expect("the search answers")
}

fn paths(output: &GrepOutput) -> Vec<&str> {
    output.matches.iter().map(|hit| hit.path.as_str()).collect()
}

/// Runs the whole suite and asserts that `name` passed, with the answer in the message when it
/// did not (the runner's detail names only the first difference).
fn passes(name: &str) {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha2/scenarios");
    let suite = Suite::read_dir(scenarios).expect("the search suite parses");
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root| {
        Box::new(Search::new(root).expect("the scenario workspace opens"))
    });
    assert!(report.refused.is_empty(), "{report}");
    let result = report
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the suite"));
    if result.passed {
        return;
    }
    let call = &suite
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .expect("the scenario is in the suite")
        .call;
    let answer = match Search::new(scratch.path().join(name))
        .expect("the scenario workspace opens")
        .invoke(&call.operation, &call.input)
    {
        Outcome::Ok(output) => output.to_string(),
        Outcome::Error { name, message, .. } => format!("{name}: {message}"),
    };
    panic!("{name}: {}\nanswered: {answer}", result.detail);
}

// semantics.md, The walk: "An entry named `.git`, ... is skipped, a directory or a file: ... a
// file so named (a worktree's `.git` file, a link of that name) is not searched either". A
// submodule's checkout has its `.git` file one or more directories below the root.
#[test]
fn a_submodules_git_file_below_the_root_is_skipped_too() {
    passes("adversary-a-submodules-git-file-below-the-root-is-skipped-too");
}

// semantics.md: "A `path` that names such an entry is searched, because it was asked for by name".
#[test]
fn a_skipped_file_below_the_root_named_as_path_is_searched() {
    passes("adversary-a-skipped-file-below-the-root-named-as-path-is-searched");
}

// semantics.md, Lines and matches: "a file is binary when it is not UTF-8, or when it holds a NUL
// byte anywhere, even as valid UTF-8". Anywhere: at the very start, past 64 KiB, and as the last
// byte of a file of exactly 1 MiB, the largest a search reads.
#[test]
fn a_nul_byte_anywhere_makes_a_file_binary_up_to_the_last_byte_of_the_largest_file_read() {
    let dir = scratch();
    let root = dir.path();
    let head = "needle\n";
    let mut last = String::with_capacity(bound());
    last.push_str(head);
    last.push_str(&"x".repeat(bound() - head.len() - 2));
    last.push('\n');
    last.push('\0');
    assert_eq!(last.len(), bound(), "exactly 1 MiB, so it is read");
    fs::write(root.join("last.dat"), &last).expect("written");
    fs::write(
        root.join("past-buffer.dat"),
        format!("needle\n{}\n\0\n", "x".repeat(70_000)),
    )
    .expect("written");
    fs::write(root.join("start.dat"), "\0\nneedle\n").expect("written");
    fs::write(root.join("text.txt"), "needle\n").expect("written");

    let value = grep(root, None);
    assert_eq!(paths(&value), ["text.txt"]);
    assert!(!value.truncated, "a binary file is not a cut in the answer");
    assert_eq!(value.skipped_large_files, 0);
    for named in ["last.dat", "past-buffer.dat", "start.dat"] {
        assert_eq!(
            grep(root, Some(named)).matches,
            [],
            "{named}, named as path"
        );
    }
}

// semantics.md: a binary file "is not searched and is not reported", and the NUL rule is "as a file
// that is not UTF-8 already is". Whatever a file over 1 MiB that is binary is answered as, the two
// kinds are answered alike, on both sides of the bound, walked or named as path.
#[test]
fn the_two_kinds_of_binary_file_are_answered_alike_on_both_sides_of_one_mebibyte() {
    let dir = scratch();
    let root = dir.path();
    let over = bound() + 1;
    let mut nul_big = format!("needle\n{}", "x".repeat(over - 8)).into_bytes();
    nul_big.push(0);
    let mut odd_big = format!("needle\n{}", "x".repeat(over - 8)).into_bytes();
    odd_big.push(0xff);
    assert_eq!((nul_big.len(), odd_big.len()), (over, over));
    fs::write(root.join("nul-big.dat"), nul_big).expect("written");
    fs::write(root.join("odd-big.dat"), odd_big).expect("written");
    fs::write(root.join("nul-small.dat"), b"needle\0\n").expect("written");
    fs::write(root.join("odd-small.dat"), b"needle\xff\n").expect("written");
    fs::write(root.join("text.txt"), "needle\n").expect("written");

    let value = grep(root, None);
    assert_eq!(paths(&value), ["text.txt"]);
    let named = |name: &str| value.skipped_large_paths.iter().any(|path| path == name);
    assert_eq!(
        named("nul-big.dat"),
        named("odd-big.dat"),
        "{:?}",
        value.skipped_large_paths
    );
    assert!(!named("nul-small.dat") && !named("odd-small.dat"));

    for (nul, odd) in [
        ("nul-big.dat", "odd-big.dat"),
        ("nul-small.dat", "odd-small.dat"),
    ] {
        let (nul_answer, odd_answer) = (grep(root, Some(nul)), grep(root, Some(odd)));
        assert_eq!(
            (
                nul_answer.matches.len(),
                nul_answer.truncated,
                nul_answer.skipped_large_files
            ),
            (
                odd_answer.matches.len(),
                odd_answer.truncated,
                odd_answer.skipped_large_files
            ),
            "{nul} and {odd}, named as path"
        );
    }
}
