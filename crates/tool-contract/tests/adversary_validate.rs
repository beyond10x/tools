//! Rules the profile document states that no other case exercises, each driven by one mutation of
//! the fixture contract: a rendering reused across operations, `default` above `max`, `timeout_ms`
//! missing, an error no outcome names, an outcome naming an undeclared error, a duplicate
//! operation, a paging field on the wrong side, a `paging` naming nothing, an unknown precondition,
//! a capability with a trailing dot, a `Map` key that is not `String`, an undeclared type, a type
//! text that does not parse, a duplicate field, an enum without variants, a duplicate outcome, an
//! unknown profile key, and the risk refusal naming its floor.

use std::path::{Path, PathBuf};

use tool_contract::{Contract, Violation, validate};

fn here(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

// The fixture contract with `from` replaced by `to`; `from` must occur in it.
fn mutated(from: &str, to: &str) -> String {
    let text =
        std::fs::read_to_string(here("tests/fixtures/notes/contract.yaml")).expect("the fixture");
    assert!(
        text.contains(from),
        "the fixture no longer contains {from:?}"
    );
    text.replace(from, to)
}

fn refused(yaml: &str) -> Vec<Violation> {
    let contract = Contract::from_yaml(yaml).expect("the mutated document still parses");
    validate(&contract).expect_err("the mutated document is refused")
}

fn shown(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn names(violations: &[Violation], operation: &str, property: &str, needle: &str) {
    let hit = violations
        .iter()
        .any(|v| v.operation == operation && v.property == property && v.message.contains(needle));
    assert!(
        hit,
        "expected `{operation}: {property}: ...{needle}...`, got:\n{}",
        shown(violations)
    );
}

#[test]
fn a_rendering_reused_across_operations_is_refused() {
    let violations = refused(&mutated(
        "claude: Write, codex: write_file",
        "claude: Read, codex: write_file",
    ));
    names(&violations, "notes.write", "renderings", "Read");
}

#[test]
fn a_default_above_max_is_refused() {
    let violations = refused(&mutated(
        "output_bytes: {default: 65536, max: 262144}",
        "output_bytes: {default: 262145, max: 262144}",
    ));
    names(&violations, "notes.read", "bounds", "262145");
}

#[test]
fn a_missing_timeout_is_refused() {
    let violations = refused(&mutated(
        "timeout_ms: {default: 10000, max: 60000}\n        output_bytes:",
        "output_bytes:",
    ));
    names(&violations, "notes.read", "bounds", "timeout_ms");
}

#[test]
fn an_error_no_outcome_names_is_refused() {
    let violations = refused(&mutated(
        "      - {name: precondition-failed, error: notes.PreconditionFailed}\n",
        "",
    ));
    names(
        &violations,
        "document",
        "errors",
        "notes.PreconditionFailed",
    );
}

#[test]
fn an_outcome_naming_an_undeclared_error_is_refused() {
    let violations = refused(&mutated("error: notes.NotFound}", "error: notes.Missing}"));
    names(&violations, "notes.read", "outcomes", "notes.Missing");
    names(&violations, "document", "errors", "notes.NotFound");
}

#[test]
fn a_duplicate_operation_name_is_refused() {
    let violations = refused(&mutated("- name: notes.write\n", "- name: notes.read\n"));
    names(&violations, "notes.read", "name", "duplicate");
}

#[test]
fn a_paging_field_on_the_wrong_side_is_refused() {
    let violations = refused(&mutated(
        "truncated: truncated, digest: sha256}",
        "truncated: offset, digest: sha256}",
    ));
    names(&violations, "notes.read", "paging", "offset");
}

#[test]
fn a_paging_that_names_nothing_is_refused() {
    let violations = refused(&mutated(
        "paging: {offset: offset, limit: limit, truncated: truncated, digest: sha256}",
        "paging: {}",
    ));
    names(&violations, "notes.read", "paging", "names no field");
}

#[test]
fn a_precondition_that_is_not_an_input_field_is_refused() {
    let violations = refused(&mutated(
        "preconditions: [expected_sha256]",
        "preconditions: [expected_md5]",
    ));
    names(&violations, "notes.write", "preconditions", "expected_md5");
}

#[test]
fn a_capability_with_a_trailing_dot_is_refused() {
    let violations = refused(&mutated(
        "capability: workspace.read\n",
        "capability: workspace.read.\n",
    ));
    names(&violations, "notes.read", "capability", "workspace.read.");
}

#[test]
fn a_map_whose_key_is_not_string_is_refused() {
    let violations = refused(&mutated(
        "\"Map<String, String>\"",
        "\"Map<Integer, String>\"",
    ));
    names(&violations, "notes.write", "input", "labels");
    names(&violations, "notes.write", "input", "key");
}

#[test]
fn a_field_naming_an_undeclared_type_is_refused() {
    let violations = refused(&mutated("type: notes.Kind}", "type: notes.Colour}"));
    names(&violations, "notes.read", "output", "notes.Colour");
}

#[test]
fn a_type_text_that_does_not_parse_is_refused() {
    let violations = refused(&mutated(
        "{name: limit, type: Optional<Integer>,",
        "{name: limit, type: Optional<,",
    ));
    names(&violations, "notes.read", "input", "limit");
}

#[test]
fn a_duplicate_field_is_refused() {
    let violations = refused(&mutated(
        "- {name: tags, type: List<String>}",
        "- {name: path, type: List<String>}",
    ));
    names(&violations, "notes.write", "input", "duplicate field path");
}

#[test]
fn an_enum_without_variants_is_refused() {
    let violations = refused(&mutated("variants: [file, directory]", "variants: []"));
    names(&violations, "document", "types", "notes.Kind");
}

#[test]
fn a_duplicate_outcome_is_refused() {
    let violations = refused(&mutated(
        "- {name: not-found, error: notes.NotFound}",
        "- {name: read, error: notes.NotFound}",
    ));
    names(
        &violations,
        "notes.read",
        "outcomes",
        "duplicate outcome read",
    );
}

#[test]
fn an_unknown_profile_key_is_refused_at_parse() {
    let yaml = mutated(
        "output: {untrusted: true, failure_as_result: true}",
        "output: {untrusted: true, failure_as_result: true, colour: red}",
    );
    let err = Contract::from_yaml(&yaml).expect_err("colour is not a profile key");
    assert!(err.to_string().contains("colour"), "{err}");
}

#[test]
fn the_risk_refusal_names_the_floor_and_what_set_it() {
    let contract =
        Contract::read(here("tests/defects/risk-below-effects.yaml")).expect("the defect parses");
    let violations = validate(&contract).expect_err("refused");
    names(&violations, "notes.write", "risk", "low");
    names(&violations, "notes.write", "risk", "write");
}
