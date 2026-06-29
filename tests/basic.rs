//! Core type and round-trip behavior.

mod common;

use common::{build_err, build_ok, js_stringify, run, run_err};
use fast_json_stringify::Value;
use serde_json::json;

/// Render a value and assert it equals `JSON.stringify(input)` and round-trips.
fn round_trip(schema: serde_json::Value, input: serde_json::Value) {
    let out = run(schema, input.clone());
    assert_eq!(out, js_stringify(&input));
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed, input);
}

#[test]
fn unsafe_string() {
    round_trip(
        json!({ "type": "string", "format": "unsafe" }),
        json!("hello world"),
    );
}

#[test]
fn basic_object() {
    round_trip(
        json!({
            "type": "object",
            "properties": {
                "firstName": { "type": "string" },
                "lastName": { "type": "string" },
                "age": { "type": "integer", "minimum": 0 },
                "magic": { "type": "number" }
            },
            "required": ["firstName", "lastName"]
        }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32, "magic": 42.42 }),
    );
}

#[test]
fn string_values() {
    round_trip(json!({ "type": "string" }), json!("hello world"));
    round_trip(json!({ "type": "string" }), json!("hello\nworld"));
    round_trip(json!({ "type": "string" }), json!("hello \"\"\"\" world"));
}

#[test]
fn booleans() {
    round_trip(json!({ "type": "boolean" }), json!(true));
    round_trip(json!({ "type": "boolean" }), json!(false));
}

#[test]
fn numbers_and_integers() {
    round_trip(json!({ "type": "integer" }), json!(42));
    round_trip(json!({ "type": "number" }), json!(42.42));
}

#[test]
fn deep_object() {
    round_trip(
        json!({
            "type": "object",
            "properties": {
                "firstName": { "type": "string" },
                "lastName": { "type": "string" },
                "more": {
                    "type": "object",
                    "properties": { "something": { "type": "string" } }
                }
            }
        }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "more": { "something": "else" } }),
    );
}

#[test]
fn null_type() {
    round_trip(json!({ "type": "null" }), json!(null));
}

#[test]
fn weird_keys() {
    round_trip(
        json!({ "type": "object", "properties": { "@version": { "type": "integer" } } }),
        json!({ "@version": 1 }),
    );
    round_trip(
        json!({
            "type": "object",
            "properties": {
                "@data": { "type": "object", "properties": { "id": { "type": "string" } } }
            }
        }),
        json!({ "@data": { "id": "string" } }),
    );
    round_trip(
        json!({
            "type": "object",
            "properties": {
                "spaces in key": {
                    "type": "object",
                    "properties": { "something": { "type": "integer" } }
                }
            }
        }),
        json!({ "spaces in key": { "something": 1 } }),
    );
}

#[test]
fn property_null() {
    round_trip(
        json!({ "type": "object", "properties": { "firstName": { "type": "null" } } }),
        json!({ "firstName": null }),
    );
}

#[test]
fn arrays() {
    round_trip(
        json!({
            "type": "array",
            "items": { "type": "object", "properties": { "name": { "type": "string" } } }
        }),
        json!([{ "name": "Matteo" }, { "name": "Dave" }]),
    );
    round_trip(
        json!({ "type": "array", "items": { "type": "string" } }),
        json!(["Matteo", "Dave"]),
    );
    round_trip(
        json!({ "type": "array", "items": { "type": "number" } }),
        json!([42.42, 24]),
    );
    round_trip(
        json!({ "type": "array", "items": { "type": "number" } }),
        json!([42, 24]),
    );
}

#[test]
fn nested_array_with_objects() {
    round_trip(
        json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "array",
                    "items": { "type": "object", "properties": { "name": { "type": "string" } } }
                }
            }
        }),
        json!({ "data": [{ "name": "Matteo" }, { "name": "Dave" }] }),
    );
}

#[test]
fn object_with_boolean() {
    round_trip(
        json!({ "type": "object", "properties": { "readonly": { "type": "boolean" } } }),
        json!({ "readonly": true }),
    );
}

#[test]
fn coerce_and_throw_numbers() {
    let schema = json!({
        "type": "object",
        "properties": { "age": { "type": "number" }, "distance": { "type": "integer" } }
    });
    let err = run_err(
        schema.clone(),
        json!({ "age": "hello  ", "distance": "long" }),
    );
    assert_eq!(
        err,
        "The value \"hello  \" cannot be converted to a number."
    );

    let out = run(schema, json!({ "age": "42", "distance": true }));
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed, json!({ "age": 42, "distance": 1 }));
}

#[test]
fn invalid_schema_unknown_type() {
    let err =
        build_err(json!({ "type": "Dinosaur", "properties": { "claws": { "type": "sharp" } } }));
    assert_eq!(
        err,
        "schema is invalid: data/type must be equal to one of the allowed values"
    );
}

#[test]
fn invalid_additional_properties_type() {
    let err = build_err(json!({
        "type": "object",
        "properties": {},
        "additionalProperties": { "type": "strangetype" }
    }));
    assert_eq!(
        err,
        "schema is invalid: data/additionalProperties/type must be equal to one of the allowed values"
    );
}

#[test]
fn invalid_pattern_properties_type() {
    let err = build_err(json!({
        "type": "object",
        "properties": {},
        "patternProperties": { "foo": { "type": "strangetype" } }
    }));
    assert_eq!(
        err,
        "schema is invalid: data/patternProperties/foo/type must be equal to one of the allowed values"
    );
}

#[test]
fn double_quote_strings() {
    round_trip(json!({ "type": "string" }), json!("\" double quote"));
    round_trip(json!({ "type": "string" }), json!("double quote \" 2"));
}

#[test]
fn boolean_schema_items_falls_back_to_native() {
    let schema = json!({ "type": "array", "items": true });
    let input = json!([1, true, "test"]);
    let out = run(schema, input.clone());
    assert_eq!(out, js_stringify(&input));
}

#[test]
fn regexp_source_input() {
    // A RegExp under a string schema serializes its source.
    let stringify = build_ok(json!({ "type": "string" }));
    let out = stringify.call(&Value::Regex("foo".to_string())).unwrap();
    assert_eq!(out, "\"foo\"");
}
