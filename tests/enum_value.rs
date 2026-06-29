//! `enum` serializes the matching value, with or without a declared type.

mod common;

use common::run;
use serde_json::json;

#[test]
fn enum_with_type() {
    let schema = json!({
        "type": "object",
        "properties": { "order": { "type": "string", "enum": ["asc", "desc"] } }
    });
    assert_eq!(
        run(schema, json!({ "order": "asc" })),
        "{\"order\":\"asc\"}"
    );
}

#[test]
fn enum_without_type() {
    let schema = json!({
        "type": "object",
        "properties": { "order": { "enum": ["asc", "desc"] } }
    });
    assert_eq!(
        run(schema, json!({ "order": "asc" })),
        "{\"order\":\"asc\"}"
    );
}
