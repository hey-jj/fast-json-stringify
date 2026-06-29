//! Schema compilation and value serialization.
//!
//! [`compile`] turns a schema into a plan: an arena of [`Node`]s linked by
//! [`NodeId`]. Recursion is handled by reusing a node id when the same schema
//! reference reappears on the build path. [`serialize`] walks the plan against a
//! value and emits JSON. The dispatch order, coercions, and error messages track
//! the source so output matches byte for byte.

use crate::error::{BuildError, StringifyError};
use crate::merge::merge_schemas;
use crate::meta;
use crate::refresolver::RefResolver;
use crate::serializer::{Rounding, Serializer};
use crate::validate_value::validate as validate_branch;
use crate::value::Value;
use serde_json::Value as Json;
use std::collections::HashMap;

/// Index into the node arena.
pub type NodeId = usize;

/// A compiled serialization step.
#[derive(Debug, Clone)]
pub enum Node {
    /// Emit the JSON literal `null`.
    Null,
    /// Emit a boolean from truthiness.
    Boolean,
    /// Emit an integer with rounding.
    Integer,
    /// Emit a number.
    Number,
    /// Emit a plain string with coercion of non-strings.
    StringPlain,
    /// Emit a string in a date/time format.
    StringFormat(StringFormat),
    /// Emit a string with no escaping.
    StringUnsafe,
    /// Emit any value through native JSON serialization.
    AnyJson,
    /// Emit a constant, pre-rendered. Carries whether a null type alternative
    /// exists so a null input renders as `null`.
    Const {
        rendered: String,
        null_alternative: bool,
    },
    /// Emit an object.
    Object(ObjectNode),
    /// Emit an array.
    Array(ArrayNode),
    /// Choose among types by the runtime value kind.
    MultiType {
        branches: Vec<(TypeName, NodeId)>,
        reference: String,
    },
    /// Choose a branch with the validator (anyOf/oneOf).
    OneOf {
        options: Vec<BranchOption>,
        reference: String,
    },
    /// Choose then or else with the validator.
    IfThenElse {
        if_schema: Json,
        base_id: String,
        then: NodeId,
        els: NodeId,
    },
    /// Wrap a child so a null input renders as `null`.
    Nullable(NodeId),
}

/// A `type`-array branch with the validator schema used to select it.
#[derive(Debug, Clone)]
pub struct BranchOption {
    /// The schema the validator tests against.
    pub schema: Json,
    /// The base document id for ref resolution inside the branch schema.
    pub base_id: String,
    /// The plan to run when this branch is selected.
    pub node: NodeId,
}

/// Date/time string formats.
#[derive(Debug, Clone, Copy)]
pub enum StringFormat {
    /// `date-time`.
    DateTime,
    /// `date`.
    Date,
    /// `time`.
    Time,
}

/// A type name used by multi-type dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeName {
    /// `null`.
    Null,
    /// `boolean`.
    Boolean,
    /// `integer`.
    Integer,
    /// `number`.
    Number,
    /// `string`.
    String,
    /// `object`.
    Object,
    /// `array`.
    Array,
}

/// A compiled object schema.
#[derive(Debug, Clone)]
pub struct ObjectNode {
    /// Whether `null` renders as `null` instead of `{}`.
    pub nullable: bool,
    /// Declared properties in serialization order (required first).
    pub properties: Vec<PropertyPlan>,
    /// Required keys not present in `properties`.
    pub required_extra: Vec<String>,
    /// Pattern property matchers.
    pub pattern_properties: Vec<PatternPlan>,
    /// Additional property handling.
    pub additional: AdditionalProperties,
}

/// A single declared property.
#[derive(Debug, Clone)]
pub struct PropertyPlan {
    /// The property key.
    pub key: String,
    /// True when required.
    pub required: bool,
    /// Pre-rendered default, if any.
    pub default: Option<String>,
    /// The plan for the property value.
    pub node: NodeId,
}

/// A pattern property matcher.
#[derive(Debug, Clone)]
pub struct PatternPlan {
    /// Compiled regex over keys.
    pub regex: regex::Regex,
    /// The plan for matching values.
    pub node: NodeId,
}

/// How additional properties serialize.
#[derive(Debug, Clone)]
pub enum AdditionalProperties {
    /// Drop unknown keys.
    None,
    /// Emit unknown keys via native JSON.
    True,
    /// Emit unknown keys via a schema plan.
    Schema(NodeId),
}

/// A compiled array schema.
#[derive(Debug, Clone)]
pub struct ArrayNode {
    /// Whether `null` renders as `null` instead of `[]`.
    pub nullable: bool,
    /// The reference used in error messages.
    pub reference: String,
    /// Item handling: homogeneous or tuple.
    pub items: ArrayItems,
    /// Large-array short circuit threshold, when the json-stringify mechanism is on.
    pub large_array: Option<usize>,
}

/// Array item shape.
#[derive(Debug, Clone)]
pub enum ArrayItems {
    /// Every item uses the same plan.
    Homogeneous(NodeId),
    /// Positional items plus an optional additional-items flag.
    Tuple {
        items: Vec<TupleItem>,
        additional_items: bool,
    },
}

/// One tuple position.
#[derive(Debug, Clone)]
pub struct TupleItem {
    /// The declared type for the per-item runtime check.
    pub type_check: Option<Json>,
    /// The plan for the item value.
    pub node: NodeId,
}

/// The compiled plan: an arena plus the root and serializer config.
#[derive(Debug)]
pub struct Plan {
    nodes: Vec<Node>,
    root: NodeId,
    serializer: Serializer,
    resolver: RefResolver,
}

impl Plan {
    /// Serialize a value to JSON.
    pub fn serialize(&self, value: &Value) -> Result<String, StringifyError> {
        let mut out = String::new();
        self.emit(self.root, value, &mut out)?;
        Ok(out)
    }
}

/// A cursor into a schema document during compilation.
#[derive(Clone)]
struct Location {
    schema: Json,
    schema_id: String,
    json_pointer: String,
}

impl Location {
    fn new(schema: Json, schema_id: String) -> Self {
        Location {
            schema,
            schema_id,
            json_pointer: "#".to_string(),
        }
    }

    fn property(&self, name: &str) -> Location {
        let child = self.schema.get(name).cloned().unwrap_or(Json::Null);
        Location {
            schema: child,
            schema_id: self.schema_id.clone(),
            json_pointer: format!("{}/{}", self.json_pointer, name),
        }
    }

    fn index(&self, i: usize) -> Location {
        let child = self.schema.get(i).cloned().unwrap_or(Json::Null);
        Location {
            schema: child,
            schema_id: self.schema_id.clone(),
            json_pointer: format!("{}/{}", self.json_pointer, i),
        }
    }

    fn schema_ref(&self) -> String {
        format!("{}{}", self.schema_id, self.json_pointer)
    }
}

/// Compilation state.
struct Compiler {
    nodes: Vec<Node>,
    resolver: RefResolver,
    root_schema_id: String,
    merged_counter: usize,
    /// Memo from a fully-resolved reference to its node, for recursion.
    memo: HashMap<String, NodeId>,
    /// References currently being built, to break cycles.
    building: Vec<String>,
    /// Map a merged schema's canonical content to the id it registered under, so
    /// a recurring merge reuses one node and recursion terminates.
    merged_by_content: HashMap<String, String>,
}

impl Compiler {
    fn push(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Reserve a slot so recursive references can point at it before the body
    /// is built.
    fn reserve(&mut self) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node::Null);
        id
    }

    fn set(&mut self, id: NodeId, node: Node) {
        self.nodes[id] = node;
    }

    /// Strip the root id prefix from a reference so error messages read as the
    /// source produces them (`#/properties/x`).
    fn safe_ref(&self, location: &Location) -> String {
        let full = location.schema_ref();
        if let Some(stripped) = full.strip_prefix(&self.root_schema_id) {
            if stripped.is_empty() {
                "#".to_string()
            } else {
                stripped.to_string()
            }
        } else {
            full
        }
    }
}

/// Compile a schema into a plan. Validates the schema and any external schemas,
/// then resolves the rounding option.
pub fn compile(
    schema: &Json,
    external: &HashMap<String, Json>,
    rounding: Rounding,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<Plan, BuildError> {
    meta::validate(schema, None)?;

    let mut resolver = RefResolver::new();
    let root_schema_id = match schema.get("$id").and_then(Json::as_str) {
        Some(id) => id.to_string(),
        None => "__fjs_root_0".to_string(),
    };

    let registered_id = schema_id_for(schema, &root_schema_id);
    if !resolver.has_schema(&registered_id) {
        resolver.add_schema(schema.clone(), &root_schema_id);
    }

    for (key, ext) in external {
        let ext_id = schema_id_for(ext, key);
        if !resolver.has_schema(&ext_id) {
            meta::validate(ext, Some(key))?;
            resolver.add_schema(ext.clone(), key);
        }
    }

    let mut compiler = Compiler {
        nodes: Vec::new(),
        resolver,
        root_schema_id: root_schema_id.clone(),
        merged_counter: 0,
        memo: HashMap::new(),
        building: Vec::new(),
        merged_by_content: HashMap::new(),
    };

    let location = Location::new(schema.clone(), root_schema_id);
    let root = build_value(
        &mut compiler,
        location,
        large_array_size,
        json_stringify_arrays,
    )?;

    Ok(Plan {
        nodes: compiler.nodes,
        root,
        serializer: Serializer::new(rounding),
        resolver: compiler.resolver,
    })
}

/// Pick the id a schema registers under: its absolute `$id` or the fallback.
fn schema_id_for(schema: &Json, fallback: &str) -> String {
    if let Some(id) = schema.get("$id").and_then(Json::as_str) {
        if !id.starts_with('#') {
            return id.to_string();
        }
    }
    fallback.to_string()
}

/// Resolve a `$ref` to a new location, chasing chains.
fn resolve_ref(compiler: &Compiler, location: &Location) -> Result<Location, BuildError> {
    let reference = location
        .schema
        .get("$ref")
        .and_then(Json::as_str)
        .ok_or_else(|| BuildError::new("missing $ref"))?
        .to_string();

    let hash = reference.find('#').unwrap_or(reference.len());
    let id_part = &reference[..hash];
    let schema_id = if id_part.is_empty() {
        location.schema_id.clone()
    } else {
        id_part.to_string()
    };
    let pointer = if hash < reference.len() {
        &reference[hash..]
    } else {
        "#"
    };
    let pointer = if pointer.is_empty() { "#" } else { pointer };

    let resolved = compiler
        .resolver
        .get_schema(&schema_id, pointer)
        .ok_or_else(|| BuildError::new(format!("Cannot find reference \"{reference}\"")))?
        .clone();

    let new_location = Location {
        schema: resolved,
        schema_id,
        json_pointer: pointer.to_string(),
    };

    if new_location
        .schema
        .get("$ref")
        .and_then(Json::as_str)
        .is_some()
    {
        return resolve_ref(compiler, &new_location);
    }
    Ok(new_location)
}

/// The dispatch heart, matching `buildValue`.
fn build_value(
    compiler: &mut Compiler,
    mut location: Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    // Boolean schema matches anything.
    if let Json::Bool(_) = location.schema {
        return Ok(compiler.push(Node::AnyJson));
    }

    if location.schema.get("$ref").and_then(Json::as_str).is_some() {
        location = resolve_ref(compiler, &location)?;
    }

    // Memoize by resolved reference so recursion terminates. Reserve a slot and
    // register it before recursing, then fill it once the body is built.
    let memo_key = location.schema_ref();
    if let Some(&id) = compiler.memo.get(&memo_key) {
        return Ok(id);
    }

    let slot = compiler.reserve();
    compiler.memo.insert(memo_key.clone(), slot);
    compiler.building.push(memo_key.clone());

    let inner = if location.schema.get("allOf").is_some() {
        build_all_of(compiler, &location, large_array_size, json_stringify_arrays)?
    } else if location.schema.get("anyOf").is_some() || location.schema.get("oneOf").is_some() {
        build_one_of(compiler, &location, large_array_size, json_stringify_arrays)?
    } else if location.schema.get("if").is_some() && location.schema.get("then").is_some() {
        build_if_then_else(compiler, &location, large_array_size, json_stringify_arrays)?
    } else if location.schema.get("const").is_some() {
        build_const(compiler, &location)
    } else if let Some(Json::Array(_)) = location.schema.get("type") {
        build_multi_type(compiler, &location, large_array_size, json_stringify_arrays)?
    } else {
        let inferred_type = infer_type(&location.schema);
        build_single_type(
            compiler,
            &location,
            inferred_type.as_deref(),
            large_array_size,
            json_stringify_arrays,
        )?
    };

    compiler.building.pop();

    let nullable = location.schema.get("nullable") == Some(&Json::Bool(true));

    // Move the built node into the reserved slot, then wrap for nullable.
    let inner_node = compiler.nodes[inner].clone();
    compiler.set(slot, inner_node);

    if nullable {
        let wrapped = compiler.push(Node::Nullable(slot));
        compiler.memo.insert(memo_key, wrapped);
        Ok(wrapped)
    } else {
        Ok(slot)
    }
}

/// Infer a type from keywords, matching `inferTypeByKeyword`.
fn infer_type(schema: &Json) -> Option<String> {
    if schema.get("type").is_some() {
        return schema
            .get("type")
            .and_then(Json::as_str)
            .map(str::to_string);
    }
    const OBJECT: [&str; 7] = [
        "properties",
        "required",
        "additionalProperties",
        "patternProperties",
        "maxProperties",
        "minProperties",
        "dependencies",
    ];
    const ARRAY: [&str; 6] = [
        "items",
        "additionalItems",
        "maxItems",
        "minItems",
        "uniqueItems",
        "contains",
    ];
    const STRING: [&str; 3] = ["maxLength", "minLength", "pattern"];
    const NUMBER: [&str; 5] = [
        "multipleOf",
        "maximum",
        "exclusiveMaximum",
        "minimum",
        "exclusiveMinimum",
    ];

    let map = schema.as_object()?;
    for k in OBJECT {
        if map.contains_key(k) {
            return Some("object".to_string());
        }
    }
    for k in ARRAY {
        if map.contains_key(k) {
            return Some("array".to_string());
        }
    }
    for k in STRING {
        if map.contains_key(k) {
            return Some("string".to_string());
        }
    }
    for k in NUMBER {
        if map.contains_key(k) {
            return Some("number".to_string());
        }
    }
    None
}

/// Build a single-type serializer, matching `buildSingleTypeSerializer`.
fn build_single_type(
    compiler: &mut Compiler,
    location: &Location,
    inferred: Option<&str>,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let type_name = location
        .schema
        .get("type")
        .and_then(Json::as_str)
        .or(inferred);

    match type_name {
        Some("null") => Ok(compiler.push(Node::Null)),
        Some("string") => {
            let format = location.schema.get("format").and_then(Json::as_str);
            let node = match format {
                Some("date-time") => Node::StringFormat(StringFormat::DateTime),
                Some("date") => Node::StringFormat(StringFormat::Date),
                Some("time") => Node::StringFormat(StringFormat::Time),
                Some("unsafe") => Node::StringUnsafe,
                _ => Node::StringPlain,
            };
            Ok(compiler.push(node))
        }
        Some("integer") => Ok(compiler.push(Node::Integer)),
        Some("number") => Ok(compiler.push(Node::Number)),
        Some("boolean") => Ok(compiler.push(Node::Boolean)),
        Some("object") => build_object(compiler, location, large_array_size, json_stringify_arrays),
        Some("array") => build_array(compiler, location, large_array_size, json_stringify_arrays),
        None => Ok(compiler.push(Node::AnyJson)),
        Some(other) => Err(BuildError::new(format!("{other} unsupported"))),
    }
}

/// Build a const serializer, matching `buildConstSerializer`.
fn build_const(compiler: &mut Compiler, location: &Location) -> NodeId {
    let value = location.schema.get("const").cloned().unwrap_or(Json::Null);
    let rendered = render_json(&value);
    let null_alternative = match location.schema.get("type") {
        Some(Json::Array(types)) => types.iter().any(|t| t.as_str() == Some("null")),
        _ => false,
    };
    let nullable_attr = location.schema.get("nullable") == Some(&Json::Bool(true));
    compiler.push(Node::Const {
        rendered,
        null_alternative: null_alternative || nullable_attr,
    })
}

/// Build a multi-type serializer, matching `buildMultiTypeSerializer`.
fn build_multi_type(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let types = location
        .schema
        .get("type")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();

    // Sort so 'null' comes first.
    let mut names: Vec<String> = types
        .iter()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    names.sort_by_key(|t| if t == "null" { 0 } else { 1 });

    let reference = compiler.safe_ref(location);
    let mut branches = Vec::new();
    for name in names {
        let mut single = location.schema.clone();
        if let Json::Object(map) = &mut single {
            map.insert("type".to_string(), Json::String(name.clone()));
        }
        let sub_location = Location {
            schema: single,
            schema_id: location.schema_id.clone(),
            json_pointer: location.json_pointer.clone(),
        };
        let inferred = infer_type(&sub_location.schema);
        let node = build_single_type(
            compiler,
            &sub_location,
            inferred.as_deref(),
            large_array_size,
            json_stringify_arrays,
        )?;
        let type_name = match name.as_str() {
            "null" => TypeName::Null,
            "boolean" => TypeName::Boolean,
            "integer" => TypeName::Integer,
            "number" => TypeName::Number,
            "string" => TypeName::String,
            "object" => TypeName::Object,
            "array" => TypeName::Array,
            _ => continue,
        };
        branches.push((type_name, node));
    }

    Ok(compiler.push(Node::MultiType {
        branches,
        reference,
    }))
}

/// Build an object serializer, matching `buildObject` and `buildInnerObject`.
fn build_object(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let schema = &location.schema;
    let nullable = schema.get("nullable") == Some(&Json::Bool(true));

    let required: Vec<String> = schema
        .get("required")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    // Property keys with required first, preserving original order otherwise.
    let mut keys: Vec<String> = schema
        .get("properties")
        .and_then(Json::as_object)
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort_by(|a, b| {
        let ra = required.contains(a);
        let rb = required.contains(b);
        match (ra, rb) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });

    let properties_location = location.property("properties");
    let mut properties = Vec::new();
    for key in &keys {
        let mut prop_location = properties_location.property(key);
        if prop_location
            .schema
            .get("$ref")
            .and_then(Json::as_str)
            .is_some()
        {
            prop_location = resolve_ref(compiler, &prop_location)?;
        }
        let default = prop_location.schema.get("default").map(render_json);
        let node = build_value(
            compiler,
            prop_location.clone(),
            large_array_size,
            json_stringify_arrays,
        )?;
        properties.push(PropertyPlan {
            key: key.clone(),
            required: required.contains(key),
            default,
            node,
        });
    }

    let required_extra: Vec<String> = required
        .iter()
        .filter(|k| !keys.contains(k))
        .cloned()
        .collect();

    // Pattern properties.
    let mut pattern_properties = Vec::new();
    if let Some(Json::Object(patterns)) = schema.get("patternProperties") {
        let pattern_location = location.property("patternProperties");
        for (pattern, _) in patterns {
            let regex = regex::Regex::new(pattern)
                .map_err(|e| BuildError::new(format!("invalid pattern: {e}")))?;
            let mut sub = pattern_location.property(pattern);
            if sub.schema.get("$ref").and_then(Json::as_str).is_some() {
                sub = resolve_ref(compiler, &sub)?;
            }
            let node = build_value(compiler, sub, large_array_size, json_stringify_arrays)?;
            pattern_properties.push(PatternPlan { regex, node });
        }
    }

    // Additional properties.
    let additional = match schema.get("additionalProperties") {
        Some(Json::Bool(true)) => AdditionalProperties::True,
        Some(Json::Bool(false)) | None => AdditionalProperties::None,
        Some(_) => {
            let mut sub = location.property("additionalProperties");
            if sub.schema.get("$ref").and_then(Json::as_str).is_some() {
                sub = resolve_ref(compiler, &sub)?;
            }
            let node = build_value(compiler, sub, large_array_size, json_stringify_arrays)?;
            AdditionalProperties::Schema(node)
        }
    };

    Ok(compiler.push(Node::Object(ObjectNode {
        nullable,
        properties,
        required_extra,
        pattern_properties,
        additional,
    })))
}

/// Build an array serializer, matching `buildArray`.
fn build_array(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let schema = &location.schema;
    let nullable = schema.get("nullable") == Some(&Json::Bool(true));
    let reference = compiler.safe_ref(location);

    let mut items_location = location.property("items");
    if items_location.schema.is_null() {
        items_location.schema = Json::Object(serde_json::Map::new());
    }
    if items_location
        .schema
        .get("$ref")
        .and_then(Json::as_str)
        .is_some()
    {
        items_location = resolve_ref(compiler, &items_location)?;
    }

    let large_array = if json_stringify_arrays {
        Some(large_array_size)
    } else {
        None
    };

    let items = if let Json::Array(tuple) = &items_location.schema {
        let additional_items = schema
            .get("additionalItems")
            .map(|v| !matches!(v, Json::Bool(false) | Json::Null))
            .unwrap_or(false);
        let mut tuple_items = Vec::new();
        for i in 0..tuple.len() {
            let mut item_location = items_location.index(i);
            if item_location
                .schema
                .get("$ref")
                .and_then(Json::as_str)
                .is_some()
            {
                item_location = resolve_ref(compiler, &item_location)?;
            }
            let type_check = item_location.schema.get("type").cloned();
            let node = build_value(
                compiler,
                item_location,
                large_array_size,
                json_stringify_arrays,
            )?;
            tuple_items.push(TupleItem { type_check, node });
        }
        ArrayItems::Tuple {
            items: tuple_items,
            additional_items,
        }
    } else {
        let node = build_value(
            compiler,
            items_location,
            large_array_size,
            json_stringify_arrays,
        )?;
        ArrayItems::Homogeneous(node)
    };

    Ok(compiler.push(Node::Array(ArrayNode {
        nullable,
        reference,
        items,
        large_array,
    })))
}

/// Build allOf, matching `buildAllOf`.
fn build_all_of(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let all_of = location
        .schema
        .get("allOf")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();

    // Base schema without allOf.
    let mut base = location.schema.clone();
    if let Json::Object(map) = &mut base {
        map.remove("allOf");
    }

    let mut locations = vec![Location {
        schema: base,
        schema_id: location.schema_id.clone(),
        json_pointer: location.json_pointer.clone(),
    }];
    let all_of_location = location.property("allOf");
    for i in 0..all_of.len() {
        locations.push(all_of_location.index(i));
    }

    let merged = merge_locations(compiler, locations)?;
    build_value(compiler, merged, large_array_size, json_stringify_arrays)
}

/// Resolve refs, clone with absolute refs, and merge a set of locations into one.
fn merge_locations(
    compiler: &mut Compiler,
    mut locations: Vec<Location>,
) -> Result<Location, BuildError> {
    for loc in &mut locations {
        if loc.schema.get("$ref").and_then(Json::as_str).is_some() {
            *loc = resolve_ref(compiler, loc)?;
        }
    }

    let mut schemas = Vec::new();
    for loc in &locations {
        let mut cloned = clone_origin_schema(&loc.schema, &loc.schema_id);
        if let Json::Object(map) = &mut cloned {
            map.remove("$id");
        }
        schemas.push(cloned);
    }

    let merged_schema = merge_schemas(&schemas)?;

    // Reuse an existing merged id when the merged content matches. This keeps a
    // recursive merge from minting a fresh id each pass, which would loop.
    let content = merged_schema.to_string();
    if let Some(existing_id) = compiler.merged_by_content.get(&content) {
        let existing = compiler
            .resolver
            .get_schema(existing_id, "#")
            .cloned()
            .unwrap_or_else(|| merged_schema.clone());
        return Ok(Location::new(existing, existing_id.clone()));
    }

    let merged_id = format!("__fjs_merged_{}", compiler.merged_counter);
    compiler.merged_counter += 1;
    compiler
        .merged_by_content
        .insert(content, merged_id.clone());
    compiler
        .resolver
        .add_schema(merged_schema.clone(), &merged_id);

    Ok(Location::new(merged_schema, merged_id))
}

/// Deep-clone a schema, rewriting local `#...` refs to absolute, matching
/// `cloneOriginSchema`.
fn clone_origin_schema(schema: &Json, schema_id: &str) -> Json {
    let mut current_id = schema_id.to_string();
    if let Some(id) = schema.get("$id").and_then(Json::as_str) {
        if !id.starts_with('#') {
            current_id = id.to_string();
        }
    }

    match schema {
        Json::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, value) in map {
                if key == "$ref" {
                    if let Json::String(r) = value {
                        if r.starts_with('#') {
                            out.insert(key.clone(), Json::String(format!("{current_id}{r}")));
                            continue;
                        }
                    }
                }
                if value.is_object() || value.is_array() {
                    out.insert(key.clone(), clone_origin_schema(value, &current_id));
                } else {
                    out.insert(key.clone(), value.clone());
                }
            }
            Json::Object(out)
        }
        Json::Array(items) => Json::Array(
            items
                .iter()
                .map(|v| clone_origin_schema(v, &current_id))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Build anyOf/oneOf, matching `buildOneOf`.
fn build_one_of(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let key = if location.schema.get("anyOf").is_some() {
        "anyOf"
    } else {
        "oneOf"
    };
    let options_json = location
        .schema
        .get(key)
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default();

    let mut base = location.schema.clone();
    if let Json::Object(map) = &mut base {
        map.remove(key);
    }
    let base_location = Location {
        schema: base,
        schema_id: location.schema_id.clone(),
        json_pointer: location.json_pointer.clone(),
    };
    let options_location = location.property(key);
    let reference = compiler.safe_ref(location);

    let mut options = Vec::new();
    for i in 0..options_json.len() {
        let option_location = options_location.index(i);
        // Resolve a ref option so the validator sees the target schema, and
        // track the document the target lives in so its inner refs resolve.
        let (option_schema_resolved, option_base_id) = if option_location
            .schema
            .get("$ref")
            .and_then(Json::as_str)
            .is_some()
        {
            let resolved = resolve_ref(compiler, &option_location)?;
            (resolved.schema, resolved.schema_id)
        } else {
            (option_location.schema.clone(), location.schema_id.clone())
        };

        let merged = merge_locations(
            compiler,
            vec![base_location.clone(), option_location.clone()],
        )?;
        let node = build_value(compiler, merged, large_array_size, json_stringify_arrays)?;
        options.push(BranchOption {
            schema: option_schema_resolved,
            base_id: option_base_id,
            node,
        });
    }

    Ok(compiler.push(Node::OneOf { options, reference }))
}

/// Build if/then/else, matching `buildIfThenElse`.
fn build_if_then_else(
    compiler: &mut Compiler,
    location: &Location,
    large_array_size: usize,
    json_stringify_arrays: bool,
) -> Result<NodeId, BuildError> {
    let if_schema = location
        .schema
        .get("if")
        .cloned()
        .unwrap_or(Json::Bool(true));
    let else_schema = location.schema.get("else").cloned();

    let mut base = location.schema.clone();
    if let Json::Object(map) = &mut base {
        map.remove("if");
        map.remove("then");
        map.remove("else");
    }
    let root_location = Location {
        schema: base,
        schema_id: location.schema_id.clone(),
        json_pointer: location.json_pointer.clone(),
    };

    let then_location = location.property("then");
    let then_merged = merge_locations(compiler, vec![root_location.clone(), then_location])?;
    let then = build_value(
        compiler,
        then_merged,
        large_array_size,
        json_stringify_arrays,
    )?;

    let els = if let Some(_else_value) = else_schema {
        let else_location = location.property("else");
        let else_merged = merge_locations(compiler, vec![root_location.clone(), else_location])?;
        build_value(
            compiler,
            else_merged,
            large_array_size,
            json_stringify_arrays,
        )?
    } else {
        build_value(
            compiler,
            root_location,
            large_array_size,
            json_stringify_arrays,
        )?
    };

    Ok(compiler.push(Node::IfThenElse {
        if_schema,
        base_id: location.schema_id.clone(),
        then,
        els,
    }))
}

/// Render a JSON literal exactly as `JSON.stringify` would, used for defaults
/// and consts.
fn render_json(value: &Json) -> String {
    crate::native::stringify(value)
}

// Serialization walk.

impl Plan {
    fn emit(&self, id: NodeId, value: &Value, out: &mut String) -> Result<(), StringifyError> {
        match &self.nodes[id] {
            Node::Null => out.push_str("null"),
            Node::Boolean => out.push_str(&self.serializer.as_boolean(value)),
            Node::Integer => out.push_str(&self.serializer.as_integer(value)?),
            Node::Number => out.push_str(&self.serializer.as_number(value)?),
            Node::StringPlain => out.push_str(&self.emit_plain_string(value)),
            Node::StringFormat(format) => {
                let rendered = match format {
                    StringFormat::DateTime => self.serializer.as_date_time(value)?,
                    StringFormat::Date => self.serializer.as_date(value)?,
                    StringFormat::Time => self.serializer.as_time(value)?,
                };
                out.push_str(&rendered);
            }
            Node::StringUnsafe => {
                if let Value::String(s) = value {
                    out.push_str(&self.serializer.as_unsafe_string(s));
                } else {
                    out.push_str(&self.emit_plain_string(value));
                }
            }
            Node::AnyJson => out.push_str(&crate::native::stringify_value(value)),
            Node::Const {
                rendered,
                null_alternative,
            } => {
                if *null_alternative && value.is_null() {
                    out.push_str("null");
                } else {
                    out.push_str(rendered);
                }
            }
            Node::Nullable(child) => {
                if value.is_null() {
                    out.push_str("null");
                } else {
                    self.emit(*child, value, out)?;
                }
            }
            Node::Object(node) => self.emit_object(node, value, out)?,
            Node::Array(node) => self.emit_array(node, value, out)?,
            Node::MultiType {
                branches,
                reference,
            } => self.emit_multi_type(branches, reference, value, out)?,
            Node::OneOf { options, reference } => {
                self.emit_one_of(options, reference, value, out)?
            }
            Node::IfThenElse {
                if_schema,
                base_id,
                then,
                els,
            } => {
                if validate_branch(if_schema, value, &self.resolver, base_id) {
                    self.emit(*then, value, out)?;
                } else {
                    self.emit(*els, value, out)?;
                }
            }
        }
        Ok(())
    }

    /// Emit a plain string with the source coercion: null to `""`, Date to ISO,
    /// RegExp to its source, other objects via toString.
    fn emit_plain_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => self.serializer.as_string(s),
            Value::Null => "\"\"".to_string(),
            Value::Date(ms) => {
                format!("\"{}\"", crate::value::iso_from_millis(*ms))
            }
            Value::Regex(src) => self.serializer.as_string(src),
            Value::Custom(inner) => self.emit_plain_string(inner),
            other => self
                .serializer
                .as_string(&crate::serializer::display_value(other)),
        }
    }

    fn emit_object(
        &self,
        node: &ObjectNode,
        value: &Value,
        out: &mut String,
    ) -> Result<(), StringifyError> {
        // Apply toJSON.
        let unwrapped = value.unwrap_to_json();
        let value = unwrapped.as_ref();

        let obj = match value {
            Value::Object(o) => o,
            Value::Null => {
                out.push_str(if node.nullable { "null" } else { "{}" });
                return Ok(());
            }
            // A non-object that reached here after toJSON unwrap renders as {}.
            _ => {
                out.push_str("{}");
                return Ok(());
            }
        };

        // Required keys not in properties must be present.
        for key in &node.required_extra {
            if obj.get(key).is_none() {
                return Err(StringifyError::required(key));
            }
        }

        out.push('{');
        let mut wrote = false;

        for prop in &node.properties {
            match obj.get(&prop.key) {
                Some(v) => {
                    if wrote {
                        out.push(',');
                    }
                    wrote = true;
                    out.push_str(&native_key(&prop.key));
                    out.push(':');
                    self.emit(prop.node, v, out)?;
                }
                None => {
                    if let Some(default) = &prop.default {
                        if wrote {
                            out.push(',');
                        }
                        wrote = true;
                        out.push_str(&native_key(&prop.key));
                        out.push(':');
                        out.push_str(default);
                    } else if prop.required {
                        return Err(StringifyError::required(&prop.key));
                    }
                }
            }
        }

        let has_extra = !node.pattern_properties.is_empty()
            || !matches!(node.additional, AdditionalProperties::None);
        if has_extra {
            let known: Vec<&str> = node.properties.iter().map(|p| p.key.as_str()).collect();
            for (key, v) in obj.iter() {
                if known.contains(&key.as_str()) {
                    continue;
                }
                if matches!(v, Value::Custom(_)) {
                    // a toJSON object still serializes, fall through
                }
                let mut matched = false;
                for pattern in &node.pattern_properties {
                    if pattern.regex.is_match(key) {
                        if wrote {
                            out.push(',');
                        }
                        wrote = true;
                        out.push_str(&self.serializer.as_string(key));
                        out.push(':');
                        self.emit(pattern.node, v, out)?;
                        matched = true;
                        break;
                    }
                }
                if matched {
                    continue;
                }
                match &node.additional {
                    AdditionalProperties::True => {
                        if wrote {
                            out.push(',');
                        }
                        wrote = true;
                        out.push_str(&self.serializer.as_string(key));
                        out.push(':');
                        out.push_str(&crate::native::stringify_value(v));
                    }
                    AdditionalProperties::Schema(child) => {
                        if wrote {
                            out.push(',');
                        }
                        wrote = true;
                        out.push_str(&self.serializer.as_string(key));
                        out.push(':');
                        self.emit(*child, v, out)?;
                    }
                    AdditionalProperties::None => {}
                }
            }
        }

        out.push('}');
        Ok(())
    }

    fn emit_array(
        &self,
        node: &ArrayNode,
        value: &Value,
        out: &mut String,
    ) -> Result<(), StringifyError> {
        let items = match value {
            Value::Array(items) => items,
            Value::Null => {
                out.push_str(if node.nullable { "null" } else { "[]" });
                return Ok(());
            }
            _ => return Err(StringifyError::no_match(&node.reference)),
        };

        match &node.items {
            ArrayItems::Tuple {
                items: tuple,
                additional_items,
            } => {
                if !additional_items && items.len() > tuple.len() {
                    return Err(StringifyError::item_mismatch(tuple.len()));
                }
                if let Some(threshold) = node.large_array {
                    if items.len() >= threshold {
                        out.push_str(&crate::native::stringify_value(value));
                        return Ok(());
                    }
                }
                out.push('[');
                let mut wrote = false;
                for (i, tuple_item) in tuple.iter().enumerate() {
                    if i >= items.len() {
                        break;
                    }
                    let v = &items[i];
                    if !item_type_matches(tuple_item.type_check.as_ref(), v) {
                        return Err(StringifyError::item_mismatch(i));
                    }
                    if wrote {
                        out.push(',');
                    }
                    wrote = true;
                    self.emit(tuple_item.node, v, out)?;
                }
                if *additional_items {
                    for v in items.iter().skip(tuple.len()) {
                        if wrote {
                            out.push(',');
                        }
                        wrote = true;
                        out.push_str(&crate::native::stringify_value(v));
                    }
                }
                out.push(']');
            }
            ArrayItems::Homogeneous(child) => {
                if let Some(threshold) = node.large_array {
                    if items.len() >= threshold {
                        out.push_str(&crate::native::stringify_value(value));
                        return Ok(());
                    }
                }
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    self.emit(*child, v, out)?;
                }
                out.push(']');
            }
        }
        Ok(())
    }

    fn emit_multi_type(
        &self,
        branches: &[(TypeName, NodeId)],
        reference: &str,
        value: &Value,
        out: &mut String,
    ) -> Result<(), StringifyError> {
        for (type_name, node) in branches {
            if multi_type_matches(*type_name, value) {
                return self.emit(*node, value, out);
            }
        }
        Err(StringifyError::no_match(reference))
    }

    fn emit_one_of(
        &self,
        options: &[BranchOption],
        reference: &str,
        value: &Value,
        out: &mut String,
    ) -> Result<(), StringifyError> {
        for option in options {
            if validate_branch(&option.schema, value, &self.resolver, &option.base_id) {
                return self.emit(option.node, value, out);
            }
        }
        Err(StringifyError::no_match(reference))
    }
}

/// The runtime condition used by multi-type dispatch, matching the source
/// `if/else if` chain.
fn multi_type_matches(type_name: TypeName, value: &Value) -> bool {
    match type_name {
        TypeName::Null => value.is_null(),
        TypeName::String => {
            matches!(
                value,
                Value::String(_) | Value::Date(_) | Value::Regex(_) | Value::Custom(_)
            ) || value.is_null()
        }
        TypeName::Array => matches!(value, Value::Array(_)),
        TypeName::Integer => match value {
            Value::Number(n) => n.fract() == 0.0 && n.is_finite(),
            Value::BigInt(_) => true,
            Value::Null => true,
            _ => false,
        },
        TypeName::Number => matches!(value, Value::Number(_) | Value::BigInt(_)) || value.is_null(),
        TypeName::Boolean => matches!(value, Value::Bool(_)) || value.is_null(),
        TypeName::Object => matches!(value, Value::Object(_) | Value::Custom(_)) || value.is_null(),
    }
}

/// Per-item type check for tuples, matching `buildArrayTypeCondition`.
fn item_type_matches(type_check: Option<&Json>, value: &Value) -> bool {
    let Some(type_value) = type_check else {
        // No type declared accepts anything.
        return true;
    };
    match type_value {
        Json::String(name) => single_item_type_matches(name, value),
        Json::Array(names) => names.iter().any(|n| {
            n.as_str()
                .map(|name| single_item_type_matches(name, value))
                .unwrap_or(false)
        }),
        _ => true,
    }
}

/// One type-name item check.
fn single_item_type_matches(name: &str, value: &Value) -> bool {
    match name {
        "null" => value.is_null(),
        "string" => {
            matches!(
                value,
                Value::String(_) | Value::Date(_) | Value::Regex(_) | Value::Custom(_)
            ) || value.is_null()
        }
        "integer" => match value {
            Value::Number(n) => n.fract() == 0.0 && n.is_finite(),
            Value::BigInt(_) => true,
            _ => false,
        },
        "number" => match value {
            Value::Number(n) => n.is_finite(),
            Value::BigInt(_) => true,
            _ => false,
        },
        "boolean" => matches!(value, Value::Bool(_)),
        "object" => matches!(value, Value::Object(_) | Value::Custom(_)),
        "array" => matches!(value, Value::Array(_)),
        _ => false,
    }
}

/// Render a property key as a quoted JSON string. Object keys are plain strings,
/// so native escaping applies.
fn native_key(key: &str) -> String {
    crate::native::stringify(&Json::String(key.to_string()))
}
