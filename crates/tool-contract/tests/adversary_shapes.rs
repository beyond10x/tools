//! The shape mapping on what the fixture does not reach: an operation with no input fields,
//! `Optional` inside `List`, `List` inside `Optional`, a `Map` of `Optional`, `Bytes` plain and
//! optional, a single-variant enum, an optional enum and an optional struct through `$ref`, an
//! inline optional enum, a newtype, integers of every format, and the path a difference inside a
//! list of structs is reported at.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use schemars::{JsonSchema, schema_for};
use tool_contract::Contract;
use tool_contract::types::{Side, check_shapes};

const OP: &str = "probe.op";

// A one-operation contract: `types_block` is the whole `types:` block or empty, `fields` are the
// input fields as YAML flow mappings; no fields means no `input` key at all.
fn contract(types_block: &str, fields: &[&str]) -> Contract {
    let mut input = String::new();
    if !fields.is_empty() {
        input.push_str("    input:\n");
        for field in fields {
            writeln!(input, "      - {field}").expect("a String never fails to write");
        }
    }
    let yaml = format!(
        "format: tool-contract/1\ntoolset: probe\nversion: v1\nsummary: s\n{types_block}\
operations:\n  - name: {OP}\n    summary: s\n{input}    profile:\n      effects: [read]\n      \
risk: none\n      idempotency: idempotent\n      determinism: deterministic\n      bounds:\n        \
timeout_ms: {{max: 1}}\n      placement: local\n      capability: probe.read\n"
    );
    Contract::from_yaml(&yaml).expect("the probe contract parses")
}

const KIND: &str = "types:\n  - {name: probe.Kind, kind: enum, variants: [file, directory]}\n";
const ONE: &str = "types:\n  - {name: probe.One, kind: enum, variants: [only]}\n";
const WINDOW: &str = "types:\n  - name: probe.Window\n    kind: struct\n    fields: [{name: from, \
type: Integer}, {name: to, type: Integer}]\n";
const DIGEST: &str = "types:\n  - {name: probe.Digest, kind: newtype, type: String}\n";

fn accepted<T: JsonSchema>(contract: &Contract) {
    let schema = schema_for!(T);
    if let Err(mismatch) = check_shapes(contract, Side::Input, OP, &schema) {
        panic!(
            "{mismatch}\nderived schema:\n{}",
            serde_json::to_string_pretty(&schema).expect("a schema serialises")
        );
    }
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct NoInput {}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct OptionalInList {
    xs: Vec<Option<String>>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct ListInOptional {
    xs: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct MapOfOptional {
    m: BTreeMap<String, Option<u64>>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct TwoBytes {
    b: Vec<u8>,
    ob: Option<Vec<u8>>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(rename_all = "lowercase")]
enum One {
    Only,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct SingleVariant {
    one: One,
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
struct OptionalKind {
    k: Option<Kind>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(rename_all = "lowercase", inline)]
enum InlineKind {
    File,
    Directory,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct OptionalInlineKind {
    k: Option<InlineKind>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Window {
    from: u64,
    to: u64,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct OptionalWindow {
    w: Option<Window>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Windows {
    ws: Vec<Window>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct WindowToText {
    from: u64,
    to: String,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct WindowsToText {
    ws: Vec<WindowToText>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Digest(String);

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Digested {
    d: Digest,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Integers {
    a: i32,
    b: u8,
    c: usize,
    d: i64,
}

#[test]
fn an_operation_with_no_input_fields_matches_an_empty_derived_struct() {
    accepted::<NoInput>(&contract("", &[]));
}

#[test]
fn optional_inside_list_matches() {
    accepted::<OptionalInList>(&contract("", &["{name: xs, type: List<Optional<String>>}"]));
}

#[test]
fn list_inside_optional_matches() {
    accepted::<ListInOptional>(&contract("", &["{name: xs, type: Optional<List<String>>}"]));
}

#[test]
fn a_map_of_optional_values_matches() {
    accepted::<MapOfOptional>(&contract(
        "",
        &["{name: m, type: \"Map<String, Optional<Integer>>\"}"],
    ));
}

#[test]
fn bytes_plain_and_optional_match() {
    accepted::<TwoBytes>(&contract(
        "",
        &[
            "{name: b, type: Bytes}",
            "{name: ob, type: Optional<Bytes>}",
        ],
    ));
}

#[test]
fn a_single_variant_enum_matches() {
    accepted::<SingleVariant>(&contract(ONE, &["{name: one, type: probe.One}"]));
}

#[test]
fn an_optional_enum_through_a_ref_matches() {
    accepted::<OptionalKind>(&contract(KIND, &["{name: k, type: Optional<probe.Kind>}"]));
}

#[test]
fn an_optional_inline_enum_matches() {
    accepted::<OptionalInlineKind>(&contract(KIND, &["{name: k, type: Optional<probe.Kind>}"]));
}

#[test]
fn an_optional_struct_through_a_ref_matches() {
    accepted::<OptionalWindow>(&contract(
        WINDOW,
        &["{name: w, type: Optional<probe.Window>}"],
    ));
}

#[test]
fn a_newtype_resolves_to_the_shape_of_its_type() {
    accepted::<Digested>(&contract(DIGEST, &["{name: d, type: probe.Digest}"]));
}

#[test]
fn every_integer_format_is_integer() {
    accepted::<Integers>(&contract(
        "",
        &[
            "{name: a, type: Integer}",
            "{name: b, type: Integer}",
            "{name: c, type: Integer}",
            "{name: d, type: Integer}",
        ],
    ));
}

#[test]
fn a_list_of_structs_matches_and_a_difference_inside_is_reported_with_the_list_path() {
    let contract = contract(WINDOW, &["{name: ws, type: List<probe.Window>}"]);
    accepted::<Windows>(&contract);
    let mismatch = check_shapes(&contract, Side::Input, OP, &schema_for!(WindowsToText))
        .expect_err("to is Integer in the contract");
    assert_eq!(mismatch.field, "ws[].to", "{mismatch}");
}
