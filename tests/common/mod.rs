//! Shared test helpers.
//!
//! Most cases compare against a literal expected string or against native
//! `JSON.stringify(input)`. These helpers wrap the build-and-call dance and a
//! small native serializer that mirrors `JSON.stringify` for the inputs the
//! suite uses.

#![allow(dead_code)]

use fast_json_stringify::{build, Options, Stringify, Value};
use serde_json::Value as Json;

/// Build a serializer, expecting success.
pub fn build_ok(schema: Json) -> Stringify {
    build(&schema, None).expect("schema should compile")
}

/// Build a serializer with options, expecting success.
pub fn build_ok_opts(schema: Json, opts: Options) -> Stringify {
    build(&schema, Some(opts)).expect("schema should compile")
}

/// Build and serialize, asserting the output equals `expected`.
pub fn assert_serializes(schema: Json, input: Json, expected: &str) {
    let stringify = build_ok(schema);
    let out = stringify
        .call(&Value::from(input))
        .expect("serialize should succeed");
    assert_eq!(out, expected);
}

/// Build and serialize a value, asserting the output equals `expected`.
pub fn assert_serializes_value(schema: Json, input: Value, expected: &str) {
    let stringify = build_ok(schema);
    let out = stringify.call(&input).expect("serialize should succeed");
    assert_eq!(out, expected);
}

/// Serialize and return the result string.
pub fn run(schema: Json, input: Json) -> String {
    let stringify = build_ok(schema);
    stringify
        .call(&Value::from(input))
        .expect("serialize should succeed")
}

/// Serialize a value, returning the error message on failure.
pub fn run_err(schema: Json, input: Json) -> String {
    let stringify = build_ok(schema);
    stringify
        .call(&Value::from(input))
        .expect_err("serialize should fail")
        .message()
        .to_string()
}

/// Build, expecting a build error, and return the message.
pub fn build_err(schema: Json) -> String {
    build(&schema, None)
        .expect_err("build should fail")
        .message()
        .to_string()
}

/// Native JSON serialization matching `JSON.stringify`, for round-trip oracles.
pub fn js_stringify(value: &Json) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Json, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else if let Some(f) = n.as_f64() {
                out.push_str(&js_number(f));
            } else {
                out.push_str(&n.to_string());
            }
        }
        Json::String(s) => write_string(s, out),
        Json::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(v, out);
            }
            out.push(']');
        }
        Json::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write(v, out);
            }
            out.push('}');
        }
    }
}

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

/// ECMAScript number-to-string, duplicated here so tests stay independent of
/// crate internals.
fn js_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let negative = value.is_sign_negative();
    let magnitude = value.abs();
    let mut buf = ryu::Buffer::new();
    let s = buf.format_finite(magnitude);

    let (mantissa, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
        None => (s, 0),
    };
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    let mut all = String::new();
    all.push_str(int_part);
    all.push_str(frac_part);
    let mut n = int_part.len() as i32 + exp;
    let leading = all.len() - all.trim_start_matches('0').len();
    if leading > 0 {
        all.drain(..leading);
        n -= leading as i32;
    }
    let trailing = all.len() - all.trim_end_matches('0').len();
    if trailing > 0 {
        all.truncate(all.len() - trailing);
    }
    let k = all.len() as i32;
    let body = if k <= n && n <= 21 {
        let mut o = all.clone();
        for _ in 0..(n - k) {
            o.push('0');
        }
        o
    } else if 0 < n && n <= 21 {
        let (a, b) = all.split_at(n as usize);
        format!("{a}.{b}")
    } else if -6 < n && n <= 0 {
        let mut o = String::from("0.");
        for _ in 0..(-n) {
            o.push('0');
        }
        o.push_str(&all);
        o
    } else {
        let e = n - 1;
        let sign = if e >= 0 { '+' } else { '-' };
        if k == 1 {
            format!("{all}e{sign}{}", e.abs())
        } else {
            let (a, b) = all.split_at(1);
            format!("{a}.{b}e{sign}{}", e.abs())
        }
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}
