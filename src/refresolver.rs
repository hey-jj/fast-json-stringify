//! `$ref` resolution and the external dependency graph.
//!
//! A schema document can carry an `$id`, embed nested `$id` scopes, and address
//! subschemas by JSON Pointer. This resolver indexes each document by id, walks
//! pointers per RFC 6901, and tracks which other documents a schema reaches so
//! the validator can be primed with them.

use serde_json::Value;
use std::collections::BTreeMap;

/// Indexes schema documents and resolves `(schemaId, jsonPointer)` lookups.
#[derive(Debug, Default, Clone)]
pub struct RefResolver {
    /// schema id to the root document value.
    schemas: BTreeMap<String, Value>,
    /// schema id to a map of `$id`/anchor scopes found inside that document.
    anchors: BTreeMap<String, BTreeMap<String, Value>>,
}

impl RefResolver {
    /// Create an empty resolver.
    pub fn new() -> Self {
        RefResolver::default()
    }

    /// True when a document is registered under `schema_id`.
    pub fn has_schema(&self, schema_id: &str) -> bool {
        self.schemas.contains_key(schema_id)
    }

    /// Register a document under `schema_id` and index every embedded `$id`
    /// scope so anchor refs resolve later.
    pub fn add_schema(&mut self, schema: Value, schema_id: &str) {
        let mut scopes = BTreeMap::new();
        collect_anchors(&schema, schema_id, &mut scopes);
        self.anchors.insert(schema_id.to_string(), scopes);
        self.schemas.insert(schema_id.to_string(), schema);
    }

    /// Resolve a schema by id and JSON Pointer. Returns `None` when either the
    /// document or the pointer target is missing.
    pub fn get_schema(&self, schema_id: &str, json_pointer: &str) -> Option<&Value> {
        let doc = self.schemas.get(schema_id)?;

        if json_pointer == "#" || json_pointer.is_empty() {
            return Some(doc);
        }

        // Anchor form: `#name` where name is not a pointer path.
        if let Some(anchor) = json_pointer.strip_prefix('#') {
            if !anchor.starts_with('/') && !anchor.is_empty() {
                if let Some(scope) = self.anchors.get(schema_id) {
                    if let Some(found) = scope.get(json_pointer) {
                        return Some(found);
                    }
                }
            }
        }

        resolve_pointer(doc, json_pointer)
    }
}

/// Walk every node and record nested `$id` scopes keyed by the absolute anchor
/// reference (`baseId#anchor` for relative ids, or the id itself).
fn collect_anchors(schema: &Value, base_id: &str, out: &mut BTreeMap<String, Value>) {
    match schema {
        Value::Object(map) => {
            let mut scope_id = base_id.to_string();
            if let Some(Value::String(id)) = map.get("$id") {
                if id.starts_with('#') {
                    // Relative anchor: addressable as base + anchor.
                    out.insert(id.clone(), schema.clone());
                } else {
                    scope_id = id.clone();
                    out.insert(id.clone(), schema.clone());
                }
            }
            for (_, v) in map {
                collect_anchors(v, &scope_id, out);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect_anchors(v, base_id, out);
            }
        }
        _ => {}
    }
}

/// Resolve a JSON Pointer fragment (`#/a/b`) against a document per RFC 6901.
fn resolve_pointer<'a>(doc: &'a Value, json_pointer: &str) -> Option<&'a Value> {
    let pointer = json_pointer.strip_prefix('#').unwrap_or(json_pointer);
    if pointer.is_empty() {
        return Some(doc);
    }
    let mut current = doc;
    for raw in pointer.split('/').skip(1) {
        let token = unescape_token(raw);
        current = match current {
            Value::Object(map) => map.get(&token)?,
            Value::Array(items) => {
                let idx: usize = token.parse().ok()?;
                items.get(idx)?
            }
            _ => return None,
        };
    }
    Some(current)
}

/// Undo RFC 6901 token escaping (`~1` to `/`, `~0` to `~`).
fn unescape_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}
