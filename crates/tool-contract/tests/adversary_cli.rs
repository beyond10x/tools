//! `tool-contract validate <dir>` on a tree holding one accepted and one refused document, and on
//! a document whose `format` is another one.

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
fn one_accepted_and_one_refused_document_exit_1_and_the_accepted_one_still_prints_ok() {
    let dir = scratch();
    let good = dir.path().join("good");
    let bad = dir.path().join("bad");
    fs::create_dir_all(&good).expect("mkdir");
    fs::create_dir_all(&bad).expect("mkdir");
    fs::copy(
        here("tests/fixtures/notes/contract.yaml"),
        good.join("contract.yaml"),
    )
    .expect("copy");
    fs::copy(
        here("tests/defects/unknown-capability.yaml"),
        bad.join("contract.yaml"),
    )
    .expect("copy");
    let output = validate(dir.path());
    let shown = text(&output);
    assert_eq!(output.status.code(), Some(1), "{shown}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("ok ") && line.ends_with("good/contract.yaml")),
        "{shown}"
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.contains("bad/contract.yaml: notes.read: capability: ")),
        "{shown}"
    );
}

#[test]
fn a_document_of_another_format_exits_1_naming_the_path_and_the_expected_format() {
    let dir = scratch();
    let doc_dir = dir.path().join("other");
    fs::create_dir_all(&doc_dir).expect("mkdir");
    fs::write(
        doc_dir.join("contract.yaml"),
        "format: tool-contract/2\ntoolset: x\nversion: v1\nsummary: s\noperations: []\n",
    )
    .expect("write");
    let output = validate(dir.path());
    let shown = text(&output);
    assert_eq!(output.status.code(), Some(1), "{shown}");
    assert!(shown.contains("other/contract.yaml"), "{shown}");
    assert!(shown.contains("tool-contract/1"), "{shown}");
    assert!(shown.contains("tool-contract/2"), "{shown}");
}
