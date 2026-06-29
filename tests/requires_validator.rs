//! A nested `$ref` whose target uses anyOf pulls in the validator.

mod common;

use common::build_ok_opts;
use fast_json_stringify::{Options, Value};
use serde_json::json;

#[test]
fn nested_ref_requires_validator() {
    let external = json!({
        "urn:schema:a": {
            "$id": "urn:schema:a",
            "definitions": { "foo": { "anyOf": [{ "type": "string" }, { "type": "null" }] } }
        }
    });
    let schema = json!({
        "$id": "urn:schema:b",
        "type": "object",
        "properties": {
            "results": {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "object",
                        "properties": {
                            "bar": { "type": "array", "items": { "$ref": "urn:schema:a#/definitions/foo" } }
                        }
                    }
                }
            }
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
    let data = json!({ "results": { "items": { "bar": ["baz"] } } });
    assert_eq!(
        stringify.call(&Value::from(data)).unwrap(),
        "{\"results\":{\"items\":{\"bar\":[\"baz\"]}}}"
    );
}
