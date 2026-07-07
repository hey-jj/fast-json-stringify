mod common;

use common::build_ok_opts;
use fast_json_stringify::{Options, Rounding, Value};
use serde_json::json;

fn rounding_opts(rounding: Rounding) -> Options {
    Options {
        rounding,
        ..Options::new()
    }
}

#[test]
fn round_keeps_sub_half_below_one() {
    let stringify = build_ok_opts(json!({ "type": "integer" }), rounding_opts(Rounding::Round));

    assert_eq!(
        stringify.call(&Value::Number(0.49999999999999994)).unwrap(),
        "0"
    );
}

#[test]
fn single_element_array_bool_number_throws() {
    let stringify = build_ok_opts(json!({ "type": "number" }), Options::new());
    let err = stringify
        .call(&Value::Array(vec![Value::Bool(true)]))
        .unwrap_err();

    assert_eq!(
        err.message(),
        "The value \"true\" cannot be converted to a number."
    );
}

#[test]
fn single_element_array_date_number_throws() {
    let stringify = build_ok_opts(json!({ "type": "number" }), Options::new());
    let err = stringify
        .call(&Value::Array(vec![Value::Date(0)]))
        .unwrap_err();

    assert_eq!(
        err.message(),
        "The value \"1970-01-01T00:00:00.000Z\" cannot be converted to a number."
    );
}
