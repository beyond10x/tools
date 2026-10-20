//! The `tool-conformance/1` suite ([`Suite`]), the [`Toolset`] an implementation exposes, and
//! [`run`], which lays out each scenario's fixture in its own workspace, invokes, matches the
//! result and the tree afterwards, and checks the derived schemas against the contract once.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use schemars::Schema;
use serde::Deserialize;
use serde_json::Value;

use crate::digest::sha256_hex;
use crate::document::{Contract, Profile, at};
use crate::types::{Side, check_shapes};

/// The `format` every suite document carries.
pub const FORMAT: &str = "tool-conformance/1";

/// The profile properties a scenario can be evidence for. Each one present and non-empty in an
/// operation's profile needs a scenario on that operation whose `holds` names it.
pub const BEHAVIOURAL: [&str; 8] = [
    "effects",
    "idempotency",
    "determinism",
    "bounds",
    "paging",
    "preconditions",
    "subjects",
    "output",
];

/// The profile properties the validator alone checks; no scenario is asked for them.
pub const DECLARATIVE: [&str; 5] = [
    "risk",
    "consequences",
    "requires",
    "placement",
    "capability",
];

/// Why a suite could not be read or parsed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or directory could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The path.
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
    /// A scenario is malformed.
    #[error("{}scenario {name}: {problem}", at(path.as_deref()))]
    Scenario {
        /// The file, when read from one.
        path: Option<PathBuf>,
        /// The scenario.
        name: String,
        /// What is wrong.
        problem: String,
    },
    /// The directory holds no `.yaml` document.
    #[error("{}: no scenario documents", dir.display())]
    Empty {
        /// The directory.
        dir: PathBuf,
    },
    /// A document names another toolset or version than the first one read.
    #[error("{}: {found} differs from {expected} in the first document", path.display())]
    Mixed {
        /// The document.
        path: PathBuf,
        /// What the first document said.
        expected: String,
        /// What this one says.
        found: String,
    },
}

impl Error {
    fn at(self, path: &Path) -> Self {
        let located = Some(path.to_path_buf());
        match self {
            Self::Yaml { source, .. } => Self::Yaml {
                path: located,
                source,
            },
            Self::Format { found, .. } => Self::Format {
                path: located,
                found,
            },
            Self::Scenario { name, problem, .. } => Self::Scenario {
                path: located,
                name,
                problem,
            },
            other => other,
        }
    }
}

/// One `tool-conformance/1` suite: the scenarios of one toolset version.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    /// Always [`FORMAT`].
    pub format: String,
    /// The toolset the scenarios exercise.
    pub toolset: String,
    /// The toolset version.
    pub version: String,
    /// The scenarios.
    pub scenarios: Vec<Scenario>,
    /// The contract directory fixture `dir`s resolve against: the parent of the `scenarios/`
    /// directory the suite was read from. `None` for a suite parsed from text.
    #[serde(skip)]
    pub base: Option<PathBuf>,
}

impl Suite {
    /// Parses one suite document.
    ///
    /// # Errors
    ///
    /// The text is not YAML of this grammar, its `format` is not [`FORMAT`], or a scenario is
    /// malformed (no `outcome: ok` or `error`, both, a name unfit for a directory, a duplicate).
    pub fn from_yaml(text: &str) -> Result<Self, Error> {
        let suite: Self =
            serde_yaml_ng::from_str(text).map_err(|source| Error::Yaml { path: None, source })?;
        if suite.format != FORMAT {
            return Err(Error::Format {
                path: None,
                found: suite.format,
            });
        }
        suite.check_scenarios()?;
        Ok(suite)
    }

    /// Reads one suite document; fixture `dir`s resolve against the parent of its directory.
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
        let mut suite = Self::from_yaml(&text).map_err(|err| err.at(path))?;
        suite.base = path.parent().and_then(Path::parent).map(Path::to_path_buf);
        Ok(suite)
    }

    /// Reads every `.yaml` document under `scenarios_dir`, in name order, into one suite.
    /// Fixture `dir`s resolve against the parent of `scenarios_dir`.
    ///
    /// # Errors
    ///
    /// The directory cannot be read or holds no document, a document is refused, or the
    /// documents name different toolsets or versions.
    pub fn read_dir(scenarios_dir: impl AsRef<Path>) -> Result<Self, Error> {
        let dir = scenarios_dir.as_ref();
        let io = |source| Error::Io {
            path: dir.to_path_buf(),
            source,
        };
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(io)?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<Result<_, _>>()
            .map_err(io)?;
        paths.retain(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|ext| ext == "yaml" || ext == "yml")
        });
        paths.sort();
        let mut merged: Option<Self> = None;
        for path in paths {
            let suite = Self::read(&path)?;
            match &mut merged {
                None => merged = Some(suite),
                Some(first) => {
                    for (expected, found) in [
                        (&first.toolset, &suite.toolset),
                        (&first.version, &suite.version),
                    ] {
                        if expected != found {
                            return Err(Error::Mixed {
                                path,
                                expected: expected.clone(),
                                found: found.clone(),
                            });
                        }
                    }
                    first.scenarios.extend(suite.scenarios);
                }
            }
        }
        let mut suite = merged.ok_or_else(|| Error::Empty {
            dir: dir.to_path_buf(),
        })?;
        suite.base = dir.parent().map(Path::to_path_buf);
        suite.check_scenarios()?;
        Ok(suite)
    }

    fn check_scenarios(&self) -> Result<(), Error> {
        let mut seen = std::collections::BTreeSet::new();
        for scenario in &self.scenarios {
            let refuse = |problem: &str| Error::Scenario {
                path: None,
                name: scenario.name.clone(),
                problem: problem.to_string(),
            };
            if !is_directory_name(&scenario.name) {
                return Err(refuse("the name must be usable as a directory name"));
            }
            if !seen.insert(scenario.name.as_str()) {
                return Err(refuse("duplicate scenario name"));
            }
            match (&scenario.expect.outcome, &scenario.expect.error) {
                (Some(outcome), None) if outcome == "ok" => {}
                (Some(outcome), None) => {
                    return Err(refuse(&format!(
                        "expect.outcome is {outcome:?}, only ok is allowed"
                    )));
                }
                (None, Some(_)) => {}
                (Some(_), Some(_)) => {
                    return Err(refuse("expect names both outcome: ok and error"));
                }
                (None, None) => return Err(refuse("expect names neither outcome: ok nor error")),
            }
        }
        Ok(())
    }
}

fn is_directory_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && !name.contains(['/', '\\', '\0'])
}

/// One scenario: a fixture tree, one call, and what to expect of the result and the tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The name; also the workspace directory under the scratch root.
    pub name: String,
    /// The profile properties this scenario is evidence for.
    #[serde(default)]
    pub holds: Vec<String>,
    /// The tree laid out before the call.
    #[serde(default)]
    pub fixture: Fixture,
    /// The call.
    pub call: Call,
    /// What to expect.
    pub expect: Expect,
}

/// The tree a scenario starts from.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// Files by workspace-relative path, with their text.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// A directory to copy in first, relative to the suite's [`Suite::base`].
    #[serde(default)]
    pub dir: Option<PathBuf>,
}

/// The one call a scenario makes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// The operation.
    pub operation: String,
    /// The input, as JSON.
    #[serde(default = "empty_object")]
    pub input: Value,
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// What a scenario expects of the result and of the tree afterwards.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// `ok`, when the call succeeds.
    #[serde(default)]
    pub outcome: Option<String>,
    /// The error name, when the call fails.
    #[serde(default)]
    pub error: Option<String>,
    /// A subset of the output (or of the error's fields), with matchers.
    #[serde(default)]
    pub output: Option<Value>,
    /// Post-conditions on files by workspace-relative path.
    #[serde(default)]
    pub files: BTreeMap<String, FileExpect>,
}

/// A post-condition on one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileExpect {
    /// The file has the content the fixture gave it.
    Unchanged,
    /// There is no such file.
    Absent,
    /// The file holds exactly this text.
    Equals(String),
    /// The file's SHA-256 is this hex digest.
    Sha256(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FileExpectRepr {
    Keyword(FileKeyword),
    Equals { equals: String },
    Sha256 { sha256: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FileKeyword {
    Unchanged,
    Absent,
}

impl<'de> Deserialize<'de> for FileExpect {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match FileExpectRepr::deserialize(deserializer)? {
            FileExpectRepr::Keyword(FileKeyword::Unchanged) => Self::Unchanged,
            FileExpectRepr::Keyword(FileKeyword::Absent) => Self::Absent,
            FileExpectRepr::Equals { equals } => Self::Equals(equals),
            FileExpectRepr::Sha256 { sha256 } => Self::Sha256(sha256),
        })
    }
}

/// What one invocation returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The output.
    Ok(Value),
    /// A declared error, with its fields.
    Error {
        /// The error name the contract declares.
        name: String,
        /// One line for a reader.
        message: String,
        /// The error's fields, as JSON.
        fields: Value,
    },
}

/// What an implementation of a toolset exposes to the runner.
pub trait Toolset {
    /// The parsed document it claims to implement.
    fn contract(&self) -> &Contract;
    /// Per operation: its name, the derived input schema, the derived output schema.
    fn schemas(&self) -> Vec<(String, Schema, Schema)>;
    /// Runs one operation over this toolset's workspace.
    fn invoke(&self, operation: &str, input: &Value) -> Outcome;
}

/// The result of one scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResult {
    /// The scenario.
    pub name: String,
    /// Whether every expectation held.
    pub passed: bool,
    /// The first expectation that did not hold; empty when `passed`.
    pub detail: String,
}

/// What one run of a suite found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Why the suite was refused before any scenario ran; empty when it was not.
    pub refused: Vec<String>,
    /// One result per scenario, in suite order.
    pub scenarios: Vec<ScenarioResult>,
    /// Derived schemas that differ from the contract, one line each.
    pub schema_mismatches: Vec<String>,
}

impl Report {
    /// Whether the suite ran, every scenario passed and every schema matched.
    pub fn passed(&self) -> bool {
        self.refused.is_empty()
            && self.schema_mismatches.is_empty()
            && self.scenarios.iter().all(|s| s.passed)
    }

    /// The scenarios that did not pass.
    pub fn failed(&self) -> impl Iterator<Item = &ScenarioResult> {
        self.scenarios.iter().filter(|s| !s.passed)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.refused {
            writeln!(f, "refused: {line}")?;
        }
        for result in &self.scenarios {
            if result.passed {
                writeln!(f, "ok   {}", result.name)?;
            } else {
                writeln!(f, "FAIL {}: {}", result.name, result.detail)?;
            }
        }
        for line in &self.schema_mismatches {
            writeln!(f, "schema: {line}")?;
        }
        let passed = self.scenarios.iter().filter(|s| s.passed).count();
        write!(f, "{passed} of {} scenarios passed", self.scenarios.len())?;
        if !self.refused.is_empty() {
            write!(f, ", suite refused ({})", self.refused.len())?;
        }
        if !self.schema_mismatches.is_empty() {
            write!(f, ", {} schema mismatches", self.schema_mismatches.len())?;
        }
        writeln!(f)
    }
}

/// The behavioural properties present and non-empty in a profile.
pub fn declared_properties(profile: &Profile) -> Vec<&'static str> {
    let mut present = Vec::new();
    if !profile.effects.is_empty() {
        present.push("effects");
    }
    present.push("idempotency");
    present.push("determinism");
    if !profile.bounds.is_empty() {
        present.push("bounds");
    }
    if profile.paging.is_some() {
        present.push("paging");
    }
    if !profile.preconditions.is_empty() {
        present.push("preconditions");
    }
    if !profile.subjects.is_empty() {
        present.push("subjects");
    }
    if profile.output.is_some() {
        present.push("output");
    }
    present
}

/// Refuses a suite that is not evidence for the contract: a declared behavioural property of an
/// operation with no scenario on that operation whose `holds` names it, a `holds` entry that is
/// not a profile property, a call to an operation the contract does not declare, or a toolset or
/// version that is not the contract's.
///
/// # Errors
///
/// Every refusal, each naming the operation and the property or the scenario.
pub fn coverage(suite: &Suite, contract: &Contract) -> Result<(), Vec<String>> {
    let mut refused = Vec::new();
    if suite.toolset != contract.toolset {
        refused.push(format!(
            "suite: toolset {} is not the contract's {}",
            suite.toolset, contract.toolset
        ));
    }
    if suite.version != contract.version {
        refused.push(format!(
            "suite: version {} is not the contract's {}",
            suite.version, contract.version
        ));
    }
    for scenario in &suite.scenarios {
        match contract.operation(&scenario.call.operation) {
            None => refused.push(format!(
                "{}: call: {} is not an operation of {}",
                scenario.name, scenario.call.operation, contract.toolset
            )),
            Some(op) => {
                if let Some(error) = &scenario.expect.error
                    && !op.names_error(error)
                {
                    refused.push(format!(
                        "{}: expect: {error} is not an outcome of {}",
                        scenario.name, op.name
                    ));
                }
                if let Some(input) = scenario.call.input.as_object() {
                    for key in input.keys().filter(|key| !op.has_input(key)) {
                        refused.push(format!(
                            "{}: call: {key} is not an input field of {}",
                            scenario.name, op.name
                        ));
                    }
                }
            }
        }
        for held in &scenario.holds {
            let known =
                BEHAVIOURAL.contains(&held.as_str()) || DECLARATIVE.contains(&held.as_str());
            if !known {
                refused.push(format!(
                    "{}: holds: {held} is not a profile property",
                    scenario.name
                ));
            }
        }
    }
    for op in &contract.operations {
        for property in declared_properties(&op.profile) {
            let held = suite
                .scenarios
                .iter()
                .any(|s| s.call.operation == op.name && s.holds.iter().any(|h| h == property));
            if !held {
                refused.push(format!("{}: {property}: no scenario holds it", op.name));
            }
        }
    }
    if refused.is_empty() {
        Ok(())
    } else {
        Err(refused)
    }
}

/// Runs `suite` against the toolset `make` builds over each scenario's workspace under `scratch`.
///
/// A suite [`coverage`] refuses is reported as refused and runs nothing. Otherwise each scenario
/// gets `<scratch>/<name>/` as a fresh workspace with its fixture laid out, the call is made,
/// `expect` is matched, the file post-conditions are checked; then the toolset's derived schemas
/// are checked against the contract once, over `<scratch>/.schemas/`.
pub fn run(
    suite: &Suite,
    contract: &Contract,
    scratch: &Path,
    make: impl Fn(&Path) -> Box<dyn Toolset>,
) -> Report {
    let mut report = Report::default();
    if let Err(refused) = coverage(suite, contract) {
        report.refused = refused;
        return report;
    }
    for scenario in &suite.scenarios {
        let detail = run_scenario(suite, contract, scenario, scratch, &make).err();
        report.scenarios.push(ScenarioResult {
            name: scenario.name.clone(),
            passed: detail.is_none(),
            detail: detail.unwrap_or_default(),
        });
    }
    report.schema_mismatches = check_schemas(contract, &scratch.join(".schemas"), &make);
    report
}

fn run_scenario(
    suite: &Suite,
    contract: &Contract,
    scenario: &Scenario,
    scratch: &Path,
    make: &dyn Fn(&Path) -> Box<dyn Toolset>,
) -> Result<(), String> {
    let op = contract
        .operation(&scenario.call.operation)
        .ok_or_else(|| {
            format!(
                "{} is not an operation of the contract",
                scenario.call.operation
            )
        })?;
    let workspace = scratch.join(&scenario.name);
    lay_out(suite, &scenario.fixture, &workspace)?;
    let before = snapshot(&workspace)?;
    let toolset = make(&workspace);
    let outcome = toolset.invoke(&scenario.call.operation, &scenario.call.input);
    if let Outcome::Error {
        name,
        message,
        fields,
    } = &outcome
    {
        if !op.names_error(name) {
            return Err(format!(
                "returned error {name} is not an outcome of {}: {message}",
                op.name
            ));
        }
        check_error_fields(contract, name, fields)?;
    }
    let actual = match (&scenario.expect.error, &outcome) {
        (None, Outcome::Ok(output)) => output,
        (
            Some(expected),
            Outcome::Error {
                name,
                message,
                fields,
            },
        ) => {
            if name != expected {
                return Err(format!(
                    "expected error {expected}, got error {name}: {message}"
                ));
            }
            fields
        }
        (None, Outcome::Error { name, message, .. }) => {
            return Err(format!("expected outcome ok, got error {name}: {message}"));
        }
        (Some(expected), Outcome::Ok(output)) => {
            return Err(format!("expected error {expected}, got ok: {output}"));
        }
    };
    if let Some(expected) = &scenario.expect.output {
        match_value(expected, actual, &workspace, "output")?;
    }
    for (rel, expect) in &scenario.expect.files {
        check_file(&workspace, rel, expect, &before)?;
    }
    Ok(())
}

fn check_error_fields(contract: &Contract, error: &str, fields: &Value) -> Result<(), String> {
    let declared = contract
        .error_decl(error)
        .ok_or_else(|| format!("returned error {error} is not declared by the contract"))?;
    let Some(returned) = fields.as_object() else {
        return Err(format!(
            "returned error {error}: fields is {fields}, not an object"
        ));
    };
    if let Some(missing) = declared
        .fields
        .iter()
        .find(|field| !returned.contains_key(&field.name))
    {
        return Err(format!(
            "returned error {error}: declared field {} is missing",
            missing.name
        ));
    }
    if let Some(extra) = returned
        .keys()
        .find(|key| !declared.fields.iter().any(|field| &field.name == *key))
    {
        return Err(format!(
            "returned error {error}: field {extra} is not declared"
        ));
    }
    Ok(())
}

fn lay_out(suite: &Suite, fixture: &Fixture, workspace: &Path) -> Result<(), String> {
    let io = |what: &str, path: &Path, err: io::Error| format!("{what} {}: {err}", path.display());
    if workspace.symlink_metadata().is_ok() {
        fs::remove_dir_all(workspace).map_err(|e| io("cannot clear", workspace, e))?;
    }
    fs::create_dir_all(workspace).map_err(|e| io("cannot create", workspace, e))?;
    if let Some(dir) = &fixture.dir {
        let Some(base) = &suite.base else {
            return Err(format!(
                "fixture dir {} needs a suite read from a directory",
                dir.display()
            ));
        };
        copy_tree(&base.join(dir), workspace)?;
    }
    for (rel, text) in &fixture.files {
        let path = workspace.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| io("cannot create", parent, e))?;
        }
        fs::write(&path, text).map_err(|e| io("cannot write", &path, e))?;
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let io = |path: &Path, err: io::Error| format!("fixture dir {}: {err}", path.display());
    for entry in fs::read_dir(from).map_err(|e| io(from, e))? {
        let entry = entry.map_err(|e| io(from, e))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = source
            .symlink_metadata()
            .map_err(|e| io(&source, e))?
            .file_type();
        if kind.is_symlink() {
            let link = fs::read_link(&source).map_err(|e| io(&source, e))?;
            make_symlink(&link, &target).map_err(|e| io(&target, e))?;
        } else if kind.is_dir() {
            fs::create_dir_all(&target).map_err(|e| io(&target, e))?;
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| io(&source, e))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_symlink(link: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(link, at)
}

#[cfg(not(unix))]
fn make_symlink(_link: &Path, _at: &Path) -> io::Result<()> {
    Err(io::Error::other(
        "a symlink in a fixture needs a unix platform",
    ))
}

/// What a workspace entry held when the fixture was laid out: a file's bytes, by digest, or a
/// symlink's target as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Content {
    File(String),
    Link(PathBuf),
}

fn content(path: &Path) -> Result<Content, String> {
    let io = |err: io::Error| format!("{}: {err}", path.display());
    let kind = path.symlink_metadata().map_err(io)?.file_type();
    if kind.is_symlink() {
        Ok(Content::Link(fs::read_link(path).map_err(io)?))
    } else {
        Ok(Content::File(sha256_hex(&fs::read(path).map_err(io)?)))
    }
}

fn snapshot(workspace: &Path) -> Result<BTreeMap<String, Content>, String> {
    let mut contents = BTreeMap::new();
    walk(workspace, workspace, &mut contents)?;
    Ok(contents)
}

fn walk(root: &Path, dir: &Path, contents: &mut BTreeMap<String, Content>) -> Result<(), String> {
    let io = |path: &Path, err: io::Error| format!("workspace {}: {err}", path.display());
    for entry in fs::read_dir(dir).map_err(|e| io(dir, e))? {
        let entry = entry.map_err(|e| io(dir, e))?;
        let path = entry.path();
        let kind = path
            .symlink_metadata()
            .map_err(|e| io(&path, e))?
            .file_type();
        if kind.is_dir() {
            walk(root, &path, contents)?;
        } else if kind.is_file() || kind.is_symlink() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("workspace {}: {e}", path.display()))?;
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            contents.insert(rel, content(&path).map_err(|e| format!("workspace {e}"))?);
        }
    }
    Ok(())
}

fn match_value(expected: &Value, actual: &Value, workspace: &Path, at: &str) -> Result<(), String> {
    if let Some((matcher, argument)) = as_matcher(expected) {
        return apply_matcher(matcher, argument, Some(actual), workspace, at);
    }
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                let path = format!("{at}.{key}");
                match (as_matcher(value), actual.get(key)) {
                    (Some((matcher, argument)), found) => {
                        apply_matcher(matcher, argument, found, workspace, &path)?;
                    }
                    (None, Some(found)) => match_value(value, found, workspace, &path)?,
                    (None, None) => return Err(format!("{path}: expected {value}, absent")),
                }
            }
            Ok(())
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                return Err(format!(
                    "{at}: expected {} items, got {}",
                    expected.len(),
                    actual.len()
                ));
            }
            for (i, (value, found)) in expected.iter().zip(actual).enumerate() {
                match_value(value, found, workspace, &format!("{at}[{i}]"))?;
            }
            Ok(())
        }
        (expected, actual) if expected == actual => Ok(()),
        (expected, actual) => Err(format!("{at}: expected {expected}, got {actual}")),
    }
}

fn as_matcher(value: &Value) -> Option<(&str, &Value)> {
    let object = value.as_object().filter(|o| o.len() == 1)?;
    let (key, argument) = object.iter().next()?;
    key.starts_with('$').then_some((key.as_str(), argument))
}

fn apply_matcher(
    matcher: &str,
    argument: &Value,
    actual: Option<&Value>,
    workspace: &Path,
    at: &str,
) -> Result<(), String> {
    let present = actual.filter(|v| !v.is_null());
    if matcher == "$absent" {
        return match present {
            None => Ok(()),
            Some(found) => Err(format!("{at}: expected absent, got {found}")),
        };
    }
    let Some(actual) = present else {
        return Err(format!("{at}: expected {matcher} {argument}, absent"));
    };
    match matcher {
        "$any" => Ok(()),
        "$sha256_of" => {
            let rel = argument
                .as_str()
                .ok_or_else(|| format!("{at}: $sha256_of takes a path"))?;
            let bytes = fs::read(workspace.join(rel))
                .map_err(|e| format!("{at}: $sha256_of {rel}: {e}"))?;
            let digest = sha256_hex(&bytes);
            if actual.as_str() == Some(digest.as_str()) {
                Ok(())
            } else {
                Err(format!(
                    "{at}: expected the sha256 of {rel} ({digest}), got {actual}"
                ))
            }
        }
        "$contains" => match actual {
            Value::String(text) => {
                let needle = argument
                    .as_str()
                    .ok_or_else(|| format!("{at}: $contains on a string takes a string"))?;
                if text.contains(needle) {
                    Ok(())
                } else {
                    Err(format!(
                        "{at}: expected a string containing {needle:?}, got {actual}"
                    ))
                }
            }
            Value::Array(items) => {
                if items.contains(argument) {
                    Ok(())
                } else {
                    Err(format!(
                        "{at}: expected a list containing {argument}, got {actual}"
                    ))
                }
            }
            other => Err(format!(
                "{at}: $contains needs a string or a list, got {other}"
            )),
        },
        "$min_len" => {
            let least = argument
                .as_u64()
                .ok_or_else(|| format!("{at}: $min_len takes a non-negative integer"))?;
            let length = match actual {
                Value::String(text) => text.chars().count(),
                Value::Array(items) => items.len(),
                Value::Object(fields) => fields.len(),
                other => {
                    return Err(format!(
                        "{at}: $min_len needs a string, list or object, got {other}"
                    ));
                }
            };
            if u64::try_from(length).is_ok_and(|len| len >= least) {
                Ok(())
            } else {
                Err(format!(
                    "{at}: expected a length of at least {least}, got {length}"
                ))
            }
        }
        other => Err(format!("{at}: unknown matcher {other}")),
    }
}

fn check_file(
    workspace: &Path,
    rel: &str,
    expect: &FileExpect,
    before: &BTreeMap<String, Content>,
) -> Result<(), String> {
    let path = workspace.join(rel);
    match expect {
        FileExpect::Unchanged => {
            let Some(was) = before.get(rel) else {
                return Err(format!(
                    "files: {rel}: expected unchanged, but the fixture had no such file"
                ));
            };
            let now =
                content(&path).map_err(|e| format!("files: {rel}: expected unchanged, {e}"))?;
            if now == *was {
                Ok(())
            } else {
                Err(format!("files: {rel}: expected unchanged, content differs"))
            }
        }
        FileExpect::Absent => {
            if path.symlink_metadata().is_ok() {
                Err(format!("files: {rel}: expected absent, exists"))
            } else {
                Ok(())
            }
        }
        FileExpect::Equals(text) => {
            let now = fs::read_to_string(&path).map_err(|e| format!("files: {rel}: {e}"))?;
            if now == *text {
                Ok(())
            } else {
                Err(format!("files: {rel}: expected {text:?}, got {now:?}"))
            }
        }
        FileExpect::Sha256(hex) => {
            let now = fs::read(&path).map_err(|e| format!("files: {rel}: {e}"))?;
            let digest = sha256_hex(&now);
            if digest == *hex {
                Ok(())
            } else {
                Err(format!("files: {rel}: expected sha256 {hex}, got {digest}"))
            }
        }
    }
}

fn check_schemas(
    contract: &Contract,
    workspace: &Path,
    make: &dyn Fn(&Path) -> Box<dyn Toolset>,
) -> Vec<String> {
    if let Err(err) = fs::create_dir_all(workspace) {
        return vec![format!(
            "schemas: cannot create {}: {err}",
            workspace.display()
        )];
    }
    let schemas = make(workspace).schemas();
    let mut lines = Vec::new();
    for op in &contract.operations {
        let found: Vec<_> = schemas
            .iter()
            .filter(|(name, _, _)| *name == op.name)
            .collect();
        match found.as_slice() {
            [] => lines.push(format!(
                "{}: schemas: the toolset derives none for it",
                op.name
            )),
            [(_, input, output)] => {
                for (side, schema) in [(Side::Input, input), (Side::Output, output)] {
                    if let Err(mismatch) = check_shapes(contract, side, &op.name, schema) {
                        lines.push(mismatch.to_string());
                    }
                }
            }
            many => lines.push(format!(
                "{}: schemas: the toolset derives {} for it",
                op.name,
                many.len()
            )),
        }
    }
    for (name, _, _) in &schemas {
        if contract.operation(name).is_none() {
            lines.push(format!(
                "{name}: schemas: the contract declares no such operation"
            ));
        }
    }
    lines
}
