//! Nested objects sharing property names.

mod common;

use common::{js_stringify, run};
use serde_json::json;

#[test]
fn nested_objects_same_properties() {
    let schema = json!({
        "type": "object",
        "properties": {
            "stringProperty": { "type": "string" },
            "objectProperty": { "type": "object", "additionalProperties": true }
        }
    });
    let input = json!({
        "stringProperty": "string1",
        "objectProperty": { "stringProperty": "string2", "numberProperty": 42 }
    });
    assert_eq!(
        run(schema, input),
        "{\"stringProperty\":\"string1\",\"objectProperty\":{\"stringProperty\":\"string2\",\"numberProperty\":42}}"
    );
}

#[test]
fn name_collision() {
    let schema = json!({
        "type": "object",
        "properties": {
            "test": { "type": "object", "properties": { "a": { "type": "string" } } },
            "tes": {
                "type": "object",
                "properties": { "b": { "type": "string" }, "t": { "type": "object" } }
            }
        }
    });
    let input = json!({ "test": { "a": "a" }, "tes": { "b": "b", "t": {} } });
    assert_eq!(run(schema, input.clone()), js_stringify(&input));
}
