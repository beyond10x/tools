//! `story:fs-climb-through-missing-refused`: a path that climbs out of the workspace through a
//! component that does not exist (`missing/../../x`) is refused as `fs.OutsideWorkspace`, the
//! refusal `../x` gets, and not as `fs.NotFound`.
//!
//! When the path does not canonicalise, `resolve` looks for its deepest existing ancestor. Found
//! with `Path::parent`, which is lexical, that ancestor of `missing/../../x` is the root itself,
//! inside, so the climb was answered `fs.NotFound`. The direct case puts a real `x` beside the
//! root so the path would land on a file; the scenario holds the same refusal in the suite, and
//! the mutant that keeps the lexical walk fails it.

use std::fs;
use std::path::{Path, PathBuf};

use tool_contract::conformance::{Report, Suite, run};
use tool_fs::{Fs, Mutation, ReadInput, contract};

/// The scenario under `contracts/fs/v1alpha2/scenarios/` that holds the refusal.
const SCENARIO: &str = "a-climb-through-a-missing-directory-is-refused-as-outside-the-workspace";

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/fs/v1alpha2")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn suite_report(mutation: Option<Mutation>) -> Report {
    let suite = Suite::read_dir(contract_dir().join("scenarios")).expect("the fs suite parses");
    let scratch = scratch();
    run(&suite, contract(), scratch.path(), |root: &Path| {
        let toolset = match mutation {
            None => Fs::new(root),
            Some(mutation) => Fs::mutated(root, mutation),
        };
        Box::new(toolset.expect("the scenario workspace opens"))
    })
}

/// A scratch directory holding `root/` as the workspace, with `root/a.txt` inside it and `x`
/// beside it.
fn workspace_with_x_beside() -> (tempfile::TempDir, PathBuf) {
    let dir = scratch();
    let root = dir.path().join("root");
    fs::create_dir(&root).expect("the workspace root is made");
    fs::write(root.join("a.txt"), b"in\n").expect("the file inside is written");
    fs::write(dir.path().join("x"), b"out\n").expect("the file beside the root is written");
    (dir, root)
}

fn refusal(local: &Fs, path: &str) -> &'static str {
    match local.read(&ReadInput::whole(path)) {
        Ok(out) => panic!("{path} was read: {:?}", out.text),
        Err(refusal) => refusal.name(),
    }
}

#[test]
fn a_climb_through_a_missing_component_is_refused_as_outside_the_workspace() {
    let (_dir, root) = workspace_with_x_beside();
    let local = Fs::new(&root).expect("the workspace opens");

    assert_eq!(
        refusal(&local, "missing/../../x"),
        "fs.OutsideWorkspace",
        "missing/../../x lands on the x beside the root"
    );
    assert_eq!(refusal(&local, "../x"), "fs.OutsideWorkspace");
    assert_eq!(
        refusal(&local, "missing/x"),
        "fs.NotFound",
        "missing/x lands inside the root and names nothing"
    );
}

#[test]
fn the_scenario_holds_the_refusal() {
    let report = suite_report(None);
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let result = report
        .scenarios
        .iter()
        .find(|s| s.name == SCENARIO)
        .unwrap_or_else(|| panic!("{SCENARIO} is not a scenario of the suite"));
    assert!(result.passed, "FAIL {SCENARIO}: {}", result.detail);
}

#[test]
fn the_lexical_walk_is_caught_by_the_scenario() {
    let (_dir, root) = workspace_with_x_beside();
    let mutant = Fs::mutated(&root, Mutation::LexicalWalk).expect("the workspace opens");
    assert_eq!(
        refusal(&mutant, "missing/../../x"),
        "fs.NotFound",
        "the mutant is the walk this story corrects"
    );

    let report = suite_report(Some(Mutation::LexicalWalk));
    println!("{}:\n{report}", Mutation::LexicalWalk.name());
    let failed: Vec<&str> = report.failed().map(|s| s.name.as_str()).collect();
    assert!(
        failed.contains(&SCENARIO),
        "{}: expected {SCENARIO} to fail, failed: {failed:?}",
        Mutation::LexicalWalk.name()
    );
}
