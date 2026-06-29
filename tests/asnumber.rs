//! Direct serializer unit tests.

use fast_json_stringify::{Serializer, Value};

#[test]
fn as_number_converts_bigint() {
    let serializer = Serializer::new(fast_json_stringify::Rounding::Trunc);
    let out = serializer.as_number(&Value::BigInt(11753021440)).unwrap();
    assert_eq!(out, "11753021440");
}

#[test]
fn as_integer_of_bigint() {
    let serializer = Serializer::new(fast_json_stringify::Rounding::Trunc);
    assert_eq!(serializer.as_integer(&Value::BigInt(1615)).unwrap(), "1615");
}

#[test]
fn as_boolean_truthiness() {
    let serializer = Serializer::new(fast_json_stringify::Rounding::Trunc);
    assert_eq!(serializer.as_boolean(&Value::Number(1.0)), "true");
    assert_eq!(serializer.as_boolean(&Value::Number(0.0)), "false");
    assert_eq!(
        serializer.as_boolean(&Value::String(String::new())),
        "false"
    );
    assert_eq!(serializer.as_boolean(&Value::String("x".into())), "true");
    assert_eq!(serializer.as_boolean(&Value::Null), "false");
}
