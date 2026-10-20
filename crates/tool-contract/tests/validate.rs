//! The validator refuses each seeded defect under `tests/defects/`, naming the operation and the
//! property, and accepts the two-operation fixture document.

use std::path::{Path, PathBuf};

use tool_contract::{Contract, Violation, validate};

fn here(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn defect(name: &str) -> Vec<Violation> {
    let contract = Contract::read(here("tests/defects").join(name))
        .expect("a seeded defect parses; the validator is what refuses it");
    validate(&contract).expect_err("the seeded defect is refused")
}

fn names(violations: &[Violation], operation: &str, property: &str) -> bool {
    violations
        .iter()
        .any(|v| v.operation == operation && v.property == property)
}

fn shown(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_valid_two_operation_document_is_accepted() {
    let contract = Contract::read(here("tests/fixtures/notes/contract.yaml")).expect("parses");
    assert_eq!(contract.operations.len(), 2);
    assert_eq!(contract.toolset, "notes");
    validate(&contract).expect("the fixture document is accepted");
}

#[test]
fn risk_under_the_floor_set_by_effects_is_refused() {
    let violations = defect("risk-below-effects.yaml");
    assert!(
        names(&violations, "notes.write", "risk"),
        "{}",
        shown(&violations)
    );
    assert!(
        shown(&violations).contains("notes.write: risk: "),
        "{}",
        shown(&violations)
    );
}

#[test]
fn a_bound_without_a_maximum_is_refused() {
    let violations = defect("bound-without-max.yaml");
    assert!(
        names(&violations, "notes.read", "bounds"),
        "{}",
        shown(&violations)
    );
    assert!(
        shown(&violations).contains("output_bytes"),
        "{}",
        shown(&violations)
    );
}

#[test]
fn a_subject_naming_an_unknown_input_field_is_refused() {
    let violations = defect("unknown-subject-field.yaml");
    assert!(
        names(&violations, "notes.read", "subjects"),
        "{}",
        shown(&violations)
    );
    assert!(
        shown(&violations).contains("file"),
        "{}",
        shown(&violations)
    );
}

#[test]
fn a_capability_that_is_not_a_canon_id_is_refused() {
    let violations = defect("unknown-capability.yaml");
    assert!(
        names(&violations, "notes.read", "capability"),
        "{}",
        shown(&violations)
    );
    assert!(
        shown(&violations).contains("WorkspaceRead"),
        "{}",
        shown(&violations)
    );
}

#[test]
fn a_violation_prints_operation_property_and_message() {
    let violation = Violation {
        operation: "notes.read".to_string(),
        property: "bounds".to_string(),
        message: "timeout_ms is missing".to_string(),
    };
    assert_eq!(
        violation.to_string(),
        "notes.read: bounds: timeout_ms is missing"
    );
}

#[test]
fn a_document_of_another_format_is_refused_at_parse() {
    let err = Contract::from_yaml(
        "format: ess.domain/1\ntoolset: x\nversion: v1\nsummary: s\noperations: []\n",
    )
    .expect_err("another format is not a contract");
    assert!(err.to_string().contains("tool-contract/1"), "{err}");
}

#[test]
fn an_operation_outside_its_family_is_refused() {
    let yaml =
        std::fs::read_to_string(here("tests/defects/unknown-capability.yaml")).expect("read");
    let yaml = yaml
        .replace("capability: WorkspaceRead", "capability: workspace.read")
        .replace("name: notes.read", "name: files.read");
    let contract = Contract::from_yaml(&yaml).expect("parses");
    let violations = validate(&contract).expect_err("refused");
    assert!(
        names(&violations, "files.read", "name"),
        "{}",
        shown(&violations)
    );
}
