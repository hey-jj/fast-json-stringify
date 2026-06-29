//! A RegExp value under a string schema serializes its source.

mod common;

use common::build_ok;
use fast_json_stringify::{Object, Value};
use serde_json::json;

#[test]
fn regexp_serializes_source() {
    let schema = json!({ "type": "object", "properties": { "reg": { "type": "string" } } });
    let stringify = build_ok(schema);
    let mut obj = Object::new();
    obj.insert("reg", Value::Regex("\"([^\"]|\\\\\")*\"".into()));
    let out = stringify.call(&Value::Object(obj)).unwrap();
    // The output is valid JSON and round-trips to the source string.
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["reg"], json!("\"([^\"]|\\\\\")*\""));
}
