//! BigInt serialization. BigInt maps to [`Value::BigInt`].

mod common;

use common::build_ok;
use fast_json_stringify::{Object, Value};
use serde_json::json;

#[test]
fn render_bigint() {
    let stringify = build_ok(json!({ "type": "integer" }));
    assert_eq!(stringify.call(&Value::BigInt(1615)).unwrap(), "1615");
}

#[test]
fn object_bigint() {
    let stringify =
        build_ok(json!({ "type": "object", "properties": { "id": { "type": "integer" } } }));
    let mut obj = Object::new();
    obj.insert("id", Value::BigInt(1615));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"id\":1615}"
    );
}

#[test]
fn array_bigint() {
    let stringify = build_ok(json!({ "type": "array", "items": { "type": "integer" } }));
    assert_eq!(
        stringify
            .call(&Value::Array(vec![Value::BigInt(1615)]))
            .unwrap(),
        "[1615]"
    );
}

#[test]
fn additional_property_bigint() {
    let stringify =
        build_ok(json!({ "type": "object", "additionalProperties": { "type": "integer" } }));
    let mut obj = Object::new();
    obj.insert("num", Value::BigInt(1615));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"num\":1615}"
    );
}

#[test]
fn large_bigint_keeps_precision() {
    let stringify = build_ok(json!({ "type": "integer" }));
    // Beyond Number.MAX_SAFE_INTEGER, exact digits must survive.
    assert_eq!(
        stringify.call(&Value::BigInt(9007199254740993)).unwrap(),
        "9007199254740993"
    );
}
