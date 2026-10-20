//! The schema shape check: a `schemars`-derived schema that matches the contract's field list is
//! accepted; one that differs is refused naming the operation, the side and the field.

use std::collections::BTreeMap;
use std::path::Path;

use schemars::{JsonSchema, schema_for};
use tool_contract::Contract;
use tool_contract::types::{Shape, Side, Type, check_shapes};

fn contract() -> Contract {
    Contract::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/notes/contract.yaml"))
        .expect("the fixture parses")
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct ReadInput {
    path: String,
    offset: Option<u64>,
    limit: Option<u64>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(rename_all = "lowercase")]
enum Kind {
    File,
    Directory,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Window {
    from: u64,
    to: u64,
    total: Option<u64>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
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
struct WriteInput {
    path: String,
    text: String,
    expected_sha256: Option<String>,
    labels: BTreeMap<String, String>,
    tags: Vec<String>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct WriteOutput {
    path: String,
    bytes: u64,
    sha256: String,
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

#[allow(dead_code)]
#[derive(JsonSchema)]
struct WindowWithRequiredTotal {
    from: u64,
    to: u64,
    total: u64,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct ReadOutputWithRequiredTotal {
    path: String,
    text: String,
    lines: WindowWithRequiredTotal,
    truncated: bool,
    sha256: Option<String>,
    kind: Kind,
}

#[test]
fn a_derived_schema_matching_the_contract_is_accepted_on_both_sides() {
    let contract = contract();
    check_shapes(
        &contract,
        Side::Input,
        "notes.read",
        &schema_for!(ReadInput),
    )
    .expect("read input");
    check_shapes(
        &contract,
        Side::Output,
        "notes.read",
        &schema_for!(ReadOutput),
    )
    .expect("read output");
    check_shapes(
        &contract,
        Side::Input,
        "notes.write",
        &schema_for!(WriteInput),
    )
    .expect("write input");
    check_shapes(
        &contract,
        Side::Output,
        "notes.write",
        &schema_for!(WriteOutput),
    )
    .expect("write output");
}

#[test]
fn a_field_of_another_type_is_refused_naming_operation_side_and_field() {
    let mismatch = check_shapes(
        &contract(),
        Side::Output,
        "notes.read",
        &schema_for!(ReadOutputWithTruncatedAsText),
    )
    .expect_err("truncated is Boolean in the contract");
    assert_eq!(mismatch.operation, "notes.read");
    assert_eq!(mismatch.side, Side::Output);
    assert_eq!(mismatch.field, "truncated");
    let shown = mismatch.to_string();
    assert!(shown.contains("notes.read"), "{shown}");
    assert!(shown.contains("output"), "{shown}");
    assert!(shown.contains("truncated"), "{shown}");
}

#[test]
fn a_nested_difference_names_the_path_to_the_field() {
    let mismatch = check_shapes(
        &contract(),
        Side::Output,
        "notes.read",
        &schema_for!(ReadOutputWithRequiredTotal),
    )
    .expect_err("lines.total is Optional in the contract");
    assert_eq!(mismatch.field, "lines.total");
}

#[test]
fn an_unknown_operation_is_refused_naming_it() {
    let mismatch = check_shapes(
        &contract(),
        Side::Input,
        "notes.stat",
        &schema_for!(ReadInput),
    )
    .expect_err("no such operation");
    assert_eq!(mismatch.operation, "notes.stat");
}

#[test]
fn both_sides_map_to_one_canonical_shape() {
    let contract = contract();
    let read = contract
        .operations
        .iter()
        .find(|op| op.name == "notes.read")
        .expect("notes.read");
    let from_contract = Shape::of_fields(&read.output, &contract.types).expect("contract shape");
    let from_schema = Shape::of_schema(&schema_for!(ReadOutput)).expect("schema shape");
    assert_eq!(from_contract, from_schema);
    let Shape::Struct(fields) = &from_contract else {
        panic!("a field list is a Struct, got {from_contract:?}");
    };
    assert_eq!(fields["truncated"], Shape::Boolean);
    assert_eq!(fields["sha256"], Shape::Optional(Box::new(Shape::String)));
    assert_eq!(
        fields["kind"],
        Shape::Enum(vec!["directory".to_string(), "file".to_string()])
    );
}

#[test]
fn the_type_grammar_parses_nested_generics() {
    assert_eq!(
        Type::parse("Optional<List<String>>").expect("parses"),
        Type::Optional(Box::new(Type::List(Box::new(Type::String))))
    );
    assert_eq!(
        Type::parse("Map<String, Integer>").expect("parses"),
        Type::Map(Box::new(Type::String), Box::new(Type::Integer))
    );
    assert_eq!(
        Type::parse("notes.Kind").expect("parses"),
        Type::Named("notes.Kind".to_string())
    );
    assert!(Type::parse("Optional<").is_err());
    assert!(Type::parse("List<String> x").is_err());
    assert!(Type::parse("Map<String>").is_err());
}
