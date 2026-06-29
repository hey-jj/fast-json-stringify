//! Native JSON serialization that matches `JSON.stringify`.
//!
//! Used where the source falls back to `JSON.stringify`: boolean schemas,
//! `additionalProperties: true`, tuple overflow items, pre-rendered defaults,
//! and `const` values. The number formatting and string escaping follow the
//! ECMAScript output so bytes match. Keys keep insertion order.

use crate::number::format_f64;
use crate::value::Value;
use serde_json::Value as Json;

/// Serialize a [`serde_json::Value`] the way `JSON.stringify` would.
pub fn stringify(value: &Json) -> String {
    let mut out = String::new();
    write_json(value, &mut out);
    out
}

/// Serialize a runtime [`Value`] the way `JSON.stringify` would, including the
/// JavaScript host objects.
pub fn stringify_value(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

/// Write a `serde_json::Value`.
fn write_json(value: &Json, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => write_number(n, out),
        Json::String(s) => write_string(s, out),
        Json::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(v, out);
            }
            out.push(']');
        }
        Json::Object(map) => {
            out.push('{');
            let mut first = true;
            for (k, v) in map {
                // JSON.stringify drops undefined, but serde has no undefined.
                if !first {
                    out.push(',');
                }
                first = false;
                write_string(k, out);
                out.push(':');
                write_json(v, out);
            }
            out.push('}');
        }
    }
}

/// Write a runtime `Value`, mapping host objects to their JSON projection.
fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.is_finite() {
                out.push_str(&format_f64(*n));
            } else {
                // JSON.stringify renders Infinity and NaN as null.
                out.push_str("null");
            }
        }
        Value::BigInt(_) => {
            // JSON.stringify throws on BigInt. The serializer never routes a
            // BigInt here, so emit its decimal form defensively.
            if let Value::BigInt(i) = value {
                out.push_str(&i.to_string());
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(v, out);
            }
            out.push(']');
        }
        Value::Object(obj) => {
            out.push('{');
            let mut first = true;
            for (k, v) in obj.iter() {
                if skip_in_json(v) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                write_string(k, out);
                out.push(':');
                write_value(v, out);
            }
            out.push('}');
        }
        Value::Date(ms) => write_string(&crate::value::iso_from_millis(*ms), out),
        Value::Regex(_) => out.push_str("{}"),
        Value::Custom(inner) => write_value(inner, out),
    }
}

/// JSON.stringify drops object members that are undefined, functions, or
/// symbols. The model has no such kinds, so nothing is skipped.
fn skip_in_json(_value: &Value) -> bool {
    false
}

/// Write a number value, preferring an exact integer string when serde holds
/// arbitrary precision.
fn write_number(n: &serde_json::Number, out: &mut String) {
    if let Some(i) = n.as_i64() {
        out.push_str(&i.to_string());
    } else if let Some(u) = n.as_u64() {
        out.push_str(&u.to_string());
    } else if let Some(f) = n.as_f64() {
        out.push_str(&format_f64(f));
    } else {
        // arbitrary precision integer beyond u64.
        out.push_str(&n.to_string());
    }
}

/// Write a JSON-escaped string, matching `JSON.stringify` escaping.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{0009}' => out.push_str("\\t"),
            '\u{000a}' => out.push_str("\\n"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{000d}' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
