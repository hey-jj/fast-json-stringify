//! Date and time formatting. Dates compute in UTC, matching the pinned harness.
//!
//! A JavaScript `Date` maps to [`Value::Date`] holding epoch milliseconds. The
//! epoch values here come from the same `new Date(...)` constructions the source
//! tests use.

mod common;

use common::build_ok;
use fast_json_stringify::{Object, Value};
use serde_json::json;

/// 2023-01-21T01:03:25.800Z.
const BASE: i64 = 1674263005800;

#[test]
fn date_as_plain_string_renders_iso() {
    let stringify = build_ok(json!({ "type": "string" }));
    assert_eq!(
        stringify.call(&Value::Date(BASE)).unwrap(),
        "\"2023-01-21T01:03:25.800Z\""
    );
}

#[test]
fn date_time_format() {
    let stringify = build_ok(json!({ "type": "string", "format": "date-time" }));
    assert_eq!(
        stringify.call(&Value::Date(BASE)).unwrap(),
        "\"2023-01-21T01:03:25.800Z\""
    );
}

#[test]
fn nullable_date_time_format() {
    let stringify = build_ok(json!({ "type": "string", "format": "date-time", "nullable": true }));
    assert_eq!(
        stringify.call(&Value::Date(BASE)).unwrap(),
        "\"2023-01-21T01:03:25.800Z\""
    );
}

#[test]
fn date_format() {
    let stringify = build_ok(json!({ "type": "string", "format": "date" }));
    assert_eq!(
        stringify.call(&Value::Date(BASE)).unwrap(),
        "\"2023-01-21\""
    );
}

#[test]
fn date_format_padding() {
    // new Date(2020, 0, 1) in UTC.
    let stringify = build_ok(json!({ "type": "string", "format": "date" }));
    assert_eq!(
        stringify.call(&Value::Date(1577836800000)).unwrap(),
        "\"2020-01-01\""
    );
}

#[test]
fn time_format() {
    let stringify = build_ok(json!({ "type": "string", "format": "time" }));
    assert_eq!(stringify.call(&Value::Date(BASE)).unwrap(), "\"01:03:25\"");
}

#[test]
fn midnight_time() {
    // new Date(BASE).setHours(24) -> next day 00:03:25.
    let stringify = build_ok(json!({ "type": "string", "format": "time" }));
    assert_eq!(
        stringify.call(&Value::Date(1674345805800)).unwrap(),
        "\"00:03:25\""
    );
}

#[test]
fn time_padding() {
    // new Date(2020, 0, 1, 1, 1, 1, 1) in UTC.
    let stringify = build_ok(json!({ "type": "string", "format": "time" }));
    assert_eq!(
        stringify.call(&Value::Date(1577840461001)).unwrap(),
        "\"01:01:01\""
    );
}

#[test]
fn nested_date_time() {
    let stringify = build_ok(json!({
        "type": "object",
        "properties": { "date": { "type": "string", "format": "date-time" } }
    }));
    let mut obj = Object::new();
    obj.insert("date", Value::Date(BASE));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"date\":\"2023-01-21T01:03:25.800Z\"}"
    );
}

#[test]
fn null_renders_empty_string() {
    for format in ["date-time", "date", "time"] {
        let stringify = build_ok(json!({
            "type": "object",
            "properties": { "updatedAt": { "type": "string", "format": format } }
        }));
        let mut obj = Object::new();
        obj.insert("updatedAt", Value::Null);
        assert_eq!(
            stringify.call(&Value::Object(obj)).unwrap(),
            "{\"updatedAt\":\"\"}"
        );
    }
}

#[test]
fn nullable_union_null_renders_null() {
    for format in ["date-time", "date", "time"] {
        let stringify = build_ok(json!({
            "type": "object",
            "properties": { "updatedAt": { "type": ["string", "null"], "format": format } }
        }));
        let mut obj = Object::new();
        obj.insert("updatedAt", Value::Null);
        assert_eq!(
            stringify.call(&Value::Object(obj)).unwrap(),
            "{\"updatedAt\":null}"
        );
    }
}

#[test]
fn bigint_under_int64_format() {
    // int64 is an unknown format, so the value coerces through plain string.
    let stringify = build_ok(json!({
        "type": "object",
        "properties": { "hello": { "type": "string", "format": "int64", "pattern": "^[0-9]*$" } }
    }));
    let mut obj = Object::new();
    obj.insert("hello", Value::BigInt(123));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"hello\":\"123\"}"
    );
}

#[test]
fn invalid_string_passes_through() {
    let stringify = build_ok(json!({ "type": "string", "format": "date-time", "nullable": true }));
    assert_eq!(
        stringify.call(&Value::String("invalid".into())).unwrap(),
        "\"invalid\""
    );
}

#[test]
fn true_input_throws_per_format() {
    for (format, target) in [
        ("date-time", "date-time"),
        ("date", "date"),
        ("time", "time"),
    ] {
        let stringify = build_ok(json!({ "type": "string", "format": format, "nullable": true }));
        let err = stringify.call(&Value::Bool(true)).unwrap_err();
        assert_eq!(
            err.message(),
            format!("The value \"true\" cannot be converted to a {target}.")
        );
    }
}
