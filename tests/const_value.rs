//! `const` always emits the constant.

mod common;

use common::run;
use serde_json::json;

fn const_schema(c: serde_json::Value) -> serde_json::Value {
    json!({ "type": "object", "properties": { "foo": { "const": c } } })
}

#[test]
fn const_string_matches_input() {
    assert_eq!(
        run(const_schema(json!("bar")), json!({ "foo": "bar" })),
        "{\"foo\":\"bar\"}"
    );
}

#[test]
fn const_emits_regardless_of_input() {
    assert_eq!(
        run(const_schema(json!("bar")), json!({ "foo": "baz" })),
        "{\"foo\":\"bar\"}"
    );
    assert_eq!(
        run(const_schema(json!("bar")), json!({ "foo": 1 })),
        "{\"foo\":\"bar\"}"
    );
}

#[test]
fn const_absent_property_omitted() {
    assert_eq!(run(const_schema(json!("bar")), json!({})), "{}");
}

#[test]
fn const_with_single_quote() {
    assert_eq!(
        run(const_schema(json!("'bar'")), json!({ "foo": "'bar'" })),
        "{\"foo\":\"'bar'\"}"
    );
}

#[test]
fn const_number() {
    assert_eq!(
        run(const_schema(json!(1)), json!({ "foo": 1 })),
        "{\"foo\":1}"
    );
    assert_eq!(
        run(const_schema(json!(1)), json!({ "foo": 2 })),
        "{\"foo\":1}"
    );
}

#[test]
fn const_bool() {
    assert_eq!(
        run(const_schema(json!(true)), json!({ "foo": true })),
        "{\"foo\":true}"
    );
}

#[test]
fn const_null() {
    assert_eq!(
        run(const_schema(json!(null)), json!({ "foo": null })),
        "{\"foo\":null}"
    );
}

#[test]
fn const_array() {
    assert_eq!(
        run(const_schema(json!([1, 2, 3])), json!({ "foo": [1, 2, 3] })),
        "{\"foo\":[1,2,3]}"
    );
}

#[test]
fn const_object() {
    assert_eq!(
        run(
            const_schema(json!({ "bar": "baz" })),
            json!({ "foo": { "bar": "baz" } })
        ),
        "{\"foo\":{\"bar\":\"baz\"}}"
    );
}

#[test]
fn const_with_null_type_union() {
    let schema = json!({ "type": "object", "properties": { "foo": { "type": ["string", "null"], "const": "baz" } } });
    assert_eq!(
        run(schema.clone(), json!({ "foo": null })),
        "{\"foo\":null}"
    );
    assert_eq!(run(schema, json!({ "foo": "baz" })), "{\"foo\":\"baz\"}");
}

#[test]
fn const_with_nullable() {
    let schema =
        json!({ "type": "object", "properties": { "foo": { "nullable": true, "const": "baz" } } });
    assert_eq!(
        run(schema.clone(), json!({ "foo": null })),
        "{\"foo\":null}"
    );
    assert_eq!(run(schema, json!({ "foo": "baz" })), "{\"foo\":\"baz\"}");
}

#[test]
fn const_object_ignores_input_shape() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "const": { "foo": "bar" } } },
        "required": ["foo"]
    });
    assert_eq!(
        run(schema, json!({ "foo": { "foo": "baz" } })),
        "{\"foo\":{\"foo\":\"bar\"}}"
    );
}
