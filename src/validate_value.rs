//! Runtime value validation for combinator branch selection.
//!
//! `anyOf`, `oneOf`, and `if/then/else` pick a branch by testing the input
//! against each candidate schema. This validator covers the keywords those
//! schemas use in practice: `type`, `enum`, `const`, `required`, `properties`,
//! numeric bounds, array bounds, and `additionalProperties: false`. A value the
//! serializer would coerce to a string (a `Date`, `RegExp`, or `toJSON` object)
//! counts as a string, mirroring the `fjs_type` leniency in the source.

use crate::refresolver::RefResolver;
use crate::value::Value;
use serde_json::Value as Json;

/// Test whether `value` satisfies `schema`, resolving `$ref` through `resolver`.
pub fn validate(schema: &Json, value: &Value, resolver: &RefResolver, base_id: &str) -> bool {
    match schema {
        Json::Bool(b) => *b,
        Json::Object(map) => {
            if let Some(Json::String(reference)) = map.get("$ref") {
                if let Some((resolved, next_base)) = resolve(reference, resolver, base_id) {
                    return validate(resolved, value, resolver, &next_base);
                }
                return false;
            }
            check_keywords(map, value, resolver, base_id)
        }
        _ => true,
    }
}

/// Resolve a `$ref` to its target schema and the base id of the owning document.
fn resolve<'a>(
    reference: &str,
    resolver: &'a RefResolver,
    base_id: &str,
) -> Option<(&'a Json, String)> {
    let hash = reference.find('#').unwrap_or(reference.len());
    let id = &reference[..hash];
    let pointer = if hash < reference.len() {
        &reference[hash..]
    } else {
        "#"
    };
    let doc_id = if id.is_empty() { base_id } else { id };
    resolver
        .get_schema(doc_id, pointer)
        .map(|s| (s, doc_id.to_string()))
}

/// Check every supported keyword on an object schema.
fn check_keywords(
    map: &serde_json::Map<String, Json>,
    value: &Value,
    resolver: &RefResolver,
    base_id: &str,
) -> bool {
    // A nullable schema accepts null outright, matching the OpenAPI extension.
    if map.get("nullable") == Some(&Json::Bool(true)) && matches!(value, Value::Null) {
        return true;
    }

    if let Some(type_value) = map.get("type") {
        if !check_type(type_value, value) {
            return false;
        }
    }

    if let Some(Json::Array(allowed)) = map.get("enum") {
        if !allowed.iter().any(|opt| json_equals_value(opt, value)) {
            return false;
        }
    }

    if let Some(constant) = map.get("const") {
        if !json_equals_value(constant, value) {
            return false;
        }
    }

    if let Some(Json::Array(required)) = map.get("required") {
        if let Value::Object(obj) = value {
            for key in required {
                if let Json::String(name) = key {
                    if obj.get(name).is_none() {
                        return false;
                    }
                }
            }
        } else {
            return false;
        }
    }

    if let Value::Object(obj) = value {
        if let Some(Json::Object(props)) = map.get("properties") {
            for (key, sub) in props {
                if let Some(field) = obj.get(key) {
                    if !validate(sub, field, resolver, base_id) {
                        return false;
                    }
                }
            }
        }
        if map.get("additionalProperties") == Some(&Json::Bool(false)) {
            let known: Vec<&String> = map
                .get("properties")
                .and_then(|p| p.as_object())
                .map(|p| p.keys().collect())
                .unwrap_or_default();
            for (key, _) in obj.iter() {
                if !known.contains(&key) {
                    return false;
                }
            }
        }
    }

    if let Some(num) = value_as_number(value) {
        if let Some(min) = map.get("minimum").and_then(Json::as_f64) {
            if num < min {
                return false;
            }
        }
        if let Some(max) = map.get("maximum").and_then(Json::as_f64) {
            if num > max {
                return false;
            }
        }
        if let Some(emin) = map.get("exclusiveMinimum").and_then(Json::as_f64) {
            if num <= emin {
                return false;
            }
        }
        if let Some(emax) = map.get("exclusiveMaximum").and_then(Json::as_f64) {
            if num >= emax {
                return false;
            }
        }
    }

    if let Value::Array(items) = value {
        if let Some(max) = map.get("maxItems").and_then(Json::as_u64) {
            if items.len() as u64 > max {
                return false;
            }
        }
        if let Some(min) = map.get("minItems").and_then(Json::as_u64) {
            if (items.len() as u64) < min {
                return false;
            }
        }
        // A single items schema applies to every element.
        if let Some(item_schema @ (Json::Object(_) | Json::Bool(_))) = map.get("items") {
            for element in items {
                if !validate(item_schema, element, resolver, base_id) {
                    return false;
                }
            }
        }
    }

    true
}

/// Check a value against a `type` keyword, applying the string leniency.
fn check_type(type_value: &Json, value: &Value) -> bool {
    match type_value {
        Json::String(t) => matches_single_type(t, value),
        Json::Array(types) => types.iter().any(|t| {
            t.as_str()
                .map(|name| matches_single_type(name, value))
                .unwrap_or(false)
        }),
        _ => true,
    }
}

/// True when a value matches one type name. A `Date`, `RegExp`, or `toJSON`
/// object satisfies `string`, matching the serializer coercion.
fn matches_single_type(name: &str, value: &Value) -> bool {
    match name {
        "null" => matches!(value, Value::Null),
        "boolean" => matches!(value, Value::Bool(_)),
        "object" => matches!(value, Value::Object(_)),
        "array" => matches!(value, Value::Array(_)),
        "string" => matches!(
            value,
            Value::String(_) | Value::Date(_) | Value::Regex(_) | Value::Custom(_)
        ),
        "number" => matches!(value, Value::Number(_) | Value::BigInt(_)),
        "integer" => match value {
            Value::Number(n) => n.fract() == 0.0 && n.is_finite(),
            Value::BigInt(_) => true,
            _ => false,
        },
        _ => false,
    }
}

/// Pull a numeric value out for bound checks.
fn value_as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => Some(*n),
        Value::BigInt(i) => Some(*i as f64),
        _ => None,
    }
}

/// Compare a schema JSON literal to a runtime value for `enum`/`const`.
fn json_equals_value(json: &Json, value: &Value) -> bool {
    match (json, value) {
        (Json::Null, Value::Null) => true,
        (Json::Bool(a), Value::Bool(b)) => a == b,
        (Json::String(a), Value::String(b)) => a == b,
        (Json::Number(a), Value::Number(b)) => a.as_f64() == Some(*b),
        (Json::Number(a), Value::BigInt(b)) => a.as_f64() == Some(*b as f64),
        (Json::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| json_equals_value(x, y))
        }
        (Json::Object(a), Value::Object(b)) => {
            a.len() == b.iter().count()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|bv| json_equals_value(v, bv)))
        }
        _ => false,
    }
}
