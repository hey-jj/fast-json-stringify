//! Number coercion and key ordering, closing gaps the source suite under-tests.

mod common;

use common::run;
use fast_json_stringify::{Rounding, Serializer, Value};
use serde_json::json;

#[test]
fn as_number_coercion_table() {
    let serializer = Serializer::new(Rounding::Trunc);
    // (input, expected) matching String(Number(input)) in JavaScript.
    let cases: &[(Value, &str)] = &[
        (Value::String(String::new()), "0"),
        (Value::String("  ".into()), "0"),
        (Value::String("1e3".into()), "1000"),
        (Value::String("0x10".into()), "16"),
        (Value::Null, "0"),
        (Value::Array(vec![]), "0"),
        (Value::Array(vec![Value::Number(5.0)]), "5"),
        (Value::Bool(true), "1"),
        (Value::Number(-0.0), "0"),
        (Value::Number(f64::MAX), "1.7976931348623157e+308"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            serializer.as_number(input).unwrap(),
            *expected,
            "input {input:?}"
        );
    }
}

#[test]
fn as_number_nan_string_throws() {
    let serializer = Serializer::new(Rounding::Trunc);
    assert!(serializer
        .as_number(&Value::String("not a number".into()))
        .is_err());
}

#[test]
fn as_integer_of_negative_zero() {
    let serializer = Serializer::new(Rounding::Trunc);
    assert_eq!(serializer.as_integer(&Value::Number(-0.0)).unwrap(), "0");
}

#[test]
fn required_keys_serialize_first() {
    // Declared out of order, required keys must lead in the output.
    let schema = json!({
        "type": "object",
        "properties": {
            "optionalA": { "type": "string" },
            "requiredB": { "type": "string" },
            "optionalC": { "type": "string" },
            "requiredD": { "type": "string" }
        },
        "required": ["requiredB", "requiredD"]
    });
    let input = json!({ "optionalA": "a", "requiredB": "b", "optionalC": "c", "requiredD": "d" });
    assert_eq!(
        run(schema, input),
        "{\"requiredB\":\"b\",\"requiredD\":\"d\",\"optionalA\":\"a\",\"optionalC\":\"c\"}"
    );
}

#[test]
fn properties_then_pattern_then_additional_order() {
    // Fixed properties first, then pattern and additional in input order, last.
    let schema = json!({
        "type": "object",
        "properties": { "known": { "type": "string" } },
        "patternProperties": { "^p_": { "type": "string" } },
        "additionalProperties": { "type": "string" }
    });
    let input = json!({ "extra": "x", "known": "k", "p_match": "p" });
    assert_eq!(
        run(schema, input),
        "{\"known\":\"k\",\"extra\":\"x\",\"p_match\":\"p\"}"
    );
}
