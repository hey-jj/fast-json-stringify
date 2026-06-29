//! String escaping across the length and content regimes.

mod common;

use common::run;
use serde_json::json;

#[test]
fn short_string() {
    assert_eq!(run(json!({ "type": "string" }), json!("abcd")), "\"abcd\"");
}

#[test]
fn control_char_escapes() {
    assert_eq!(
        run(json!({ "type": "string" }), json!("\u{0000}")),
        "\"\\u0000\""
    );
}

#[test]
fn long_string_with_escapes() {
    // 20000 NUL characters, each escaped.
    let input: String = "\u{0000}".repeat(20_000);
    let expected_inner: String = "\\u0000".repeat(20_000);
    let out = run(json!({ "type": "string" }), json!(input));
    assert_eq!(out, format!("\"{expected_inner}\""));
}

#[test]
fn unsafe_string_emits_raw_quotes() {
    let schema = json!({ "type": "string", "format": "unsafe" });
    assert_eq!(run(schema.clone(), json!("abcd")), "\"abcd\"");
    // The unsafe path does no escaping, so embedded quotes break the JSON.
    let out = run(schema, json!("abcd \"abcd\""));
    assert_eq!(out, "\"abcd \"abcd\"\"");
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_err());
}

#[test]
fn unsafe_non_string_coerces_then_wraps() {
    // asUnsafeString is '"' + value + '"'. String concatenation coerces any
    // value through its default toString, with no JSON escaping.
    let schema = json!({ "type": "string", "format": "unsafe" });
    assert_eq!(run(schema.clone(), json!(null)), "\"null\"");
    assert_eq!(run(schema.clone(), json!(5)), "\"5\"");
    assert_eq!(run(schema.clone(), json!(true)), "\"true\"");
    assert_eq!(run(schema.clone(), json!([1, 2])), "\"1,2\"");
    assert_eq!(run(schema, json!({ "a": 1 })), "\"[object Object]\"");
}

#[test]
fn unsafe_string_with_specials_stays_raw() {
    // The point of unsafe is no escaping, so control characters pass through.
    let schema = json!({ "type": "string", "format": "unsafe" });
    assert_eq!(run(schema, json!("a\tb")), "\"a\tb\"");
}

#[test]
fn surrogate_pair_round_trips() {
    // A valid astral character serializes verbatim.
    assert_eq!(run(json!({ "type": "string" }), json!("𝌆")), "\"𝌆\"");
}

#[test]
fn boundary_lengths_match_native() {
    // Exercise the length thresholds with and without an escapable character.
    for len in [41usize, 42, 43, 4999, 5000, 5001] {
        let plain: String = "a".repeat(len);
        assert_eq!(
            run(json!({ "type": "string" }), json!(plain)),
            format!("\"{plain}\"")
        );

        let mut with_quote = plain.clone();
        with_quote.pop();
        with_quote.push('"');
        let out = run(json!({ "type": "string" }), json!(with_quote));
        let expected = with_quote.replace('"', "\\\"");
        assert_eq!(out, format!("\"{expected}\""));
    }
}
