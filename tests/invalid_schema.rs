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
