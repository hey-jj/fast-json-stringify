//! An unknown string format serializes the value as a plain string.

mod common;

use common::run;
use serde_json::json;

#[test]
fn custom_format_serializes_plain() {
    let schema = json!({
        "type": "object",
        "properties": { "str": { "type": "string", "format": "test-format" } }
    });
    assert_eq!(
        run(schema, json!({ "str": "string" })),
        "{\"str\":\"string\"}"
    );
}
