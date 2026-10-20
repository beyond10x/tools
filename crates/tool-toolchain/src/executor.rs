//! Runs a planned toolchain operation through a [`Runner`], `b10x-tool-process` by default: the
//! `build-test` toolset's executor.
//!
//! Ported from harness 0.14.1 (`f109612e`), `crates/harness-tools/src/`: `toolchain.rs`'s
//! `programs`, `entry_names` and `plan` (with its `CommandPlan`, here [`Plan`] and [`Group`]),
//! and `catalogue.rs`'s `invoke_toolchain` (here [`Executor::run`]). Each argv runs through
//! [`Runner::run`], harness's `run_within`: [`tool_process::Process`] for
//! [`Executor::new`], whose declared program set is the active providers' [`programs`], or the
//! caller's own runner for [`Executor::with_runner`]. `toolchain.rs`'s `entries` stays in
//! harness: its per-tool half is [`crate::contract`]'s operations, and its role entries are
//! publication over harness's own wire types; [`entry_names`] names them.
//!
//! # Where it differs from harness 0.14.1
//!
//! Each pinned by a test or a scenario.
//!
//! - **The answer shape is the contract's.** Harness answered one object when a plan started one
//!   process and an array when it started several, decided at run time. The `build-test`
//!   contract declares, per operation, either the eight fields of one run or `runs`, a list of
//!   them ([`crate::contract`]), so a tool whose plan *can* start several processes answers
//!   `{"runs": [...]}` even when it started one, and every other tool answers the one run. A role
//!   spanning several providers answers `{"toolchains": {<provider>: <answer>}}` as harness did.
//! - **The time left is shared, and nothing starts once it is gone.** Harness handed every argv
//!   of a plan the same `remaining`, so a plan of two processes could run for twice what its
//!   caller had left. Here a deadline is taken when the run starts, and each argv gets what is
//!   left of it. Once nothing is left no further argv starts: a call that started no process is
//!   refused, and one that did answers the runs that started (correction round 1, F5).
//! - **Every written path is judged before anything starts, where it lands.** Harness asked its
//!   write scope one provider group at a time, so a role whose second provider wrote a refused
//!   path was refused after the first provider's processes had run, and it asked about the path
//!   as spelled, so `alias.go`, a link into a refused directory, was rewritten through
//!   (`catalogue.rs:524-527`). Here every path of every group is judged first, as spelled and
//!   then where it lands (harness's `lands`, which its file writes used), and a path that lands
//!   outside the workspace or through a link that leads nowhere is refused with or without a
//!   scope (correction round 1, F1). A path-kind argument the call only reads is judged where
//!   it lands too, and refused the same way, so `go_fmt_check` on `up/outside.go` through
//!   `up -> ..` reads nothing outside; the scope is not asked about it (correction round 2, P1).
//! - **Refusals are named.** Harness answered a sentence. Here a refusal is
//!   [`RunError::Refused`], `build-test.Refused` in the contract, with the same sentence; the
//!   formatting check is [`RunError::CheckFailed`], `build-test.CheckFailed`, whose message is
//!   harness's `formatting check failed: <stdout as a JSON string>`.
//! - **A role over no active provider is refused.** Harness never reached it: its catalogue
//!   publishes a role entry only when every active provider implements the role, and there is
//!   at least one.
//! - **The declared program set is the executor's own check.** Harness left it to the runner's
//!   `run_within`. Here every argv of a plan is checked against [`programs`] before the first
//!   one starts, whatever the runner, so a runner that checks nothing is still handed only a
//!   declared program, and a plan whose later argv names an undeclared one starts nothing.
//!
//! # What it does not confine
//!
//! The default runner is the process toolset's: the program starts in the workspace root, as
//! the caller's own user, with an environment cleared down to that crate's allow-list (the
//! caller's `HOME` and toolchain caches included), under the bound. The provider's sandbox
//! environment (`{workspace}` and `{mount:…}` placeholders, `GOENV=off`, `NPM_CONFIG_OFFLINE` and
//! the like) names a confined root that this runner does not build, so it is not applied; the
//! runner is handed it ([`RunRequest::sandbox`]) and leaves it, and the derived profile's
//! `requires` keeps declaring it. Beyond the environment, the bounds and the written paths a call
//! names, nothing is confined, so every operation declares `placement: local` and every effect
//! and consequence such a runner allows ([`crate::contract`]). A confined runner passed to
//! [`Executor::with_runner`] applies the sandbox; the contract's declarations stay those of the
//! local runner.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Map, Value, json};
use tool_process::{Process, ProcessError, RunInput, RunOutput};

use crate::{CommandPlan, ParameterKind, ResolvedProvider, ResolvedTool, Role, SandboxSpec};

/// The error a refusal is returned as: nothing was started, or a started program could not be
/// reported on.
pub const REFUSED: &str = crate::contract::REFUSED;
/// The error a formatting check that wrote to standard output is returned as.
pub const CHECK_FAILED: &str = crate::contract::CHECK_FAILED;

/// The roles a generic `toolchain_<role>` name can name, in harness's order.
const ROLES: [Role; 4] = [Role::Check, Role::Build, Role::Test, Role::FmtCheck];

/// What one call runs: one group per provider. Harness's `toolchain::CommandPlan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// One group per provider, in the order they run: one for a tool's own name, one per active
    /// provider for a role.
    pub groups: Vec<Group>,
}

/// What one provider's tool runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The provider, which keys its answer under `toolchains` when a plan has several groups.
    pub provider: String,
    /// The tool's own name.
    pub tool: String,
    /// Whether the tool's plan can start several processes, so that it answers `runs`.
    pub several: bool,
    /// Every value a `workspace-path` or `workspace-path-list` parameter takes in this call, read
    /// or written, its default included; each is judged where it lands before anything starts.
    pub paths: Vec<String>,
    /// The argv calls, the paths they write and whether standard output must stay empty.
    pub plan: CommandPlan,
}

/// The output of an operation whose plan can start several processes: one run per process, in
/// the order they ran. The contract's `runs: List<toolchain.Run>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct Runs {
    /// One run per process the plan started, in the order they ran.
    pub runs: Vec<RunOutput>,
}

/// Why a run answered no output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// `build-test.Refused`: an argument the registry refuses, a written path the write scope
    /// refuses, a role no active provider implements, a program outside the declared set, or the
    /// runner's own refusal (for the process runner, a program that could not be started or
    /// reported on). The sentence is the refusing party's own.
    Refused(String),
    /// `build-test.CheckFailed`: an `empty-stdout` tool's program wrote to standard output.
    CheckFailed {
        /// What it wrote, as the runner kept it.
        stdout: String,
    },
}

impl RunError {
    /// The contract error this is returned as.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Refused(_) => REFUSED,
            Self::CheckFailed { .. } => CHECK_FAILED,
        }
    }

    /// The contract error's fields, as the contract declares them.
    pub fn fields(&self) -> Value {
        match self {
            Self::Refused(message) => json!({ "message": message }),
            Self::CheckFailed { stdout } => json!({ "stdout": stdout }),
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(message) => f.write_str(message),
            Self::CheckFailed { stdout } => {
                write!(
                    f,
                    "formatting check failed: {}",
                    Value::String(stdout.clone())
                )
            }
        }
    }
}

impl std::error::Error for RunError {}

impl From<ProcessError> for RunError {
    fn from(error: ProcessError) -> Self {
        Self::Refused(error.to_string())
    }
}

/// One property switched off, for the mutant check the conformance suite is held to.
///
/// Only a mutant build (`--features mutants`, or the crate's own tests) can set one, through
/// `Executor::mutated`; outside it every [`Executor`] is intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    /// The `empty-stdout` check is skipped: a check that wrote to standard output answers its
    /// run.
    NoEmptyStdoutCheck,
    /// The executor checks each argv against no declared program set instead of the active
    /// providers' programs.
    NoDeclaredPrograms,
    /// The time the caller has left is not passed on: every process gets the contract's bound.
    NoTimeLeft,
    /// The write scope is never asked about a planned written path.
    NoWriteScope,
    /// A planned written path is judged as spelled only: where it lands is never resolved, so
    /// neither a link into a path the scope refuses nor one out of the workspace is caught.
    NoLanding,
    /// A path-kind argument the call only reads is never resolved, so one that leaves the
    /// workspace through a link reaches the program.
    NoReadLanding,
}

impl Mutation {
    /// Every mutation, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::NoEmptyStdoutCheck,
        Self::NoDeclaredPrograms,
        Self::NoTimeLeft,
        Self::NoWriteScope,
        Self::NoLanding,
        Self::NoReadLanding,
    ];

    /// The mutation's name in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoEmptyStdoutCheck => "no-empty-stdout-check",
            Self::NoDeclaredPrograms => "no-declared-programs",
            Self::NoTimeLeft => "no-time-left",
            Self::NoWriteScope => "no-write-scope",
            Self::NoLanding => "no-landing",
            Self::NoReadLanding => "no-read-landing",
        }
    }
}

/// The declared program set of the active providers: every provider's `programs`, sorted, each
/// once. Harness's `toolchain::programs`.
pub fn programs(active: &[ResolvedProvider]) -> Vec<String> {
    let mut programs = active
        .iter()
        .flat_map(|provider| provider.programs.iter().cloned())
        .collect::<Vec<_>>();
    programs.sort();
    programs.dedup();
    programs
}

/// Every name the active providers contribute: each tool's own name, in provider and declaration
/// order, then `toolchain_<role>` for each role every active provider implements. Harness's
/// `toolchain::entry_names`, without the entries' wire envelopes.
pub fn entry_names(active: &[ResolvedProvider]) -> Vec<String> {
    let mut names: Vec<String> = active
        .iter()
        .flat_map(|provider| provider.tools.iter().map(|tool| tool.name.clone()))
        .collect();
    for role in ROLES {
        let complete = !active.is_empty()
            && active
                .iter()
                .all(|provider| provider.tools.iter().any(|tool| tool.role == Some(role)));
        if complete {
            names.push(format!("toolchain_{}", role.name()));
        }
    }
    names
}

/// The plan for `name` over the active providers. Harness's `toolchain::plan`.
///
/// `toolchain_<role>` plans the role's tool in every active provider, each with the arguments
/// under its provider's name. Any other name is tried against each provider in turn, by a tool's
/// own name or its published operation name ([`ResolvedProvider::plan`]).
///
/// `None` when no active provider has a tool or a role by that name; `Some(Err)` with the
/// registry's sentence when the arguments are refused or a provider lacks the role.
pub fn plan(
    name: &str,
    arguments: &Value,
    active: &[ResolvedProvider],
) -> Option<Result<Plan, String>> {
    if let Some(role_name) = name.strip_prefix("toolchain_") {
        let role = ROLES.into_iter().find(|role| role.name() == role_name)?;
        let groups = active
            .iter()
            .map(|provider| {
                let tool = provider
                    .tools
                    .iter()
                    .find(|tool| tool.role == Some(role))
                    .ok_or_else(|| {
                        format!(
                            "provider `{}` has no `{}` role implementation",
                            provider.name,
                            role.name()
                        )
                    })?;
                let provider_arguments = arguments.get(&provider.name).ok_or_else(|| {
                    format!("`{}` arguments are required by `{name}`", provider.name)
                })?;
                tool.plan(provider_arguments).map(|plan| Group {
                    provider: provider.name.clone(),
                    tool: tool.name.clone(),
                    several: tool.several_processes(),
                    paths: path_arguments(tool, provider_arguments),
                    plan,
                })
            })
            .collect::<Result<Vec<_>, _>>();
        return Some(groups.map(|groups| Plan { groups }));
    }
    active.iter().find_map(|provider| {
        let tool = provider
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .or_else(|| provider.tool_for(name))?;
        Some(tool.plan(arguments).map(|plan| Plan {
            groups: vec![Group {
                provider: provider.name.clone(),
                tool: tool.name.clone(),
                several: tool.several_processes(),
                paths: path_arguments(tool, arguments),
                plan,
            }],
        }))
    })
}

/// Every value a `workspace-path` or `workspace-path-list` parameter of `tool` takes in this
/// call, its default included, in parameter order.
fn path_arguments(tool: &ResolvedTool, arguments: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    for (name, spec) in &tool.parameters {
        if !matches!(
            spec.kind,
            ParameterKind::WorkspacePath | ParameterKind::WorkspacePathList
        ) {
            continue;
        }
        let Some(value) = arguments.get(name).or(spec.default.as_ref()) else {
            continue;
        };
        if let Some(path) = value.as_str() {
            paths.push(path.to_owned());
        }
        paths.extend(
            value
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned),
        );
    }
    paths
}

/// One argv for a [`Runner`] to run: what the executor hands it once every check of its own has
/// passed.
///
/// Non-exhaustive: a runner reads one and never builds one, so what a call carries can grow
/// without breaking a runner.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RunRequest<'a> {
    /// The program, then its arguments, one item each; `argv[0]` is one of the active providers'
    /// [`programs`].
    pub argv: &'a [String],
    /// The provider whose tool planned the argv, which keys its answer under `toolchains`.
    pub provider: &'a str,
    /// The provider's sandbox as declared ([`ResolvedProvider::sandbox`]): its programs, and its
    /// environment with the `{workspace}` and `{mount:…}` placeholders unexpanded, for a
    /// confined runner to expand against its own mounts and apply.
    ///
    /// **The local [`Process`] runner does not apply it.** Its values move `HOME`, `PATH` and the
    /// toolchains (`RUSTUP_HOME`, `GOROOT`, `CARGO_HOME` and the caches) into the workspace and
    /// onto mounts that a local run does not have, so applied on the operator's own machine they
    /// would point the programs at directories that do not exist. The process runner starts each
    /// program with its own allow-listed environment instead.
    pub sandbox: &'a SandboxSpec,
    /// What is left of the call's deadline as this argv starts, never zero: no argv starts once
    /// nothing is left. `None` when the caller gave no deadline, so the runner's own bound alone
    /// applies.
    pub time_left: Option<Duration>,
}

/// Runs one argv in one workspace root and answers what it did: the seam between the executor's
/// checks and a process, local or confined.
///
/// [`Process`] is the default, for [`Executor::new`]; [`Executor::with_runner`] takes any other,
/// such as an adapter over a harness's own `run_within(argv, remaining)`, whose answer has
/// [`RunOutput`]'s fields.
///
/// # What the executor checks, and the runner need not
///
/// Every check of the `build-test` contract is the executor's, made before or around the call
/// whatever the runner is: the arguments the registry accepts and the dash rule, where every
/// path-kind argument lands under [`Runner::root`] (read or written), the write scope, the
/// declared program set (every argv of the plan against [`programs`], before the first starts),
/// the deadline (no argv is handed over once nothing is left of it) and `empty-stdout` (judged
/// on the answer's `stdout`). A refused call never reaches the runner, and none of these is the
/// runner's to repeat; a runner may refuse more, never less.
///
/// # What a confined runner owes
///
/// - **The sandbox, applied.** [`RunRequest::sandbox`]'s programs and environment, its
///   placeholders expanded against the runner's own mounts. The executor cannot see whether it
///   was.
/// - **The bound.** The run ends by [`RunRequest::time_left`] or the runner's own ceiling,
///   whichever comes first, its process group killed if it has not ended by then.
/// - **An answer that says so.** `timed_out: true` when the bound ended the run, and
///   `timeout_ms` the bound it was given, so a kill is not read as the program's own failure.
pub trait Runner {
    /// The runner's own refusal. The executor answers it as [`RunError::Refused`] with this
    /// sentence and starts no further argv of the plan.
    type Error: fmt::Display;

    /// The workspace root every argv runs in. The executor canonicalises it once, when it is
    /// built, and judges every path-kind argument by where it lands under it.
    fn root(&self) -> &Path;

    /// Runs `request.argv` in [`Runner::root`] and answers its output; a program that exits
    /// non-zero is an output, not an error.
    ///
    /// # Errors
    ///
    /// The runner would not or could not run it: a program it cannot start or report on, or one
    /// its own confinement refuses.
    fn run(&self, request: RunRequest<'_>) -> Result<RunOutput, Self::Error>;
}

/// The local runner: each argv through [`Process::run`] in the process's root, with its own
/// allow-listed environment and bound. It ignores [`RunRequest::provider`], does **not** apply
/// [`RunRequest::sandbox`] (the field says why) and checks the declared set a second time.
impl Runner for Process {
    type Error = ProcessError;

    fn root(&self) -> &Path {
        Process::root(self)
    }

    fn run(&self, request: RunRequest<'_>) -> Result<RunOutput, ProcessError> {
        Process::run(self, &RunInput::argv(request.argv), request.time_left)
    }
}

/// Judges one planned written path: `Some(sentence)` refuses it.
type WriteScope = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Runs planned toolchain operations in one workspace root: harness's `invoke_toolchain` over a
/// [`Runner`], [`Process`] unless the caller supplies one, with the active providers'
/// [`programs`] as the declared set.
pub struct Executor<R = Process> {
    providers: Vec<ResolvedProvider>,
    /// The declared set every argv is checked against before the first one starts.
    programs: Vec<String>,
    /// The runner's root, canonical.
    root: PathBuf,
    runner: R,
    write_scope: Option<WriteScope>,
    time_left: Option<Duration>,
    #[cfg(any(test, feature = "mutants"))]
    mutation: Option<Mutation>,
}

impl<R: fmt::Debug> fmt::Debug for Executor<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor")
            .field(
                "providers",
                &self
                    .providers
                    .iter()
                    .map(|provider| provider.name.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("programs", &self.programs)
            .field("root", &self.root)
            .field("runner", &self.runner)
            .field("write_scope", &self.write_scope.is_some())
            .field("time_left", &self.time_left)
            .finish_non_exhaustive()
    }
}

impl Executor {
    /// Opens `workspace` as the root every planned argv runs in, over `providers` as resolved
    /// there, with their [`programs`] as the declared set, on the local [`Process`] runner.
    ///
    /// # Errors
    ///
    /// `workspace` cannot be canonicalised, or is not a directory.
    pub fn new(
        workspace: impl AsRef<Path>,
        providers: Vec<ResolvedProvider>,
    ) -> Result<Self, ProcessError> {
        let process = Process::new(workspace, programs(&providers))?;
        Ok(Self::assemble(
            process.root().to_path_buf(),
            process,
            providers,
        ))
    }

    /// The same executor with one property switched off, for the mutant check.
    ///
    /// # Errors
    ///
    /// As [`Executor::new`].
    #[cfg(any(test, feature = "mutants"))]
    pub fn mutated(
        workspace: impl AsRef<Path>,
        providers: Vec<ResolvedProvider>,
        mutation: Mutation,
    ) -> Result<Self, ProcessError> {
        let mut executor = Self::new(workspace, providers)?;
        if mutation == Mutation::NoDeclaredPrograms {
            executor.programs = Vec::new();
        }
        executor.mutation = Some(mutation);
        Ok(executor)
    }
}

impl<R: Runner> Executor<R> {
    /// Opens `runner`'s root as the root every planned argv runs in, over `providers` as resolved
    /// there, with their [`programs`] as the declared set: every check is the executor's, and
    /// each argv that passes them is the runner's to run.
    ///
    /// # Errors
    ///
    /// As [`Executor::new`]: the runner's root cannot be canonicalised, or is not a directory.
    pub fn with_runner(runner: R, providers: Vec<ResolvedProvider>) -> Result<Self, ProcessError> {
        let root = canonical(runner.root())?;
        Ok(Self::assemble(root, runner, providers))
    }
}

impl<R> Executor<R> {
    fn assemble(root: PathBuf, runner: R, providers: Vec<ResolvedProvider>) -> Self {
        Self {
            programs: programs(&providers),
            providers,
            root,
            runner,
            write_scope: None,
            time_left: None,
            #[cfg(any(test, feature = "mutants"))]
            mutation: None,
        }
    }

    /// The same executor, asking `judge` about every path a plan declares it writes before
    /// anything starts; a path it answers a sentence for refuses the whole call with that
    /// sentence. Harness's write-scope refusal (`Catalogue::refusal("file.write", path)`), here
    /// the caller's: with none, every contained path may be written.
    #[must_use]
    pub fn refusing_writes(
        mut self,
        judge: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        self.write_scope = Some(Box::new(judge));
        self
    }

    /// The same executor, answering every [`tool_contract::Toolset::invoke`] as a call whose
    /// caller has `time_left` of its run. [`Executor::run`] takes the time left per call instead.
    #[must_use]
    pub fn within(mut self, time_left: Duration) -> Self {
        self.time_left = Some(time_left);
        self
    }

    /// The time left [`Executor::within`] set, which [`tool_contract::Toolset::invoke`] runs under.
    pub fn time_left(&self) -> Option<Duration> {
        self.time_left
    }

    /// The active providers, as resolved in the workspace.
    pub fn providers(&self) -> &[ResolvedProvider] {
        &self.providers
    }

    /// The declared program set every argv is checked against: the active providers'
    /// [`programs`].
    pub fn programs(&self) -> &[String] {
        &self.programs
    }

    /// The canonical workspace root: the runner's, canonicalised when the executor was built.
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[cfg(any(test, feature = "mutants"))]
    fn mutated_by(&self, mutation: Mutation) -> bool {
        self.mutation == Some(mutation)
    }

    #[cfg(not(any(test, feature = "mutants")))]
    #[allow(clippy::unused_self)]
    fn mutated_by(&self, _mutation: Mutation) -> bool {
        false
    }
}

impl<R: Runner> Executor<R> {
    /// Plans `name` with `arguments` and runs it; harness's `invoke_within` for a toolchain entry.
    ///
    /// `remaining` is how much of the caller's own wall-clock budget is left. A deadline is taken
    /// when the run starts and each argv gets what is left of it, so the plan as a whole ends by
    /// it; the runner caps each at its own bound. Once nothing is left, no further argv starts,
    /// and the answer holds the runs that did (a role's `toolchains` only the providers whose
    /// processes started). `None` leaves each argv the runner's bound.
    ///
    /// `None` when no active provider has a tool or a role by that name. Otherwise the answer:
    /// one run, `{"runs": [...]}` for a tool whose plan can start several processes, or
    /// `{"toolchains": {<provider>: <answer>}}` for a role over several providers.
    ///
    /// # Errors
    ///
    /// [`RunError::Refused`] when the registry refuses the arguments, a provider lacks the role,
    /// the role spans no provider, a path-kind argument lands outside the workspace or nowhere,
    /// read or written, a planned written path is refused by the write scope (as spelled or
    /// where it lands), an argv of the plan names a program outside the declared set (all of
    /// these before any process starts), the deadline has passed before any process started, or
    /// the runner refuses an argv (and the processes before it have run);
    /// [`RunError::CheckFailed`] when an `empty-stdout` tool's run answered anything but
    /// whitespace on standard output, after which no further argv of the plan starts.
    pub fn run(
        &self,
        name: &str,
        arguments: &Value,
        remaining: Option<Duration>,
    ) -> Option<Result<Value, RunError>> {
        let planned = plan(name, arguments, &self.providers)?;
        Some(
            planned
                .map_err(RunError::Refused)
                .and_then(|planned| self.execute(name, planned, remaining)),
        )
    }

    fn execute(
        &self,
        name: &str,
        plan: Plan,
        remaining: Option<Duration>,
    ) -> Result<Value, RunError> {
        if plan.groups.is_empty() {
            return Err(RunError::Refused(format!(
                "`{name}` names no active toolchain provider; nothing was started"
            )));
        }
        for group in &plan.groups {
            for path in &group.paths {
                if group.plan.writes.contains(path) || self.mutated_by(Mutation::NoReadLanding) {
                    continue;
                }
                lands(self.root(), path).map_err(RunError::Refused)?;
            }
        }
        for path in plan.groups.iter().flat_map(|group| &group.plan.writes) {
            self.judge_written(path)?;
        }
        let mut sandboxes = Vec::with_capacity(plan.groups.len());
        for group in &plan.groups {
            for argv in &group.plan.argv {
                self.declared(argv)?;
            }
            sandboxes.push(self.sandbox(&group.provider)?);
        }
        let several_providers = plan.groups.len() > 1;
        let deadline = remaining.and_then(|left| Instant::now().checked_add(left));
        let mut started = false;
        let mut grouped = Map::new();
        'groups: for (group, sandbox) in plan.groups.into_iter().zip(sandboxes) {
            let mut runs = Vec::with_capacity(group.plan.argv.len());
            let mut out_of_time = false;
            for argv in &group.plan.argv {
                let left = if self.mutated_by(Mutation::NoTimeLeft) {
                    None
                } else {
                    deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()))
                };
                if left.is_some_and(|left| left.is_zero()) {
                    if !started {
                        return Err(RunError::Refused(format!(
                            "the call's deadline passed before `{name}` started a process; \
                             nothing was started"
                        )));
                    }
                    out_of_time = true;
                    break;
                }
                started = true;
                let output = self
                    .runner
                    .run(RunRequest {
                        argv,
                        provider: &group.provider,
                        sandbox,
                        time_left: left,
                    })
                    .map_err(|refusal| RunError::Refused(refusal.to_string()))?;
                if group.plan.empty_stdout
                    && !self.mutated_by(Mutation::NoEmptyStdoutCheck)
                    && !output.stdout.trim().is_empty()
                {
                    return Err(RunError::CheckFailed {
                        stdout: output.stdout,
                    });
                }
                runs.push(output);
            }
            if !runs.is_empty() {
                let answer = if group.several {
                    serde_json::to_value(Runs { runs })
                } else {
                    serde_json::to_value(runs.remove(0))
                }
                .expect("a run is strings, integers and booleans, which always serialise");
                grouped.insert(group.provider, answer);
            }
            if out_of_time {
                break 'groups;
            }
        }
        if several_providers {
            return Ok(json!({ "toolchains": grouped }));
        }
        grouped
            .into_iter()
            .next()
            .map(|(_, answer)| answer)
            .ok_or_else(|| RunError::Refused("toolchain provider produced no output".to_owned()))
    }

    /// Refuses an argv whose program is not in the declared set, in the process runner's own
    /// words, so the sentence is the one a [`Process`] that refused it would have answered.
    fn declared(&self, argv: &[String]) -> Result<(), RunError> {
        let Some(program) = argv.first() else {
            return Err(ProcessError::EmptyArgv.into());
        };
        if self.programs.contains(program) {
            return Ok(());
        }
        Err(ProcessError::ProgramNotDeclared {
            program: program.clone(),
            declared: self.programs.clone(),
        }
        .into())
    }

    /// The declared sandbox of `provider`, a group's provider, which [`plan`] took from the
    /// active providers.
    fn sandbox(&self, provider: &str) -> Result<&SandboxSpec, RunError> {
        self.providers
            .iter()
            .find(|active| active.name == provider)
            .map(|active| &active.sandbox)
            .ok_or_else(|| {
                RunError::Refused(format!(
                    "`{provider}` is not an active toolchain provider; nothing was started"
                ))
            })
    }

    /// Refuses a path a call would write: one the write scope refuses as spelled, one that lands
    /// outside the workspace or through a link that leads nowhere, and one whose landing the
    /// scope refuses. Harness's `Catalogue::invoke_within` judged a file write this way (the
    /// spelling, then `Operations::lands`); its `invoke_toolchain` judged the spelling alone.
    fn judge_written(&self, path: &str) -> Result<(), RunError> {
        let judge = self
            .write_scope
            .as_ref()
            .filter(|_| !self.mutated_by(Mutation::NoWriteScope));
        if let Some(refusal) = judge.and_then(|judge| judge(path)) {
            return Err(RunError::Refused(refusal));
        }
        if self.mutated_by(Mutation::NoLanding) {
            return Ok(());
        }
        let landing = lands(self.root(), path).map_err(RunError::Refused)?;
        if landing != path
            && let Some(refusal) = judge.and_then(|judge| judge(&landing))
        {
            return Err(RunError::Refused(format!(
                "`{path}` leads to `{landing}`, and {refusal}"
            )));
        }
        Ok(())
    }
}

/// A runner's root, canonical, as [`Process::new`] opens one: [`lands`] compares canonical paths,
/// so a root spelled through a link would put every path outside it.
fn canonical(root: &Path) -> Result<PathBuf, ProcessError> {
    let given = root.display().to_string();
    let canonical = root
        .canonicalize()
        .map_err(|source| ProcessError::Workspace {
            path: given.clone(),
            source,
        })?;
    if !canonical.is_dir() {
        return Err(ProcessError::WorkspaceNotADirectory { path: given });
    }
    Ok(canonical)
}

/// Where `relative`, read or written, lands, as a path relative to `root`, which is canonical:
/// harness 0.14.1's `LocalOperations::resolve_new` and `lands` (`local.rs`). The deepest part of
/// the path that exists, a link included, is resolved, checked to lie inside the root, and the
/// part that does not exist yet is put back on it.
///
/// Presence is asked of the entry itself (`symlink_metadata`), so a link that leads nowhere
/// stops the walk and is refused rather than written through.
///
/// # Errors
///
/// The path lands outside the root, through a link that leads nowhere, or climbs out of a
/// directory that does not exist.
fn lands(root: &Path, relative: &str) -> Result<String, String> {
    let candidate = root.join(relative);
    let mut tail = Vec::new();
    let mut existing = candidate.as_path();
    while existing.symlink_metadata().is_err() {
        let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
            return Err(format!(
                "`{relative}` names no path this workspace can write; nothing was started"
            ));
        };
        tail.push(name.to_owned());
        existing = parent;
    }
    let mut resolved = existing.canonicalize().map_err(|error| {
        format!(
            "`{relative}`: `{}` is a link that leads nowhere this workspace can write ({error}); \
             nothing was started",
            relative_to(root, existing)
        )
    })?;
    if !resolved.starts_with(root) {
        return Err(format!(
            "`{relative}` lands outside the workspace; nothing was started"
        ));
    }
    for name in tail.iter().rev() {
        if name == ".." {
            return Err(format!(
                "`{relative}` climbs out of a directory that does not exist; nothing was started"
            ));
        }
        resolved.push(name);
    }
    Ok(relative_to(root, &resolved))
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests;
