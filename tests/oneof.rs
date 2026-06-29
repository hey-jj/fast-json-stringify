//! oneOf branch selection.

mod common;

use common::{build_err, build_ok, run};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn multiple_types_field() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "oneOf": [{ "type": "string" }, { "type": "boolean" }] } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "str": "string" })),
        "{\"str\":\"string\"}"
    );
    assert_eq!(run(schema, json!({ "str": true })), "{\"str\":true}");
}

#[test]
fn object_or_null_strips_extra() {
    let schema = json!({
        "type": "object",
        "properties": {
            "prop": {
                "oneOf": [
                    { "type": "object", "properties": { "str": { "type": "string" } } },
                    { "type": "null" }
                ]
            }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "prop": null })),
        "{\"prop\":null}"
    );
    assert_eq!(
        run(
            schema,
            json!({ "prop": { "str": "string", "remove": "this" } })
        ),
        "{\"prop\":{\"str\":\"string\"}}"
    );
}

#[test]
fn object_or_array() {
    let schema = json!({
        "type": "object",
        "properties": {
            "prop": {
                "oneOf": [
                    { "type": "object", "properties": {}, "additionalProperties": true },
                    { "type": "array", "items": { "type": "string" } }
                ]
            }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "prop": { "str": "string" } })),
        "{\"prop\":{\"str\":\"string\"}}"
    );
    assert_eq!(
        run(schema, json!({ "prop": ["string"] })),
        "{\"prop\":[\"string\"]}"
    );
}

#[test]
fn coercion_disabled_throws() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "oneOf": [{ "type": "string" }] } }
    });
    let stringify = build_ok(schema);
    assert!(stringify.call(&Value::from(json!({ "str": 1 }))).is_err());
}

#[test]
fn union_of_objects() {
    let schema = json!({
        "type": "object",
        "properties": {
            "oneOfSchema": {
                "oneOf": [
                    { "type": "object", "properties": { "baz": { "type": "number" } }, "required": ["baz"] },
                    { "type": "object", "properties": { "bar": { "type": "string" } }, "required": ["bar"] }
                ]
            }
        },
        "required": ["oneOfSchema"]
    });
    assert_eq!(
        run(schema.clone(), json!({ "oneOfSchema": { "baz": 5 } })),
        "{\"oneOfSchema\":{\"baz\":5}}"
    );
    assert_eq!(
        run(schema, json!({ "oneOfSchema": { "bar": "foo" } })),
        "{\"oneOfSchema\":{\"bar\":\"foo\"}}"
    );
}

#[test]
fn oneof_and_ref_one_level() {
    let schema = json!({
        "type": "object",
        "properties": {
            "cs": { "oneOf": [{ "$ref": "#/definitions/Option" }, { "type": "boolean" }] }
        },
        "definitions": { "Option": { "type": "string" } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "cs": "franco" })),
        "{\"cs\":\"franco\"}"
    );
    assert_eq!(run(schema, json!({ "cs": true })), "{\"cs\":true}");
}

fn array_oneof_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["data"],
        "properties": {
            "data": {
                "type": "array",
                "minItems": 1,
                "items": { "oneOf": [{ "type": "string" }, { "type": "number" }] }
            }
        }
    })
}

#[test]
fn one_array_item_matches() {
    let schema = array_oneof_schema();
    assert_eq!(
        run(schema.clone(), json!({ "data": ["foo"] })),
        "{\"data\":[\"foo\"]}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "data": [1] })),
        "{\"data\":[1]}"
    );
    let stringify = build_ok(schema);
    assert!(stringify
        .call(&Value::from(json!({ "data": [false, "foo"] })))
        .is_err());
}

#[test]
fn some_array_items_match() {
    let schema = array_oneof_schema();
    assert_eq!(
        run(schema.clone(), json!({ "data": ["foo", 5] })),
        "{\"data\":[\"foo\",5]}"
    );
    let stringify = build_ok(schema);
    assert!(stringify
        .call(&Value::from(json!({ "data": [false, "foo", true, 5] })))
        .is_err());
}

#[test]
fn no_array_items_match_throws() {
    let stringify = build_ok(array_oneof_schema());
    assert!(stringify
        .call(&Value::from(json!({ "data": [null, false, true, [], {}] })))
        .is_err());
}

#[test]
fn invalid_oneof_schema() {
    let err = build_err(json!({
        "type": "object",
        "properties": { "prop": { "oneOf": "not array" } }
    }));
    assert!(err.contains("schema is invalid"));
}
