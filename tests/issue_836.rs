//! An external `$ref` schema referenced many times serializes correctly.

mod common;

use common::{build_ok, build_ok_opts, js_stringify};
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
fn external_ref_reused_in_properties() {
    let external = json!({
        "contact.json": {
            "$id": "contact.json",
            "type": "object",
            "properties": {
                "firstName": { "type": "string" },
                "lastName": { "type": "string" },
                "email": { "type": "string" }
            }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "owner": { "$ref": "contact.json" },
            "assignee": { "$ref": "contact.json" },
            "reporter": { "$ref": "contact.json" }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let data = json!({
        "owner": { "firstName": "John", "lastName": "Doe", "email": "john@example.com" },
        "assignee": { "firstName": "Jane", "lastName": "Smith", "email": "jane@example.com" },
        "reporter": { "firstName": "Bob", "lastName": "Jones", "email": "bob@example.com" }
    });
    assert_eq!(
        stringify.call(&Value::from(data.clone())).unwrap(),
        js_stringify(&data)
    );
}

#[test]
fn external_ref_reused_with_anyof() {
    let external = json!({
        "contact.json": {
            "$id": "contact.json",
            "type": "object",
            "properties": { "firstName": { "type": "string" }, "lastName": { "type": "string" }, "email": { "type": "string" } }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "owner": { "anyOf": [{ "type": ["string", "null"] }, { "$ref": "contact.json" }] },
            "assignee": { "anyOf": [{ "type": ["string", "null"] }, { "$ref": "contact.json" }] }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let data = json!({
        "owner": { "firstName": "John", "lastName": "Doe", "email": "john@example.com" },
        "assignee": { "firstName": "Jane", "lastName": "Smith", "email": "jane@example.com" }
    });
    assert_eq!(
        stringify.call(&Value::from(data.clone())).unwrap(),
        js_stringify(&data)
    );
}

#[test]
fn external_ref_reused_in_array_items() {
    let external = json!({
        "contact.json": {
            "$id": "contact.json",
            "type": "object",
            "properties": { "firstName": { "type": "string" }, "lastName": { "type": "string" } }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "contacts": { "type": "array", "items": { "$ref": "contact.json" } },
            "primary": { "$ref": "contact.json" }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let data = json!({
        "contacts": [{ "firstName": "Alice", "lastName": "Wonder" }, { "firstName": "Bob", "lastName": "Builder" }],
        "primary": { "firstName": "Charlie", "lastName": "Charm" }
    });
    assert_eq!(
        stringify.call(&Value::from(data.clone())).unwrap(),
        js_stringify(&data)
    );
}

#[test]
fn external_array_ref_reused() {
    let external = json!({ "tags.json": { "$id": "tags.json", "type": "array", "items": { "type": "string" } } });
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "$ref": "tags.json" },
            "b": { "$ref": "tags.json" },
            "c": { "$ref": "tags.json" }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let data = json!({ "a": ["x", "y"], "b": ["z"], "c": [] });
    assert_eq!(
        stringify.call(&Value::from(data.clone())).unwrap(),
        js_stringify(&data)
    );
}

#[test]
fn inline_anonymous_schemas_still_work() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "object", "properties": { "x": { "type": "string" } } },
            "b": { "type": "object", "properties": { "x": { "type": "string" } } }
        }
    });
    let stringify = build_ok(schema);
    assert_eq!(
        stringify
            .call(&Value::from(
                json!({ "a": { "x": "hello" }, "b": { "x": "world" } })
            ))
            .unwrap(),
        "{\"a\":{\"x\":\"hello\"},\"b\":{\"x\":\"world\"}}"
    );
}
