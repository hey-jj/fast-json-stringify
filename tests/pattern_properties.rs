//! `patternProperties` coercion, precedence, and regex validation.

mod common;

use common::{build_err, build_ok, run};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn pattern_with_properties() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string" } },
        "patternProperties": { "foo": { "type": "string" } }
    });
    let obj = json!({ "str": "test", "foo": 42, "ofoo": true, "foof": "string", "objfoo": { "a": true }, "notMe": false });
    assert_eq!(
        run(schema, obj),
        "{\"str\":\"test\",\"foo\":\"42\",\"ofoo\":\"true\",\"foof\":\"string\",\"objfoo\":\"[object Object]\"}"
    );
}

#[test]
fn pattern_does_not_change_properties() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "patternProperties": { "foo": { "type": "number" } }
    });
    assert_eq!(
        run(schema, json!({ "foo": "42", "ofoo": 42 })),
        "{\"foo\":\"42\",\"ofoo\":42}"
    );
}

#[test]
fn pattern_string_coerce() {
    let schema = json!({ "type": "object", "properties": {}, "patternProperties": { "foo": { "type": "string" } } });
    let obj =
        json!({ "foo": true, "ofoo": 42, "arrfoo": ["array", "test"], "objfoo": { "a": "world" } });
    assert_eq!(
        run(schema, obj),
        "{\"foo\":\"true\",\"ofoo\":\"42\",\"arrfoo\":\"array,test\",\"objfoo\":\"[object Object]\"}"
    );
}

#[test]
fn pattern_number_coerce_and_throw() {
    let schema = json!({ "type": "object", "properties": {}, "patternProperties": { "foo": { "type": "number" } } });
    assert_eq!(
        run(schema.clone(), json!({ "foo": true, "ofoo": "42" })),
        "{\"foo\":1,\"ofoo\":42}"
    );

    let stringify = build_ok(schema);
    let bad = Value::from(json!({ "xfoo": "string", "arrfoo": [1, 2], "objfoo": { "num": 42 } }));
    assert!(stringify.call(&bad).is_err());
}

#[test]
fn pattern_boolean_coerce() {
    let schema = json!({ "type": "object", "properties": {}, "patternProperties": { "foo": { "type": "boolean" } } });
    let obj = json!({ "foo": "true", "ofoo": 0, "arrfoo": [1, 2], "objfoo": { "a": true } });
    assert_eq!(
        run(schema, obj),
        "{\"foo\":true,\"ofoo\":false,\"arrfoo\":true,\"objfoo\":true}"
    );
}

#[test]
fn pattern_object_coerce() {
    let schema = json!({
        "type": "object",
        "properties": {},
        "patternProperties": { "foo": { "type": "object", "properties": { "answer": { "type": "number" } } } }
    });
    assert_eq!(
        run(schema, json!({ "objfoo": { "answer": 42 } })),
        "{\"objfoo\":{\"answer\":42}}"
    );
}

#[test]
fn pattern_array_coerce_and_throw() {
    let schema = json!({
        "type": "object",
        "properties": {},
        "patternProperties": { "foo": { "type": "array", "items": { "type": "string" } } }
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
fn invalid_regex_rejected_at_build() {
    let err = build_err(json!({
        "type": "object",
        "properties": {},
        "patternProperties": { "foo/\\": { "type": "array", "items": { "type": "string" } } }
    }));
    assert_eq!(
        err,
        "schema is invalid: data/patternProperties must match format \"regex\""
    );
}
