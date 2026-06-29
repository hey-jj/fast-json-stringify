//! Invalid schemas are rejected at build with a descriptive message.

mod common;

use fast_json_stringify::{build, Options};
use serde_json::json;

#[test]
fn invalid_external_schema() {
    let mut opts = Options::new();
    opts.schema
        .insert("invalid".to_string(), json!({ "type": "Dinosaur" }));
    let err = build(&json!({}), Some(opts)).unwrap_err();
    assert!(
        err.message().starts_with("\"invalid\" schema is invalid:"),
        "{}",
        err.message()
    );
}

#[test]
fn invalid_not_schema() {
    let schema = json!({
        "type": "object",
        "properties": { "prop": { "not": "not object" } }
    });
    let err = build(&schema, None).unwrap_err();
    assert!(err.message().contains("schema is invalid"));
}

#[test]
fn meta_schema_messages_are_pinned() {
    // Pin the exact data<path> <message> text for each structural rule the
    // meta-schema check models, so the covered surface cannot drift silently.
    let cases: &[(serde_json::Value, &str)] = &[
        (
            json!({ "type": "Dinosaur" }),
            "schema is invalid: data/type must be equal to one of the allowed values",
        ),
        (
            json!({ "type": ["string", "Dinosaur"] }),
            "schema is invalid: data/type/1 must be equal to one of the allowed values",
        ),
        (
            json!({ "anyOf": { "type": "string" } }),
            "schema is invalid: data/anyOf must be array",
        ),
        (
            json!({ "anyOf": [] }),
            "schema is invalid: data/anyOf must NOT have fewer than 1 items",
        ),
        (
            json!({ "properties": [1, 2] }),
            "schema is invalid: data/properties must be object",
        ),
        (
            json!({ "patternProperties": { "[": { "type": "string" } } }),
            "schema is invalid: data/patternProperties must match format \"regex\"",
        ),
    ];
    for (schema, expected) in cases {
        let err = build(schema, None).unwrap_err();
        assert_eq!(err.message(), *expected, "schema {schema}");
    }
}
