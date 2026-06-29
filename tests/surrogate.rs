//! Surrogate-pair string handling.
//!
//! A Rust `String` is always well-formed UTF-8, so the lone and unpaired
//! surrogate cases from the source cannot be represented as input here. Those
//! paths only trigger for invalid UTF-16, which the model excludes by
//! construction. Valid astral characters serialize verbatim.

mod common;

use common::run;
use serde_json::json;

#[test]
fn astral_character_round_trips() {
    // U+1D306 TETRAGRAM FOR CENTRE.
    assert_eq!(run(json!({ "type": "string" }), json!("𝌆")), "\"𝌆\"");
}

#[test]
fn astral_character_in_object() {
    let schema = json!({ "type": "object", "properties": { "glyph": { "type": "string" } } });
    let out = run(schema, json!({ "glyph": "𝌆" }));
    assert_eq!(out, "{\"glyph\":\"𝌆\"}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["glyph"], json!("𝌆"));
}

#[test]
fn mixed_emoji_and_text() {
    let input = "hello 👋 world 🌍";
    let out = run(json!({ "type": "string" }), json!(input));
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed, json!(input));
}
