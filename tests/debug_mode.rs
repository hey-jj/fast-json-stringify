//! Debug and standalone modes have no codegen analog here.
//!
//! The source `mode: 'debug'` returns a code string plus validator and
//! serializer that `build.restore` re-runs into an identical stringify. The
//! `standalone` mode emits a source file. This crate compiles to an in-memory
//! plan, so there is no code string to round-trip. The observable contract is
//! the same output, which these cases lock: a cloned or rebuilt serializer
//! produces identical bytes, and rounding survives a rebuild.

mod common;

use common::{build_ok, build_ok_opts};
use fast_json_stringify::{Options, Rounding, Value};
use serde_json::json;

#[test]
fn clone_produces_identical_output() {
    let schema = json!({
        "type": "object",
        "properties": { "name": { "type": "string" }, "age": { "type": "integer" } }
    });
    let stringify = build_ok(schema);
    let restored = stringify.clone();
    let input = Value::from(json!({ "name": "Ada", "age": 36 }));
    assert_eq!(
        stringify.call(&input).unwrap(),
        restored.call(&input).unwrap()
    );
}

#[test]
fn rebuild_produces_identical_output() {
    let schema = json!({ "type": "object", "properties": { "magic": { "type": "integer" } } });
    let input = Value::from(json!({ "magic": 4.5 }));

    let a = build_ok_opts(
        schema.clone(),
        Options {
            rounding: Rounding::Ceil,
            ..Options::new()
        },
    );
    let b = build_ok_opts(
        schema,
        Options {
            rounding: Rounding::Ceil,
            ..Options::new()
        },
    );
    assert_eq!(a.call(&input).unwrap(), b.call(&input).unwrap());
    // Rounding is preserved through the rebuild.
    assert_eq!(a.call(&input).unwrap(), "{\"magic\":5}");
}

#[test]
fn standalone_io_matches_normal() {
    // The if/then/else and external-ref schemas from the standalone suite must
    // produce the same output as a normal build.
    let schema = json!({
        "type": "object",
        "if": { "type": "object", "properties": { "kind": { "type": "string", "enum": ["a"] } } },
        "then": { "type": "object", "properties": { "kind": { "type": "string" }, "x": { "type": "string" } } },
        "else": { "type": "object", "properties": { "kind": { "type": "string" }, "y": { "type": "string" } } }
    });
    let stringify = build_ok(schema);
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "kind": "a", "x": "hi" })))
            .unwrap(),
        "{\"kind\":\"a\",\"x\":\"hi\"}"
    );
    assert_eq!(
        stringify
            .call(&Value::from(json!({ "kind": "b", "y": "yo" })))
            .unwrap(),
        "{\"kind\":\"b\",\"y\":\"yo\"}"
    );
}
