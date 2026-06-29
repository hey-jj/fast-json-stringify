//! `nullable: true` and `type: [..., "null"]` behavior.

mod common;

use common::{build_ok, build_ok_opts, js_stringify, run};
use fast_json_stringify::{LargeArrayMechanism, Options, Value};
use serde_json::json;

#[test]
fn nullable_per_type_renders_null() {
    for ty in ["string", "number", "integer", "boolean", "null"] {
        let schema = json!({ "type": ty, "nullable": true });
        assert_eq!(run(schema, json!(null)), "null");
    }
    assert_eq!(
        run(
            json!({ "type": "array", "nullable": true, "items": {} }),
            json!(null)
        ),
        "null"
    );
    assert_eq!(
        run(json!({ "type": "object", "nullable": true }), json!(null)),
        "null"
    );
}

#[test]
fn nullable_date_formats() {
    for format in ["date-time", "date", "time"] {
        let schema = json!({ "type": "string", "format": format, "nullable": true });
        assert_eq!(run(schema, json!(null)), "null");
    }
}

#[test]
fn complex_nullable_object() {
    let schema = json!({
        "type": "object",
        "properties": {
            "nullableString": { "type": "string", "nullable": true },
            "nullableNumber": { "type": "number", "nullable": true },
            "nullableInteger": { "type": "integer", "nullable": true },
            "nullableBoolean": { "type": "boolean", "nullable": true },
            "nullableNull": { "type": "null", "nullable": true },
            "nullableArray": { "type": "array", "nullable": true, "items": {} },
            "nullableObject": { "type": "object", "nullable": true },
            "objectWithNullableProps": {
                "type": "object",
                "nullable": false,
                "additionalProperties": true,
                "properties": {
                    "nullableString": { "type": "string", "nullable": true },
                    "nullableArray": { "type": "array", "nullable": true, "items": {} }
                }
            },
            "arrayWithNullableItems": {
                "type": "array",
                "nullable": true,
                "items": { "type": ["integer", "string"], "nullable": true }
            }
        }
    });
    let data = json!({
        "nullableString": null,
        "nullableNumber": null,
        "nullableInteger": null,
        "nullableBoolean": null,
        "nullableNull": null,
        "nullableArray": null,
        "nullableObject": null,
        "objectWithNullableProps": {
            "additionalProp": null,
            "nullableString": null,
            "nullableArray": null
        },
        "arrayWithNullableItems": [1, 2, null]
    });
    let out = run(schema, data.clone());
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed, data);
}

#[test]
fn nullable_type_array_in_root() {
    let schema = json!({
        "type": ["object", "null"],
        "properties": { "foo": { "type": "string" } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "foo": "bar" })),
        "{\"foo\":\"bar\"}"
    );
    assert_eq!(run(schema, json!(null)), "null");
}

#[test]
fn large_array_of_nullable_strings_default() {
    let schema = json!({
        "type": "object",
        "properties": { "ids": { "type": "array", "items": { "type": "string", "nullable": true } } }
    });
    let opts = Options {
        large_array_size: 20_000,
        large_array_mechanism: LargeArrayMechanism::Default,
        ..Options::new()
    };
    let stringify = build_ok_opts(schema, opts);
    let data = json!({ "ids": vec![serde_json::Value::Null; 20_000] });
    let out = stringify.call(&Value::from(data.clone())).unwrap();
    assert_eq!(out, js_stringify(&data));
}

#[test]
fn oneof_with_nullable_item() {
    let schema = json!({
        "type": "object",
        "properties": {
            "data": {
                "oneOf": [
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": { "id": { "type": "integer", "minimum": 1 } },
                            "additionalProperties": false,
                            "required": ["id"]
                        }
                    },
                    {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": { "job": { "type": "string", "nullable": true } },
                            "additionalProperties": false,
                            "required": ["job"]
                        }
                    }
                ]
            }
        },
        "required": ["data"],
        "additionalProperties": false
    });
    assert_eq!(
        run(schema, json!({ "data": [{ "job": null }] })),
        "{\"data\":[{\"job\":null}]}"
    );
}

#[test]
fn type_mismatch_in_oneof_items_throws() {
    let schema = json!({
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
    });
    assert_eq!(
        run(schema.clone(), json!({ "data": [1, "testing"] })),
        "{\"data\":[1,\"testing\"]}"
    );
    let stringify = build_ok(schema);
    assert!(stringify
        .call(&Value::from(json!({ "data": [false, "testing"] })))
        .is_err());
}
