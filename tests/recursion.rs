//! Recursive schemas.

mod common;

use common::{build_ok_opts, run};
use fast_json_stringify::{Options, Value};
use serde_json::json;

#[test]
fn recursive_directory_tree() {
    let schema = json!({
        "definitions": {
            "directory": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "subDirectories": {
                        "type": "array",
                        "items": { "$ref": "#/definitions/directory" },
                        "default": []
                    }
                }
            }
        },
        "type": "array",
        "items": { "$ref": "#/definitions/directory" }
    });
    let input = json!([
        { "name": "directory 1", "subDirectories": [] },
        {
            "name": "directory 2",
            "subDirectories": [
                { "name": "directory 2.1", "subDirectories": [] },
                { "name": "directory 2.2", "subDirectories": [] }
            ]
        }
    ]);
    assert_eq!(
        run(schema, input),
        "[{\"name\":\"directory 1\",\"subDirectories\":[]},{\"name\":\"directory 2\",\"subDirectories\":[{\"name\":\"directory 2.1\",\"subDirectories\":[]},{\"name\":\"directory 2.2\",\"subDirectories\":[]}]}]"
    );
}

#[test]
fn recursion_in_external_schema() {
    let external = json!({
        "person": {
            "$id": "person",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "children": { "type": "array", "items": { "$ref": "#" } }
            }
        }
    });
    let schema = json!({
        "$id": "mainSchema",
        "type": "object",
        "properties": { "people": { "$ref": "person" } }
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
    let input = json!({ "people": { "name": "Elizabeth", "children": [{ "name": "Charles" }] } });
    assert_eq!(
        stringify.call(&Value::from(input)).unwrap(),
        "{\"people\":{\"name\":\"Elizabeth\",\"children\":[{\"name\":\"Charles\"}]}}"
    );
}

#[test]
fn recursive_object_types() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "parentCategory": {
                "type": "object",
                "properties": { "parent": { "$ref": "#/definitions/parentCategory" } }
            }
        },
        "properties": {
            "category": {
                "type": "object",
                "properties": { "parent": { "$ref": "#/definitions/parentCategory" } }
            }
        }
    });
    let input = json!({ "category": { "parent": { "parent": { "parent": { "parent": {} } } } } });
    assert_eq!(
        run(schema, input),
        "{\"category\":{\"parent\":{\"parent\":{\"parent\":{\"parent\":{}}}}}}"
    );
}

#[test]
fn recursive_inline_id_references() {
    let schema = json!({
        "$id": "Node",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "nodes": { "type": "array", "items": { "$ref": "Node" } }
        },
        "required": ["id", "nodes"]
    });
    let input = json!({
        "id": "0",
        "nodes": [
            { "id": "1", "nodes": [{ "id": "2", "nodes": [{ "id": "3", "nodes": [] }] }] }
        ]
    });
    assert_eq!(
        run(schema, input),
        "{\"id\":\"0\",\"nodes\":[{\"id\":\"1\",\"nodes\":[{\"id\":\"2\",\"nodes\":[{\"id\":\"3\",\"nodes\":[]}]}]}]}"
    );
}

#[test]
fn anyof_direct_self_reference() {
    // The merged anyOf option points back at the root through a content id, so
    // build terminates the recursion. An empty object serializes unchanged.
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "additionalProperties": false, "anyOf": [{ "$ref": "#" }] } }
    });
    assert_eq!(run(schema, json!({ "foo": {} })), "{\"foo\":{}}");
}

#[test]
fn anyof_nested_self_reference() {
    // A self-ref nested one anyOf deep reuses the same merged id, so it also
    // terminates.
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "additionalProperties": false, "anyOf": [{ "anyOf": [{ "$ref": "#" }] }] }
        }
    });
    assert_eq!(run(schema, json!({ "foo": {} })), "{\"foo\":{}}");
}

#[test]
fn anyof_external_recursive_shared_fragment() {
    // Two properties reference the same recursive fragment in an external doc.
    // Each renders the merged shape with the extra surrounding properties.
    let external = json!({ "externalSchema": {
        "type": "object",
        "properties": {
            "foo": { "properties": { "bar": { "type": "string" } }, "anyOf": [{ "$ref": "#" }] }
        }
    }});
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "$ref": "externalSchema#/properties/foo" },
            "b": { "$ref": "externalSchema#/properties/foo" }
        }
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
    let input = json!({
        "a": { "foo": {}, "bar": "42", "baz": 42 },
        "b": { "foo": {}, "bar": "42", "baz": 42 }
    });
    assert_eq!(
        stringify.call(&Value::from(input)).unwrap(),
        "{\"a\":{\"bar\":\"42\",\"foo\":{}},\"b\":{\"bar\":\"42\",\"foo\":{}}}"
    );
}
