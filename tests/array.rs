//! Array serialization: tuples, additionalItems, and large arrays.

mod common;

use common::{build_ok, build_ok_opts, js_stringify, run, run_err};
use fast_json_stringify::{LargeArrayMechanism, Object, Options, Value};
use serde_json::json;

#[test]
fn dates_tuple() {
    let schema = json!({
        "type": "object",
        "properties": {
            "dates": {
                "type": "array",
                "minItems": 2,
                "maxItems": 2,
                "items": [
                    { "type": "string", "format": "date-time" },
                    { "type": "string", "format": "date-time" }
                ]
            }
        }
    });
    let stringify = build_ok(schema);
    let mut obj = Object::new();
    obj.insert("dates", Value::Array(vec![Value::Date(1), Value::Date(2)]));
    assert_eq!(
        stringify.call(&Value::Object(obj)).unwrap(),
        "{\"dates\":[\"1970-01-01T00:00:00.001Z\",\"1970-01-01T00:00:00.002Z\"]}"
    );
}

#[test]
fn string_and_number_arrays() {
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "string" } } } }),
            json!({ "ids": ["test"] })
        ),
        "{\"ids\":[\"test\"]}"
    );
    assert_eq!(
        run(
            json!({ "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "number" } } } }),
            json!({ "ids": [1] })
        ),
        "{\"ids\":[1]}"
    );
}

#[test]
fn mixed_tuple() {
    let schema = json!({
        "type": "object",
        "properties": {
            "ids": {
                "type": "array",
                "items": [
                    { "type": "null" },
                    { "type": "string" },
                    { "type": "integer" },
                    { "type": "number" },
                    { "type": "boolean" },
                    { "type": "object", "properties": { "a": { "type": "string" } } },
                    { "type": "array", "items": { "type": "string" } }
                ]
            }
        }
    });
    let input = json!({ "ids": [null, "test", 1, 1.1, true, { "a": "test" }, ["test"]] });
    assert_eq!(run(schema, input.clone()), js_stringify(&input));
}

#[test]
fn pattern_properties_tuple() {
    let schema = json!({
        "type": "object",
        "properties": {
            "args": {
                "type": "array",
                "items": [
                    { "type": "object", "patternProperties": { ".*": { "type": "string" } } },
                    { "type": "object", "patternProperties": { ".*": { "type": "number" } } }
                ]
            }
        }
    });
    assert_eq!(
        run(schema, json!({ "args": [{ "a": "test" }, { "b": 1 }] })),
        "{\"args\":[{\"a\":\"test\"},{\"b\":1}]}"
    );
}

#[test]
fn invalid_tuple_item_throws() {
    let schema = json!({
        "type": "object",
        "properties": {
            "args": {
                "type": "array",
                "items": [{ "type": "object", "patternProperties": { ".*": { "type": "string" } } }]
            }
        }
    });
    let stringify = build_ok(schema);
    assert!(stringify
        .call(&Value::from(json!({ "args": ["invalid"] })))
        .is_err());
}

#[test]
fn untyped_array_items_default_to_any() {
    let schema = json!({ "type": "object", "properties": { "foo": { "type": "array" } } });
    let input = json!({ "foo": [1, "string", {}, null] });
    assert_eq!(run(schema, input.clone()), js_stringify(&input));
}

#[test]
fn additional_items_true_appends_extra() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "array", "items": [{ "type": "string" }], "additionalItems": true }
        }
    });
    assert_eq!(
        run(schema, json!({ "foo": ["foo", "bar", 1] })),
        "{\"foo\":[\"foo\",\"bar\",1]}"
    );
}

#[test]
fn additional_items_true_with_fewer_items() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "array", "items": [{ "type": "string" }, { "type": "number" }], "additionalItems": true }
        }
    });
    assert_eq!(
        run(schema, json!({ "foo": ["foo"] })),
        "{\"foo\":[\"foo\"]}"
    );
}

#[test]
fn additional_items_false_overflow_throws() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "array", "items": [{ "type": "string" }], "additionalItems": false }
        }
    });
    let err = run_err(schema, json!({ "foo": ["foo", "bar"] }));
    assert_eq!(err, "Item at 1 does not match schema definition.");
}

#[test]
fn tuple_item_type_mismatch_throws() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "array", "items": [{ "type": "string" }, { "type": "string" }], "additionalItems": false }
        }
    });
    assert_eq!(
        run_err(schema.clone(), json!({ "foo": [1, "bar"] })),
        "Item at 0 does not match schema definition."
    );
    assert_eq!(
        run_err(schema.clone(), json!({ "foo": ["foo", 1] })),
        "Item at 1 does not match schema definition."
    );
    assert_eq!(
        run_err(schema, json!({ "foo": ["foo", "bar", "baz"] })),
        "Item at 2 does not match schema definition."
    );
}

#[test]
fn additional_items_ignored_when_items_not_tuple() {
    let schema = json!({
        "type": "object",
        "properties": { "foo": { "type": "array", "items": { "type": "string" }, "additionalItems": false } }
    });
    assert_eq!(
        run(schema, json!({ "foo": ["foo", "bar"] })),
        "{\"foo\":[\"foo\",\"bar\"]}"
    );
}

#[test]
fn additional_items_schema_appends() {
    let schema = json!({
        "type": "object",
        "properties": {
            "foo": { "type": "array", "items": [{ "type": "string" }], "additionalItems": { "type": "number" } }
        }
    });
    assert_eq!(
        run(schema, json!({ "foo": ["foo", 42] })),
        "{\"foo\":[\"foo\",42]}"
    );
}

#[test]
fn array_with_anyof_items() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "option": {
                    "anyOf": [
                        { "type": "string", "enum": ["Foo"] },
                        { "type": "string", "enum": ["Bar"] }
                    ]
                }
            },
            "required": ["name", "option"]
        }
    });
    assert_eq!(
        run(
            schema,
            json!([{ "name": "name-0", "option": "Foo" }, { "name": "name-1", "option": "Bar" }])
        ),
        "[{\"name\":\"name-0\",\"option\":\"Foo\"},{\"name\":\"name-1\",\"option\":\"Bar\"}]"
    );
}

#[test]
fn shared_item_schema_via_ref() {
    let schema = json!({
        "type": "object",
        "properties": {
            "array1": { "type": "array", "items": [{ "type": "string" }], "additionalItems": false },
            "array2": { "type": "array", "items": { "$ref": "#/properties/array1/items" }, "additionalItems": true }
        }
    });
    assert_eq!(
        run(
            schema,
            json!({ "array1": ["bar"], "array2": ["foo", "bar"] })
        ),
        "{\"array1\":[\"bar\"],\"array2\":[\"foo\",\"bar\"]}"
    );
}

#[test]
fn large_array_default_mechanism() {
    let schema = json!({
        "type": "object",
        "properties": {
            "ids": { "type": "array", "items": { "type": "object", "properties": { "a": { "type": "string" }, "b": { "type": "number" } } } }
        }
    });
    let opts = Options {
        large_array_size: 20_000,
        ..Options::new()
    };
    let stringify = build_ok_opts(schema, opts);
    let item = json!({ "a": "test", "b": 1 });
    let data = json!({ "ids": vec![item; 20_000] });
    assert_eq!(
        stringify.call(&Value::from(data.clone())).unwrap(),
        js_stringify(&data)
    );
}

#[test]
fn large_array_json_stringify_matches_default() {
    // The two mechanisms must produce identical output, only performance differs.
    let schema = json!({
        "type": "object",
        "properties": {
            "ids": { "type": "array", "items": { "type": "object", "properties": { "a": { "type": "string" }, "b": { "type": "number" } } } }
        }
    });
    let item = json!({ "a": "test", "b": 1 });
    let data = Value::from(json!({ "ids": vec![item; 20_000] }));

    let default = build_ok_opts(
        schema.clone(),
        Options {
            large_array_size: 20_000,
            ..Options::new()
        },
    );
    let js = build_ok_opts(
        schema,
        Options {
            large_array_size: 20_000,
            large_array_mechanism: LargeArrayMechanism::JsonStringify,
            ..Options::new()
        },
    );
    assert_eq!(default.call(&data).unwrap(), js.call(&data).unwrap());
}

#[test]
fn large_array_size_as_smaller_threshold() {
    // json-stringify short circuits once the array reaches the configured size.
    let schema = json!({ "type": "array", "items": { "type": "integer" } });
    let opts = Options {
        large_array_size: 3,
        large_array_mechanism: LargeArrayMechanism::JsonStringify,
        ..Options::new()
    };
    let stringify = build_ok_opts(schema, opts);
    // Three elements hits the threshold and goes through native JSON.
    assert_eq!(
        stringify.call(&Value::from(json!([1, 2, 3]))).unwrap(),
        "[1,2,3]"
    );
    // Two elements stays on the element path.
    assert_eq!(
        stringify.call(&Value::from(json!([1, 2]))).unwrap(),
        "[1,2]"
    );
}
