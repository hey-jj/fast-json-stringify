//! Number coercion and key ordering, closing gaps the source suite under-tests.

mod common;

use common::{build_ok, run};
use serde_json::json;

#[test]
fn string_to_number_through_build() {
    // A string under type: number coerces with JavaScript Number() rules.
    let schema = json!({ "type": "number" });
    let ok: &[(&str, &str)] = &[
        ("123", "123"),
        ("  12  ", "12"),
        ("+.5", "0.5"),
        ("5.0", "5"),
        ("1.5e3", "1500"),
        ("0x1f", "31"),
        ("0o17", "15"),
        ("0b101", "5"),
        ("", "0"),
        ("Infinity", "null"),
        ("-Infinity", "null"),
    ];
    for (input, expected) in ok {
        assert_eq!(
            run(schema.clone(), json!(input)),
            *expected,
            "input {input}"
        );
    }

    let stringify = build_ok(schema);
    for input in ["1e", "0x", "1_2", ".", "12px"] {
        let err = stringify
            .call(&fast_json_stringify::Value::String(input.into()))
            .expect_err("should fail");
        assert_eq!(
            err.message(),
            format!("The value \"{input}\" cannot be converted to a number.")
        );
    }
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
