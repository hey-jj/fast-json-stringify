//! Type inference from keywords when `type` is absent.

mod common;

use common::{js_stringify, run};
use serde_json::json;

fn check(schema: serde_json::Value, input: serde_json::Value) {
    let out = run(schema, input.clone());
    assert_eq!(out, js_stringify(&input));
}

#[test]
fn infer_object_by_properties() {
    check(
        json!({ "properties": { "name": { "type": "string" } } }),
        json!({ "name": "foo" }),
    );
}

#[test]
fn infer_nested_object_by_properties() {
    check(
        json!({
            "properties": {
                "more": { "properties": { "something": { "type": "string" } } }
            }
        }),
        json!({ "more": { "something": "else" } }),
    );
}

#[test]
fn infer_array_by_items() {
    check(
        json!({ "type": "object", "properties": { "ids": { "items": { "type": "string" } } } }),
        json!({ "ids": ["test"] }),
    );
}

#[test]
fn infer_string_by_max_length() {
    check(
        json!({ "type": "object", "properties": { "name": { "maxLength": 3 } } }),
        json!({ "name": "foo" }),
    );
}

#[test]
fn infer_number_by_maximum() {
    check(
        json!({ "type": "object", "properties": { "age": { "maximum": 18 } } }),
        json!({ "age": 18 }),
    );
}
