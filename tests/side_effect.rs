//! Building must not mutate the input schema.
//!
//! The builder works on owned data, so the caller's schema stays intact. Each
//! case asserts both the output and that the schema is unchanged.

mod common;

use common::build_ok_opts;
use fast_json_stringify::{Options, Value};
use serde_json::json;

fn ext_opts(external: serde_json::Value) -> Options {
    Options {
        schema: external
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        ..Options::new()
    }
}

#[test]
fn oneof_with_ref_does_not_mutate() {
    let external = json!({
        "externalId": { "$id": "externalId", "type": "object", "properties": { "name": { "type": "string" } } }
    });
    let schema = json!({
        "$id": "mainSchema",
        "type": "object",
        "properties": { "people": { "oneOf": [{ "$ref": "externalId" }] } }
    });
    let cloned = schema.clone();
    let stringify = build_ok_opts(schema.clone(), ext_opts(external));
    let out = stringify
        .call(&Value::from(
            json!({ "people": { "name": "hello", "foo": "bar" } }),
        ))
        .unwrap();
    assert_eq!(out, "{\"people\":{\"name\":\"hello\"}}");
    assert_eq!(schema, cloned);
}

#[test]
fn oneof_and_anyof_with_ref_does_not_mutate() {
    let external = json!({
        "externalSchema": { "$id": "externalSchema", "type": "object", "properties": { "name": { "type": "string" } } }
    });
    let schema = json!({
        "$id": "rootSchema",
        "type": "object",
        "properties": {
            "people": { "oneOf": [{ "$ref": "externalSchema" }] },
            "love": { "anyOf": [{ "$ref": "#/definitions/foo" }, { "type": "boolean" }] }
        },
        "definitions": { "foo": { "type": "string" } }
    });
    let cloned = schema.clone();
    let stringify = build_ok_opts(schema.clone(), ext_opts(external));
    assert_eq!(
        stringify
            .call(&Value::from(
                json!({ "people": { "name": "hello", "foo": "bar" }, "love": "music" })
            ))
            .unwrap(),
        "{\"people\":{\"name\":\"hello\"},\"love\":\"music\"}"
    );
    assert_eq!(
        stringify
            .call(&Value::from(
                json!({ "people": { "name": "hello", "foo": "bar" }, "love": true })
            ))
            .unwrap(),
        "{\"people\":{\"name\":\"hello\"},\"love\":true}"
    );
    assert_eq!(schema, cloned);
}

#[test]
fn multiple_ref_tree_does_not_mutate() {
    let external = json!({
        "deepId": { "$id": "deepId", "type": "number" },
        "externalId": {
            "$id": "externalId",
            "type": "object",
            "properties": { "name": { "$ref": "#/definitions/foo" }, "age": { "$ref": "deepId" } },
            "definitions": { "foo": { "type": "string" } }
        }
    });
    let schema = json!({
        "$id": "mainSchema",
        "type": "object",
        "properties": { "people": { "oneOf": [{ "$ref": "externalId" }] } }
    });
    let cloned = schema.clone();
    let stringify = build_ok_opts(schema.clone(), ext_opts(external));
    assert_eq!(
        stringify
            .call(&Value::from(
                json!({ "people": { "name": "hello", "foo": "bar", "age": 42 } })
            ))
            .unwrap(),
        "{\"people\":{\"name\":\"hello\",\"age\":42}}"
    );
    assert_eq!(schema, cloned);
}

#[test]
fn items_ref_not_mutated() {
    let external = json!({
        "ShowSchema": { "$id": "ShowSchema", "type": "object", "properties": { "name": { "type": "string" } } }
    });
    let schema = json!({
        "$id": "ListSchema",
        "type": "array",
        "items": { "$ref": "ShowSchema#" }
    });
    let cloned = schema.clone();
    let stringify = build_ok_opts(schema.clone(), ext_opts(external));
    assert_eq!(
        stringify
            .call(&Value::from(json!([{ "name": "foo" }])))
            .unwrap(),
        "[{\"name\":\"foo\"}]"
    );
    assert_eq!(schema, cloned);
}
