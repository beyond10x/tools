//! The `tool-conformance/1` suite ([`Suite`]), the [`Toolset`] an implementation exposes, and
//! [`run_with`] (and [`run`] over it), which lays out each scenario's fixture in its own
//! workspace, serves its HTTP routes on loopback, builds a toolset from the scenario's host
//! configuration, makes its call or calls, matches each result and the tree afterwards, and
//! checks the derived schemas against the contract once.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use schemars::Schema;
use serde::Deserialize;
use serde_json::{Map, Value};

mod serve;

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
            if let Some(problem) = &scenario.refusal {
                return Err(refuse(problem));
            }
            for (path, route) in &scenario.fixture.serve {
                if let Some(problem) = serve::path_problem(path) {
                    return Err(refuse(&format!("fixture.serve: route {path:?} {problem}")));
                }
                if let Some(problem) = serve::route_problem(route) {
                    return Err(refuse(&format!("fixture.serve: route {path} {problem}")));
                }
            }
            if let Some(problem) = serve::shared_log(&scenario.fixture.serve) {
                return Err(refuse(&format!("fixture.serve: {problem}")));
            }
            for step in scenario.steps() {
                check_expect(step.expect)
                    .map_err(|problem| refuse(&format!("{}{problem}", step.at)))?;
            }
        }
        Ok(())
    }
}

fn check_expect(expect: &Expect) -> Result<(), String> {
    match (&expect.outcome, &expect.error) {
        (Some(outcome), None) if outcome == "ok" => Ok(()),
        (Some(outcome), None) => Err(format!("expect.outcome is {outcome:?}, only ok is allowed")),
        (None, Some(_)) => Ok(()),
        (Some(_), Some(_)) => Err("expect names both outcome: ok and error".to_string()),
        (None, None) => Err("expect names neither outcome: ok nor error".to_string()),
    }
}

fn is_directory_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && !name.contains(['/', '\\', '\0'])
}

/// One scenario: a fixture tree, the host configuration its toolset is built with, one call (or
/// an ordered list of calls on one toolset), and what to expect of each result and the tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(from = "ScenarioRepr")]
pub struct Scenario {
    /// The name; also the workspace directory under the scratch root.
    pub name: String,
    /// The profile properties this scenario is evidence for; in a `calls` scenario each entry
    /// carries its own and this is empty.
    pub holds: Vec<String>,
    /// Handed to the toolset factory ([`ScenarioContext::host`]), never to an operation, after
    /// `{{serve}}` and `{{workspace}}` are substituted in its strings.
    pub host: Map<String, Value>,
    /// The tree laid out before the call, and the routes served while the scenario runs.
    pub fixture: Fixture,
    /// The call; in a `calls` scenario, the operation and input of its first entry.
    pub call: Call,
    /// The calls of a `calls` scenario, in order, on one toolset; empty for a `call` scenario.
    pub calls: Vec<Step>,
    /// What to expect of the call; in a `calls` scenario each entry carries its own and this is
    /// empty.
    pub expect: Expect,
    /// Why the scenario's keys are refused at parse (`call` and `calls` both or neither, or a key
    /// that belongs on each entry of `calls`).
    refusal: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioRepr {
    name: String,
    #[serde(default)]
    holds: Option<Vec<String>>,
    #[serde(default)]
    host: Map<String, Value>,
    #[serde(default)]
    fixture: Fixture,
    #[serde(default)]
    call: Option<Call>,
    #[serde(default)]
    calls: Option<Vec<Step>>,
    #[serde(default)]
    expect: Option<Expect>,
}

impl From<ScenarioRepr> for Scenario {
    fn from(repr: ScenarioRepr) -> Self {
        let refusal = match (&repr.call, &repr.calls) {
            (Some(_), Some(_)) => Some("names both call and calls"),
            (None, None) => Some("names neither call nor calls"),
            (None, Some(calls)) if calls.is_empty() => Some("calls is empty"),
            (None, Some(_)) if repr.expect.is_some() => {
                Some("expect belongs on each entry of calls, not on the scenario")
            }
            (None, Some(_)) if repr.holds.is_some() => {
                Some("holds belongs on each entry of calls, not on the scenario")
            }
            (Some(_), None) if repr.expect.is_none() => Some("expect is missing"),
            _ => None,
        };
        let calls = repr.calls.unwrap_or_default();
        let call = repr.call.unwrap_or_else(|| match calls.first() {
            Some(first) => Call {
                operation: first.operation.clone(),
                input: first.input.clone(),
            },
            None => Call {
                operation: String::new(),
                input: empty_object(),
            },
        });
        Self {
            name: repr.name,
            holds: repr.holds.unwrap_or_default(),
            host: repr.host,
            fixture: repr.fixture,
            call,
            calls,
            expect: repr.expect.unwrap_or_default(),
            refusal: refusal.map(ToOwned::to_owned),
        }
    }
}

/// One call a scenario makes, as the runner sees it whichever key named it.
struct View<'a> {
    /// `""` for a `call` scenario, `calls[<i>]: ` for an entry of `calls`.
    at: String,
    operation: &'a str,
    input: &'a Value,
    holds: &'a [String],
    expect: &'a Expect,
}

impl Scenario {
    /// Its calls in order: the one `call`, or each entry of `calls`.
    fn steps(&self) -> Vec<View<'_>> {
        if self.calls.is_empty() {
            return vec![View {
                at: String::new(),
                operation: &self.call.operation,
                input: &self.call.input,
                holds: &self.holds,
                expect: &self.expect,
            }];
        }
        self.calls
            .iter()
            .enumerate()
            .map(|(i, step)| View {
                at: format!("calls[{i}]: "),
                operation: &step.operation,
                input: &step.input,
                holds: &step.holds,
                expect: &step.expect,
            })
            .collect()
    }
}

/// The tree a scenario starts from, and the HTTP routes served while it runs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// Files by workspace-relative path, with their text.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// A directory to copy in first, relative to the suite's [`Suite::base`].
    #[serde(default)]
    pub dir: Option<PathBuf>,
    /// Routes by path (starting with `/`), served on `127.0.0.1:<ephemeral>` over plain HTTP/1.1
    /// while the scenario runs; `{{serve}}` is their base URL.
    #[serde(default)]
    pub serve: BTreeMap<String, Route>,
}

/// What a served route answers. Each request it receives is appended to
/// `.serve/<path with / as _>.jsonl` in the workspace as `{"method", "path", "headers", "body"}`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// The status; `200`, or `302` with `redirect`.
    #[serde(default)]
    pub status: Option<u16>,
    /// Response headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// The body text.
    #[serde(default)]
    pub body: Option<String>,
    /// A file whose bytes are the body, relative to the suite's [`Suite::base`].
    #[serde(default)]
    pub body_file: Option<PathBuf>,
    /// The `Location` header.
    #[serde(default)]
    pub redirect: Option<String>,
    /// How long to wait before answering.
    #[serde(default)]
    pub delay_ms: Option<u64>,
}

/// The one call a scenario makes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// The operation.
    pub operation: String,
    /// The input, as JSON; `{{serve}}`, `{{workspace}}` and `{{calls.<n>.output.<path>}}` are
    /// substituted in its strings.
    #[serde(default = "empty_object")]
    pub input: Value,
}

/// One entry of a scenario's `calls`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// The operation.
    pub operation: String,
    /// The input, as JSON, substituted as [`Call::input`] is.
    #[serde(default = "empty_object")]
    pub input: Value,
    /// The profile properties this entry is evidence for, on its operation.
    #[serde(default)]
    pub holds: Vec<String>,
    /// What to expect of this entry.
    pub expect: Expect,
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
    /// The file holds this many lines (`{$lines: n}`); a last line needs no newline.
    Lines(u64),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FileExpectRepr {
    Keyword(FileKeyword),
    Equals {
        equals: String,
    },
    Sha256 {
        sha256: String,
    },
    Lines {
        #[serde(rename = "$lines")]
        lines: u64,
    },
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
            FileExpectRepr::Lines { lines } => Self::Lines(lines),
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

/// What the runner hands the toolset factory for one scenario.
#[derive(Debug, Clone, Copy)]
pub struct ScenarioContext<'a> {
    /// The scenario's workspace, its fixture laid out.
    pub workspace: &'a Path,
    /// The scenario's `host` map, `{{serve}}` and `{{workspace}}` substituted; empty when it
    /// names none. Never passed to an operation.
    pub host: &'a Map<String, Value>,
    /// The base URL of the scenario's served routes (`http://127.0.0.1:<port>`), when it has
    /// any.
    pub served: Option<&'a str>,
}

/// What a toolset factory could not find: the run fails naming it, and no scenario counts as
/// passed. There is no skipped state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    /// A program the toolset starts, by name.
    Program(String),
    /// A setting the toolset reads (an environment variable, a host key), by name.
    Setting(String),
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Program(name) => write!(f, "program {name}"),
            Self::Setting(name) => write!(f, "setting {name}"),
        }
    }
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
    /// What the toolset factory reported missing, one line each; when any, no scenario passed.
    pub missing: Vec<String>,
}

impl Report {
    /// Whether the suite ran, nothing was missing, every scenario passed and every schema
    /// matched.
    pub fn passed(&self) -> bool {
        self.refused.is_empty()
            && self.missing.is_empty()
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
        for line in &self.missing {
            writeln!(f, "missing: {line}")?;
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
        if !self.missing.is_empty() {
            write!(f, ", {} missing", self.missing.len())?;
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
        for step in scenario.steps() {
            let (name, at) = (&scenario.name, &step.at);
            let call = if at.is_empty() { "call: " } else { at.as_str() };
            match contract.operation(step.operation) {
                None => refused.push(format!(
                    "{name}: {call}{} is not an operation of {}",
                    step.operation, contract.toolset
                )),
                Some(op) => {
                    if let Some(error) = &step.expect.error
                        && !op.names_error(error)
                    {
                        refused.push(format!(
                            "{name}: {at}expect: {error} is not an outcome of {}",
                            op.name
                        ));
                    }
                    if let Some(input) = step.input.as_object() {
                        for key in input.keys().filter(|key| !op.has_input(key)) {
                            refused.push(format!(
                                "{name}: {call}{key} is not an input field of {}",
                                op.name
                            ));
                        }
                    }
                }
            }
            for held in step.holds {
                let known =
                    BEHAVIOURAL.contains(&held.as_str()) || DECLARATIVE.contains(&held.as_str());
                if !known {
                    refused.push(format!(
                        "{name}: {at}holds: {held} is not a profile property"
                    ));
                }
            }
        }
    }
    for op in &contract.operations {
        for property in declared_properties(&op.profile) {
            let held = suite
                .scenarios
                .iter()
                .flat_map(Scenario::steps)
                .any(|step| step.operation == op.name && step.holds.iter().any(|h| h == property));
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

/// Runs `suite` against the toolset `make` builds over each scenario's workspace under `scratch`:
/// [`run_with`] for a factory that needs only the workspace and never reports anything missing.
pub fn run(
    suite: &Suite,
    contract: &Contract,
    scratch: &Path,
    make: impl Fn(&Path) -> Box<dyn Toolset>,
) -> Report {
    run_with(suite, contract, scratch, |context: &ScenarioContext<'_>| {
        Ok(make(context.workspace))
    })
}

/// Runs `suite` against the toolset `make` builds for each scenario under `scratch`.
///
/// A suite [`coverage`] refuses is reported as refused and runs nothing. Otherwise each scenario
/// gets `<scratch>/<name>/` as a fresh workspace with its fixture laid out and its routes served,
/// `make` builds one toolset from the [`ScenarioContext`], and the call (or each of `calls`, in
/// order, on that one toolset) is made, its `expect` matched and its file post-conditions
/// checked; the server stops when the scenario ends. Then the toolset's derived schemas are
/// checked against the contract once, with a toolset built from the context of the first
/// scenario whose factory was called (its workspace and its substituted `host`, no `served`).
/// When none was, it is built with an empty `host` over `<scratch>/.schemas/`, and a [`Missing`]
/// from that build is reported only for a suite with no scenario.
///
/// When `make` reports something [`Missing`], the report names it and no scenario counts as
/// passed.
pub fn run_with(
    suite: &Suite,
    contract: &Contract,
    scratch: &Path,
    make: impl Fn(&ScenarioContext<'_>) -> Result<Box<dyn Toolset>, Missing>,
) -> Report {
    let mut report = Report::default();
    if let Err(refused) = coverage(suite, contract) {
        report.refused = refused;
        return report;
    }
    let mut missing: Vec<String> = Vec::new();
    let mut note = |what: &Missing| {
        let line = what.to_string();
        if !missing.contains(&line) {
            missing.push(line.clone());
        }
        line
    };
    let mut reached: Reached = None;
    for scenario in &suite.scenarios {
        let keep = if reached.is_none() {
            Some(&mut reached)
        } else {
            None
        };
        let detail = match run_scenario(suite, contract, scenario, scratch, &make, keep) {
            Ok(()) => None,
            Err(Failure::Detail(detail)) => Some(detail),
            Err(Failure::Missing(what)) => Some(format!("the toolset is missing {}", note(&what))),
        };
        report.scenarios.push(ScenarioResult {
            name: scenario.name.clone(),
            passed: detail.is_none(),
            detail: detail.unwrap_or_default(),
        });
    }
    let built = reached.is_some();
    let (workspace, host) = reached.unwrap_or_else(|| (scratch.join(".schemas"), Map::new()));
    match check_schemas(contract, &workspace, &host, &make) {
        Ok(mismatches) => report.schema_mismatches = mismatches,
        Err(what) if built || suite.scenarios.is_empty() => {
            note(&what);
        }
        Err(_) => {}
    }
    if !missing.is_empty() {
        let all = missing.join(", ");
        for result in report.scenarios.iter_mut().filter(|result| result.passed) {
            result.passed = false;
            result.detail = format!("not counted: the run is missing {all}");
        }
    }
    report.missing = missing;
    report
}

/// The workspace and substituted `host` of the first scenario whose factory was called.
type Reached = Option<(PathBuf, Map<String, Value>)>;

/// The toolset factory [`run_with`] takes, as the runner passes it on.
type Factory<'m> = dyn Fn(&ScenarioContext<'_>) -> Result<Box<dyn Toolset>, Missing> + 'm;

/// Why a scenario did not pass.
enum Failure {
    /// The first expectation that did not hold.
    Detail(String),
    /// What the factory could not find.
    Missing(Missing),
}

impl From<String> for Failure {
    fn from(detail: String) -> Self {
        Self::Detail(detail)
    }
}

fn run_scenario(
    suite: &Suite,
    contract: &Contract,
    scenario: &Scenario,
    scratch: &Path,
    make: &Factory<'_>,
    keep: Option<&mut Reached>,
) -> Result<(), Failure> {
    let workspace = scratch.join(&scenario.name);
    lay_out(suite, &scenario.fixture, &workspace)?;
    let before = snapshot(&workspace)?;
    let shared = serve::Shared::new(&scenario.fixture.serve, suite.base.as_deref(), &workspace)?;
    std::thread::scope(|scope| -> Result<(), Failure> {
        let running = if scenario.fixture.serve.is_empty() {
            None
        } else {
            Some(serve::Server::start(scope, &shared)?)
        };
        let served = running.as_ref().map(serve::Server::base);
        let result = run_calls(contract, scenario, &workspace, served, &before, make, keep);
        drop(running);
        result?;
        match shared.problem() {
            Some(problem) => Err(Failure::Detail(problem)),
            None => Ok(()),
        }
    })
}

/// The values `{{serve}}` and `{{workspace}}` stand for in one scenario.
struct Vars<'a> {
    serve: Option<&'a str>,
    workspace: String,
}

fn run_calls(
    contract: &Contract,
    scenario: &Scenario,
    workspace: &Path,
    served: Option<&str>,
    before: &BTreeMap<String, Content>,
    make: &Factory<'_>,
    keep: Option<&mut Reached>,
) -> Result<(), Failure> {
    let vars = Vars {
        serve: served,
        workspace: workspace.to_string_lossy().into_owned(),
    };
    let host = match substitute(&Value::Object(scenario.host.clone()), &vars, &[]) {
        Ok(Value::Object(host)) => host,
        Ok(_) => Map::new(),
        Err(problem) => return Err(Failure::Detail(format!("host: {problem}"))),
    };
    if let Some(keep) = keep {
        *keep = Some((workspace.to_path_buf(), host.clone()));
    }
    let context = ScenarioContext {
        workspace,
        host: &host,
        served,
    };
    let toolset = make(&context).map_err(Failure::Missing)?;
    let mut answers: Vec<Value> = Vec::new();
    for step in scenario.steps() {
        let answer = run_step(
            contract,
            &step,
            toolset.as_ref(),
            workspace,
            before,
            &vars,
            &answers,
        )
        .map_err(|detail| format!("{}{detail}", step.at))?;
        answers.push(answer);
    }
    Ok(())
}

/// Makes one call and checks it; answers the output (or the error's fields) a later
/// `{{calls.<n>.output.<path>}}` reads.
fn run_step(
    contract: &Contract,
    step: &View<'_>,
    toolset: &dyn Toolset,
    workspace: &Path,
    before: &BTreeMap<String, Content>,
    vars: &Vars<'_>,
    answers: &[Value],
) -> Result<Value, String> {
    let op = contract
        .operation(step.operation)
        .ok_or_else(|| format!("{} is not an operation of the contract", step.operation))?;
    let input =
        substitute(step.input, vars, answers).map_err(|problem| format!("input: {problem}"))?;
    let outcome = toolset.invoke(step.operation, &input);
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
    let actual = match (&step.expect.error, &outcome) {
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
    if let Some(expected) = &step.expect.output {
        match_value(expected, actual, workspace, "output")?;
    }
    for (rel, expect) in &step.expect.files {
        check_file(workspace, rel, expect, before)?;
    }
    Ok(actual.clone())
}

/// Substitutes `{{serve}}`, `{{workspace}}` and `{{calls.<n>.output.<path>}}` in every string of
/// `value`. A string that is one `{{calls...}}` reference whole takes the referenced value as it
/// is; inside a longer string a non-string value is written as JSON. Any other `{{...}}` is left
/// as written.
fn substitute(value: &Value, vars: &Vars<'_>, answers: &[Value]) -> Result<Value, String> {
    match value {
        Value::String(text) => substitute_text(text, vars, answers),
        Value::Array(items) => items
            .iter()
            .map(|item| substitute(item, vars, answers))
            .collect::<Result<_, _>>()
            .map(Value::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| Ok((key.clone(), substitute(item, vars, answers)?)))
            .collect::<Result<_, String>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

fn substitute_text(text: &str, vars: &Vars<'_>, answers: &[Value]) -> Result<Value, String> {
    if let Some(name) = text.strip_prefix("{{").and_then(|t| t.strip_suffix("}}"))
        && name.starts_with("calls.")
        && !name.contains("}}")
    {
        return reference(name, answers).cloned();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            break;
        };
        out.push_str(&rest[..start]);
        let name = &after[..end];
        match name {
            "serve" => out.push_str(
                vars.serve
                    .ok_or("{{serve}} needs routes in fixture.serve")?,
            ),
            "workspace" => out.push_str(&vars.workspace),
            _ if name.starts_with("calls.") => match reference(name, answers)? {
                Value::String(found) => out.push_str(found),
                found => out.push_str(&found.to_string()),
            },
            _ => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(Value::String(out))
}

/// The value `calls.<n>.output.<path>` names among the answers so far; `<path>` is keys and list
/// indices joined by `.`.
fn reference<'v>(name: &str, answers: &'v [Value]) -> Result<&'v Value, String> {
    let mut parts = name.split('.').skip(1);
    let index = parts
        .next()
        .and_then(|n| n.parse::<usize>().ok())
        .ok_or_else(|| format!("{{{{{name}}}}}: calls.<n> needs a call index"))?;
    let mut found = answers.get(index).ok_or_else(|| {
        format!("{{{{{name}}}}}: calls.{index} is not an earlier call of the scenario")
    })?;
    if parts.next() != Some("output") {
        return Err(format!("{{{{{name}}}}}: expected calls.{index}.output"));
    }
    for key in parts {
        found = match found {
            Value::Object(fields) => fields.get(key),
            Value::Array(items) => key.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        }
        .ok_or_else(|| format!("{{{{{name}}}}}: the answer of calls.{index} has no {key}"))?;
    }
    Ok(found)
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
        FileExpect::Lines(lines) => {
            let now = fs::read_to_string(&path).map_err(|e| format!("files: {rel}: {e}"))?;
            let count = now.lines().count();
            if u64::try_from(count).is_ok_and(|count| count == *lines) {
                Ok(())
            } else {
                Err(format!("files: {rel}: expected {lines} lines, got {count}"))
            }
        }
    }
}

fn check_schemas(
    contract: &Contract,
    workspace: &Path,
    host: &Map<String, Value>,
    make: &Factory<'_>,
) -> Result<Vec<String>, Missing> {
    if let Err(err) = fs::create_dir_all(workspace) {
        return Ok(vec![format!(
            "schemas: cannot create {}: {err}",
            workspace.display()
        )]);
    }
    let schemas = make(&ScenarioContext {
        workspace,
        host,
        served: None,
    })?
    .schemas();
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
    Ok(lines)
}
