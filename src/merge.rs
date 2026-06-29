//! Merge a set of JSON schemas into one.
//!
//! `allOf`, `oneOf`, `anyOf`, and `if/then/else` each combine a base schema
//! with a branch before serializing. The merge unions `required`, deep-merges
//! object keyword maps, and keeps the first value on a scalar conflict. The
//! `type` and `format` keywords raise an error on an irreconcilable conflict,
//! matching the source merge behavior.

use crate::error::BuildError;
use serde_json::{Map, Value};

/// Merge schemas left to right. The first schema seeds the result, later
/// schemas contribute keywords not yet present and extend the mergeable maps.
pub fn merge_schemas(schemas: &[Value]) -> Result<Value, BuildError> {
    let mut result = Map::new();
    for schema in schemas {
        let Value::Object(map) = schema else {
            // A boolean schema contributes nothing to a keyword merge.
            continue;
        };
        for (key, value) in map {
            merge_keyword(&mut result, key, value)?;
        }
    }
    Ok(Value::Object(result))
}

/// Fold one keyword from a source schema into the accumulating result.
fn merge_keyword(
    result: &mut Map<String, Value>,
    key: &str,
    value: &Value,
) -> Result<(), BuildError> {
    match key {
        "type" => merge_type(result, value),
        "format" => merge_format(result, value),
        "required" => {
            merge_required(result, value);
            Ok(())
        }
        "properties" | "patternProperties" => {
            merge_object_map(result, key, value)?;
            Ok(())
        }
        _ => {
            // First writer wins for everything else, matching onConflict: skip.
            result
                .entry(key.to_string())
                .or_insert_with(|| value.clone());
            Ok(())
        }
    }
}

/// Merge `type`. Equal or compatible types combine, an incompatible pair errors.
fn merge_type(result: &mut Map<String, Value>, value: &Value) -> Result<(), BuildError> {
    match result.get("type") {
        None => {
            result.insert("type".to_string(), value.clone());
            Ok(())
        }
        Some(existing) => {
            let a = as_type_set(existing);
            let b = as_type_set(value);
            // Intersection of the two type sets. An empty intersection cannot
            // be satisfied, which the source reports as a merge failure.
            let merged: Vec<String> = a.iter().filter(|t| b.contains(*t)).cloned().collect();
            if merged.is_empty() {
                return Err(BuildError::new("Failed to merge \"type\" keyword schemas."));
            }
            let new_value = if merged.len() == 1 {
                Value::String(merged[0].clone())
            } else {
                Value::Array(merged.into_iter().map(Value::String).collect())
            };
            result.insert("type".to_string(), new_value);
            Ok(())
        }
    }
}

/// Merge `format`. Equal formats keep, differing formats error.
fn merge_format(result: &mut Map<String, Value>, value: &Value) -> Result<(), BuildError> {
    match result.get("format") {
        None => {
            result.insert("format".to_string(), value.clone());
            Ok(())
        }
        Some(existing) if existing == value => Ok(()),
        Some(_) => Err(BuildError::new(
            "Failed to merge \"format\" keyword schemas.",
        )),
    }
}

/// Union two `required` arrays, preserving order and dropping duplicates.
fn merge_required(result: &mut Map<String, Value>, value: &Value) {
    let Value::Array(incoming) = value else {
        return;
    };
    let entry = result
        .entry("required".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(existing) = entry {
        for item in incoming {
            if !existing.contains(item) {
                existing.push(item.clone());
            }
        }
    }
}

/// Deep-merge a keyword whose value is a map of subschemas, recursing into
/// overlapping keys.
fn merge_object_map(
    result: &mut Map<String, Value>,
    key: &str,
    value: &Value,
) -> Result<(), BuildError> {
    let Value::Object(incoming) = value else {
        result
            .entry(key.to_string())
            .or_insert_with(|| value.clone());
        return Ok(());
    };
    let entry = result
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(existing) = entry {
        for (k, v) in incoming {
            match existing.get(k) {
                Some(prev) => {
                    let merged = merge_schemas(&[prev.clone(), v.clone()])?;
                    existing.insert(k.clone(), merged);
                }
                None => {
                    existing.insert(k.clone(), v.clone());
                }
            }
        }
    }
    Ok(())
}

/// Read a `type` value as a set of type names.
fn as_type_set(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}
