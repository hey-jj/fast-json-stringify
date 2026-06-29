//! Primitive value serializers.
//!
//! Each method maps one JavaScript value to the exact JSON bytes the compiled
//! function would append. The escaping, number formatting, and date math follow
//! the source behavior closely so output is byte identical.

use crate::error::StringifyError;
use crate::number::format_f64;
use crate::value::{date_slice_from_millis, iso_from_millis, time_slice_from_millis, Value};

/// Integer rounding strategy for non-integer numbers under `type: "integer"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    /// Round toward zero. The default.
    #[default]
    Trunc,
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round to the nearest integer, ties toward positive infinity.
    Round,
}

impl Rounding {
    /// Parse an option string. Returns `None` for an unknown method so the
    /// caller can raise the documented build error.
    pub fn parse(name: &str) -> Option<Rounding> {
        match name {
            "trunc" => Some(Rounding::Trunc),
            "floor" => Some(Rounding::Floor),
            "ceil" => Some(Rounding::Ceil),
            "round" => Some(Rounding::Round),
            _ => None,
        }
    }

    /// Apply the rounding to a finite or non-finite double.
    fn apply(self, x: f64) -> f64 {
        match self {
            Rounding::Trunc => x.trunc(),
            Rounding::Floor => x.floor(),
            Rounding::Ceil => x.ceil(),
            // JavaScript Math.round rounds half toward positive infinity, which
            // differs from Rust's round-half-away-from-zero for negatives.
            Rounding::Round => (x + 0.5).floor(),
        }
    }
}

/// Holds the configured rounding mode and produces JSON fragments for each
/// primitive type.
#[derive(Debug, Clone, Default)]
pub struct Serializer {
    rounding: Rounding,
}

impl Serializer {
    /// Build a serializer with the given rounding mode.
    pub fn new(rounding: Rounding) -> Self {
        Serializer { rounding }
    }

    /// Serialize a value as an integer.
    ///
    /// Integer numbers and BigInts pass through as decimal strings. Non-integer
    /// numbers are rounded by the configured mode. Infinity or NaN after
    /// rounding raises the conversion error.
    pub fn as_integer(&self, value: &Value) -> Result<String, StringifyError> {
        match value {
            Value::Number(n) if is_integer(*n) => Ok(format_f64(*n)),
            Value::BigInt(i) => Ok(i.to_string()),
            Value::Number(n) => {
                let rounded = self.rounding.apply(*n);
                if !rounded.is_finite() {
                    return Err(StringifyError::cannot_convert(
                        display_value(value),
                        "an integer",
                    ));
                }
                Ok(format_f64(rounded))
            }
            // Null coerces through Math.trunc(null) === 0, plus other JS coercions.
            other => {
                let n = coerce_number(other);
                match n {
                    Some(n) if is_integer(n) => Ok(format_f64(n)),
                    Some(n) => {
                        let rounded = self.rounding.apply(n);
                        if !rounded.is_finite() {
                            return Err(StringifyError::cannot_convert(
                                display_value(value),
                                "an integer",
                            ));
                        }
                        Ok(format_f64(rounded))
                    }
                    None => Err(StringifyError::cannot_convert(
                        display_value(value),
                        "an integer",
                    )),
                }
            }
        }
    }

    /// Serialize a value as a number.
    ///
    /// Coerces with JavaScript `Number(x)` rules. NaN raises the conversion
    /// error. Both infinities render as the literal `null`.
    pub fn as_number(&self, value: &Value) -> Result<String, StringifyError> {
        match coerce_number(value) {
            Some(n) if n.is_nan() => Err(StringifyError::cannot_convert(
                display_value(value),
                "a number",
            )),
            Some(n) if n.is_infinite() => Ok("null".to_string()),
            Some(n) => Ok(format_f64(n)),
            None => Err(StringifyError::cannot_convert(
                display_value(value),
                "a number",
            )),
        }
    }

    /// Serialize a value as a boolean using JavaScript truthiness.
    pub fn as_boolean(&self, value: &Value) -> String {
        if is_truthy(value) {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }

    /// Serialize a value as a full date-time. `null` yields `""`, a `Date`
    /// yields its ISO string, a string passes through unchanged.
    pub fn as_date_time(&self, value: &Value) -> Result<String, StringifyError> {
        match value {
            Value::Null => Ok("\"\"".to_string()),
            Value::Date(ms) => Ok(format!("\"{}\"", iso_from_millis(*ms))),
            Value::String(s) => Ok(format!("\"{s}\"")),
            other => Err(StringifyError::cannot_convert(
                display_value(other),
                "a date-time",
            )),
        }
    }

    /// Serialize a value as a calendar date (`YYYY-MM-DD`). Dates are rendered
    /// in UTC, matching the test harness which pins the timezone.
    pub fn as_date(&self, value: &Value) -> Result<String, StringifyError> {
        match value {
            Value::Null => Ok("\"\"".to_string()),
            Value::Date(ms) => Ok(format!("\"{}\"", date_slice_from_millis(*ms))),
            Value::String(s) => Ok(format!("\"{s}\"")),
            other => Err(StringifyError::cannot_convert(
                display_value(other),
                "a date",
            )),
        }
    }

    /// Serialize a value as a wall-clock time (`HH:mm:ss`) in UTC.
    pub fn as_time(&self, value: &Value) -> Result<String, StringifyError> {
        match value {
            Value::Null => Ok("\"\"".to_string()),
            Value::Date(ms) => Ok(format!("\"{}\"", time_slice_from_millis(*ms))),
            Value::String(s) => Ok(format!("\"{s}\"")),
            other => Err(StringifyError::cannot_convert(
                display_value(other),
                "a time",
            )),
        }
    }

    /// Serialize a string with full JSON escaping. The result is a quoted JSON
    /// string whose bytes match `JSON.stringify` for any well-formed UTF-8.
    pub fn as_string(&self, s: &str) -> String {
        escape_json_string(s)
    }

    /// Serialize a string without any escaping, used only by `format: "unsafe"`.
    /// The caller is responsible for the result being valid JSON.
    pub fn as_unsafe_string(&self, s: &str) -> String {
        format!("\"{s}\"")
    }
}

/// True when a double has no fractional part and is finite, matching
/// `Number.isInteger`.
fn is_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0
}

/// Reproduce JavaScript `Number(x)` coercion for the value kinds that reach the
/// numeric serializers. Returns `None` only for kinds that produce NaN through
/// an object path the serializer treats as a hard failure.
fn coerce_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        Value::BigInt(i) => Some(*i as f64),
        Value::Null => Some(0.0),
        Value::Bool(true) => Some(1.0),
        Value::Bool(false) => Some(0.0),
        Value::String(s) => Some(coerce_string_number(s)),
        Value::Array(items) => match items.as_slice() {
            // Number([]) === 0, Number([x]) === Number(x), else NaN.
            [] => Some(0.0),
            [one] => Some(coerce_number(one).unwrap_or(f64::NAN)),
            _ => Some(f64::NAN),
        },
        // Date coerces to its epoch millis under Number().
        Value::Date(ms) => Some(*ms as f64),
        // Plain objects and regexps coerce to NaN under Number().
        Value::Object(_) | Value::Regex(_) | Value::Custom(_) => Some(f64::NAN),
    }
}

/// Reproduce JavaScript `Number(string)`: trims whitespace, empty is 0, accepts
/// decimal, hex, octal, binary, and exponent forms, otherwise NaN.
fn coerce_string_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i128::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return i128::from_str_radix(oct, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return i128::from_str_radix(bin, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// JavaScript truthiness for `asBoolean`.
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
        Value::BigInt(i) => *i != 0,
        Value::String(s) => !s.is_empty(),
        // Every object, array, date, and regexp is truthy.
        Value::Array(_)
        | Value::Object(_)
        | Value::Date(_)
        | Value::Regex(_)
        | Value::Custom(_) => true,
    }
}

/// Render a value the way it appears inside an error message, mirroring the
/// `${i}` interpolation in the source.
pub(crate) fn display_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => format_f64(*n),
        Value::BigInt(i) => i.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            // Array.prototype.toString joins with commas.
            items
                .iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(",")
        }
        Value::Object(_) | Value::Custom(_) => "[object Object]".to_string(),
        Value::Date(ms) => iso_from_millis(*ms),
        Value::Regex(src) => format!("/{src}/"),
    }
}

/// Escape a string into a quoted JSON string. Matches `JSON.stringify`: short
/// escapes for the common control characters, `\u00XX` for the rest below
/// 0x20, and backslash escapes for `"` and `\`.
fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
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
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
