//! Required property handling.

mod common;

use common::{build_ok, run, run_err};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn required_field_present_and_missing() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string" }, "num": { "type": "integer" } },
        "required": ["str"]
    });
    assert_eq!(
        run(schema.clone(), json!({ "str": "string" })),
        "{\"str\":\"string\"}"
    );
    let err = run_err(schema, json!({ "num": 42 }));
    assert_eq!(err, "\"str\" is required!");
}

#[test]
fn required_not_in_properties() {
    let schema = json!({
        "type": "object",
        "properties": { "num": { "type": "integer" } },
        "required": ["str"]
    });
    assert_eq!(run_err(schema.clone(), json!({})), "\"str\" is required!");
    assert_eq!(
        run_err(schema, json!({ "num": 42 })),
        "\"str\" is required!"
    );
}

#[test]
fn required_with_additional_true() {
    let schema = json!({
        "type": "object",
        "properties": { "num": { "type": "integer" } },
        "additionalProperties": true,
        "required": ["str"]
    });
    assert_eq!(run_err(schema.clone(), json!({})), "\"str\" is required!");
    assert_eq!(
        run_err(schema, json!({ "num": 42 })),
        "\"str\" is required!"
    );
}

#[test]
fn multiple_required_not_in_properties() {
    let schema = json!({
        "type": "object",
        "properties": { "num": { "type": "integer" } },
        "additionalProperties": true,
        "required": ["num", "key1", "key2"]
    });
    assert_eq!(run_err(schema.clone(), json!({})), "\"key1\" is required!");
    assert_eq!(
        run_err(schema.clone(), json!({ "key1": 42, "key2": 42 })),
        "\"num\" is required!"
    );
    assert_eq!(
        run_err(schema, json!({ "num": 42, "key1": "some" })),
        "\"key2\" is required!"
    );
}

#[test]
fn required_bool() {
    let schema = json!({
        "type": "object",
        "properties": { "num": { "type": "integer" } },
        "additionalProperties": true,
        "required": ["bool"]
    });
    assert_eq!(run_err(schema.clone(), json!({})), "\"bool\" is required!");
    let stringify = build_ok(schema);
    assert!(stringify
        .call(&Value::from(json!({ "bool": false })))
        .is_ok());
}

#[test]
fn required_numbers() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string" }, "num": { "type": "integer" } },
        "required": ["num"]
    });
    assert_eq!(run(schema.clone(), json!({ "num": 42 })), "{\"num\":42}");
    assert_eq!(
        run_err(schema, json!({ "num": "aaa" })),
        "The value \"aaa\" cannot be converted to an integer."
    );
}
