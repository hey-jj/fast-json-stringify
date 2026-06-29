//! `$ref` resolution across the major scenarios.

mod common;

use common::{build_ok, build_ok_opts, run};
use fast_json_stringify::{Options, Value};
use serde_json::json;

fn ext_opts(external: serde_json::Value) -> Options {
    Options {
        schema: external
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        ..Options::new()
    }
}

#[test]
fn internal_ref_properties() {
    let schema = json!({
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } },
        "type": "object",
        "properties": { "obj": { "$ref": "#/definitions/def" } }
    });
    assert_eq!(
        run(schema, json!({ "obj": { "str": "test" } })),
        "{\"obj\":{\"str\":\"test\"}}"
    );
}

#[test]
fn internal_ref_items() {
    let schema = json!({
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } },
        "type": "array",
        "items": { "$ref": "#/definitions/def" }
    });
    assert_eq!(
        run(schema, json!([{ "str": "test" }])),
        "[{\"str\":\"test\"}]"
    );
}

#[test]
fn internal_ref_pattern_properties() {
    let schema = json!({
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } },
        "type": "object",
        "properties": {},
        "patternProperties": { "obj": { "$ref": "#/definitions/def" } }
    });
    assert_eq!(
        run(schema, json!({ "obj": { "str": "test" } })),
        "{\"obj\":{\"str\":\"test\"}}"
    );
}

#[test]
fn internal_ref_additional_properties() {
    let schema = json!({
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } },
        "type": "object",
        "properties": {},
        "additionalProperties": { "$ref": "#/definitions/def" }
    });
    assert_eq!(
        run(schema, json!({ "obj": { "str": "test" } })),
        "{\"obj\":{\"str\":\"test\"}}"
    );
}

#[test]
fn external_ref_properties() {
    let external = json!({
        "first": { "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } } },
        "second": { "definitions": { "num": { "type": "object", "properties": { "int": { "type": "integer" } } } } },
        "third": { "type": "string" }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "obj": { "$ref": "first#/definitions/def" },
            "num": { "$ref": "second#/definitions/num" },
            "strPlain": { "$ref": "third" },
            "strHash": { "$ref": "third#" }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let input = json!({ "obj": { "str": "test" }, "num": { "int": 42 }, "strPlain": "test", "strHash": "test" });
    assert_eq!(
        stringify.call(&Value::from(input)).unwrap(),
        "{\"obj\":{\"str\":\"test\"},\"num\":{\"int\":42},\"strPlain\":\"test\",\"strHash\":\"test\"}"
    );
}

#[test]
fn internal_plain_name_fragment() {
    let schema = json!({
        "definitions": {
            "def": {
                "$id": "#uri",
                "type": "object",
                "properties": { "str": { "type": "string" } },
                "required": ["str"]
            }
        },
        "type": "object",
        "properties": { "obj": { "$ref": "#uri" } }
    });
    assert_eq!(
        run(schema, json!({ "obj": { "str": "test" } })),
        "{\"obj\":{\"str\":\"test\"}}"
    );
}

#[test]
fn external_plain_name_fragment() {
    let external = json!({
        "first": { "$id": "#first-schema", "type": "object", "properties": { "str": { "type": "string" } } },
        "second": {
            "definitions": {
                "second": { "$id": "#second-schema", "type": "object", "properties": { "int": { "type": "integer" } } }
            }
        }
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "first": { "$ref": "first#first-schema" },
            "second": { "$ref": "second#second-schema" }
        }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    let input = json!({ "first": { "str": "test" }, "second": { "int": 42 } });
    assert_eq!(
        stringify.call(&Value::from(input)).unwrap(),
        "{\"first\":{\"str\":\"test\"},\"second\":{\"int\":42}}"
    );
}

#[test]
fn ref_in_root_internal() {
    let schema = json!({
        "$ref": "#/definitions/num",
        "definitions": {
            "num": { "type": "object", "properties": { "int": { "$ref": "#/definitions/int" } } },
            "int": { "type": "integer" }
        }
    });
    assert_eq!(run(schema, json!({ "int": 42 })), "{\"int\":42}");
}

#[test]
fn ref_in_root_external() {
    let external = json!({
        "numbers": {
            "$id": "numbers",
            "definitions": { "num": { "type": "object", "properties": { "int": { "type": "integer" } } } }
        }
    });
    let schema = json!({ "type": "object", "$ref": "numbers#/definitions/num" });
    let stringify = build_ok_opts(schema, ext_opts(external));
    assert_eq!(
        stringify.call(&Value::from(json!({ "int": 42 }))).unwrap(),
        "{\"int\":42}"
    );
}

#[test]
fn ref_in_root_external_multiple_times() {
    let external = json!({
        "numbers": { "$id": "numbers", "$ref": "subnumbers#/definitions/num" },
        "subnumbers": {
            "$id": "subnumbers",
            "definitions": { "num": { "type": "object", "properties": { "int": { "type": "integer" } } } }
        }
    });
    let schema = json!({ "type": "object", "$ref": "numbers" });
    let stringify = build_ok_opts(schema, ext_opts(external));
    assert_eq!(
        stringify.call(&Value::from(json!({ "int": 42 }))).unwrap(),
        "{\"int\":42}"
    );
}

#[test]
fn ref_external_relative_definition() {
    let external = json!({
        "relative:to:local": {
            "$id": "relative:to:local",
            "type": "object",
            "properties": { "foo": { "$ref": "#/definitions/foo" } },
            "definitions": { "foo": { "type": "string" } }
        }
    });
    let schema = json!({
        "type": "object",
        "required": ["fooParent"],
        "properties": { "fooParent": { "$ref": "relative:to:local" } }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "fooParent": { "foo": "bar" } })))
            .unwrap(),
        "{\"fooParent\":{\"foo\":\"bar\"}}"
    );
}

#[test]
fn ref_to_nested_ref_definition() {
    let external = json!({
        "a:b:c1": {
            "$id": "a:b:c1",
            "type": "object",
            "definitions": { "foo": { "$ref": "a:b:c2#/definitions/foo" } }
        },
        "a:b:c2": { "$id": "a:b:c2", "type": "object", "definitions": { "foo": { "type": "string" } } }
    });
    let schema = json!({
        "type": "object",
        "required": ["foo"],
        "properties": { "foo": { "$ref": "a:b:c1#/definitions/foo" } }
    });
    let stringify = build_ok_opts(schema, ext_opts(external));
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "foo": "foo" })))
            .unwrap(),
        "{\"foo\":\"foo\"}"
    );
}

#[test]
fn ref_reused_multiple_times() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "$ref": "#/definitions/def" },
            "b": { "$ref": "#/definitions/def" }
        },
        "definitions": { "def": { "type": "object", "properties": { "str": { "type": "string" } } } }
    });
    assert_eq!(
        run(schema, json!({ "a": { "str": "x" }, "b": { "str": "y" } })),
        "{\"a\":{\"str\":\"x\"},\"b\":{\"str\":\"y\"}}"
    );
}

#[test]
fn input_schema_is_not_mutated() {
    let schema = json!({
        "type": "object",
        "definitions": { "def": { "type": "string" } },
        "properties": { "obj": { "$ref": "#/definitions/def" } }
    });
    let cloned = schema.clone();
    let stringify = build_ok(schema.clone());
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "obj": "test" })))
            .unwrap(),
        "{\"obj\":\"test\"}"
    );
    // The builder works on owned data, so the caller's schema is unchanged.
    assert_eq!(schema, cloned);
}

#[test]
fn ref_to_fixture_file() {
    // The external schema loaded from a JSON fixture, copied verbatim.
    let external: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/ref.json")).unwrap();
    let schema = json!({
        "type": "object",
        "properties": { "obj": { "$ref": "first#/definitions/def" } }
    });
    let stringify = build_ok_opts(schema, ext_opts(json!({ "first": external })));
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "obj": { "str": "test" } })))
            .unwrap(),
        "{\"obj\":{\"str\":\"test\"}}"
    );
}
