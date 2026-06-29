//! Quotes and backslashes in string values round-trip through JSON.

mod common;

use common::run;
use serde_json::json;

#[test]
fn quotes_in_object_property() {
    let schema = json!({ "type": "object", "properties": { "message": { "type": "string" } } });
    let message = "Error: Property \"name\" is required";
    let out = run(schema, json!({ "message": message }));
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["message"], json!(message));
}

#[test]
fn various_quote_types() {
    let schema = json!({ "type": "string" });
    for input in [
        "Property \"name\" is required",
        "Property 'name' is required",
        "Error: \"Property 'name' is required\"",
    ] {
        let out = run(schema.clone(), json!(input));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, json!(input));
    }
}

#[test]
fn backslashes_round_trip() {
    let schema = json!({ "type": "string" });
    for input in ["a\\b", "C:\\path\\to\\file", "regex: \\d+\\w*"] {
        let out = run(schema.clone(), json!(input));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, json!(input));
    }
}
