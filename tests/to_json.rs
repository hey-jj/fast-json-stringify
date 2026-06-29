//! `toJSON` hooks and null-object coercion.
//!
//! A value carrying a `toJSON` method maps to [`Value::Custom`], which holds the
//! projection the hook would return. The serializer unwraps it before applying
//! the object plan.

mod common;

use common::build_ok;
use fast_json_stringify::{Object, Value};
use serde_json::json;

/// Build an object whose toJSON projection is `inner`.
fn custom(inner: serde_json::Value) -> Value {
    Value::Custom(Box::new(Value::from(inner)))
}

#[test]
fn to_json_on_object() {
    let stringify = build_ok(json!({
        "type": "object",
        "properties": { "productName": { "type": "string" } }
    }));
    let value = custom(json!({ "productName": "cola" }));
    assert_eq!(
        stringify.call(&value).unwrap(),
        "{\"productName\":\"cola\"}"
    );
}

#[test]
fn to_json_on_nested_items() {
    let stringify = build_ok(json!({
        "type": "array",
        "items": { "type": "object", "properties": { "productName": { "type": "string" } } }
    }));
    let value = Value::Array(vec![
        custom(json!({ "productName": "cola" })),
        custom(json!({ "productName": "sprite" })),
    ]);
    assert_eq!(
        stringify.call(&value).unwrap(),
        "[{\"productName\":\"cola\"},{\"productName\":\"sprite\"}]"
    );
}

#[test]
fn no_to_json_when_absent() {
    let stringify = build_ok(json!({
        "type": "object",
        "properties": { "product": { "type": "object", "properties": { "name": { "type": "string" } } } }
    }));
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "product": { "name": "cola" } })))
            .unwrap(),
        "{\"product\":{\"name\":\"cola\"}}"
    );
}

#[test]
fn nullable_object_null() {
    let stringify = build_ok(json!({
        "type": "object",
        "nullable": true,
        "properties": { "product": { "type": "object", "properties": { "name": { "type": "string" } } } }
    }));
    assert_eq!(stringify.call(&Value::Null).unwrap(), "null");
}

#[test]
fn nullable_sub_object_null() {
    let stringify = build_ok(json!({
        "type": "object",
        "properties": {
            "product": { "nullable": true, "type": "object", "properties": { "name": { "type": "string" } } }
        }
    }));
    let mut obj = Object::new();
    obj.insert("product", Value::Null);
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"product\":null}"
    );
}

#[test]
fn non_nullable_null_sub_object_coerces_to_empty() {
    let stringify = build_ok(json!({
        "type": "object",
        "properties": {
            "product": { "nullable": false, "type": "object", "properties": { "name": { "type": "string" } } }
        }
    }));
    let mut obj = Object::new();
    obj.insert("product", Value::Null);
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"product\":{}}"
    );
}

#[test]
fn non_nullable_null_root_coerces_to_empty() {
    let stringify = build_ok(json!({
        "type": "object",
        "nullable": false,
        "properties": {
            "product": { "nullable": false, "type": "object", "properties": { "name": { "type": "string" } } }
        }
    }));
    assert_eq!(stringify.call(&Value::Null).unwrap(), "{}");
}

#[test]
fn non_nullable_null_root_skips_required() {
    let stringify = build_ok(json!({
        "type": "object",
        "nullable": false,
        "properties": {
            "product": { "nullable": false, "type": "object", "properties": { "name": { "type": "string" } } }
        },
        "required": ["product"]
    }));
    assert_eq!(stringify.call(&Value::Null).unwrap(), "{}");
}
