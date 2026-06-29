//! Newlines in string values round-trip through JSON.

mod common;

use common::run;
use serde_json::json;

#[test]
fn newlines_in_object_property() {
    let schema = json!({ "type": "object", "properties": { "message": { "type": "string" } } });
    let message = "This is a string\nwith multiple\nnewlines in it\nFoo";
    let out = run(schema, json!({ "message": message }));
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["message"], json!(message));
}

#[test]
fn various_newline_characters() {
    let schema = json!({ "type": "string" });
    for input in [
        "line1\nline2",
        "line1\rline2",
        "line1\r\nline2",
        "line1\nline2\rline3\r\nline4",
    ] {
        let out = run(schema.clone(), json!(input));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, json!(input));
    }
}
