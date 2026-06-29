//! anyOf is validated after an allOf merge.

mod common;

use common::run;
use serde_json::json;

#[test]
fn anyof_after_allof_merge() {
    let schema = json!({
        "$id": "schema",
        "type": "object",
        "allOf": [
            {
                "$id": "base",
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            },
            {
                "$id": "inner_schema",
                "type": "object",
                "properties": {
                    "union": {
                        "$id": "#id",
                        "anyOf": [
                            { "$id": "guid", "type": "string" },
                            { "$id": "email", "type": "string" }
                        ]
                    }
                },
                "required": ["union"]
            }
        ]
    });
    assert_eq!(
        run(
            schema,
            json!({ "name": "foo", "union": "a8f1cc50-5530-5c62-9109-5ba9589a6ae1" })
        ),
        "{\"name\":\"foo\",\"union\":\"a8f1cc50-5530-5c62-9109-5ba9589a6ae1\"}"
    );
}
