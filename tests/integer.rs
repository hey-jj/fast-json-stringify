//! Integer serialization and rounding.

mod common;

use common::{build_ok_opts, run, run_err};
use fast_json_stringify::{Options, Rounding, Value};
use serde_json::json;

fn rounding_opts(rounding: Rounding) -> Options {
    Options {
        rounding,
        ..Options::new()
    }
}

#[test]
fn render_integer() {
    assert_eq!(run(json!({ "type": "integer" }), json!(1615)), "1615");
}

#[test]
fn throws_on_nan() {
    let stringify = build_ok_opts(json!({ "type": "integer" }), Options::new());
    let err = stringify.call(&Value::Number(f64::NAN)).unwrap_err();
    assert_eq!(
        err.message(),
        "The value \"NaN\" cannot be converted to an integer."
    );
}

#[test]
fn rounding_table() {
    // (input, expected, rounding)
    let cases: &[(f64, &str, Rounding)] = &[
        (std::f64::consts::PI, "3", Rounding::Trunc),
        (5.0, "5", Rounding::Trunc),
        (0.0, "0", Rounding::Trunc),
        (42.0, "42", Rounding::Trunc),
        (1.99999, "1", Rounding::Trunc),
        (-45.05, "-45", Rounding::Trunc),
        (3333333333333333.0, "3333333333333333", Rounding::Trunc),
        (0.95, "1", Rounding::Ceil),
        (0.2, "1", Rounding::Ceil),
        (45.95, "45", Rounding::Floor),
        (-45.05, "-46", Rounding::Floor),
        (45.44, "45", Rounding::Round),
        (45.95, "46", Rounding::Round),
    ];
    for (input, expected, rounding) in cases {
        let stringify = build_ok_opts(json!({ "type": "integer" }), rounding_opts(*rounding));
        let out = stringify.call(&Value::Number(*input)).unwrap();
        assert_eq!(out, *expected, "input {input} with {rounding:?}");
    }
}

#[test]
fn null_rounds_to_zero() {
    for r in [
        Rounding::Trunc,
        Rounding::Ceil,
        Rounding::Floor,
        Rounding::Round,
    ] {
        let stringify = build_ok_opts(json!({ "type": "integer" }), rounding_opts(r));
        assert_eq!(stringify.call(&Value::Null).unwrap(), "0");
    }
}

#[test]
fn object_integer() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "id": { "type": "integer" } } }),
            json!({ "id": 1615 })
        ),
        "{\"id\":1615}"
    );
}

#[test]
fn array_integer() {
    assert_eq!(
        run(
            json!({ "type": "array", "items": { "type": "integer" } }),
            json!([1615])
        ),
        "[1615]"
    );
}

#[test]
fn additional_property_integer() {
    assert_eq!(
        run(
            json!({ "type": "object", "additionalProperties": { "type": "integer" } }),
            json!({ "num": 1615 })
        ),
        "{\"num\":1615}"
    );
}

#[test]
fn round_object_property() {
    let stringify = build_ok_opts(
        json!({ "type": "object", "properties": { "magic": { "type": "integer" } } }),
        rounding_opts(Rounding::Ceil),
    );
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "magic": 4.2 })))
            .unwrap(),
        "{\"magic\":5}"
    );
}

#[test]
fn missing_property_omitted() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "age": { "type": "integer" } } }),
            json!({})
        ),
        "{}"
    );
    for r in [Rounding::Ceil, Rounding::Floor, Rounding::Round] {
        let stringify = build_ok_opts(
            json!({ "type": "object", "properties": { "magic": { "type": "integer" } } }),
            rounding_opts(r),
        );
        assert_eq!(stringify.call(&Value::from(json!({}))).unwrap(), "{}");
    }
}

#[test]
fn non_numeric_throws() {
    let err = run_err(
        json!({ "type": "object", "properties": { "num": { "type": "integer" } }, "required": ["num"] }),
        json!({ "num": "aaa" }),
    );
    assert_eq!(err, "The value \"aaa\" cannot be converted to an integer.");
}
