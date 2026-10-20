//! `tool-contract validate <dir>`: exit 0 and `ok <path>` on the fixture, exit 1 naming the
//! operation and the property on each seeded defect, a sentence and exit 0 on a directory with no
//! documents, exit 1 when a sibling `scenarios/` leaves a property without a scenario.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn here(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn validate(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tool-contract"))
        .arg("validate")
        .arg(dir)
        .output()
        .expect("the binary runs")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn validate_accepts_the_fixture_directory_and_prints_ok_per_document() {
    let output = validate(&here("tests/fixtures"));
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("ok "), "{stdout}");
    assert!(stdout.contains("notes/contract.yaml"), "{stdout}");
}

#[test]
fn validate_exits_1_on_each_seeded_defect_naming_operation_and_property() {
    let defects = [
        ("risk-below-effects.yaml", "notes.write", "risk"),
        ("bound-without-max.yaml", "notes.read", "bounds"),
        ("unknown-subject-field.yaml", "notes.read", "subjects"),
        ("unknown-capability.yaml", "notes.read", "capability"),
    ];
    for (file, operation, property) in defects {
        let dir = scratch();
        let doc_dir = dir.path().join("notes");
        fs::create_dir_all(&doc_dir).expect("mkdir");
        fs::copy(
            here("tests/defects").join(file),
            doc_dir.join("contract.yaml"),
        )
        .expect("copy");
        let output = validate(dir.path());
        assert_eq!(output.status.code(), Some(1), "{file}: {}", text(&output));
        assert!(
            text(&output).contains(&format!("{operation}: {property}: ")),
            "{file}: {}",
            text(&output)
        );
    }
}

#[test]
fn validate_on_a_directory_with_no_documents_says_so_and_exits_0() {
    let dir = scratch();
    let output = validate(dir.path());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("no contract documents under {}\n", dir.path().display())
    );
}

#[test]
fn validate_exits_1_on_a_document_that_does_not_parse() {
    let dir = scratch();
    let doc_dir = dir.path().join("broken");
    fs::create_dir_all(&doc_dir).expect("mkdir");
    fs::write(
        doc_dir.join("contract.yaml"),
        "format: tool-contract/1\ntoolset: [\n",
    )
    .expect("write");
    let output = validate(dir.path());
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(text(&output).contains("contract.yaml"), "{}", text(&output));
}

#[test]
fn validate_refuses_a_sibling_scenarios_directory_that_leaves_a_property_uncovered() {
    let dir = scratch();
    let doc_dir = dir.path().join("notes");
    fs::create_dir_all(doc_dir.join("scenarios")).expect("mkdir");
    fs::copy(
        here("tests/fixtures/notes/contract.yaml"),
        doc_dir.join("contract.yaml"),
    )
    .expect("copy");
    let suite =
        fs::read_to_string(here("tests/fixtures/notes/scenarios/notes.yaml")).expect("read");
    let without_paging = suite.replace("holds: [paging, bounds,", "holds: [bounds,");
    assert_ne!(suite, without_paging);
    fs::write(doc_dir.join("scenarios/notes.yaml"), without_paging).expect("write");
    let output = validate(dir.path());
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    assert!(
        text(&output).contains("notes.read: paging"),
        "{}",
        text(&output)
    );
}

#[cfg(unix)]
#[test]
fn validate_does_not_follow_a_fixture_link_that_points_at_its_own_parent() {
    // A toolset's fixtures may hold a link back up the tree (`fixtures/linked/up -> ..`), which
    // the suites lay out on purpose. The walk that finds documents must not follow it: on the
    // merged contracts tree it recursed until the kernel answered ELOOP, and `validate` exited 1
    // on a tree whose every document was valid.
    let dir = scratch();
    let doc_dir = dir.path().join("notes");
    fs::create_dir_all(doc_dir.join("fixtures/linked")).expect("mkdir");
    fs::copy(
        here("tests/fixtures/notes/contract.yaml"),
        doc_dir.join("contract.yaml"),
    )
    .expect("copy");
    std::os::unix::fs::symlink("..", doc_dir.join("fixtures/linked/up")).expect("symlink");
    std::os::unix::fs::symlink("missing.txt", doc_dir.join("fixtures/linked/nowhere"))
        .expect("symlink");
    // A second document reachable through a link as well as directly: a walk that follows links
    // reports it twice, under two names, and the second name is a path no reader should see.
    let other_dir = dir.path().join("other");
    fs::create_dir_all(&other_dir).expect("mkdir");
    fs::copy(
        here("tests/fixtures/notes/contract.yaml"),
        other_dir.join("contract.yaml"),
    )
    .expect("copy");
    std::os::unix::fs::symlink("../../other", doc_dir.join("fixtures/alias")).expect("symlink");
    let output = validate(dir.path());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("ok ").count(), 2, "{stdout}");
    assert!(stdout.contains("notes/contract.yaml"), "{stdout}");
    assert!(stdout.contains("other/contract.yaml"), "{stdout}");
    assert!(!stdout.contains("alias"), "{stdout}");
}

#[test]
fn validate_accepts_a_sibling_scenarios_directory_that_covers_every_property() {
    let output = validate(&here("tests/fixtures/notes"));
    assert_eq!(output.status.code(), Some(0), "{}", text(&output));
}
