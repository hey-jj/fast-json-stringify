//! Missing optional properties and null coercion per type.

mod common;

use common::run;
use serde_json::json;

#[test]
fn missing_optional_omitted() {
    let schema = json!({
        "type": "object",
        "properties": {
            "str": { "type": "string" },
            "num": { "type": "number" },
            "val": { "type": "string" }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "val": "value" })),
        "{\"val\":\"value\"}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "str": "string", "val": "value" })),
        "{\"str\":\"string\",\"val\":\"value\"}"
    );
    assert_eq!(
        run(
            schema,
            json!({ "str": "string", "num": 42, "val": "value" })
        ),
        "{\"str\":\"string\",\"num\":42,\"val\":\"value\"}"
    );
}

#[test]
fn null_string_becomes_empty() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "str": { "type": "string" } } }),
            json!({ "str": null })
        ),
        "{\"str\":\"\"}"
    );
}

#[test]
fn null_integer_becomes_zero() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "int": { "type": "integer" } } }),
            json!({ "int": null })
        ),
        "{\"int\":0}"
    );
}

#[test]
fn null_number_becomes_zero() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "num": { "type": "number" } } }),
            json!({ "num": null })
        ),
        "{\"num\":0}"
    );
}

#[test]
fn null_boolean_becomes_false() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "bool": { "type": "boolean" } } }),
            json!({ "bool": null })
        ),
        "{\"bool\":false}"
    );
}
