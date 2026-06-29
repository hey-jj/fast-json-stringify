//! if/then/else branch selection via the validator.

mod common;

use common::{build_err, build_ok, build_ok_opts, run};
use fast_json_stringify::{Object, Options, Value};
use serde_json::json;

fn base_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {},
        "if": { "type": "object", "properties": { "kind": { "type": "string", "enum": ["foobar"] } } },
        "then": {
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["foobar"] },
                "foo": { "type": "string" },
                "bar": { "type": "number" },
                "list": {
                    "type": "array",
                    "items": { "type": "object", "properties": { "name": { "type": "string" }, "value": { "type": "string" } } }
                }
            }
        },
        "else": {
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["greeting"] },
                "hi": { "type": "string" },
                "hello": { "type": "number" },
                "list": {
                    "type": "array",
                    "items": { "type": "object", "properties": { "name": { "type": "string" }, "value": { "type": "string" } } }
                }
            }
        }
    })
}

#[test]
fn then_branch() {
    let input = json!({
        "kind": "foobar", "foo": "FOO",
        "list": [{ "name": "name", "value": "foo" }],
        "bar": 42, "hi": "HI", "hello": 45
    });
    assert_eq!(
        run(base_schema(), input),
        "{\"kind\":\"foobar\",\"foo\":\"FOO\",\"bar\":42,\"list\":[{\"name\":\"name\",\"value\":\"foo\"}]}"
    );
}

#[test]
fn else_branch() {
    let input = json!({ "kind": "greeting", "foo": "FOO", "bar": 42, "hi": "HI", "hello": 45 });
    assert_eq!(
        run(base_schema(), input),
        "{\"kind\":\"greeting\",\"hi\":\"HI\",\"hello\":45}"
    );
}

#[test]
fn nested_if_then() {
    let schema = json!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "if": { "type": "object", "properties": { "foo": { "type": "string" } } },
        "then": {
            "properties": { "bar": { "type": "string" } },
            "if": { "type": "object", "properties": { "foo1": { "type": "string" } } },
            "then": { "properties": { "bar1": { "type": "string" } } }
        }
    });
    assert_eq!(
        run(
            schema.clone(),
            json!({ "a": "A", "foo": "foo", "bar": "bar" })
        ),
        "{\"a\":\"A\",\"bar\":\"bar\"}"
    );
    assert_eq!(
        run(
            schema,
            json!({ "a": "A", "foo": "foo", "bar": "bar", "foo1": "foo1", "bar1": "bar1" })
        ),
        "{\"a\":\"A\",\"bar\":\"bar\",\"bar1\":\"bar1\"}"
    );
}

#[test]
fn if_else_with_string_format() {
    let schema = json!({
        "if": { "type": "string" },
        "then": { "type": "string", "format": "date" },
        "else": { "const": "Invalid" }
    });
    let stringify = build_ok(schema);
    assert_eq!(
        stringify.call(&Value::Date(1674263005800)).unwrap(),
        "\"2023-01-21\""
    );
    assert_eq!(
        stringify.call(&Value::String("Invalid".into())).unwrap(),
        "\"Invalid\""
    );
}

#[test]
fn if_else_with_const_integers() {
    let schema = json!({
        "type": "number",
        "if": { "type": "number", "minimum": 42 },
        "then": { "const": 66 },
        "else": { "const": 33 }
    });
    let stringify = build_ok(schema);
    assert_eq!(stringify.call(&Value::Number(100.32)).unwrap(), "66");
    assert_eq!(stringify.call(&Value::Number(10.12)).unwrap(), "33");
}

#[test]
fn if_else_with_array() {
    let schema = json!({
        "type": "array",
        "if": { "type": "array", "maxItems": 1 },
        "then": { "items": { "type": "string" } },
        "else": { "items": { "type": "number" } }
    });
    assert_eq!(run(schema.clone(), json!(["1"])), "[\"1\"]");
    assert_eq!(run(schema, json!(["1", "2"])), "[1,2]");
}

#[test]
fn external_recursive_if_then_else() {
    let external = json!({
        "externalSchema": {
            "type": "object",
            "properties": { "base": { "type": "string" }, "self": { "$ref": "externalSchema#" } },
            "if": { "type": "object", "properties": { "foo": { "type": "string", "const": "41" } } },
            "then": { "type": "object", "properties": { "bar": { "type": "string", "const": "42" } } },
            "else": { "type": "object", "properties": { "baz": { "type": "string", "const": "43" } } }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "$ref": "externalSchema#/properties/self" },
            "b": { "$ref": "externalSchema#/properties/self" }
        }
    });
    let opts = Options {
        schema: external
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        ..Options::new()
    };
    let stringify = build_ok_opts(schema, opts);
    let data = json!({
        "a": { "base": "a", "foo": "41", "bar": "42", "baz": "43", "ignore": "ignored" },
        "b": { "base": "b", "foo": "not-41", "bar": "42", "baz": "43", "ignore": "ignored" }
    });
    assert_eq!(
        stringify.call(&Value::from(data)).unwrap(),
        "{\"a\":{\"base\":\"a\",\"bar\":\"42\"},\"b\":{\"base\":\"b\",\"baz\":\"43\"}}"
    );
}

#[test]
fn invalid_if_then_else_schema() {
    let schema = json!({
        "type": "object",
        "if": { "type": "object", "properties": { "kind": { "type": "string", "enum": ["foobar"] } } },
        "then": { "type": "object", "properties": { "foo": { "type": "string" } } },
        "else": { "type": "object", "properties": "invalid" }
    });
    let err = build_err(schema);
    assert!(err.contains("schema is invalid"));
}

#[test]
fn if_then_else_with_allof() {
    let schema = json!({
        "type": "object",
        "allOf": [{ "type": "object", "properties": { "base": { "type": "string" } } }],
        "if": { "type": "object", "properties": { "kind": { "type": "string", "enum": ["foobar"] } } },
        "then": { "type": "object", "properties": { "foo": { "type": "string" } } },
        "else": { "type": "object", "properties": { "bar": { "type": "string" } } }
    });
    let stringify = build_ok(schema);
    let mut obj = Object::new();
    obj.insert("base", Value::String("test".into()));
    obj.insert("kind", Value::String("foobar".into()));
    obj.insert("foo", Value::String("value".into()));
    let out = stringify.call(&Value::Object(obj)).unwrap();
    assert_eq!(out, "{\"base\":\"test\",\"foo\":\"value\"}");
}

#[test]
fn no_else_falls_to_root() {
    let schema = json!({
        "type": "object",
        "properties": {},
        "if": { "type": "object", "properties": { "kind": { "type": "string", "enum": ["foobar"] } } },
        "then": {
            "type": "object",
            "properties": { "kind": { "type": "string", "enum": ["foobar"] }, "foo": { "type": "string" } }
        }
    });
    let input = json!({ "kind": "greeting", "foo": "FOO" });
    assert_eq!(run(schema, input), "{}");
}
