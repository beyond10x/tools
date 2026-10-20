//! The `tool-conformance/1` runner over an in-crate fake toolset: the fixture suite passes against
//! the intact toolset, each mutant fails a scenario, a suite that leaves a declared property
//! without a scenario is refused naming it, and the schema check runs once per run.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Report, Suite, Toolset, run};
use tool_contract::digest::sha256_hex;
use tool_contract::mutants::each_mutant_fails;

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/notes")
        .join(rel)
}

fn contract() -> Contract {
    Contract::read(fixture("contract.yaml")).expect("the fixture contract parses")
}

fn suite() -> Suite {
    Suite::read_dir(fixture("scenarios")).expect("the fixture suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mutant {
    Intact,
    PageDigest,
    IgnorePrecondition,
    WrongSchema,
}

struct Notes {
    root: PathBuf,
    contract: Contract,
    mutant: Mutant,
}

#[derive(Deserialize, JsonSchema)]
struct ReadInput {
    path: String,
    offset: Option<u64>,
    limit: Option<u64>,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum Kind {
    File,
    #[allow(dead_code)]
    Directory,
}

#[derive(Serialize, JsonSchema)]
struct Window {
    from: u64,
    to: u64,
    total: Option<u64>,
}

#[derive(Serialize, JsonSchema)]
struct ReadOutput {
    path: String,
    text: String,
    lines: Window,
    truncated: bool,
    sha256: Option<String>,
    kind: Kind,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct ReadOutputWithTruncatedAsText {
    path: String,
    text: String,
    lines: Window,
    truncated: String,
    sha256: Option<String>,
    kind: Kind,
}

#[derive(Deserialize, JsonSchema)]
struct WriteInput {
    path: String,
    text: String,
    expected_sha256: Option<String>,
    #[allow(dead_code)]
    labels: BTreeMap<String, String>,
    #[allow(dead_code)]
    tags: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
struct WriteOutput {
    path: String,
    bytes: u64,
    sha256: String,
}

fn count(n: usize) -> u64 {
    u64::try_from(n).expect("a line count fits in u64")
}

impl Notes {
    fn read(&self, input: &Value) -> Outcome {
        let input: ReadInput = match serde_json::from_value(input.clone()) {
            Ok(input) => input,
            Err(err) => return bad_input(&err),
        };
        let Ok(bytes) = fs::read(self.root.join(&input.path)) else {
            return Outcome::Error {
                name: "notes.NotFound".to_string(),
                message: format!("no file at {}", input.path),
                fields: json!({ "path": input.path }),
            };
        };
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().collect();
        let total = count(lines.len());
        let from = input.offset.unwrap_or(1).max(1);
        let limit = input.limit.unwrap_or(total);
        let page: String = lines
            .iter()
            .skip(usize::try_from(from - 1).expect("fits"))
            .take(usize::try_from(limit).expect("fits"))
            .flat_map(|line| [*line, "\n"])
            .collect();
        let to = (from + limit - 1).min(total);
        let digest = if self.mutant == Mutant::PageDigest {
            sha256_hex(page.as_bytes())
        } else {
            sha256_hex(&bytes)
        };
        Outcome::Ok(
            serde_json::to_value(ReadOutput {
                path: input.path,
                text: page,
                lines: Window {
                    from,
                    to,
                    total: Some(total),
                },
                truncated: to < total,
                sha256: Some(digest),
                kind: Kind::File,
            })
            .expect("serialises"),
        )
    }

    fn write(&self, input: &Value) -> Outcome {
        let input: WriteInput = match serde_json::from_value(input.clone()) {
            Ok(input) => input,
            Err(err) => return bad_input(&err),
        };
        let path = self.root.join(&input.path);
        if let Some(expected) = &input.expected_sha256
            && self.mutant != Mutant::IgnorePrecondition
        {
            let actual = fs::read(&path).map(|b| sha256_hex(&b)).unwrap_or_default();
            if &actual != expected {
                return Outcome::Error {
                    name: "notes.PreconditionFailed".to_string(),
                    message: format!("{} has digest {actual}, expected {expected}", input.path),
                    fields: json!({ "expected": expected, "actual": actual }),
                };
            }
        }
        fs::write(&path, &input.text).expect("the fixture tree is writable");
        Outcome::Ok(
            serde_json::to_value(WriteOutput {
                path: input.path,
                bytes: count(input.text.len()),
                sha256: sha256_hex(input.text.as_bytes()),
            })
            .expect("serialises"),
        )
    }
}

fn bad_input(err: &serde_json::Error) -> Outcome {
    Outcome::Error {
        name: "notes.BadInput".to_string(),
        message: err.to_string(),
        fields: Value::Null,
    }
}

impl Toolset for Notes {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        let read_output = if self.mutant == Mutant::WrongSchema {
            schema_for!(ReadOutputWithTruncatedAsText)
        } else {
            schema_for!(ReadOutput)
        };
        vec![
            (
                "notes.read".to_string(),
                schema_for!(ReadInput),
                read_output,
            ),
            (
                "notes.write".to_string(),
                schema_for!(WriteInput),
                schema_for!(WriteOutput),
            ),
        ]
    }

    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        match operation {
            "notes.read" => self.read(input),
            "notes.write" => self.write(input),
            other => Outcome::Error {
                name: "notes.UnknownOperation".to_string(),
                message: format!("no operation {other}"),
                fields: Value::Null,
            },
        }
    }
}

fn run_with(suite: &Suite, mutant: Mutant) -> Report {
    let contract = contract();
    let scratch = scratch();
    run(suite, &contract, scratch.path(), |root| {
        Box::new(Notes {
            root: root.to_path_buf(),
            contract: contract.clone(),
            mutant,
        })
    })
}

fn failed(report: &Report) -> Vec<&str> {
    report
        .scenarios
        .iter()
        .filter(|s| !s.passed)
        .map(|s| s.name.as_str())
        .collect()
}

#[test]
fn the_fixture_suite_passes_against_the_intact_toolset() {
    let report = run_with(&suite(), Mutant::Intact);
    assert!(report.passed(), "{report}");
    assert_eq!(report.scenarios.len(), 4, "{report}");
    assert!(report.refused.is_empty(), "{report}");
    assert!(report.schema_mismatches.is_empty(), "{report}");
    let shown = report.to_string();
    assert!(shown.contains("4 of 4 scenarios passed"), "{shown}");
    assert!(
        shown.contains("ok   a-window-is-answered-with-the-whole-files-digest"),
        "{shown}"
    );
}

#[test]
fn a_digest_of_the_page_fails_the_whole_file_digest_scenario() {
    let report = run_with(&suite(), Mutant::PageDigest);
    assert!(!report.passed(), "{report}");
    assert_eq!(
        failed(&report),
        ["a-window-is-answered-with-the-whole-files-digest"],
        "{report}"
    );
    let detail = &report.scenarios[0].detail;
    assert!(detail.contains("sha256"), "{detail}");
}

#[test]
fn an_ignored_precondition_fails_the_precondition_scenario() {
    let report = run_with(&suite(), Mutant::IgnorePrecondition);
    assert!(!report.passed(), "{report}");
    assert_eq!(
        failed(&report),
        ["a-stale-digest-leaves-the-file-unchanged"],
        "{report}"
    );
}

#[test]
fn each_named_mutant_must_fail_at_least_one_scenario() {
    let suite = suite();
    let both = each_mutant_fails(&[
        ("page-digest", run_with(&suite, Mutant::PageDigest)),
        (
            "ignore-precondition",
            run_with(&suite, Mutant::IgnorePrecondition),
        ),
    ]);
    assert_eq!(both, Ok(()));
    let one_intact = each_mutant_fails(&[
        ("intact", run_with(&suite, Mutant::Intact)),
        ("page-digest", run_with(&suite, Mutant::PageDigest)),
    ]);
    assert_eq!(one_intact, Err(vec!["intact".to_string()]));
}

const UNCOVERED: &str = "\
format: tool-conformance/1
toolset: notes
version: v1alpha1
scenarios:
  - name: read-without-paging
    holds: [effects, idempotency, determinism, bounds, subjects, output]
    fixture: {files: {a.txt: \"x\\n\"}}
    call: {operation: notes.read, input: {path: a.txt}}
    expect: {outcome: ok}
  - name: write
    holds: [effects, idempotency, determinism, bounds, preconditions, subjects, output]
    fixture: {files: {a.txt: \"x\\n\"}}
    call: {operation: notes.write, input: {path: a.txt, text: \"y\\n\", labels: {}, tags: []}}
    expect: {outcome: ok}
";

#[test]
fn a_suite_leaving_a_declared_property_without_a_scenario_is_refused_naming_it() {
    let suite = Suite::from_yaml(UNCOVERED).expect("parses");
    let report = run_with(&suite, Mutant::Intact);
    assert!(!report.passed(), "{report}");
    assert!(
        report.scenarios.is_empty(),
        "a refused suite runs nothing: {report}"
    );
    assert_eq!(report.refused.len(), 1, "{report}");
    assert!(
        report.refused[0].contains("notes.read") && report.refused[0].contains("paging"),
        "{report}"
    );
}

#[test]
fn the_declarative_properties_need_no_scenario() {
    let report = run_with(&suite(), Mutant::Intact);
    assert!(report.refused.is_empty(), "{report}");
    let fully_covered = Suite::from_yaml(&UNCOVERED.replace(
        "holds: [effects, idempotency, determinism, bounds, subjects, output]",
        "holds: [effects, idempotency, determinism, bounds, subjects, output, paging]",
    ))
    .expect("parses");
    let report = run_with(&fully_covered, Mutant::Intact);
    assert!(report.refused.is_empty(), "{report}");
}

#[test]
fn a_holds_entry_that_is_not_a_profile_property_is_refused() {
    let suite =
        Suite::from_yaml(&UNCOVERED.replace("subjects, output]", "subjects, output, colour]"))
            .expect("parses");
    let report = run_with(&suite, Mutant::Intact);
    assert!(
        report.refused.iter().any(|r| r.contains("colour")),
        "{report}"
    );
}

#[test]
fn the_schema_check_runs_once_per_run_and_names_the_field() {
    let report = run_with(&suite(), Mutant::WrongSchema);
    assert!(!report.passed(), "{report}");
    assert!(
        failed(&report).is_empty(),
        "the scenarios themselves pass: {report}"
    );
    assert_eq!(report.schema_mismatches.len(), 1, "{report}");
    let line = &report.schema_mismatches[0];
    assert!(
        line.contains("notes.read") && line.contains("output") && line.contains("truncated"),
        "{line}"
    );
}

#[test]
fn a_scenario_calling_an_undeclared_operation_fails() {
    let suite =
        Suite::from_yaml(&UNCOVERED.replace("operation: notes.write", "operation: notes.stat"))
            .expect("parses");
    let report = run_with(&suite, Mutant::Intact);
    assert!(
        report.refused.iter().any(|r| r.contains("notes.stat")),
        "{report}"
    );
}
