//! `additionalProperties` coercion and precedence.

mod common;

use common::{build_ok, run};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn additional_string_coerce() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string" } },
        "additionalProperties": { "type": "string" }
    });
    let obj = json!({ "str": "test", "foo": 42, "ofoo": true, "foof": "string", "objfoo": { "a": true } });
    assert_eq!(
        run(schema, obj),
        "{\"str\":\"test\",\"foo\":\"42\",\"ofoo\":\"true\",\"foof\":\"string\",\"objfoo\":\"[object Object]\"}"
    );
}

#[test]
fn additional_does_not_change_properties() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "additionalProperties": { "type": "number" }
    });
    assert_eq!(
        run(schema, json!({ "foo": "42", "ofoo": 42 })),
        "{\"foo\":\"42\",\"ofoo\":42}"
    );
}

#[test]
fn properties_pattern_additional_precedence() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "patternProperties": { "foo": { "type": "string" } },
        "additionalProperties": { "type": "number" }
    });
    // foo -> property, ofoo -> pattern (contains foo), test -> additional.
    assert_eq!(
        run(schema, json!({ "foo": "42", "ofoo": 42, "test": "42" })),
        "{\"foo\":\"42\",\"ofoo\":\"42\",\"test\":42}"
    );
}

#[test]
fn additional_true_native() {
    let schema = json!({ "type": "object", "properties": {}, "additionalProperties": true });
    let obj =
        json!({ "foo": true, "ofoo": 42, "arrfoo": ["array", "test"], "objfoo": { "a": "world" } });
    assert_eq!(
        run(schema, obj),
        "{\"foo\":true,\"ofoo\":42,\"arrfoo\":[\"array\",\"test\"],\"objfoo\":{\"a\":\"world\"}}"
    );
}

#[test]
fn additional_string_coerce_all() {
    let schema =
        json!({ "type": "object", "properties": {}, "additionalProperties": { "type": "string" } });
    let obj =
        json!({ "foo": true, "ofoo": 42, "arrfoo": ["array", "test"], "objfoo": { "a": "world" } });
    assert_eq!(
        run(schema, obj),
        "{\"foo\":\"true\",\"ofoo\":\"42\",\"arrfoo\":\"array,test\",\"objfoo\":\"[object Object]\"}"
    );
}

#[test]
fn additional_number_coerce() {
    let schema =
        json!({ "type": "object", "properties": {}, "additionalProperties": { "type": "number" } });
    assert_eq!(
        run(schema, json!({ "foo": true, "ofoo": "42" })),
        "{\"foo\":1,\"ofoo\":42}"
    );
}

#[test]
fn additional_boolean_coerce() {
    let schema = json!({ "type": "object", "properties": {}, "additionalProperties": { "type": "boolean" } });
    let obj = json!({ "foo": "true", "ofoo": 0, "arrfoo": [1, 2], "objfoo": { "a": true } });
    assert_eq!(
        run(schema, obj),
        "{\"foo\":true,\"ofoo\":false,\"arrfoo\":true,\"objfoo\":true}"
    );
}

#[test]
fn additional_object_coerce() {
    let schema = json!({
        "type": "object",
        "properties": {},
        "additionalProperties": { "type": "object", "properties": { "answer": { "type": "number" } } }
    });
    assert_eq!(
        run(schema, json!({ "objfoo": { "answer": 42 } })),
        "{\"objfoo\":{\"answer\":42}}"
    );
}

#[test]
fn additional_array_coerce_and_throw() {
    let schema = json!({
        "type": "object",
        "properties": {},
        "additionalProperties": { "type": "array", "items": { "type": "string" } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "arrfoo": [1, 2] })),
        "{\"arrfoo\":[\"1\",\"2\"]}"
    );

    let stringify = build_ok(schema);
    let bad = Value::from(json!({ "foo": "true", "ofoo": 0, "objfoo": { "tyrion": "lannister" } }));
    assert!(stringify.call(&bad).is_err());
}

#[test]
fn additional_empty_schema() {
    let schema = json!({ "type": "object", "additionalProperties": {} });
    assert_eq!(
        run(schema, json!({ "a": 1, "b": true, "c": null })),
        "{\"a\":1,\"b\":true,\"c\":null}"
    );
}

#[test]
fn additional_nested_empty_schema() {
    let schema = json!({
        "type": "object",
        "properties": { "data": { "type": "object", "additionalProperties": {} } },
        "required": ["data"]
    });
    assert_eq!(
        run(schema, json!({ "data": { "a": 1, "b": true, "c": null } })),
        "{\"data\":{\"a\":1,\"b\":true,\"c\":null}}"
    );
}

#[test]
fn nested_additional_properties() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": { "ap": { "type": "object", "additionalProperties": { "type": "string" } } }
        }
    });
    assert_eq!(
        run(schema, json!([{ "ap": { "value": "string" } }])),
        "[{\"ap\":{\"value\":\"string\"}}]"
    );
}

#[test]
fn nested_additional_true() {
    let schema = json!({
        "type": "object",
        "properties": { "ap": { "type": "object", "additionalProperties": true } }
    });
    assert_eq!(
        run(
            schema,
            json!({ "ap": { "value": "string", "someNumber": 42 } })
        ),
        "{\"ap\":{\"value\":\"string\",\"someNumber\":42}}"
    );
}

#[test]
fn enum_without_type_acts_as_any() {
    let schema = json!({
        "type": "object",
        "properties": { "ap": { "enum": ["foobar", 42, ["foo", "bar"], {}] } }
    });
    assert_eq!(
        run(schema, json!({ "ap": { "additional": "field" } })),
        "{\"ap\":{\"additional\":\"field\"}}"
    );
}

#[test]
fn enum_with_additional_false_blocks() {
    let schema = json!({
        "type": "object",
        "properties": { "ap": { "additionalProperties": false, "enum": ["foobar", 42, ["foo", "bar"], {}] } }
    });
    assert_eq!(
        run(schema, json!({ "ap": { "additional": "field" } })),
        "{\"ap\":{}}"
    );
}

#[test]
fn additional_false_blocks_unknown() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string" } },
        "additionalProperties": false
    });
    assert_eq!(
        run(schema, json!({ "str": "x", "extra": "dropped" })),
        "{\"str\":\"x\"}"
    );
}
