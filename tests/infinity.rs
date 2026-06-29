//! Infinity and finite-number parity.

mod common;

use common::{build_ok, js_stringify};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn finite_numbers_match_native() {
    let values = [
        -5.0,
        0.0,
        -0.0,
        1.33,
        99.0,
        100.0,
        std::f64::consts::E,
        f64::EPSILON,
        9007199254740991.0, // MAX_SAFE_INTEGER
        f64::MAX,
        -9007199254740991.0, // MIN_SAFE_INTEGER
        5e-324,              // MIN_VALUE
    ];
    let stringify = build_ok(json!({ "type": "number" }));
    for v in values {
        let out = stringify.call(&Value::Number(v)).unwrap();
        let expected = js_stringify(
            &serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .unwrap_or(json!(null)),
        );
        // -0.0 has no f64 Number in serde, so compare against the known JS form.
        let expected = if v == 0.0 { "0".to_string() } else { expected };
        assert_eq!(out, expected, "value {v}");
    }
}

#[test]
fn infinite_integers_throw() {
    let stringify = build_ok(json!({ "type": "integer" }));
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        let err = stringify.call(&Value::Number(v)).unwrap_err();
        let label = if v > 0.0 { "Infinity" } else { "-Infinity" };
        assert_eq!(
            err.message(),
            format!("The value \"{label}\" cannot be converted to an integer.")
        );
    }
}

#[test]
fn infinite_numbers_render_null() {
    let stringify = build_ok(json!({ "type": "number" }));
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(stringify.call(&Value::Number(v)).unwrap(), "null");
    }
}

#[test]
fn overflow_magnitude_input_renders_null() {
    // A JSON number whose magnitude overflows f64 parses to infinity, the same
    // as JSON.parse. Under type: number that infinity renders as null.
    let stringify = build_ok(json!({ "type": "number" }));
    for raw in ["1e400", "1e309"] {
        let input = Value::from(serde_json::from_str::<serde_json::Value>(raw).unwrap());
        assert_eq!(stringify.call(&input).unwrap(), "null", "input {raw}");
    }
    let neg = Value::from(serde_json::from_str::<serde_json::Value>("-1e400").unwrap());
    assert_eq!(stringify.call(&neg).unwrap(), "null");
}
