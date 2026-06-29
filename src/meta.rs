//! Draft-7 meta-schema validation.
//!
//! Before compiling, the source validates the schema against the Draft-7
//! meta-schema and throws a message shaped like
//! `schema is invalid: data<path> <reason>`. This module checks the keywords the
//! tests exercise and reproduces that message format. It is not a full
//! meta-schema, it covers the structural rules that the suite asserts on.
//!
//! Covered: `type` values, combinator array shape and minimum length,
//! schema-typed keywords (`not`, `if`, `then`, `else`, `additionalProperties`,
//! `additionalItems`), `patternProperties` regex validity, schema-map keyword
//! shapes (`properties`, `definitions`, `$defs`), and `items` shape.
//!
//! Not covered: value-level rules such as `required` being an array of unique
//! strings, `enum` being a non-empty array, or numeric keywords being numbers.
//! A schema invalid under Draft-7 in one of those ways compiles here instead of
//! throwing.

use crate::error::BuildError;
use serde_json::Value;

/// Valid `type` keyword values.
const VALID_TYPES: [&str; 7] = [
    "null", "boolean", "object", "array", "number", "string", "integer",
];

/// Validate a schema document. `name` labels external schemas in the error.
pub fn validate(schema: &Value, name: Option<&str>) -> Result<(), BuildError> {
    if let Some(failure) = check(schema, "") {
        let prefix = match name {
            Some(n) => format!("\"{n}\" "),
            None => String::new(),
        };
        return Err(BuildError::new(format!(
            "{prefix}schema is invalid: data{} {}",
            failure.instance_path, failure.message
        )));
    }
    Ok(())
}

/// A single meta-schema failure with the JSON Pointer to the offending node.
struct Failure {
    instance_path: String,
    message: String,
}

/// Recurse through a schema, returning the first structural failure.
fn check(schema: &Value, path: &str) -> Option<Failure> {
    let map = match schema {
        Value::Bool(_) => return None,
        Value::Object(map) => map,
        // A schema must be an object or a boolean.
        _ => {
            return Some(Failure {
                instance_path: path.to_string(),
                message: "must be object,boolean".to_string(),
            })
        }
    };

    if let Some(type_value) = map.get("type") {
        if let Some(failure) = check_type(type_value, &format!("{path}/type")) {
            return Some(failure);
        }
    }

    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(value) = map.get(combinator) {
            let sub_path = format!("{path}/{combinator}");
            match value {
                Value::Array(items) => {
                    if items.is_empty() {
                        return Some(Failure {
                            instance_path: sub_path,
                            message: "must NOT have fewer than 1 items".to_string(),
                        });
                    }
                    for (i, item) in items.iter().enumerate() {
                        if let Some(f) = check(item, &format!("{sub_path}/{i}")) {
                            return Some(f);
                        }
                    }
                }
                _ => {
                    return Some(Failure {
                        instance_path: sub_path,
                        message: "must be array".to_string(),
                    })
                }
            }
        }
    }

    for keyword in [
        "not",
        "if",
        "then",
        "else",
        "additionalProperties",
        "additionalItems",
    ] {
        if let Some(value) = map.get(keyword) {
            // These take a schema (object or boolean).
            if !matches!(value, Value::Object(_) | Value::Bool(_)) {
                return Some(Failure {
                    instance_path: format!("{path}/{keyword}"),
                    message: "must be object,boolean".to_string(),
                });
            }
            if let Some(f) = check(value, &format!("{path}/{keyword}")) {
                return Some(f);
            }
        }
    }

    if let Some(Value::Object(patterns)) = map.get("patternProperties") {
        for key in patterns.keys() {
            if regex::Regex::new(key).is_err() {
                return Some(Failure {
                    instance_path: format!("{path}/patternProperties"),
                    message: "must match format \"regex\"".to_string(),
                });
            }
        }
    }

    for keyword in ["properties", "patternProperties", "definitions", "$defs"] {
        match map.get(keyword) {
            Some(Value::Object(props)) => {
                for (key, sub) in props {
                    if let Some(f) = check(sub, &format!("{path}/{keyword}/{key}")) {
                        return Some(f);
                    }
                }
            }
            // A non-object value for a schema-map keyword is invalid.
            Some(_) => {
                return Some(Failure {
                    instance_path: format!("{path}/{keyword}"),
                    message: "must be object".to_string(),
                })
            }
            None => {}
        }
    }

    if let Some(items) = map.get("items") {
        match items {
            Value::Array(arr) => {
                for (i, sub) in arr.iter().enumerate() {
                    if let Some(f) = check(sub, &format!("{path}/items/{i}")) {
                        return Some(f);
                    }
                }
            }
            other => {
                if let Some(f) = check(other, &format!("{path}/items")) {
                    return Some(f);
                }
            }
        }
    }

    None
}

/// Validate a `type` keyword value.
fn check_type(value: &Value, path: &str) -> Option<Failure> {
    match value {
        Value::String(s) => {
            if VALID_TYPES.contains(&s.as_str()) {
                None
            } else {
                Some(Failure {
                    instance_path: path.to_string(),
                    message: "must be equal to one of the allowed values".to_string(),
                })
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                match item {
                    Value::String(s) if VALID_TYPES.contains(&s.as_str()) => {}
                    _ => {
                        return Some(Failure {
                            instance_path: format!("{path}/{i}"),
                            message: "must be equal to one of the allowed values".to_string(),
                        })
                    }
                }
            }
            None
        }
        _ => Some(Failure {
            instance_path: path.to_string(),
            message: "must be string,array".to_string(),
        }),
    }
}
