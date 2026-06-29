//! Building the same `$id` schema twice does not error.

mod common;

use common::build_ok;
use serde_json::json;

#[test]
fn build_same_id_twice() {
    let schema = json!({ "$id": "test", "type": "string" });
    let _ = build_ok(schema.clone());
    let _ = build_ok(schema);
}

#[test]
fn build_same_id_with_external_refs_twice() {
    let schema = json!({
        "$id": "test",
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } },
        "type": "object",
        "properties": { "obj": { "$ref": "#/definitions/def" } }
    });
    let _ = build_ok(schema.clone());
    let _ = build_ok(schema);
}
