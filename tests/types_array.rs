//! Multi-type (`type: [...]`) unions.

mod common;

use common::{build_ok, js_stringify, run};
use fast_json_stringify::{Object, Value};
use serde_json::json;

#[test]
fn single_element_type_array() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["integer"] } } }),
            json!({ "data": 4 })
        ),
        "{\"data\":4}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["number"] } } }),
            json!({ "data": 4 })
        ),
        "{\"data\":4}"
    );
}

#[test]
fn single_element_type_array_null_coerces() {
    // Without 'null' in the union, a null falls into the first branch and coerces.
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["integer"] } } }),
            json!({ "data": null })
        ),
        "{\"data\":0}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["number"] } } }),
            json!({ "data": null })
        ),
        "{\"data\":0}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["boolean"] } } }),
            json!({ "data": null })
        ),
        "{\"data\":false}"
    );
}

#[test]
fn nullable_primitive_union() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["integer", "null"] } } }),
            json!({ "data": 4 })
        ),
        "{\"data\":4}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["integer", "null"] } } }),
            json!({ "data": null })
        ),
        "{\"data\":null}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "data": { "type": ["number", "null"] } } }),
            json!({ "data": null })
        ),
        "{\"data\":null}"
    );
}

#[test]
fn object_or_null_with_multi_type_property() {
    let schema = json!({
        "type": "object",
        "properties": {
            "objectOrNull": {
                "type": ["object", "null"],
                "properties": { "stringOrNumber": { "type": ["string", "number"] } }
            }
        }
    });
    assert_eq!(
        run(
            schema.clone(),
            json!({ "objectOrNull": { "stringOrNumber": "string" } })
        ),
        "{\"objectOrNull\":{\"stringOrNumber\":\"string\"}}"
    );
    assert_eq!(
        run(
            schema.clone(),
            json!({ "objectOrNull": { "stringOrNumber": 42 } })
        ),
        "{\"objectOrNull\":{\"stringOrNumber\":42}}"
    );
    assert_eq!(
        run(schema, json!({ "objectOrNull": null })),
        "{\"objectOrNull\":null}"
    );
}

#[test]
fn array_or_null_of_multiple_types() {
    let schema = json!({
        "type": "object",
        "properties": {
            "arr": { "type": ["array", "null"], "items": { "type": ["string", "number", "null"] } }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "arr": null })),
        "{\"arr\":null}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "arr": ["string1", "string2"] })),
        "{\"arr\":[\"string1\",\"string2\"]}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "arr": [42, 7] })),
        "{\"arr\":[42,7]}"
    );
    assert_eq!(
        run(
            schema.clone(),
            json!({ "arr": ["string1", 42, 7, "string2"] })
        ),
        "{\"arr\":[\"string1\",42,7,\"string2\"]}"
    );
    assert_eq!(
        run(
            schema,
            json!({ "arr": ["string1", null, 42, 7, "string2", null] })
        ),
        "{\"arr\":[\"string1\",null,42,7,\"string2\",null]}"
    );
}

#[test]
fn tuple_of_multiple_types() {
    let schema = json!({
        "type": "object",
        "properties": {
            "t": {
                "type": "array",
                "items": [{ "type": "string" }, { "type": "number" }, { "type": ["string", "number"] }]
            }
        }
    });
    assert_eq!(
        run(schema.clone(), json!({ "t": ["string1", 42, 7] })),
        "{\"t\":[\"string1\",42,7]}"
    );
    assert_eq!(
        run(schema, json!({ "t": ["string1", 42, "string2"] })),
        "{\"t\":[\"string1\",42,\"string2\"]}"
    );
}

#[test]
fn string_type_array_handles_dates() {
    let schema = json!({
        "type": "object",
        "properties": {
            "date": { "type": ["string"] },
            "dateObject": { "type": ["string"], "format": "date-time" }
        }
    });
    let stringify = build_ok(schema);
    let mut obj = Object::new();
    // 2018-04-20T07:52:31.017Z and 2018-04-21T07:52:31.017Z.
    obj.insert("date", Value::Date(1524210751017));
    obj.insert("dateObject", Value::Date(1524297151017));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"date\":\"2018-04-20T07:52:31.017Z\",\"dateObject\":\"2018-04-21T07:52:31.017Z\"}"
    );
}

#[test]
fn array_null_coerces_to_empty() {
    let schema = json!({
        "type": "object",
        "properties": { "arr": { "type": "array", "items": { "type": "number" } } }
    });
    assert_eq!(
        run(schema, json!({ "arr": null })),
        js_stringify(&json!({ "arr": [] }))
    );
}

#[test]
fn array_non_array_throws() {
    let schema = json!({
        "type": "object",
        "properties": { "arr": { "type": "array", "items": { "type": "number" } } }
    });
    let stringify = build_ok(schema);
    let err = stringify
        .call(&Value::from(json!({ "arr": { "foo": "hello" } })))
        .unwrap_err();
    assert_eq!(
        err.message(),
        "The value of '#/properties/arr' does not match schema definition."
    );
}

#[test]
fn none_of_types_matches_throws() {
    let schema = json!({
        "type": "object",
        "properties": { "data": { "type": ["number", "boolean"] } }
    });
    let stringify = build_ok(schema);
    let err = stringify
        .call(&Value::from(json!({ "data": "string" })))
        .unwrap_err();
    assert_eq!(
        err.message(),
        "The value of '#/properties/data' does not match schema definition."
    );
}
