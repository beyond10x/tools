//! The ESS type grammar a contract's field lists use ([`Type`]), the canonical [`Shape`] both a
//! field list and a `schemars`-derived schema map to, and [`check_shapes`], which refuses the
//! first difference between them naming the operation, the side and the field.

use std::collections::BTreeMap;
use std::fmt;

use schemars::Schema;
use serde_json::{Map, Value};

use crate::document::{Contract, Field, TypeDecl, TypeKind};

/// A type as a field list spells it: a primitive, a generic over one or two types, or a named type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// `String`.
    String,
    /// `Integer`.
    Integer,
    /// `Boolean`.
    Boolean,
    /// `Bytes`.
    Bytes,
    /// `Optional<T>`.
    Optional(Box<Type>),
    /// `List<T>`.
    List(Box<Type>),
    /// `Map<K, V>`; `K` must resolve to `String`.
    Map(Box<Type>, Box<Type>),
    /// A type the document declares under `types`.
    Named(String),
}

/// Why a type text does not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{text}: {reason}")]
pub struct TypeParseError {
    /// The text that was parsed.
    pub text: String,
    /// What is wrong with it.
    pub reason: String,
}

impl Type {
    /// Parses a type text such as `Optional<List<String>>`.
    ///
    /// # Errors
    ///
    /// The text is not in the grammar.
    pub fn parse(text: &str) -> Result<Self, TypeParseError> {
        let fail = |reason: String| TypeParseError {
            text: text.to_string(),
            reason,
        };
        let (ty, rest) = parse_prefix(text.trim()).map_err(fail)?;
        if rest.trim().is_empty() {
            Ok(ty)
        } else {
            Err(fail(format!("unexpected {:?} after the type", rest.trim())))
        }
    }
}

fn parse_prefix(text: &str) -> Result<(Type, &str), String> {
    let text = text.trim_start();
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_'))
        .unwrap_or(text.len());
    let (ident, rest) = text.split_at(end);
    if ident.is_empty() {
        return Err(if text.is_empty() {
            "expected a type name".to_string()
        } else {
            format!("expected a type name, found {text:?}")
        });
    }
    let rest = rest.trim_start();
    let primitive = match ident {
        "String" => Some(Type::String),
        "Integer" => Some(Type::Integer),
        "Boolean" => Some(Type::Boolean),
        "Bytes" => Some(Type::Bytes),
        _ => None,
    };
    match ident {
        "Optional" | "List" => {
            let (inner, rest) = parse_argument(rest, ident)?;
            let rest = expect_char(rest, '>', ident)?;
            let ty = if ident == "Optional" {
                Type::Optional(Box::new(inner))
            } else {
                Type::List(Box::new(inner))
            };
            Ok((ty, rest))
        }
        "Map" => {
            let (key, rest) = parse_argument(rest, ident)?;
            let rest = expect_char(rest, ',', ident)?;
            let (value, rest) = parse_prefix(rest)?;
            let rest = expect_char(rest, '>', ident)?;
            Ok((Type::Map(Box::new(key), Box::new(value)), rest))
        }
        _ if rest.starts_with('<') => Err(format!("{ident} takes no type arguments")),
        _ => Ok((
            primitive.unwrap_or_else(|| Type::Named(ident.to_string())),
            rest,
        )),
    }
}

fn parse_argument<'a>(rest: &'a str, generic: &str) -> Result<(Type, &'a str), String> {
    let rest = expect_char(rest, '<', generic)?;
    parse_prefix(rest)
}

fn expect_char<'a>(rest: &'a str, expected: char, generic: &str) -> Result<&'a str, String> {
    let rest = rest.trim_start();
    rest.strip_prefix(expected).ok_or_else(|| {
        if rest.is_empty() {
            format!("{generic}: expected {expected:?}, found the end of the text")
        } else {
            format!("{generic}: expected {expected:?}, found {rest:?}")
        }
    })
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => f.write_str("String"),
            Self::Integer => f.write_str("Integer"),
            Self::Boolean => f.write_str("Boolean"),
            Self::Bytes => f.write_str("Bytes"),
            Self::Optional(inner) => write!(f, "Optional<{inner}>"),
            Self::List(inner) => write!(f, "List<{inner}>"),
            Self::Map(key, value) => write!(f, "Map<{key}, {value}>"),
            Self::Named(name) => f.write_str(name),
        }
    }
}

/// The canonical shape a field list and a derived schema both map to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// A string.
    String,
    /// An integer of any width.
    Integer,
    /// A boolean.
    Boolean,
    /// A byte string.
    Bytes,
    /// A value that may be absent or `null`.
    Optional(Box<Shape>),
    /// A list.
    List(Box<Shape>),
    /// A map with string keys.
    Map(Box<Shape>, Box<Shape>),
    /// Named fields.
    Struct(BTreeMap<String, Shape>),
    /// One of a fixed set of strings, sorted.
    Enum(Vec<String>),
}

/// Why a field list or a schema has no shape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShapeError {
    /// A field's type text does not parse.
    #[error("field {field}: {source}")]
    Type {
        /// The field.
        field: String,
        /// The parse error.
        #[source]
        source: TypeParseError,
    },
    /// A named type is not declared.
    #[error("type {name} is not declared")]
    UnknownType {
        /// The name.
        name: String,
    },
    /// A named type refers to itself.
    #[error("type {name} refers to itself")]
    Recursive {
        /// The name.
        name: String,
    },
    /// A type declaration does not fit its kind.
    #[error("type {name} is kind {kind} but {problem}")]
    Malformed {
        /// The name.
        name: String,
        /// The declared kind.
        kind: TypeKind,
        /// What does not fit.
        problem: String,
    },
    /// A `Map` key does not resolve to `String`.
    #[error("map key {key} does not resolve to String")]
    MapKey {
        /// The key type.
        key: String,
    },
    /// A schema node is not in the grammar.
    #[error("schema at {at}: {problem}")]
    Schema {
        /// A JSON-pointer-like path to the node.
        at: String,
        /// What is unsupported.
        problem: String,
    },
}

impl Shape {
    /// `Optional<inner>`, collapsing `Optional<Optional<T>>` to `Optional<T>`.
    pub fn optional(inner: Self) -> Self {
        if matches!(inner, Self::Optional(_)) {
            inner
        } else {
            Self::Optional(Box::new(inner))
        }
    }

    /// The shape of a field list: a [`Shape::Struct`] of its fields.
    ///
    /// # Errors
    ///
    /// A field's type does not parse or resolve against `types`.
    pub fn of_fields(fields: &[Field], types: &[TypeDecl]) -> Result<Self, ShapeError> {
        let mut stack = Vec::new();
        fields_shape(fields, types, &mut stack)
    }

    /// The shape of one type, resolved against `types`.
    ///
    /// # Errors
    ///
    /// A named type is missing, malformed or recursive, or a `Map` key is not `String`.
    pub fn of_type(ty: &Type, types: &[TypeDecl]) -> Result<Self, ShapeError> {
        let mut stack = Vec::new();
        type_shape(ty, types, &mut stack)
    }

    /// The shape of a `schemars`-derived schema.
    ///
    /// `$ref`s resolve into the root's `$defs`; a property missing from `required` or typed
    /// `null` is `Optional`; every integer format is `Integer`; a string with `enum` (or
    /// `const`) is `Enum`; an object with `additionalProperties` and no `properties` is `Map`; an
    /// array of `uint8` or a string with `contentEncoding: base64` is `Bytes`.
    ///
    /// # Errors
    ///
    /// A node is outside the grammar (a number, a union, an untyped object, a far `$ref`).
    pub fn of_schema(schema: &Schema) -> Result<Self, ShapeError> {
        let root = schema.as_value();
        let defs = root
            .get("$defs")
            .or_else(|| root.get("definitions"))
            .and_then(Value::as_object);
        let mut stack = Vec::new();
        schema_shape(root, defs, "#", &mut stack)
    }
}

fn fields_shape(
    fields: &[Field],
    types: &[TypeDecl],
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let mut shape = BTreeMap::new();
    for field in fields {
        let ty = Type::parse(&field.ty).map_err(|source| ShapeError::Type {
            field: field.name.clone(),
            source,
        })?;
        shape.insert(field.name.clone(), type_shape(&ty, types, stack)?);
    }
    Ok(Shape::Struct(shape))
}

fn type_shape(ty: &Type, types: &[TypeDecl], stack: &mut Vec<String>) -> Result<Shape, ShapeError> {
    Ok(match ty {
        Type::String => Shape::String,
        Type::Integer => Shape::Integer,
        Type::Boolean => Shape::Boolean,
        Type::Bytes => Shape::Bytes,
        Type::Optional(inner) => Shape::optional(type_shape(inner, types, stack)?),
        Type::List(inner) => Shape::List(Box::new(type_shape(inner, types, stack)?)),
        Type::Map(key, value) => {
            let key_shape = type_shape(key, types, stack)?;
            if key_shape != Shape::String {
                return Err(ShapeError::MapKey {
                    key: key.to_string(),
                });
            }
            Shape::Map(
                Box::new(key_shape),
                Box::new(type_shape(value, types, stack)?),
            )
        }
        Type::Named(name) => named_shape(name, types, stack)?,
    })
}

fn named_shape(
    name: &str,
    types: &[TypeDecl],
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let decl =
        types
            .iter()
            .find(|decl| decl.name == name)
            .ok_or_else(|| ShapeError::UnknownType {
                name: name.to_string(),
            })?;
    if stack.iter().any(|seen| seen == name) {
        return Err(ShapeError::Recursive {
            name: name.to_string(),
        });
    }
    stack.push(name.to_string());
    let shape = match decl.kind {
        TypeKind::Enum => {
            if decl.variants.is_empty() {
                return Err(malformed(decl, "has no variants"));
            }
            let mut variants = decl.variants.clone();
            variants.sort();
            Ok(Shape::Enum(variants))
        }
        TypeKind::Struct => {
            if decl.fields.is_empty() {
                return Err(malformed(decl, "has no fields"));
            }
            fields_shape(&decl.fields, types, stack)
        }
        TypeKind::Newtype => {
            let Some(inner) = decl.ty.as_deref() else {
                return Err(malformed(decl, "names no type"));
            };
            let inner = Type::parse(inner).map_err(|err| malformed(decl, &err.to_string()))?;
            type_shape(&inner, types, stack)
        }
    };
    stack.pop();
    shape
}

fn malformed(decl: &TypeDecl, problem: &str) -> ShapeError {
    ShapeError::Malformed {
        name: decl.name.clone(),
        kind: decl.kind,
        problem: problem.to_string(),
    }
}

fn schema_shape(
    node: &Value,
    defs: Option<&Map<String, Value>>,
    at: &str,
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let unsupported = |problem: String| ShapeError::Schema {
        at: at.to_string(),
        problem,
    };
    let Some(object) = node.as_object() else {
        return Err(unsupported(if node == &Value::Bool(true) {
            "accepts any value".to_string()
        } else {
            format!("{node} is not an object schema")
        }));
    };
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        return ref_shape(reference, defs, at, stack);
    }
    if let Some(branches) = object
        .get("anyOf")
        .or_else(|| object.get("oneOf"))
        .and_then(Value::as_array)
    {
        return union_shape(branches, defs, at, stack);
    }
    let (ty, nullable) = declared_type(object).map_err(unsupported)?;
    let shape = match ty {
        Some("string") => string_shape(object),
        Some("integer") => Shape::Integer,
        Some("boolean") => Shape::Boolean,
        Some("array") => array_shape(object, defs, at, stack)?,
        Some("object") => object_shape(object, defs, at, stack)?,
        Some(other) => return Err(unsupported(format!("type {other} is not in the grammar"))),
        None => {
            if object.contains_key("enum") || object.contains_key("const") {
                string_shape(object)
            } else {
                return Err(unsupported("names no type".to_string()));
            }
        }
    };
    Ok(if nullable {
        Shape::optional(shape)
    } else {
        shape
    })
}

fn ref_shape(
    reference: &str,
    defs: Option<&Map<String, Value>>,
    at: &str,
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let name = reference
        .strip_prefix("#/$defs/")
        .or_else(|| reference.strip_prefix("#/definitions/"))
        .ok_or_else(|| ShapeError::Schema {
            at: at.to_string(),
            problem: format!("$ref {reference} is not into the root's $defs"),
        })?;
    if stack.iter().any(|seen| seen == name) {
        return Err(ShapeError::Recursive {
            name: name.to_string(),
        });
    }
    let target = defs
        .and_then(|d| d.get(name))
        .ok_or_else(|| ShapeError::Schema {
            at: at.to_string(),
            problem: format!("$ref {reference} does not resolve"),
        })?;
    stack.push(name.to_string());
    let shape = schema_shape(target, defs, &format!("#/$defs/{name}"), stack);
    stack.pop();
    shape
}

fn union_shape(
    branches: &[Value],
    defs: Option<&Map<String, Value>>,
    at: &str,
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let is_null = |branch: &Value| branch.get("type").and_then(Value::as_str) == Some("null");
    let others: Vec<&Value> = branches.iter().filter(|b| !is_null(b)).collect();
    match others.as_slice() {
        [only] if others.len() < branches.len() => {
            Ok(Shape::optional(schema_shape(only, defs, at, stack)?))
        }
        _ => Err(ShapeError::Schema {
            at: at.to_string(),
            problem: format!(
                "a union of {} branches is not in the grammar",
                branches.len()
            ),
        }),
    }
}

fn declared_type(object: &Map<String, Value>) -> Result<(Option<&str>, bool), String> {
    match object.get("type") {
        None => Ok((None, false)),
        Some(Value::String(ty)) => Ok((Some(ty.as_str()), ty == "null")),
        Some(Value::Array(types)) => {
            let names: Vec<&str> = types.iter().filter_map(Value::as_str).collect();
            let nullable = names.contains(&"null");
            let others: Vec<&str> = names.iter().copied().filter(|t| *t != "null").collect();
            match others.as_slice() {
                [only] => Ok((Some(only), nullable)),
                [] => Err("type is null alone".to_string()),
                _ => Err(format!("type names {} alternatives", others.len())),
            }
        }
        Some(other) => Err(format!("type {other} is neither a string nor a list")),
    }
}

fn string_shape(object: &Map<String, Value>) -> Shape {
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        let mut variants: Vec<String> = values
            .iter()
            .filter(|v| !v.is_null())
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        variants.sort();
        return Shape::Enum(variants);
    }
    if let Some(Value::String(only)) = object.get("const") {
        return Shape::Enum(vec![only.clone()]);
    }
    if object.get("contentEncoding").and_then(Value::as_str) == Some("base64") {
        return Shape::Bytes;
    }
    Shape::String
}

fn array_shape(
    object: &Map<String, Value>,
    defs: Option<&Map<String, Value>>,
    at: &str,
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let items = object.get("items").ok_or_else(|| ShapeError::Schema {
        at: at.to_string(),
        problem: "an array without items".to_string(),
    })?;
    let is_byte = items.get("type").and_then(Value::as_str) == Some("integer")
        && items.get("format").and_then(Value::as_str) == Some("uint8");
    if is_byte {
        return Ok(Shape::Bytes);
    }
    Ok(Shape::List(Box::new(schema_shape(
        items,
        defs,
        &format!("{at}/items"),
        stack,
    )?)))
}

fn object_shape(
    object: &Map<String, Value>,
    defs: Option<&Map<String, Value>>,
    at: &str,
    stack: &mut Vec<String>,
) -> Result<Shape, ShapeError> {
    let properties = object.get("properties").and_then(Value::as_object);
    let additional = object.get("additionalProperties");
    match (properties, additional) {
        (None, Some(values)) if values.is_object() => Ok(Shape::Map(
            Box::new(Shape::String),
            Box::new(schema_shape(
                values,
                defs,
                &format!("{at}/additionalProperties"),
                stack,
            )?),
        )),
        (None, Some(Value::Bool(true))) => Err(ShapeError::Schema {
            at: at.to_string(),
            problem: "an object whose values have no declared shape".to_string(),
        }),
        (properties, _) => {
            let required: Vec<&str> = object
                .get("required")
                .and_then(Value::as_array)
                .map(|r| r.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let mut fields = BTreeMap::new();
            for (name, property) in properties.into_iter().flatten() {
                let shape =
                    schema_shape(property, defs, &format!("{at}/properties/{name}"), stack)?;
                let shape = if required.contains(&name.as_str()) {
                    shape
                } else {
                    Shape::optional(shape)
                };
                fields.insert(name.clone(), shape);
            }
            Ok(Shape::Struct(fields))
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => f.write_str("String"),
            Self::Integer => f.write_str("Integer"),
            Self::Boolean => f.write_str("Boolean"),
            Self::Bytes => f.write_str("Bytes"),
            Self::Optional(inner) => write!(f, "Optional<{inner}>"),
            Self::List(inner) => write!(f, "List<{inner}>"),
            Self::Map(key, value) => write!(f, "Map<{key}, {value}>"),
            Self::Struct(fields) => {
                f.write_str("Struct{")?;
                for (i, (name, shape)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {shape}")?;
                }
                f.write_str("}")
            }
            Self::Enum(variants) => write!(f, "Enum[{}]", variants.join(", ")),
        }
    }
}

/// Which field list of an operation a schema is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The operation's input.
    Input,
    /// The operation's output.
    Output,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Input => "input",
            Self::Output => "output",
        })
    }
}

/// The first difference between a contract's field list and a derived schema.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{operation}: {side}: {field}: {detail}")]
pub struct ShapeMismatch {
    /// The operation.
    pub operation: String,
    /// The side.
    pub side: Side,
    /// The path to the field, such as `lines.total` or `entries[].kind`; `(root)` when the whole
    /// side has no shape.
    pub field: String,
    /// What differs.
    pub detail: String,
}

/// Refuses the first difference between `operation`'s field list on `side` and `derived`.
///
/// # Errors
///
/// The operation is not declared, either side has no shape, or a field differs.
pub fn check_shapes(
    contract: &Contract,
    side: Side,
    operation: &str,
    derived: &Schema,
) -> Result<(), ShapeMismatch> {
    let mismatch = |field: &str, detail: String| ShapeMismatch {
        operation: operation.to_string(),
        side,
        field: field.to_string(),
        detail,
    };
    let op = contract.operation(operation).ok_or_else(|| {
        mismatch(
            "(root)",
            "the contract declares no such operation".to_string(),
        )
    })?;
    let fields = match side {
        Side::Input => &op.input,
        Side::Output => &op.output,
    };
    let expected = Shape::of_fields(fields, &contract.types).map_err(|err| {
        mismatch(
            "(root)",
            format!("the contract's field list has no shape: {err}"),
        )
    })?;
    let actual = Shape::of_schema(derived)
        .map_err(|err| mismatch("(root)", format!("the derived schema has no shape: {err}")))?;
    match first_difference(&expected, &actual, "") {
        None => Ok(()),
        Some((path, detail)) => Err(mismatch(
            if path.is_empty() { "(root)" } else { &path },
            detail,
        )),
    }
}

fn first_difference(expected: &Shape, actual: &Shape, path: &str) -> Option<(String, String)> {
    match (expected, actual) {
        (Shape::Struct(expected), Shape::Struct(actual)) => {
            for (name, shape) in expected {
                let path = join(path, name);
                match actual.get(name) {
                    None => {
                        return Some((
                            path,
                            format!("{shape} in the contract, absent from the derived schema"),
                        ));
                    }
                    Some(derived) => {
                        if let Some(found) = first_difference(shape, derived, &path) {
                            return Some(found);
                        }
                    }
                }
            }
            actual
                .iter()
                .find(|(name, _)| !expected.contains_key(*name))
                .map(|(name, shape)| {
                    (
                        join(path, name),
                        format!("{shape} in the derived schema, absent from the contract"),
                    )
                })
        }
        (Shape::Optional(expected), Shape::Optional(actual)) => {
            first_difference(expected, actual, path)
        }
        (Shape::List(expected), Shape::List(actual)) => {
            first_difference(expected, actual, &format!("{path}[]"))
        }
        (Shape::Map(_, expected), Shape::Map(_, actual)) => {
            first_difference(expected, actual, &format!("{path}{{}}"))
        }
        _ if expected == actual => None,
        _ => Some((
            path.to_string(),
            format!("contract {expected}, derived {actual}"),
        )),
    }
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}
