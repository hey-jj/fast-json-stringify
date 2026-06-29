//! Empty (any) schemas serialize via native JSON.

mod common;

use common::{build_ok, run};
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn nested_random_property() {
    let schema = json!({
        "type": "object",
        "properties": { "id": { "type": "number" }, "name": {} }
    });
    assert_eq!(
        run(schema.clone(), json!({ "id": 1, "name": "string" })),
        "{\"id\":1,\"name\":\"string\"}"
    );
    assert_eq!(
        run(
            schema.clone(),
            json!({ "id": 1, "name": { "first": "name", "last": "last" } })
        ),
        "{\"id\":1,\"name\":{\"first\":\"name\",\"last\":\"last\"}}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "id": 1, "name": null })),
        "{\"id\":1,\"name\":null}"
    );
    assert_eq!(
        run(schema, json!({ "id": 1, "name": ["first", "last"] })),
        "{\"id\":1,\"name\":[\"first\",\"last\"]}"
    );
}

#[test]
fn array_with_random_items() {
    assert_eq!(
        run(
            json!({ "type": "array", "items": {} }),
            json!([1, "string", null])
        ),
        "[1,\"string\",null]"
    );
}

#[test]
fn empty_schema_any_value() {
    let stringify = build_ok(json!({}));
    assert_eq!(stringify.call(&Value::Null).unwrap(), "null");
    assert_eq!(stringify.call(&Value::from(json!(1))).unwrap(), "1");
    assert_eq!(stringify.call(&Value::from(json!(true))).unwrap(), "true");
    assert_eq!(
        stringify.call(&Value::from(json!("hello"))).unwrap(),
        "\"hello\""
    );
    assert_eq!(stringify.call(&Value::from(json!({}))).unwrap(), "{}");
    assert_eq!(
        stringify.call(&Value::from(json!({ "x": 10 }))).unwrap(),
        "{\"x\":10}"
    );
    assert_eq!(
        stringify
            .call(&Value::from(json!([true, 1, "hello"])))
            .unwrap(),
        "[true,1,\"hello\"]"
    );
}

#[test]
fn empty_schema_on_nested_object() {
    let schema = json!({ "type": "object", "properties": { "x": {} } });
    assert_eq!(run(schema.clone(), json!({ "x": null })), "{\"x\":null}");
    assert_eq!(run(schema.clone(), json!({ "x": 1 })), "{\"x\":1}");
    assert_eq!(run(schema.clone(), json!({ "x": true })), "{\"x\":true}");
    assert_eq!(
        run(schema.clone(), json!({ "x": "hello" })),
        "{\"x\":\"hello\"}"
    );
    assert_eq!(run(schema.clone(), json!({ "x": {} })), "{\"x\":{}}");
    assert_eq!(
        run(schema.clone(), json!({ "x": { "x": 10 } })),
        "{\"x\":{\"x\":10}}"
    );
    assert_eq!(
        run(schema, json!({ "x": [true, 1, "hello"] })),
        "{\"x\":[true,1,\"hello\"]}"
    );
}

#[test]
fn empty_schema_on_array() {
    let schema = json!({ "type": "array", "items": {} });
    assert_eq!(
        run(schema, json!([1, true, "hello", [], { "x": 1 }])),
        "[1,true,\"hello\",[],{\"x\":1}]"
    );
}

#[test]
fn empty_schema_on_anyof() {
    let schema = json!({
        "anyOf": [
            {
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["Foo"] }, "value": {} }
            },
            {
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["Bar"] }, "value": { "type": "number" } }
            }
        ]
    });
    assert_eq!(
        run(schema.clone(), json!({ "kind": "Bar", "value": 1 })),
        "{\"kind\":\"Bar\",\"value\":1}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "kind": "Foo", "value": 1 })),
        "{\"kind\":\"Foo\",\"value\":1}"
    );
    assert_eq!(
        run(schema.clone(), json!({ "kind": "Foo", "value": true })),
        "{\"kind\":\"Foo\",\"value\":true}"
    );
    assert_eq!(
        run(schema, json!({ "kind": "Foo", "value": "hello" })),
        "{\"kind\":\"Foo\",\"value\":\"hello\"}"
    );
}

#[test]
fn anyof_no_branch_throws_at_root() {
    let schema = json!({
        "anyOf": [
            {
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["Foo"] }, "value": {} }
            },
            {
                "type": "object",
                "properties": { "kind": { "type": "string", "enum": ["Bar"] }, "value": { "type": "number" } }
            }
        ]
    });
    let stringify = build_ok(schema);
    let err = stringify
        .call(&Value::from(json!({ "kind": "Baz", "value": 1 })))
        .unwrap_err();
    assert_eq!(
        err.message(),
        "The value of '#' does not match schema definition."
    );
}

#[test]
fn anyof_no_branch_throws_at_property() {
    let schema = json!({
        "type": "object",
        "properties": {
            "data": {
                "anyOf": [
                    {
                        "type": "object",
                        "properties": { "kind": { "type": "string", "enum": ["Foo"] }, "value": {} }
                    },
                    {
                        "type": "object",
                        "properties": { "kind": { "type": "string", "enum": ["Bar"] }, "value": { "type": "number" } }
                    }
                ]
            }
        }
    });
    let stringify = build_ok(schema);
    let err = stringify
        .call(&Value::from(
            json!({ "data": { "kind": "Baz", "value": 1 } }),
        ))
        .unwrap_err();
    assert_eq!(
        err.message(),
        "The value of '#/properties/data' does not match schema definition."
    );
}
