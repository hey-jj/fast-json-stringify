//! Comma placement when declared properties mix with extra properties.

mod common;

use common::run;
use serde_json::json;

#[test]
fn additional_false() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "additionalProperties": false
    });
    assert_eq!(
        run(schema, json!({ "foo": "a", "bar": "b", "baz": "c" })),
        "{\"foo\":\"a\"}"
    );
}

#[test]
fn additional_empty_schema() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "additionalProperties": {}
    });
    assert_eq!(
        run(schema, json!({ "foo": "a", "bar": "b", "baz": "c" })),
        "{\"foo\":\"a\",\"bar\":\"b\",\"baz\":\"c\"}"
    );
}

#[test]
fn additional_string_schema() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "string" } },
        "additionalProperties": { "type": "string" }
    });
    assert_eq!(
        run(schema, json!({ "foo": "a", "bar": "b", "baz": "c" })),
        "{\"foo\":\"a\",\"bar\":\"b\",\"baz\":\"c\"}"
    );
}
