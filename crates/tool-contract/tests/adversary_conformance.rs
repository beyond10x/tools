//! The runner on what the fixture suite does not reach: a scenario whose `expect.error` is not an
//! outcome of the operation it calls (declared elsewhere, or not at all), a symlink inside a
//! fixture `dir`, one scenario name in two documents, and a suite naming another version.

use std::fs;
use std::path::{Path, PathBuf};

use schemars::Schema;
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Report, Suite, Toolset, run};

fn here(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn contract() -> Contract {
    Contract::read(here("tests/fixtures/notes/contract.yaml")).expect("the fixture contract parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

// A toolset that answers `notes.read` with one canned outcome and every other call with `ok {}`.
// It derives no schemas, so `Report::passed()` is never true for it; the cases below look at
// `refused` and at the scenarios, not at `passed()`.
struct Canned {
    contract: Contract,
    read: Outcome,
}

impl Toolset for Canned {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        Vec::new()
    }

    fn invoke(&self, operation: &str, _input: &Value) -> Outcome {
        if operation == "notes.read" {
            self.read.clone()
        } else {
            Outcome::Ok(json!({}))
        }
    }
}

fn run_canned(suite: &Suite, scratch: &Path, read: &Outcome) -> Report {
    let contract = contract();
    run(suite, &contract, scratch, |_| {
        Box::new(Canned {
            contract: contract.clone(),
            read: read.clone(),
        })
    })
}

// Every behavioural property of both operations is held, so the suite is never refused for
// coverage; `READ_FIXTURE` and `READ_EXPECT` are filled in per case.
const SUITE: &str = "\
format: tool-conformance/1
toolset: notes
version: v1alpha1
scenarios:
  - name: read
    holds: [effects, idempotency, determinism, bounds, paging, subjects, output]
    fixture: READ_FIXTURE
    call: {operation: notes.read, input: {path: a.txt}}
    expect: READ_EXPECT
  - name: write
    holds: [effects, idempotency, determinism, bounds, preconditions, subjects, output]
    fixture: {files: {a.txt: \"x\\n\"}}
    call: {operation: notes.write, input: {path: a.txt, text: \"y\\n\", labels: {}, tags: []}}
    expect: {outcome: ok}
";

const INLINE_FIXTURE: &str = "{files: {a.txt: \"x\\n\"}}";

fn suite_expecting(error: &str) -> Suite {
    let text = SUITE
        .replace("READ_FIXTURE", INLINE_FIXTURE)
        .replace("READ_EXPECT", &format!("{{error: {error}}}"));
    Suite::from_yaml(&text).expect("the suite parses")
}

fn canned_error(name: &str) -> Outcome {
    Outcome::Error {
        name: name.to_string(),
        message: format!("{name} from notes.read"),
        fields: json!({ "expected": "a", "actual": "b", "path": "a.txt" }),
    }
}

// The hole: a scenario expects an error, the implementation returns that very error, and nothing
// compares either against the called operation's `outcomes`. The profile document says every
// error an operation can return is one of its outcomes; the runner is the only thing that could
// check it against an implementation, and it reports `ok`.
fn assert_not_evidence(report: &Report, error: &str) {
    let refused = report
        .refused
        .iter()
        .any(|line| line.contains("notes.read") && line.contains(error));
    let failed = report
        .scenarios
        .iter()
        .any(|s| s.name == "read" && !s.passed && s.detail.contains(error));
    assert!(
        refused || failed,
        "{error} is not an outcome of notes.read, yet the runner took the scenario as evidence:\n{report}"
    );
}

#[test]
fn an_expected_error_that_is_an_outcome_of_another_operation_only_is_not_evidence() {
    let scratch = scratch();
    let report = run_canned(
        &suite_expecting("notes.PreconditionFailed"),
        scratch.path(),
        &canned_error("notes.PreconditionFailed"),
    );
    assert_not_evidence(&report, "notes.PreconditionFailed");
}

#[test]
fn an_expected_error_the_contract_never_declares_is_not_evidence() {
    let scratch = scratch();
    let report = run_canned(
        &suite_expecting("notes.Undeclared"),
        scratch.path(),
        &canned_error("notes.Undeclared"),
    );
    assert_not_evidence(&report, "notes.Undeclared");
}

#[cfg(unix)]
#[test]
fn a_symlink_in_a_fixture_dir_is_laid_out_as_a_symlink() {
    let base = scratch();
    let linked = base.path().join("fixtures/linked");
    fs::create_dir_all(&linked).expect("mkdir");
    fs::write(linked.join("a.txt"), "one\n").expect("write");
    std::os::unix::fs::symlink("a.txt", linked.join("link")).expect("symlink");
    let scenarios = base.path().join("scenarios");
    fs::create_dir_all(&scenarios).expect("mkdir");
    let text = SUITE
        .replace("READ_FIXTURE", "{dir: fixtures/linked}")
        .replace("READ_EXPECT", "{outcome: ok}");
    fs::write(scenarios.join("notes.yaml"), text).expect("write");
    let suite = Suite::read_dir(&scenarios).expect("the suite parses");

    let work = scratch();
    let report = run_canned(&suite, work.path(), &Outcome::Ok(json!({})));

    let link = work.path().join("read").join("link");
    let kind = fs::symlink_metadata(&link)
        .expect("the fixture's link is in the workspace")
        .file_type();
    assert!(
        kind.is_symlink(),
        "fixture dir copied `link -> a.txt` as {kind:?}, not as a symlink:\n{report}"
    );
}

#[test]
fn one_scenario_name_in_two_documents_is_refused() {
    let base = scratch();
    let scenarios = base.path().join("scenarios");
    fs::create_dir_all(&scenarios).expect("mkdir");
    let text = SUITE
        .replace("READ_FIXTURE", INLINE_FIXTURE)
        .replace("READ_EXPECT", "{outcome: ok}");
    fs::write(scenarios.join("a.yaml"), &text).expect("write");
    fs::write(scenarios.join("b.yaml"), &text).expect("write");
    let err = Suite::read_dir(&scenarios).expect_err("the name read is in both documents");
    assert!(err.to_string().contains("duplicate scenario name"), "{err}");
}

#[test]
fn a_suite_naming_another_version_is_refused_naming_both() {
    let text = SUITE
        .replace("READ_FIXTURE", INLINE_FIXTURE)
        .replace("READ_EXPECT", "{outcome: ok}")
        .replace("version: v1alpha1", "version: v1alpha2");
    let suite = Suite::from_yaml(&text).expect("the suite parses");
    let scratch = scratch();
    let report = run_canned(&suite, scratch.path(), &Outcome::Ok(json!({})));
    assert!(report.scenarios.is_empty(), "{report}");
    assert!(
        report
            .refused
            .iter()
            .any(|line| line.contains("v1alpha2") && line.contains("v1alpha1")),
        "{report}"
    );
}
