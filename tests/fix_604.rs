//! A non-numeric value under a number schema raises the conversion error.

mod common;

use common::run_err;
use serde_json::json;

#[test]
fn non_numeric_for_number_throws() {
    let schema = json!({
        "type": "object",
        "properties": { "fullName": { "type": "string" }, "phone": { "type": "number" } }
    });
    let err = run_err(schema, json!({ "fullName": "Jone", "phone": "phone" }));
    assert_eq!(err, "The value \"phone\" cannot be converted to a number.");
}
