//! Pass 2, the correction's new checks: a `call.input` key that is not an input field (refused by
//! name) and an omitted optional field (not refused); a returned error that is a declared outcome
//! (passes) and one that is not (fails the scenario whatever `expect` says); and the symlink
//! handling: a link whose target changes, a link replaced by a file holding its target's content,
//! `equals` through a link, and a dangling link under `unchanged` and `absent`.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use schemars::Schema;
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Report, ScenarioResult, Suite, Toolset, run};

fn here(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn contract() -> Contract {
    Contract::read(here("tests/fixtures/notes/contract.yaml")).expect("the fixture contract parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

#[derive(Clone, Copy)]
enum Tamper {
    Nothing,
    RepointLinkToB,
    ReplaceLinkWithFile,
}

// A toolset over one workspace: on `notes.read` it tampers with the tree as told, then answers
// with the canned outcome; every other call answers `ok {}`. It derives no schemas, so
// `Report::passed()` is never true for it; the cases look at `refused` and at the scenarios.
struct Tampering {
    contract: Contract,
    root: PathBuf,
    tamper: Tamper,
    read: Outcome,
}

impl Toolset for Tampering {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        Vec::new()
    }

    fn invoke(&self, operation: &str, _input: &Value) -> Outcome {
        if operation != "notes.read" {
            return Outcome::Ok(json!({}));
        }
        let link = self.root.join("link");
        match self.tamper {
            Tamper::Nothing => {}
            Tamper::RepointLinkToB => {
                fs::remove_file(&link).expect("the fixture laid the link out");
                std::os::unix::fs::symlink("b.txt", &link).expect("symlink");
            }
            Tamper::ReplaceLinkWithFile => {
                let text = fs::read(&link).expect("the link resolves to a.txt");
                fs::remove_file(&link).expect("the fixture laid the link out");
                fs::write(&link, text).expect("write");
            }
        }
        self.read.clone()
    }
}

fn run_tampering(suite: &Suite, scratch: &Path, tamper: Tamper, read: &Outcome) -> Report {
    let contract = contract();
    run(suite, &contract, scratch, |root| {
        Box::new(Tampering {
            contract: contract.clone(),
            root: root.to_path_buf(),
            tamper,
            read: read.clone(),
        })
    })
}

// Every behavioural property of both operations is held, so the suite is never refused for
// coverage; the read scenario's input, fixture and expectation are filled in per case.
const SUITE: &str = "\
format: tool-conformance/1
toolset: notes
version: v1alpha1
scenarios:
  - name: read
    holds: [effects, idempotency, determinism, bounds, paging, subjects, output]
    fixture: READ_FIXTURE
    call: {operation: notes.read, input: READ_INPUT}
    expect: READ_EXPECT
  - name: write
    holds: [effects, idempotency, determinism, bounds, preconditions, subjects, output]
    fixture: {files: {a.txt: \"x\\n\"}}
    call: {operation: notes.write, input: {path: a.txt, text: \"y\\n\", labels: {}, tags: []}}
    expect: {outcome: ok}
";

const INLINE_FIXTURE: &str = "{files: {a.txt: \"x\\n\"}}";

fn suite_text(input: &str, fixture: &str, expect: &str) -> String {
    SUITE
        .replace("READ_INPUT", input)
        .replace("READ_FIXTURE", fixture)
        .replace("READ_EXPECT", expect)
}

fn inline_suite(input: &str, expect: &str) -> Suite {
    Suite::from_yaml(&suite_text(input, INLINE_FIXTURE, expect)).expect("the suite parses")
}

// A contract directory whose `fixtures/linked/` holds `a.txt`, `b.txt`, `link -> a.txt` and
// `dangling -> nowhere`, and one suite document whose read scenario lays it out.
fn linked_suite(expect: &str) -> (tempfile::TempDir, Suite) {
    let base = scratch();
    let linked = base.path().join("fixtures/linked");
    fs::create_dir_all(&linked).expect("mkdir");
    fs::write(linked.join("a.txt"), "one\n").expect("write");
    fs::write(linked.join("b.txt"), "two\n").expect("write");
    std::os::unix::fs::symlink("a.txt", linked.join("link")).expect("symlink");
    std::os::unix::fs::symlink("nowhere", linked.join("dangling")).expect("symlink");
    let scenarios = base.path().join("scenarios");
    fs::create_dir_all(&scenarios).expect("mkdir");
    fs::write(
        scenarios.join("notes.yaml"),
        suite_text("{path: a.txt}", "{dir: fixtures/linked}", expect),
    )
    .expect("write");
    let suite = Suite::read_dir(&scenarios).expect("the suite parses");
    (base, suite)
}

fn read_result(report: &Report) -> &ScenarioResult {
    report
        .scenarios
        .iter()
        .find(|s| s.name == "read")
        .unwrap_or_else(|| panic!("the suite was refused or ran no read scenario:\n{report}"))
}

fn ok() -> Outcome {
    Outcome::Ok(json!({}))
}

#[test]
fn a_call_input_key_that_is_not_an_input_field_is_refused_naming_it() {
    let suite = inline_suite("{path: a.txt, lmit: 1}", "{outcome: ok}");
    let scratch = scratch();
    let report = run_tampering(&suite, scratch.path(), Tamper::Nothing, &ok());
    assert!(report.scenarios.is_empty(), "{report}");
    assert!(
        report.refused.iter().any(|line| {
            line.starts_with("read:") && line.contains("lmit") && line.contains("notes.read")
        }),
        "{report}"
    );
}

#[test]
fn a_call_omitting_an_optional_input_field_is_not_refused() {
    let suite = inline_suite("{path: a.txt}", "{outcome: ok}");
    let scratch = scratch();
    let report = run_tampering(&suite, scratch.path(), Tamper::Nothing, &ok());
    assert!(report.refused.is_empty(), "{report}");
    assert!(read_result(&report).passed, "{report}");
}

#[test]
fn a_returned_error_that_is_a_declared_outcome_passes() {
    let suite = inline_suite(
        "{path: a.txt}",
        "{error: notes.NotFound, output: {path: a.txt}}",
    );
    let scratch = scratch();
    let not_found = Outcome::Error {
        name: "notes.NotFound".to_string(),
        message: "no file at a.txt".to_string(),
        fields: json!({ "path": "a.txt" }),
    };
    let report = run_tampering(&suite, scratch.path(), Tamper::Nothing, &not_found);
    assert!(report.refused.is_empty(), "{report}");
    assert!(read_result(&report).passed, "{report}");
}

#[test]
fn a_returned_error_that_is_not_an_outcome_fails_the_scenario_even_when_ok_was_expected() {
    let suite = inline_suite("{path: a.txt}", "{outcome: ok}");
    let scratch = scratch();
    let foreign = Outcome::Error {
        name: "notes.PreconditionFailed".to_string(),
        message: "an outcome of notes.write only".to_string(),
        fields: json!({ "expected": "a", "actual": "b" }),
    };
    let report = run_tampering(&suite, scratch.path(), Tamper::Nothing, &foreign);
    let read = read_result(&report);
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("not an outcome of notes.read"),
        "{}",
        read.detail
    );
}

#[test]
fn a_link_whose_target_changes_is_a_change() {
    let (_base, suite) = linked_suite("{outcome: ok, files: {link: unchanged}}");
    let work = scratch();
    let report = run_tampering(&suite, work.path(), Tamper::RepointLinkToB, &ok());
    let read = read_result(&report);
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("link") && read.detail.contains("unchanged"),
        "{}",
        read.detail
    );
}

#[test]
fn a_link_replaced_by_a_file_holding_its_targets_content_is_a_change() {
    let (_base, suite) = linked_suite("{outcome: ok, files: {link: unchanged}}");
    let work = scratch();
    let report = run_tampering(&suite, work.path(), Tamper::ReplaceLinkWithFile, &ok());
    let read = read_result(&report);
    assert!(!read.passed, "{report}");
    assert!(read.detail.contains("link"), "{}", read.detail);
}

#[test]
fn equals_follows_an_untouched_link_and_the_files_around_it_are_unchanged() {
    let (_base, suite) = linked_suite(
        "{outcome: ok, files: {link: {equals: \"one\\n\"}, a.txt: unchanged, b.txt: unchanged}}",
    );
    let work = scratch();
    let report = run_tampering(&suite, work.path(), Tamper::Nothing, &ok());
    assert!(read_result(&report).passed, "{report}");
}

#[test]
fn an_untouched_link_and_a_dangling_link_are_unchanged() {
    let (_base, suite) =
        linked_suite("{outcome: ok, files: {link: unchanged, dangling: unchanged}}");
    let work = scratch();
    let report = run_tampering(&suite, work.path(), Tamper::Nothing, &ok());
    assert!(read_result(&report).passed, "{report}");
}

#[test]
fn a_dangling_link_is_not_absent() {
    let (_base, suite) = linked_suite("{outcome: ok, files: {dangling: absent}}");
    let work = scratch();
    let report = run_tampering(&suite, work.path(), Tamper::Nothing, &ok());
    let read = read_result(&report);
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("dangling") && read.detail.contains("exists"),
        "{}",
        read.detail
    );
}
