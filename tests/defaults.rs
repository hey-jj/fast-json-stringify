//! Default values fill missing properties.

mod common;

use common::{js_stringify, run};
use serde_json::json;

/// Build with the schema, serialize the input, and compare to `JSON.stringify(expected)`.
fn check(schema: serde_json::Value, input: serde_json::Value, expected: serde_json::Value) {
    assert_eq!(run(schema, input), js_stringify(&expected));
}

fn base_props() -> serde_json::Value {
    json!({
        "firstName": { "type": "string" },
        "lastName": { "type": "string" },
        "age": { "type": "integer", "minimum": 0 },
        "magic": { "type": "number" }
    })
}

#[test]
fn default_string_fills_when_missing() {
    let mut props = base_props();
    props["lastName"] = json!({ "type": "string", "default": "Collina" });
    check(
        json!({ "type": "object", "properties": props, "required": ["firstName", "lastName"] }),
        json!({ "firstName": "Matteo", "magic": 42, "age": 32 }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32, "magic": 42 }),
    );
}

#[test]
fn default_string_not_overridden() {
    let mut props = base_props();
    props["lastName"] = json!({ "type": "string", "default": "Collina" });
    check(
        json!({ "type": "object", "properties": props, "required": ["firstName", "lastName"] }),
        json!({ "firstName": "Matteo", "lastName": "collina", "magic": 42, "age": 32 }),
        json!({ "firstName": "Matteo", "lastName": "collina", "age": 32, "magic": 42 }),
    );
}

#[test]
fn default_number() {
    let mut props = base_props();
    props["magic"] = json!({ "type": "number", "default": 42 });
    check(
        json!({ "type": "object", "properties": props, "required": ["firstName", "lastName"] }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32 }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32, "magic": 42 }),
    );
}

#[test]
fn default_object() {
    let props = json!({
        "firstName": { "type": "string" },
        "lastName": { "type": "string" },
        "age": { "type": "integer", "minimum": 0 },
        "otherProps": { "type": "object", "default": { "foo": "bar" } }
    });
    check(
        json!({ "type": "object", "properties": props, "required": ["firstName", "lastName"] }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32 }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32, "otherProps": { "foo": "bar" } }),
    );
}

#[test]
fn default_array() {
    let props = json!({
        "firstName": { "type": "string" },
        "lastName": { "type": "string" },
        "age": { "type": "integer", "minimum": 0 },
        "otherProps": { "type": "array", "items": { "type": "string" }, "default": ["FOO"] }
    });
    check(
        json!({ "type": "object", "properties": props, "required": ["firstName", "lastName"] }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32 }),
        json!({ "firstName": "Matteo", "lastName": "Collina", "age": 32, "otherProps": ["FOO"] }),
    );
}

#[test]
fn default_deeper_value() {
    let schema = json!({
        "type": "object",
        "properties": {
            "level1": {
                "type": "object",
                "properties": {
                    "level2": {
                        "type": "object",
                        "properties": {
                            "level3": {
                                "type": "object",
                                "properties": {
                                    "level4": { "type": "object", "default": { "foo": "bar" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    check(
        schema.clone(),
        json!({ "level1": { "level2": { "level3": {} } } }),
        json!({ "level1": { "level2": { "level3": { "level4": { "foo": "bar" } } } } }),
    );
    // A present empty object is not replaced.
    check(
        schema,
        json!({ "level1": { "level2": { "level3": { "level4": {} } } } }),
        json!({ "level1": { "level2": { "level3": { "level4": {} } } } }),
    );
}

#[test]
fn default_boolean_false() {
    check(
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "default": "foo" },
                "dev": { "type": "boolean", "default": false }
            },
            "required": ["name", "dev"]
        }),
        json!({}),
        json!({ "name": "foo", "dev": false }),
    );
}

#[test]
fn default_with_optional_present() {
    check(
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "default": "foo" },
                "dev": { "type": "boolean" },
                "job": { "type": "string", "default": "awesome" }
            },
            "required": ["name", "dev"]
        }),
        json!({ "dev": true }),
        json!({ "name": "foo", "dev": true, "job": "awesome" }),
    );
}
