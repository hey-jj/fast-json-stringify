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
