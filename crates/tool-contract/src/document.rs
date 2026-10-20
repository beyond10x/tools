//! The `tool-contract/1` document: [`Contract`] and its parts, read from YAML with unknown fields
//! refused. The grammar is `docs/design/tool-contracts.md` § The contract document; the rules the
//! validator applies to a parsed document are in [`crate::validate`].

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The `format` every contract document carries.
pub const FORMAT: &str = "tool-contract/1";

/// Why a document could not be read or parsed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: io::Error,
    },
    /// The text is not a document of this grammar.
    #[error("{}{source}", at(path.as_deref()))]
    Yaml {
        /// The file, when read from one.
        path: Option<PathBuf>,
        /// The underlying error.
        #[source]
        source: serde_yaml_ng::Error,
    },
    /// The document's `format` is another one.
    #[error("{}format is {found:?}, expected {FORMAT:?}", at(path.as_deref()))]
    Format {
        /// The file, when read from one.
        path: Option<PathBuf>,
        /// The `format` the document carries.
        found: String,
    },
}

impl Error {
    fn at(self, path: &Path) -> Self {
        match self {
            Self::Yaml { source, .. } => Self::Yaml {
                path: Some(path.to_path_buf()),
                source,
            },
            Self::Format { found, .. } => Self::Format {
                path: Some(path.to_path_buf()),
                found,
            },
            other @ Self::Io { .. } => other,
        }
    }
}

pub(crate) fn at(path: Option<&Path>) -> String {
    path.map(|p| format!("{}: ", p.display()))
        .unwrap_or_default()
}

/// One `tool-contract/1` document: a toolset version and its operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// Always [`FORMAT`].
    pub format: String,
    /// The toolset, which is also the directory under `contracts/`.
    pub toolset: String,
    /// The family prefix every operation name carries, when it differs from the toolset
    /// (`code` for the `code-editor` toolset). Defaults to `toolset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// The toolset version, such as `v1alpha1`.
    pub version: String,
    /// One paragraph.
    pub summary: String,
    /// The named types the field lists use.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeDecl>,
    /// The errors the outcomes name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorDecl>,
    /// The operations.
    pub operations: Vec<Operation>,
}

impl Contract {
    /// Parses one document.
    ///
    /// # Errors
    ///
    /// The text is not YAML of this grammar, or its `format` is not [`FORMAT`].
    pub fn from_yaml(text: &str) -> Result<Self, Error> {
        let contract: Self =
            serde_yaml_ng::from_str(text).map_err(|source| Error::Yaml { path: None, source })?;
        if contract.format == FORMAT {
            Ok(contract)
        } else {
            Err(Error::Format {
                path: None,
                found: contract.format,
            })
        }
    }

    /// Reads and parses one document from a file.
    ///
    /// # Errors
    ///
    /// The file cannot be read, or [`Self::from_yaml`] refuses its text.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_yaml(&text).map_err(|err| err.at(path))
    }

    /// The prefix every operation name carries: `family` when set, else `toolset`.
    pub fn family(&self) -> &str {
        self.family.as_deref().unwrap_or(&self.toolset)
    }

    /// The operation with this name.
    pub fn operation(&self, name: &str) -> Option<&Operation> {
        self.operations.iter().find(|op| op.name == name)
    }

    /// The named type with this name.
    pub fn type_decl(&self, name: &str) -> Option<&TypeDecl> {
        self.types.iter().find(|decl| decl.name == name)
    }

    /// The declared error with this name.
    pub fn error_decl(&self, name: &str) -> Option<&ErrorDecl> {
        self.errors.iter().find(|decl| decl.name == name)
    }
}

/// A named type: an `enum` of unit variants, a `struct` of fields, or a `newtype` over one type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDecl {
    /// The name the field lists refer to, such as `fs.Kind`.
    pub name: String,
    /// Which kind of type it is.
    pub kind: TypeKind,
    /// One line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The variants of an `enum`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    /// The fields of a `struct`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// The type a `newtype` wraps.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
}

/// A typed, named field of an input, an output, an error or a `struct` type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// The field name.
    pub name: String,
    /// The type, in the grammar [`crate::types::Type`] parses.
    #[serde(rename = "type")]
    pub ty: String,
    /// What the field holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An error an operation can return, named by an outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorDecl {
    /// The error name, such as `fs.NotFound`.
    pub name: String,
    /// One line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The error's fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}

/// One operation: an ESS-shaped command plus its profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// `<family>.<op>`.
    pub name: String,
    /// One line a reader chooses between operations by.
    pub summary: String,
    /// How to call it well; presentation, kept apart from `summary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
    /// Generated names per harness; never the source of the name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub renderings: BTreeMap<String, String>,
    /// The input fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<Field>,
    /// The output fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<Field>,
    /// The outcomes; every error the operation can return is one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<Outcome>,
    /// The `tool-contract/1` profile.
    pub profile: Profile,
}

impl Operation {
    /// Whether one of this operation's outcomes returns the error named `error`.
    pub fn names_error(&self, error: &str) -> bool {
        self.outcomes
            .iter()
            .any(|outcome| outcome.error.as_deref() == Some(error))
    }

    /// Whether `field` is one of this operation's input fields.
    pub fn has_input(&self, field: &str) -> bool {
        self.input.iter().any(|f| f.name == field)
    }
}

/// One outcome of an operation; an error outcome names the error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    /// The outcome name.
    pub name: String,
    /// One line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The declared error this outcome returns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The `tool-contract/1` profile of one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// What a call can touch.
    pub effects: Vec<Effect>,
    /// What a call can bring about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consequences: Vec<Consequence>,
    /// Never under the floor the effects and consequences set.
    pub risk: Risk,
    /// Whether a repeated call is safe.
    pub idempotency: Idempotency,
    /// Input fields that are preconditions, such as `expected_sha256`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<String>,
    /// Whether the same input gives the same output.
    pub determinism: Determinism,
    /// Every bound names a default and a maximum; `timeout_ms` is required.
    #[serde(default)]
    pub bounds: Bounds,
    /// The paging fields, when the operation pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paging: Option<Paging>,
    /// What the operation needs from its host.
    #[serde(default)]
    pub requires: Requires,
    /// The input fields that name what a call touches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subjects: Vec<Subject>,
    /// Where the operation runs.
    pub placement: Placement,
    /// The Canon capability the operation serves.
    pub capability: String,
    /// How the output is to be treated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMarks>,
}

/// The bounds a call runs under.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    /// Wall time, in milliseconds. Required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<Bound>,
    /// Output size, in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_bytes: Option<Bound>,
    /// Processes a call may start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processes: Option<Bound>,
    /// CPU time, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_ms: Option<Bound>,
    /// Memory, in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<Bound>,
}

impl Bounds {
    /// The bounds that are set, by name.
    pub fn named(&self) -> impl Iterator<Item = (&'static str, &Bound)> {
        [
            ("timeout_ms", self.timeout_ms.as_ref()),
            ("output_bytes", self.output_bytes.as_ref()),
            ("processes", self.processes.as_ref()),
            ("cpu_ms", self.cpu_ms.as_ref()),
            ("memory_bytes", self.memory_bytes.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, bound)| bound.map(|b| (name, b)))
    }

    /// Whether any bound is set.
    pub fn is_empty(&self) -> bool {
        self.named().next().is_none()
    }
}

/// One bound: the value a call gets when it asks for none, and the most it may ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bound {
    /// The value when the call names none; `max` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<u64>,
    /// The most a call may ask for. Required by the validator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
}

/// The fields an operation pages with: `offset` and `limit` on the input, `truncated` and
/// `digest` on the output.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paging {
    /// The input field that names where the page starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    /// The input field that names how much the page holds at most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    /// The output field that says whether anything was left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<String>,
    /// The output field that carries the digest of the whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// What an operation needs from its host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    /// Programs on the path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binaries: Vec<String>,
    /// Environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
    /// Settings of the harness.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<String>,
    /// Secrets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}

/// An input field that names what a call touches, and the permission pattern it suggests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// The input field.
    pub field: String,
    /// What the field names.
    pub kind: SubjectKind,
    /// The permission pattern, such as `fs.read({path})`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// How an operation's output is to be treated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputMarks {
    /// The output carries content a caller did not write and must not trust.
    #[serde(default)]
    pub untrusted: bool,
    /// A failure comes back as a result, not as a transport error.
    #[serde(default)]
    pub failure_as_result: bool,
}

macro_rules! kebab_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum $name { $($variant),+ }

        impl $name {
            /// The value as the document spells it.
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

kebab_enum! {
    /// The kind of a named type.
    TypeKind { Enum => "enum", Struct => "struct", Newtype => "newtype" }
}

kebab_enum! {
    /// What a call can touch.
    Effect {
        Read => "read",
        Write => "write",
        Process => "process",
        Network => "network",
        Secret => "secret",
        External => "external",
        HumanVisible => "human-visible",
    }
}

kebab_enum! {
    /// What a call can bring about.
    Consequence {
        Money => "money",
        Delete => "delete",
        SendExternal => "send-external",
        Irreversible => "irreversible",
        SensitiveData => "sensitive-data",
    }
}

kebab_enum! {
    /// The risk of an operation, ordered from `none` to `critical`.
    Risk {
        None => "none",
        Low => "low",
        Medium => "medium",
        High => "high",
        Critical => "critical",
    }
}

kebab_enum! {
    /// Whether a repeated call is safe.
    Idempotency {
        Idempotent => "idempotent",
        Keyed => "keyed",
        NonIdempotent => "non-idempotent",
    }
}

kebab_enum! {
    /// Whether the same input gives the same output.
    Determinism { Deterministic => "deterministic", Nondeterministic => "nondeterministic" }
}

kebab_enum! {
    /// Where the operation runs.
    Placement { Local => "local", Confined => "confined", Container => "container" }
}

kebab_enum! {
    /// What a subject field names.
    SubjectKind { Path => "path", Url => "url", Argv => "argv" }
}

impl Effect {
    /// The least risk an operation with this effect may declare.
    pub fn floor(self) -> Risk {
        match self {
            Self::Read => Risk::None,
            Self::Network | Self::Write | Self::HumanVisible => Risk::Low,
            Self::Process | Self::Secret => Risk::Medium,
            Self::External => Risk::High,
        }
    }
}

impl Consequence {
    /// The least risk an operation with this consequence may declare.
    pub fn floor(self) -> Risk {
        match self {
            Self::Delete | Self::SensitiveData => Risk::Medium,
            Self::Irreversible | Self::Money | Self::SendExternal => Risk::High,
        }
    }
}

impl Profile {
    /// The least risk the effects and consequences allow.
    pub fn risk_floor(&self) -> Risk {
        self.effects
            .iter()
            .map(|e| e.floor())
            .chain(self.consequences.iter().map(|c| c.floor()))
            .max()
            .unwrap_or(Risk::None)
    }
}
