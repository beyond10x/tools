//! A returned error's `fields` must carry exactly the keys the contract declares for that error:
//! the exact set passes, a missing key fails naming the error and the key, an extra key fails the
//! same way, a non-object `fields` fails, and the order of the keys is free.

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

// Answers the named operation with the given error and fields, every other call with `ok {}`.
// It derives no schemas, so `Report::passed()` is never true; the cases look at one scenario's
// result.
struct Failing {
    contract: Contract,
    operation: &'static str,
    error: &'static str,
    fields: Value,
}

impl Toolset for Failing {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        Vec::new()
    }

    fn invoke(&self, operation: &str, _input: &Value) -> Outcome {
        if operation == self.operation {
            Outcome::Error {
                name: self.error.to_string(),
                message: format!("{} from {operation}", self.error),
                fields: self.fields.clone(),
            }
        } else {
            Outcome::Ok(json!({}))
        }
    }
}

// Every behavioural property of both operations is held, so the suite is never refused for
// coverage; each scenario's expectation is filled in per case.
const SUITE: &str = "\
format: tool-conformance/1
toolset: notes
version: v1alpha1
scenarios:
  - name: read
    holds: [effects, idempotency, determinism, bounds, paging, subjects, output]
    fixture: {files: {b.txt: \"x\\n\"}}
    call: {operation: notes.read, input: {path: a.txt}}
    expect: READ_EXPECT
  - name: write
    holds: [effects, idempotency, determinism, bounds, preconditions, subjects, output]
    fixture: {files: {a.txt: \"x\\n\"}}
    call: {operation: notes.write, input: {path: a.txt, text: \"y\\n\", labels: {}, tags: []}}
    expect: WRITE_EXPECT
";

fn suite(read_expect: &str, write_expect: &str) -> Suite {
    let text = SUITE
        .replace("READ_EXPECT", read_expect)
        .replace("WRITE_EXPECT", write_expect);
    Suite::from_yaml(&text).expect("the suite parses")
}

fn run_failing(
    suite: &Suite,
    operation: &'static str,
    error: &'static str,
    fields: &Value,
) -> Report {
    let contract = contract();
    let scratch = scratch();
    run(suite, &contract, scratch.path(), |_| {
        Box::new(Failing {
            contract: contract.clone(),
            operation,
            error,
            fields: fields.clone(),
        })
    })
}

fn result<'a>(report: &'a Report, name: &str) -> &'a ScenarioResult {
    report
        .scenarios
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the suite was refused or ran no {name} scenario:\n{report}"))
}

fn not_found_with(fields: &Value) -> Report {
    run_failing(
        &suite("{error: notes.NotFound}", "{outcome: ok}"),
        "notes.read",
        "notes.NotFound",
        fields,
    )
}

#[test]
fn exactly_the_declared_fields_pass() {
    let report = not_found_with(&json!({ "path": "a.txt" }));
    assert!(report.refused.is_empty(), "{report}");
    assert!(result(&report, "read").passed, "{report}");
}

#[test]
fn a_missing_declared_field_fails_naming_the_error_and_the_field() {
    let report = not_found_with(&json!({}));
    let read = result(&report, "read");
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("notes.NotFound") && read.detail.contains("path"),
        "{}",
        read.detail
    );
    assert!(read.detail.contains("missing"), "{}", read.detail);
}

#[test]
fn an_extra_field_fails_naming_the_error_and_the_field() {
    let report = not_found_with(&json!({ "path": "a.txt", "hint": "try b.txt" }));
    let read = result(&report, "read");
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("notes.NotFound") && read.detail.contains("hint"),
        "{}",
        read.detail
    );
    assert!(read.detail.contains("not declared"), "{}", read.detail);
}

#[test]
fn fields_that_are_not_an_object_fail() {
    let report = not_found_with(&Value::Null);
    let read = result(&report, "read");
    assert!(!read.passed, "{report}");
    assert!(
        read.detail.contains("notes.NotFound") && read.detail.contains("not an object"),
        "{}",
        read.detail
    );
}

#[test]
fn the_order_of_the_fields_is_free() {
    // notes.PreconditionFailed declares `expected` then `actual`; the toolset returns them the
    // other way round.
    let report = run_failing(
        &suite("{outcome: ok}", "{error: notes.PreconditionFailed}"),
        "notes.write",
        "notes.PreconditionFailed",
        &json!({ "actual": "b", "expected": "a" }),
    );
    assert!(report.refused.is_empty(), "{report}");
    assert!(result(&report, "write").passed, "{report}");
}
