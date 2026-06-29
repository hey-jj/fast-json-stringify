//! JSON Schema Test Suite conformance for the `required` keyword.
//!
//! Vendored draft4, draft6, and draft7 `required.json` fixtures drive the same
//! check the source harness runs: build the scenario schema, serialize each
//! test datum, and compare to native JSON. A datum that throws must be marked
//! invalid in the fixture.

mod common;

use common::{build_ok, js_stringify};
use fast_json_stringify::Value;
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Deserialize)]
struct Scenario {
    schema: Json,
    tests: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    description: String,
    data: Json,
    valid: bool,
}

/// Skipped descriptions, matching the source harness.
const SKIP: [&str; 3] = [
    "ignores arrays",
    "ignores strings",
    "ignores other non-objects",
];

fn run_suite(source: &str) {
    let scenarios: Vec<Scenario> = serde_json::from_str(source).unwrap();
    for scenario in scenarios {
        let stringify = build_ok(scenario.schema);
        for case in scenario.tests {
            if SKIP.contains(&case.description.as_str()) {
                continue;
            }
            match stringify.call(&Value::from(case.data.clone())) {
                Ok(out) => assert_eq!(out, js_stringify(&case.data), "{}", case.description),
                Err(_) => assert!(!case.valid, "{}", case.description),
            }
        }
    }
}

#[test]
fn draft4_required() {
    run_suite(include_str!("fixtures/jsts/draft4_required.json"));
}

#[test]
fn draft6_required() {
    run_suite(include_str!("fixtures/jsts/draft6_required.json"));
}

#[test]
fn draft7_required() {
    run_suite(include_str!("fixtures/jsts/draft7_required.json"));
}
