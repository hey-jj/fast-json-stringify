//! anyOf branch selection.

mod common;

use common::{build_err, build_ok, build_ok_opts, run};
use fast_json_stringify::{Object, Options, Value};
use serde_json::json;

#[test]
fn multiple_types_field() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "anyOf": [{ "type": "string" }, { "type": "boolean" }] } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "str": "string" })),
        "{\"str\":\"string\"}"
    );
    assert_eq!(run(schema, json!({ "str": true })), "{\"str\":true}");
}

#[test]
fn object_or_null() {
    let schema = json!({
        "type": "object",
        "properties": {
            "prop": {
                "anyOf": [
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
        run(schema, json!({ "prop": { "str": "string" } })),
        "{\"prop\":{\"str\":\"string\"}}"
    );
}

#[test]
fn object_or_array() {
    let schema = json!({
        "type": "object",
        "properties": {
            "prop": {
                "anyOf": [
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
        "properties": { "str": { "anyOf": [{ "type": "string" }] } }
    });
    let stringify = build_ok(schema);
    assert!(stringify.call(&Value::from(json!({ "str": 1 }))).is_err());
}

#[test]
fn union_of_objects() {
    let schema = json!({
        "type": "object",
        "properties": {
            "anyOfSchema": {
                "anyOf": [
                    { "type": "object", "properties": { "baz": { "type": "number" } }, "required": ["baz"] },
                    { "type": "object", "properties": { "bar": { "type": "string" } }, "required": ["bar"] }
                ]
            }
        },
        "required": ["anyOfSchema"]
    });
    assert_eq!(
        run(schema.clone(), json!({ "anyOfSchema": { "baz": 5 } })),
        "{\"anyOfSchema\":{\"baz\":5}}"
    );
    assert_eq!(
        run(schema, json!({ "anyOfSchema": { "bar": "foo" } })),
        "{\"anyOfSchema\":{\"bar\":\"foo\"}}"
    );
}

#[test]
fn anyof_and_ref_one_level() {
    let schema = json!({
        "type": "object",
        "properties": {
            "cs": { "anyOf": [{ "$ref": "#/definitions/Option" }, { "type": "boolean" }] }
        },
        "definitions": { "Option": { "type": "string" } }
    });
    assert_eq!(
        run(schema.clone(), json!({ "cs": "franco" })),
        "{\"cs\":\"franco\"}"
    );
    assert_eq!(run(schema, json!({ "cs": true })), "{\"cs\":true}");
}

#[test]
fn anyof_and_ref_two_levels() {
    let schema = json!({
        "type": "object",
        "properties": {
            "cs": { "anyOf": [{ "$ref": "#/definitions/Option" }, { "type": "boolean" }] }
        },
        "definitions": {
            "Option": { "anyOf": [{ "type": "number" }, { "type": "boolean" }] }
        }
    });
    assert_eq!(run(schema, json!({ "cs": 3 })), "{\"cs\":3}");
}

#[test]
fn anyof_external_ref() {
    let external = json!({
        "external": {
            "definitions": {
                "def": {
                    "type": "object",
                    "properties": { "prop": { "anyOf": [{ "$ref": "external2#/definitions/other" }] } }
                }
            }
        },
        "external2": {
            "definitions": {
                "internal": { "type": "string" },
                "other": {
                    "type": "object",
                    "properties": { "prop2": { "$ref": "#/definitions/internal" } }
                }
            }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": { "obj": { "$ref": "external#/definitions/def" } }
    });
    let opts = Options {
        schema: external
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        ..Options::new()
    };
    let stringify = build_ok_opts(schema, opts);
    let out = stringify
        .call(&Value::from(
            json!({ "obj": { "prop": { "prop2": "test" } } }),
        ))
        .unwrap();
    assert_eq!(out, "{\"obj\":{\"prop\":{\"prop2\":\"test\"}}}");
}

#[test]
fn anyof_array_items() {
    let schema = json!({
        "type": "array",
        "items": {
            "anyOf": [
                { "type": "object", "properties": { "savedId": { "type": "string" } }, "required": ["savedId"] },
                { "type": "object", "properties": { "error": { "type": "string" } }, "required": ["error"] }
            ]
        }
    });
    assert_eq!(
        run(schema, json!([{ "savedId": "great" }, { "error": "oops" }])),
        "[{\"savedId\":\"great\"},{\"error\":\"oops\"}]"
    );
}

#[test]
fn anyof_nested_date_formats() {
    // date format inside an anyOf object branch.
    let schema = json!({
        "type": "object",
        "properties": {
            "prop": {
                "anyOf": [{
                    "type": "object",
                    "properties": { "nestedProp": { "type": "string", "format": "date" } }
                }]
            }
        }
    });
    let stringify = build_ok(schema);
    let mut inner = Object::new();
    inner.insert("nestedProp", Value::Date(1674263005800));
    let mut outer = Object::new();
    outer.insert("prop", Value::Object(inner));
    assert_eq!(
        stringify.call(&Value::Object(outer)).unwrap(),
        "{\"prop\":{\"nestedProp\":\"2023-01-21\"}}"
    );
}

#[test]
fn anyof_string_date_passthrough() {
    let schema = json!({
        "type": "object",
        "properties": { "prop": { "anyOf": [{ "type": "string", "format": "date" }, { "type": "null" }] } }
    });
    assert_eq!(
        run(schema, json!({ "prop": "2011-01-01" })),
        "{\"prop\":\"2011-01-01\"}"
    );
}

#[test]
fn anyof_required_props() {
    let schema = json!({
        "type": "object",
        "properties": {
            "prop1": { "type": "string" },
            "prop2": { "type": "string" },
            "prop3": { "type": "string" }
        },
        "required": ["prop1"],
        "anyOf": [{ "required": ["prop2"] }, { "required": ["prop3"] }]
    });
    assert_eq!(
        run(schema.clone(), json!({ "prop1": "test", "prop2": "test2" })),
        "{\"prop1\":\"test\",\"prop2\":\"test2\"}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "prop1": "test", "prop3": "test3" })),
        "{\"prop1\":\"test\",\"prop3\":\"test3\"}"
    );
    assert_eq!(
        run(
            schema,
            json!({ "prop1": "test", "prop2": "test2", "prop3": "test3" })
        ),
        "{\"prop1\":\"test\",\"prop2\":\"test2\",\"prop3\":\"test3\"}"
    );
}

#[test]
fn build_merged_schemas_twice() {
    let schema = json!({
        "type": "object",
        "properties": {
            "enums": {
                "type": "string",
                "anyOf": [{ "type": "string", "const": "FOO" }, { "type": "string", "const": "BAR" }]
            }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "enums": "FOO" })),
        "{\"enums\":\"FOO\"}"
    );
    assert_eq!(
        run(schema, json!({ "enums": "BAR" })),
        "{\"enums\":\"BAR\"}"
    );
}

#[test]
fn invalid_anyof_schema() {
    let err = build_err(json!({
        "type": "object",
        "properties": { "prop": { "anyOf": "not array" } }
    }));
    assert!(err.contains("schema is invalid"));
}

#[test]
fn large_enum_anyof_array_items() {
    // A string-enum branch with more than 100 entries, plus a null branch.
    // Branch selection must hold for a large enum, not just a small one.
    let mut codes: Vec<String> = vec!["EUR".to_string(), "USD".to_string()];
    for i in 0..200u32 {
        let a = (b'A' + (i % 26) as u8) as char;
        let b = (b'A' + ((i / 26) % 26) as u8) as char;
        let c = (b'A' + ((i / 7) % 26) as u8) as char;
        codes.push(format!("{a}{b}{c}"));
    }
    let schema = json!({
        "type": "array",
        "items": { "anyOf": [
            { "type": "string", "enum": codes },
            { "type": "null" }
        ] }
    });
    assert_eq!(
        run(schema, json!(["EUR", "USD", null])),
        "[\"EUR\",\"USD\",null]"
    );
}
