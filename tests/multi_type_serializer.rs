//! The multi-type mismatch error carries the property path.

mod common;

use common::build_ok;
use fast_json_stringify::Value;
use serde_json::json;

#[test]
fn object_input_for_number_type_array_throws() {
    let schema = json!({
        "type": "object",
        "properties": { "num": { "type": ["number"] } }
    });
    let stringify = build_ok(schema);
    let err = stringify
        .call(&Value::from(json!({ "num": { "bla": 123 } })))
        .unwrap_err();
    assert_eq!(
        err.message(),
        "The value of '#/properties/num' does not match schema definition."
    );
}
