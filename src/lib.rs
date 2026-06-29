//! Compile a JSON Schema into a fast JSON serializer.
//!
//! [`build`] takes a Draft-7 JSON Schema and returns a [`Stringify`] closure that
//! turns a matching value into a JSON string. The work of inspecting the schema
//! happens once, at build time. Each call to the returned closure walks a
//! pre-compiled plan instead of the schema, so serialization stays cheap.
//!
//! This is not a validator. The output is correct JSON when the input matches
//! the schema. A mismatched input may produce malformed JSON or an error. Treat
//! schemas as trusted.
//!
//! # Value model
//!
//! Inputs use [`Value`], a JavaScript-shaped value type. A plain
//! [`serde_json::Value`] converts in with [`From`], so JSON inputs stay simple:
//!
//! ```
//! use fast_json_stringify::{build, Value};
//! use serde_json::json;
//!
//! let stringify = build(&json!({ "type": "string" }), None).unwrap();
//! assert_eq!(stringify(&Value::from(json!("hello"))).unwrap(), "\"hello\"");
//! ```
//!
//! The model adds the host objects the serializer coerces:
//! [`Value::Date`] (epoch milliseconds), [`Value::BigInt`], [`Value::Regex`], and
//! [`Value::Custom`] for a value already passed through a `toJSON` hook.
//!
//! # Objects and required fields
//!
//! ```
//! use fast_json_stringify::{build, Value};
//! use serde_json::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": { "name": { "type": "string" }, "age": { "type": "integer" } },
//!     "required": ["name"]
//! });
//! let stringify = build(&schema, None).unwrap();
//! let input = Value::from(json!({ "name": "Ada", "age": 36 }));
//! assert_eq!(stringify(&input).unwrap(), r#"{"name":"Ada","age":36}"#);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Compile the README examples as doc tests so they cannot drift.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
struct ReadmeDoctests;

mod engine;
mod error;
mod merge;
mod meta;
mod native;
mod number;
mod refresolver;
mod serializer;
mod validate_value;
mod value;

use std::collections::HashMap;
use std::sync::Arc;

pub use error::{BuildError, StringifyError};
pub use serializer::{Rounding, Serializer};
pub use value::{Object, Value};

use engine::Plan;

/// The valid `largeArrayMechanism` names, mirroring the source export.
pub const VALID_LARGE_ARRAY_MECHANISMS: [&str; 2] = ["default", "json-stringify"];

/// Default large-array threshold.
const DEFAULT_LARGE_ARRAY_SIZE: usize = 20_000;

/// The boxed closure type stored inside [`Stringify`].
type Callable = Arc<dyn Fn(&Value) -> Result<String, StringifyError> + Send + Sync>;

/// How large arrays serialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LargeArrayMechanism {
    /// Serialize every element through its plan. The default.
    #[default]
    Default,
    /// Short circuit to native JSON once the array reaches the threshold.
    JsonStringify,
}

/// Build options, mirroring the source `Options` object.
pub struct Options {
    /// External schemas keyed by name for `$ref` resolution.
    pub schema: HashMap<String, serde_json::Value>,
    /// Integer rounding for non-integer numbers under `type: "integer"`.
    pub rounding: Rounding,
    /// The large-array mechanism.
    pub large_array_mechanism: LargeArrayMechanism,
    /// The large-array threshold.
    pub large_array_size: usize,
}

impl Options {
    /// Build options with the documented defaults.
    pub fn new() -> Self {
        Options::default()
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            schema: HashMap::new(),
            rounding: Rounding::Trunc,
            large_array_mechanism: LargeArrayMechanism::Default,
            large_array_size: DEFAULT_LARGE_ARRAY_SIZE,
        }
    }
}

/// A compiled serializer. Call it with a [`Value`] to produce a JSON string.
///
/// [`Stringify`] derefs to a `Fn(&Value) -> Result<String, StringifyError>`, so
/// it can be called like a closure (`stringify(&value)`) as well as through
/// [`Stringify::call`].
#[derive(Clone)]
pub struct Stringify {
    plan: Arc<Plan>,
    callable: Callable,
}

impl Stringify {
    fn new(plan: Plan) -> Self {
        let plan = Arc::new(plan);
        let for_closure = plan.clone();
        let callable: Callable = Arc::new(move |value: &Value| for_closure.serialize(value));
        Stringify { plan, callable }
    }

    /// Serialize a value to a JSON string.
    pub fn call(&self, value: &Value) -> Result<String, StringifyError> {
        self.plan.serialize(value)
    }
}

impl std::ops::Deref for Stringify {
    type Target = dyn Fn(&Value) -> Result<String, StringifyError> + Send + Sync;
    fn deref(&self) -> &Self::Target {
        &*self.callable
    }
}

impl std::fmt::Debug for Stringify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stringify").finish_non_exhaustive()
    }
}

/// Compile a schema into a serializer.
///
/// Validates the schema against the supported Draft-7 structure, resolves
/// options, and returns a [`Stringify`]. Returns a [`BuildError`] for an invalid
/// schema, an unknown rounding method, or an unresolvable reference.
///
/// ```
/// use fast_json_stringify::{build, Value};
/// use serde_json::json;
///
/// let stringify = build(&json!({ "type": "integer" }), None).unwrap();
/// assert_eq!(stringify(&Value::from(json!(1615))).unwrap(), "1615");
/// ```
pub fn build(
    schema: &serde_json::Value,
    options: Option<Options>,
) -> Result<Stringify, BuildError> {
    let options = options.unwrap_or_default();

    let json_stringify_arrays = options.large_array_mechanism == LargeArrayMechanism::JsonStringify;
    let large_array_size = options.large_array_size;

    let plan = engine::compile(
        schema,
        &options.schema,
        options.rounding,
        large_array_size,
        json_stringify_arrays,
    )?;

    Ok(Stringify::new(plan))
}
