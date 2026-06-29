# fast-json-stringify

Compile a JSON Schema into a fast JSON serializer.

`build` reads a Draft-7 JSON Schema once and returns a closure that turns a
matching value into a JSON string. The schema is inspected at build time, so each
serialize call walks a pre-compiled plan instead of the schema. Output matches
`JSON.stringify` byte for byte for conforming input.

This is not a validator. The output is correct JSON when the input matches the
schema. A mismatched input may produce malformed JSON or an error. Treat schemas
as trusted.

## Installation

```toml
[dependencies]
fast-json-stringify = "0.1"
```

## Usage

```rust
use fast_json_stringify::{build, Value};
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "name": { "type": "string" },
        "age": { "type": "integer" }
    },
    "required": ["name"]
});

let stringify = build(&schema, None).unwrap();
let input = Value::from(json!({ "name": "Ada", "age": 36 }));
assert_eq!(stringify.call(&input).unwrap(), r#"{"name":"Ada","age":36}"#);
```

`Stringify` also derefs to a `Fn`, so it can be called like a closure:

```rust
# use fast_json_stringify::{build, Value};
# use serde_json::json;
let stringify = build(&json!({ "type": "integer" }), None).unwrap();
assert_eq!(stringify(&Value::from(json!(1615))).unwrap(), "1615");
```

## Value model

Inputs use `Value`, a JavaScript-shaped value type. A plain `serde_json::Value`
converts in with `From`. The model adds the host objects the serializer coerces:

- `Value::Date(i64)` holds epoch milliseconds and renders as ISO 8601, or as a
  calendar date or wall-clock time under `format: "date"` and `format: "time"`.
- `Value::BigInt(i128)` serializes as an arbitrary-precision integer.
- `Value::Regex(String)` serializes as its source pattern under a string schema.
- `Value::Custom(Box<Value>)` carries a value already passed through a `toJSON`
  hook.

## Supported keywords

Serialization honors `type` (single or array), `properties`, `required`,
`additionalProperties`, `patternProperties`, `items` (schema or tuple),
`additionalItems`, `oneOf`, `anyOf`, `allOf`, `if`/`then`/`else`, `$ref`, `$id`,
`definitions`, `const`, `default`, `nullable`, and `format`
(`date-time`/`date`/`time`/`unsafe`). Other validation keywords such as `enum`,
`minLength`, and `maximum` drive type inference or are ignored. They are not
enforced.

`anyOf`, `oneOf`, and `if`/`then`/`else` pick a branch by validating the value at
serialize time, then serialize through the matching schema.

## Options

`Options` controls rounding for non-integer numbers under `type: "integer"`,
external schemas for `$ref` resolution, and the large-array threshold and
mechanism. The defaults are `trunc` rounding, a `20000` threshold, and the
element-by-element mechanism.

## License

Licensed under the [MIT license](LICENSE).
