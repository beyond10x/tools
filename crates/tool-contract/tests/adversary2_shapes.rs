//! Pass 2, the empty-struct rule: an empty derived struct against a contract that has input
//! fields names the missing field, a derived field against an empty contract input names the
//! extra one, `additionalProperties: true` is refused naming the values, and a
//! `deny_unknown_fields` empty struct is the empty struct.

use schemars::{JsonSchema, Schema, schema_for};
use serde_json::json;
use tool_contract::Contract;
use tool_contract::types::{Side, check_shapes};

const OP: &str = "probe.op";

// A one-operation contract; `input_block` is the whole `input:` block or empty.
fn contract(input_block: &str) -> Contract {
    let yaml = format!(
        "format: tool-contract/1\ntoolset: probe\nversion: v1\nsummary: s\noperations:\n  - name: \
{OP}\n    summary: s\n{input_block}    profile:\n      effects: [read]\n      risk: none\n      \
idempotency: idempotent\n      determinism: deterministic\n      bounds:\n        timeout_ms: \
{{max: 1}}\n      placement: local\n      capability: probe.read\n"
    );
    Contract::from_yaml(&yaml).expect("the probe contract parses")
}

const PATH_INPUT: &str = "    input:\n      - {name: path, type: String}\n";

#[allow(dead_code)]
#[derive(JsonSchema)]
struct Empty {}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct Closed {}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct WithPath {
    path: String,
}

#[test]
fn an_empty_derived_struct_against_an_input_list_names_the_missing_field() {
    let mismatch = check_shapes(&contract(PATH_INPUT), Side::Input, OP, &schema_for!(Empty))
        .expect_err("path is in the contract and not in the schema");
    assert_eq!(mismatch.field, "path", "{mismatch}");
    assert!(
        mismatch.detail.contains("absent from the derived schema"),
        "{mismatch}"
    );
}

#[test]
fn a_derived_field_against_an_empty_input_names_the_extra_field() {
    let mismatch = check_shapes(&contract(""), Side::Input, OP, &schema_for!(WithPath))
        .expect_err("path is in the schema and not in the contract");
    assert_eq!(mismatch.field, "path", "{mismatch}");
    assert!(
        mismatch.detail.contains("absent from the contract"),
        "{mismatch}"
    );
}

#[test]
fn additional_properties_true_is_refused_naming_the_values() {
    let schema: Schema = serde_json::from_value(json!({
        "type": "object",
        "additionalProperties": true
    }))
    .expect("a schema");
    let mismatch = check_shapes(&contract(""), Side::Input, OP, &schema)
        .expect_err("values of any shape have no shape");
    assert_eq!(mismatch.field, "(root)", "{mismatch}");
    assert!(mismatch.detail.contains("no declared shape"), "{mismatch}");
}

#[test]
fn a_closed_empty_struct_is_the_empty_struct() {
    check_shapes(&contract(""), Side::Input, OP, &schema_for!(Closed))
        .expect("additionalProperties: false without properties is the empty struct");
}
