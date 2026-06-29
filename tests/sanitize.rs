//! Malicious property keys, defaults, and patterns produce valid JSON.
//!
//! The serializer interprets a plan rather than generating code, so a crafted
//! key cannot inject anything. These cases lock the byte output and confirm the
//! result stays parseable.

mod common;

use common::{build_ok, run};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn malicious_keys_and_defaults_stay_valid() {
    let schema = json!({
        "type": "object",
        "properties": {
            "firstName": { "type": "string" },
            "age": { "type": "integer" },
            "phra'&& process.exit(1) ||'phra": {},
            "now": { "type": "string" },
            "reg": { "type": "string", "default": "a'&& process.exit(1) ||'" },
            "\"'w00t": { "type": "string", "default": "\"'w00t" }
        },
        "required": ["now"],
        "additionalProperties": { "type": "string" }
    });
    let input = json!({
        "firstName": "Matteo",
        "age": 32,
        "now": "2020-01-01"
    });
    let out = run(schema, input);
    // The output parses as valid JSON and carries the escaped defaults.
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["reg"], json!("a'&& process.exit(1) ||'"));
    assert_eq!(parsed["\"'w00t"], json!("\"'w00t"));
}

#[test]
fn malicious_property_key_escapes() {
    let schema = json!({
        "type": "object",
        "properties": { "\"phra\\'&&(console.log(42))//||'phra": {} }
    });
    let stringify = build_ok(schema);
    let mut obj = fast_json_stringify::Object::new();
    obj.insert("\"phra\\'&&(console.log(42))//||'phra", Value::Number(42.0));
    let out = stringify.call(&Value::Object(obj)).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
}

#[test]
fn malicious_pattern_keys() {
    // A pattern key with quotes and slashes must compile and match safely.
    let schema = json!({
        "type": "object",
        "patternProperties": { "\"'w00t.*////": { "type": "number" } }
    });
    let out = run(schema, json!({ "\"'phra////": 42, "asd": 42 }));
    // Neither key matches the pattern, so nothing serializes.
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed, json!({}));
}
