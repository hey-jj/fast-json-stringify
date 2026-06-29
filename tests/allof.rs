//! allOf merges subschemas before serializing.

mod common;

use common::{build_err, build_ok, build_ok_opts, js_stringify, run};
use fast_json_stringify::{Object, Options, Value};
use serde_json::json;

#[test]
fn combine_type_and_format() {
    let schema = json!({ "allOf": [{ "type": "string" }, { "format": "time" }] });
    let stringify = build_ok(schema);
    // 2023-01-21T01:03:25.800Z -> 01:03:25.
    assert_eq!(
        stringify.call(&Value::Date(1674263005800)).unwrap(),
        "\"01:03:25\""
    );
}

#[test]
fn combine_additional_properties() {
    let schema = json!({
        "allOf": [
            { "type": "object" },
            { "type": "object", "additionalProperties": { "type": "boolean" } }
        ]
    });
    let data = json!({ "property": true });
    assert_eq!(run(schema, data.clone()), js_stringify(&data));
}

#[test]
fn combine_pattern_properties() {
    let schema = json!({
        "allOf": [
            { "type": "object" },
            { "type": "object", "patternProperties": { "foo": { "type": "number" } } }
        ]
    });
    let data = json!({ "foo": 42 });
    assert_eq!(run(schema, data.clone()), js_stringify(&data));
}

#[test]
fn multiple_schemas_required_first() {
    let schema = json!({
        "type": "object",
        "allOf": [
            {
                "type": "object",
                "required": ["name"],
                "properties": { "name": { "type": "string" }, "tag": { "type": "string" } }
            },
            {
                "required": ["id"],
                "type": "object",
                "properties": { "id": { "type": "integer" } }
            }
        ]
    });
    let stringify = build_ok(schema);
    assert!(stringify.call(&Value::from(json!({ "id": 1 }))).is_err());
    assert!(stringify
        .call(&Value::from(json!({ "name": "string" })))
        .is_err());
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "id": 1, "name": "string" })))
            .unwrap(),
        "{\"name\":\"string\",\"id\":1}"
    );
    assert_eq!(
        stringify
            .call(&Value::from(
                json!({ "id": 1, "name": "string", "tag": "otherString" })
            ))
            .unwrap(),
        "{\"name\":\"string\",\"id\":1,\"tag\":\"otherString\"}"
    );
}

#[test]
fn single_schema() {
    let schema = json!({
        "type": "object",
        "allOf": [{ "required": ["id"], "type": "object", "properties": { "id": { "type": "integer" } } }]
    });
    assert_eq!(run(schema, json!({ "id": 1 })), "{\"id\":1}");
}

#[test]
fn empty_allof_rejected() {
    let err = build_err(json!({ "type": "object", "allOf": [] }));
    assert_eq!(
        err,
        "schema is invalid: data/allOf must NOT have fewer than 1 items"
    );
}

#[test]
fn nested_allofs() {
    let schema = json!({
        "type": "object",
        "allOf": [
            { "required": ["id1"], "type": "object", "properties": { "id1": { "type": "integer" } } },
            {
                "allOf": [
                    { "type": "object", "properties": { "id2": { "type": "integer" } } },
                    { "type": "object", "properties": { "id3": { "type": "integer" } } }
                ]
            }
        ]
    });
    assert_eq!(
        run(schema, json!({ "id1": 1, "id2": 2, "id3": 3, "id4": 4 })),
        "{\"id1\":1,\"id2\":2,\"id3\":3}"
    );
}

#[test]
fn ref_in_allof() {
    let schema = json!({
        "type": "object",
        "definitions": { "id1": { "type": "object", "properties": { "id1": { "type": "integer" } } } },
        "allOf": [{ "$ref": "#/definitions/id1" }]
    });
    assert_eq!(run(schema, json!({ "id1": 1, "id2": 2 })), "{\"id1\":1}");
}

#[test]
fn ref_and_object_in_allof() {
    let schema = json!({
        "type": "object",
        "definitions": { "id1": { "type": "object", "properties": { "id1": { "type": "integer" } } } },
        "allOf": [
            { "$ref": "#/definitions/id1" },
            { "type": "object", "properties": { "id2": { "type": "integer" } } }
        ]
    });
    assert_eq!(
        run(schema, json!({ "id1": 1, "id2": 2, "id3": 3 })),
        "{\"id1\":1,\"id2\":2}"
    );
}

#[test]
fn multiple_refs_in_allof() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "id1": { "type": "object", "properties": { "id1": { "type": "integer" } } },
            "id2": { "type": "object", "properties": { "id2": { "type": "integer" } } }
        },
        "allOf": [{ "$ref": "#/definitions/id1" }, { "$ref": "#/definitions/id2" }]
    });
    assert_eq!(
        run(schema, json!({ "id1": 1, "id2": 2, "id3": 3 })),
        "{\"id1\":1,\"id2\":2}"
    );
}

#[test]
fn nested_allof_in_ref() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "group": {
                "type": "object",
                "allOf": [
                    { "properties": { "id2": { "type": "integer" } } },
                    { "properties": { "id3": { "type": "integer" } } }
                ]
            }
        },
        "allOf": [
            { "type": "object", "properties": { "id1": { "type": "integer" } }, "required": ["id1"] },
            { "$ref": "#/definitions/group" }
        ]
    });
    assert_eq!(
        run(schema, json!({ "id1": 1, "id2": 2, "id3": 3, "id4": 4 })),
        "{\"id1\":1,\"id2\":2,\"id3\":3}"
    );
}

#[test]
fn external_refs_in_allof() {
    let external = json!({
        "first": { "definitions": { "id1": { "type": "object", "properties": { "id1": { "type": "integer" } } } } },
        "second": { "definitions": { "id2": { "$id": "#id2", "type": "object", "properties": { "id2": { "type": "integer" } } } } }
    });
    let schema = json!({
        "type": "object",
        "allOf": [{ "$ref": "first#/definitions/id1" }, { "$ref": "second#/definitions/id2" }]
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
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "id1": 1, "id2": 2, "id3": 3 })))
            .unwrap(),
        "{\"id1\":1,\"id2\":2}"
    );
}

#[test]
fn type_mismatch_throws() {
    let err = build_err(json!({ "allOf": [{ "type": "string" }, { "type": "number" }] }));
    assert_eq!(err, "Failed to merge \"type\" keyword schemas.");
}

#[test]
fn format_mismatch_throws() {
    let err = build_err(json!({ "allOf": [{ "format": "date" }, { "format": "time" }] }));
    assert_eq!(err, "Failed to merge \"format\" keyword schemas.");
}

#[test]
fn recursive_nested_allof() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "additionalProperties": false, "allOf": [{ "$ref": "#" }] } }
    });
    let data = json!({ "foo": {} });
    assert_eq!(run(schema, data.clone()), js_stringify(&data));
}

#[test]
fn recursive_double_nested_allof() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "additionalProperties": false, "allOf": [{ "allOf": [{ "$ref": "#" }] }] } }
    });
    let data = json!({ "foo": {} });
    assert_eq!(run(schema, data.clone()), js_stringify(&data));
}

#[test]
fn dollar_ref_property_name() {
    let schema = json!({
        "type": "object",
        "properties": { "outside": { "$ref": "#/$defs/outside" } },
        "$defs": {
            "inside": { "type": "object", "properties": { "$ref": { "type": "string" } } },
            "outside": { "allOf": [{ "$ref": "#/$defs/inside" }] }
        }
    });
    let stringify = build_ok(schema);
    let mut inner = Object::new();
    inner.insert("$ref", Value::String("true".into()));
    let mut outer = Object::new();
    outer.insert("outside", Value::Object(inner));
    assert_eq!(
        stringify.call(&Value::Object(outer)).unwrap(),
        "{\"outside\":{\"$ref\":\"true\"}}"
    );
}
