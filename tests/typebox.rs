//! Interop with a TypeBox-generated schema, ported as a plain schema.
//!
//! TypeBox `Record(String, T)` compiles to an object with a `.*`
//! patternProperties entry. The schema literal below is what TypeBox would emit.

mod common;

use common::run;
use serde_json::json;

#[test]
fn nested_object_in_pattern_properties() {
    let nested = json!({
        "type": "object",
        "properties": { "nestedKey1": { "type": "string" } },
        "required": ["nestedKey1"]
    });
    let schema = json!({
        "type": "object",
        "properties": {
            "key1": { "type": "object", "patternProperties": { "^.*$": nested } },
            "key2": { "type": "object", "patternProperties": { "^.*$": nested } }
        },
        "required": ["key1", "key2"]
    });
    let input = json!({
        "key1": { "nestedKey": { "nestedKey1": "value1" } },
        "key2": { "nestedKey": { "nestedKey1": "value2" } }
    });
    assert_eq!(
        run(schema, input),
        "{\"key1\":{\"nestedKey\":{\"nestedKey1\":\"value1\"}},\"key2\":{\"nestedKey\":{\"nestedKey1\":\"value2\"}}}"
    );
}
