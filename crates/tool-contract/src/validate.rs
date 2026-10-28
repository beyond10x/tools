//! The profile's rules over a parsed [`Contract`]: every rule in `docs/design/tool-contracts.md`
//! § The contract document, each refusal naming the operation and the property.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::document::{Contract, ErrorDecl, Field, Operation, Profile, TypeDecl, TypeKind};
use crate::types::{Shape, Type};

/// The `operation` a document-level violation carries.
pub const DOCUMENT: &str = "document";

/// One refused rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The operation, or [`DOCUMENT`] for a rule over the document as a whole.
    pub operation: String,
    /// The property (`risk`, `bounds`, `subjects`, `capability`, `name`, ...).
    pub property: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.operation, self.property, self.message)
    }
}

/// Applies every rule of the profile.
///
/// # Errors
///
/// Every violation found, in document order.
pub fn validate(contract: &Contract) -> Result<(), Vec<Violation>> {
    let mut found = Vec::new();
    let mut at = |operation: &str, property: &str, message: String| {
        found.push(Violation {
            operation: operation.to_string(),
            property: property.to_string(),
            message,
        });
    };
    check_types(&contract.types, &mut at);
    check_errors(&contract.errors, &contract.types, &mut at);
    let mut names = BTreeSet::new();
    let mut renderings: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    let mut named_errors = BTreeSet::new();
    for op in &contract.operations {
        check_name(contract, op, &mut names, &mut at);
        check_renderings(op, &mut renderings, &mut at);
        check_fields(&op.name, "input", &op.input, &contract.types, &mut at);
        check_fields(&op.name, "output", &op.output, &contract.types, &mut at);
        check_outcomes(contract, op, &mut named_errors, &mut at);
        check_profile(op, &mut at);
    }
    for error in &contract.errors {
        if !named_errors.contains(error.name.as_str()) {
            at(
                DOCUMENT,
                "errors",
                format!("{} is declared but no outcome names it", error.name),
            );
        }
    }
    if found.is_empty() { Ok(()) } else { Err(found) }
}

/// Applies rule `directory` to the `<toolset>` and `<version>` directory names a document sits in:
/// the build script makes each part of a constant and a string literal, and stops every crate
/// building on a name outside `[a-z][a-z0-9-]*`.
///
/// # Errors
///
/// One violation per name outside the grammar, in the order given.
pub fn directories(names: &[&str]) -> Result<(), Vec<Violation>> {
    let found: Vec<Violation> = names
        .iter()
        .filter(|name| !is_directory_name(name))
        .map(|name| Violation {
            operation: DOCUMENT.to_string(),
            property: "directory".to_string(),
            message: format!(
                "{name:?} is not [a-z][a-z0-9-]*, so the build script cannot embed the document"
            ),
        })
        .collect();
    if found.is_empty() { Ok(()) } else { Err(found) }
}

fn is_directory_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

type At<'a> = dyn FnMut(&str, &str, String) + 'a;

fn check_types(types: &[TypeDecl], at: &mut At<'_>) {
    let mut seen = BTreeSet::new();
    for decl in types {
        if !seen.insert(decl.name.as_str()) {
            at(DOCUMENT, "types", format!("duplicate type {}", decl.name));
        }
        let problem = match decl.kind {
            TypeKind::Enum if decl.variants.is_empty() => Some("has no variants"),
            TypeKind::Enum if !decl.fields.is_empty() || decl.ty.is_some() => {
                Some("has fields or a type")
            }
            TypeKind::Struct if decl.fields.is_empty() => Some("has no fields"),
            TypeKind::Struct if !decl.variants.is_empty() || decl.ty.is_some() => {
                Some("has variants or a type")
            }
            TypeKind::Newtype if decl.ty.is_none() => Some("names no type"),
            TypeKind::Newtype if !decl.variants.is_empty() || !decl.fields.is_empty() => {
                Some("has variants or fields")
            }
            _ => None,
        };
        if let Some(problem) = problem {
            at(
                DOCUMENT,
                "types",
                format!("{} is kind {} but {problem}", decl.name, decl.kind),
            );
            continue;
        }
        match decl.kind {
            TypeKind::Struct => {
                check_fields(DOCUMENT, "types", &decl.fields, types, at);
            }
            TypeKind::Newtype => {
                if let Some(ty) = &decl.ty
                    && let Err(err) = resolve(ty, types)
                {
                    at(DOCUMENT, "types", format!("{}: {err}", decl.name));
                }
            }
            TypeKind::Enum => {}
        }
    }
}

fn check_errors(errors: &[ErrorDecl], types: &[TypeDecl], at: &mut At<'_>) {
    let mut seen = BTreeSet::new();
    for error in errors {
        if !seen.insert(error.name.as_str()) {
            at(
                DOCUMENT,
                "errors",
                format!("duplicate error {}", error.name),
            );
        }
        check_fields(DOCUMENT, "errors", &error.fields, types, at);
    }
}

fn check_fields(
    operation: &str,
    property: &str,
    fields: &[Field],
    types: &[TypeDecl],
    at: &mut At<'_>,
) {
    let mut seen = BTreeSet::new();
    for field in fields {
        if !seen.insert(field.name.as_str()) {
            at(
                operation,
                property,
                format!("duplicate field {}", field.name),
            );
        }
        if let Err(err) = resolve(&field.ty, types) {
            at(operation, property, format!("field {}: {err}", field.name));
        }
    }
}

fn resolve(ty: &str, types: &[TypeDecl]) -> Result<Shape, String> {
    let parsed = Type::parse(ty).map_err(|err| format!("type {err}"))?;
    Shape::of_type(&parsed, types).map_err(|err| err.to_string())
}

fn check_name<'a>(
    contract: &Contract,
    op: &'a Operation,
    names: &mut BTreeSet<&'a str>,
    at: &mut At<'_>,
) {
    let family = contract.family();
    let segment = op
        .name
        .strip_prefix(family)
        .and_then(|rest| rest.strip_prefix('.'));
    if !segment.is_some_and(is_segment) {
        at(
            &op.name,
            "name",
            format!(
                "{} is not {family}.<op> with <op> in [a-z][a-z0-9_-]*",
                op.name
            ),
        );
    }
    if !names.insert(op.name.as_str()) {
        at(&op.name, "name", "duplicate operation name".to_string());
    }
}

fn check_renderings<'a>(
    op: &'a Operation,
    renderings: &mut BTreeMap<(&'a str, &'a str), &'a str>,
    at: &mut At<'_>,
) {
    for (harness, rendered) in &op.renderings {
        if let Some(other) = renderings.insert((harness, rendered), &op.name)
            && other != op.name
        {
            at(
                &op.name,
                "renderings",
                format!("{harness} name {rendered} is also used by {other}"),
            );
        }
    }
}

fn check_outcomes<'a>(
    contract: &'a Contract,
    op: &'a Operation,
    named_errors: &mut BTreeSet<&'a str>,
    at: &mut At<'_>,
) {
    let mut seen = BTreeSet::new();
    for outcome in &op.outcomes {
        if !seen.insert(outcome.name.as_str()) {
            at(
                &op.name,
                "outcomes",
                format!("duplicate outcome {}", outcome.name),
            );
        }
        if let Some(error) = &outcome.error {
            match contract.errors.iter().find(|e| &e.name == error) {
                Some(declared) => {
                    named_errors.insert(declared.name.as_str());
                }
                None => at(
                    &op.name,
                    "outcomes",
                    format!("{} names error {error} which is not declared", outcome.name),
                ),
            }
        }
    }
}

fn check_profile(op: &Operation, at: &mut At<'_>) {
    let profile = &op.profile;
    check_risk(op, profile, at);
    check_bounds(op, profile, at);
    let input: BTreeSet<&str> = op.input.iter().map(|f| f.name.as_str()).collect();
    let output: BTreeSet<&str> = op.output.iter().map(|f| f.name.as_str()).collect();
    for subject in &profile.subjects {
        if !input.contains(subject.field.as_str()) {
            at(
                &op.name,
                "subjects",
                format!("field {} is not an input field", subject.field),
            );
        }
    }
    for field in &profile.preconditions {
        if !input.contains(field.as_str()) {
            at(
                &op.name,
                "preconditions",
                format!("field {field} is not an input field"),
            );
        }
    }
    if let Some(paging) = &profile.paging {
        let named = [
            ("offset", paging.offset.as_deref(), &input, "input"),
            ("limit", paging.limit.as_deref(), &input, "input"),
            ("truncated", paging.truncated.as_deref(), &output, "output"),
            ("digest", paging.digest.as_deref(), &output, "output"),
        ];
        if named.iter().all(|(_, field, _, _)| field.is_none()) {
            at(&op.name, "paging", "names no field".to_string());
        }
        for (which, field, side, side_name) in named {
            if let Some(field) = field
                && !side.contains(field)
            {
                at(
                    &op.name,
                    "paging",
                    format!("{which} names {field}, which is not an {side_name} field"),
                );
            }
        }
    }
    if !is_capability(&profile.capability) {
        at(
            &op.name,
            "capability",
            format!(
                "{} is not a Canon capability id ([a-z][a-z0-9_-]*(.[a-z][a-z0-9_-]*)+)",
                profile.capability
            ),
        );
    }
}

fn check_risk(op: &Operation, profile: &Profile, at: &mut At<'_>) {
    let floor = profile.risk_floor();
    if profile.risk < floor {
        let effects: Vec<&str> = profile.effects.iter().map(|e| e.as_str()).collect();
        let consequences: Vec<&str> = profile.consequences.iter().map(|c| c.as_str()).collect();
        at(
            &op.name,
            "risk",
            format!(
                "{} is under the floor {floor} set by effects [{}] and consequences [{}]",
                profile.risk,
                effects.join(", "),
                consequences.join(", ")
            ),
        );
    }
}

fn check_bounds(op: &Operation, profile: &Profile, at: &mut At<'_>) {
    if profile.bounds.timeout_ms.is_none() {
        at(&op.name, "bounds", "timeout_ms is missing".to_string());
    }
    for (name, bound) in profile.bounds.named() {
        match (bound.default, bound.max) {
            (_, None) => at(&op.name, "bounds", format!("{name} has no max")),
            (Some(default), Some(max)) if default > max => at(
                &op.name,
                "bounds",
                format!("{name} default {default} exceeds max {max}"),
            ),
            _ => {}
        }
    }
}

fn is_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Whether `text` is a Canon capability id: `[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)+`.
pub fn is_capability(text: &str) -> bool {
    let segments: Vec<&str> = text.split('.').collect();
    segments.len() >= 2 && segments.iter().all(|s| is_segment(s))
}

#[cfg(test)]
mod tests {
    use super::{is_capability, is_directory_name};

    #[test]
    fn a_directory_name_is_lowercase_digits_and_hyphens_after_a_letter() {
        assert!(is_directory_name("build-test2"));
        assert!(is_directory_name("v1alpha1"));
        assert!(!is_directory_name("build_test"));
        assert!(!is_directory_name("v1.2"));
        assert!(!is_directory_name("V1"));
        assert!(!is_directory_name("1v"));
        assert!(!is_directory_name(""));
    }

    #[test]
    fn a_capability_is_two_or_more_dotted_lowercase_segments() {
        assert!(is_capability("workspace.read"));
        assert!(is_capability("code.write-files_2"));
        assert!(!is_capability("workspace"));
        assert!(!is_capability("Workspace.read"));
        assert!(!is_capability("workspace..read"));
        assert!(!is_capability("workspace.9read"));
        assert!(!is_capability(""));
    }
}
