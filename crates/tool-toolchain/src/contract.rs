//! The `build-test` operations a resolved provider derives under the `tool-contract/1` profile.
//!
//! A tool's YAML already declares most of what the profile asks for: the programs it starts
//! (`requires.binaries`), the environment it runs under (`requires.env`), the installation roots
//! it mounts (`requires.settings`), which parameters name the paths it writes (`subjects`),
//! whether its plan can start several processes (`output`) and whether it is a check that reads
//! standard output as failure (`outcomes`). The effects, consequences, risk and placement are
//! the executor's, which confines the program to nothing but its environment, bounds and
//! written paths, so they are the same for every tool. The operations are resolved per
//! workspace, like the tools they come from. What
//! the five built-in providers derive is frozen in `contracts/build-test/v1alpha1/contract.yaml`
//! ([`crate::document`]); a test holds the two equal, operation for operation, apart from the
//! values an enum parameter takes, which only a workspace supplies.
//!
//! A published name is the handle a harness gets, and the derivation that produces it is not
//! one-to-one with tool names ([`operation_name`]). A harness routes a published operation
//! through [`ResolvedProvider::plan_operation`], or finds its tool with
//! [`ResolvedProvider::tool_for`]. [`ResolvedProvider::plan`] matches a tool's own name first
//! and falls back to `tool_for` when no tool carries that exact name; the fallback is one of the
//! documented changes to the source moved from harness, so the published name plans the right
//! tool by that route too.

use std::collections::BTreeMap;

use serde_json::Value;
use tool_contract::{
    Bound, Bounds, Consequence, Contract, Determinism, Effect, ErrorDecl, Field, Idempotency,
    Operation, Outcome, OutputMarks, Placement, Profile, Requires, Risk, Subject, SubjectKind,
    TypeDecl, TypeKind,
};

use crate::{CommandPlan, ParameterKind, ParameterSpec, ResolvedProvider, ResolvedTool};

/// The toolset every derived operation belongs to, and the family prefix of its name.
pub const TOOLSET: &str = "build-test";
/// The toolset version the derived operations are assembled under.
pub const VERSION: &str = "v1alpha1";
/// The Canon capability every derived operation serves.
pub const CAPABILITY: &str = "build.run";
/// The error the `refused` outcome names: a call the registry or the sandbox would not start.
pub const REFUSED: &str = "build-test.Refused";
/// The error the `check-failed` outcome of an `empty-stdout` tool names: the program ran and
/// wrote to standard output.
pub const CHECK_FAILED: &str = "build-test.CheckFailed";
/// The named type one process answers with, which an operation whose plan can start several
/// processes lists. The type grammar admits `[A-Za-z0-9._]` in a type name, so the toolset's
/// hyphen cannot prefix it the way `fs.Kind` is prefixed.
pub const RUN_TYPE: &str = "toolchain.Run";
/// The wall time a run gets, which is also the most it may ask for: the run ceiling of the
/// harness this crate came from (`MAX_RUN_SECONDS`, 600 s), in milliseconds.
const TIMEOUT_MS: u64 = 600 * 1000;
/// The output a run keeps across both streams: that harness's `MAX_RUN_OUTPUT_BYTES`, 64 KiB.
const OUTPUT_BYTES: u64 = 64 * 1024;

/// One `tool-contract/1` document over the operations of these providers: toolset [`TOOLSET`],
/// version [`VERSION`], every provider's operations in order, and only the errors and types
/// those operations name, so that the document a workspace with no provider derives is still
/// one the profile accepts.
pub fn contract(providers: &[ResolvedProvider]) -> Contract {
    let operations: Vec<Operation> = providers
        .iter()
        .flat_map(ResolvedProvider::operations)
        .collect();
    let named = |error: &str| operations.iter().any(|op| op.names_error(error));
    let mut errors = Vec::new();
    if named(REFUSED) {
        errors.push(refused());
    }
    if named(CHECK_FAILED) {
        errors.push(check_failed());
    }
    let runs = runs_type();
    let lists_runs = operations
        .iter()
        .any(|op| op.output.iter().any(|field| field.ty == runs));
    Contract {
        format: tool_contract::document::FORMAT.to_owned(),
        toolset: TOOLSET.to_owned(),
        family: None,
        version: VERSION.to_owned(),
        summary: "Toolchain runs the declarative registry derives per workspace: one operation \
                  per resolved tool of every provider whose project markers the workspace \
                  carries, each running fixed argv inside the provider's sandbox."
            .to_owned(),
        types: if lists_runs {
            vec![run_type()]
        } else {
            Vec::new()
        },
        errors,
        operations,
    }
}

/// The [`REFUSED`] error: the call was not started, and `message` says why.
pub fn refused() -> ErrorDecl {
    ErrorDecl {
        name: REFUSED.to_owned(),
        summary: Some(
            "The call was not started: an argument was missing, mistyped, outside its enum, \
             escaping the workspace or spelled as an option, a path it writes lands outside the \
             workspace or outside the caller's write scope, no provider the workspace resolves \
             declares the operation, the call's deadline had passed, or the program is not one \
             the sandbox admits or could not be started; or a started program could not be \
             reported on."
                .to_owned(),
        ),
        fields: vec![Field {
            name: "message".to_owned(),
            ty: "String".to_owned(),
            description: Some("Why the call was refused, in one sentence.".to_owned()),
        }],
    }
}

/// The [`CHECK_FAILED`] error: the program of an `empty-stdout` tool ran and wrote to standard
/// output, and `stdout` carries what it wrote.
pub fn check_failed() -> ErrorDecl {
    ErrorDecl {
        name: CHECK_FAILED.to_owned(),
        summary: Some(
            "The program ran and wrote to standard output, which this check reads as a failure \
             whatever the exit status."
                .to_owned(),
        ),
        fields: vec![Field {
            name: "stdout".to_owned(),
            ty: "String".to_owned(),
            description: Some(
                "What the program wrote to standard output, cut to the output bound.".to_owned(),
            ),
        }],
    }
}

/// The [`RUN_TYPE`] struct: what one process answers, in the eight fields a plain operation
/// carries as its output.
pub fn run_type() -> TypeDecl {
    TypeDecl {
        name: RUN_TYPE.to_owned(),
        kind: TypeKind::Struct,
        summary: Some("What one process answers.".to_owned()),
        variants: Vec::new(),
        fields: run_fields(),
        ty: None,
    }
}

/// The operation name a tool derives: `build-test.<segment>`, the segment being the tool's
/// name brought into the profile's `[a-z][a-z0-9_-]*`: lower-cased, every other character
/// replaced by `_`, and prefixed `t_` when it does not start with a letter. A name already in
/// the grammar is unchanged, which is every built-in. Two tools whose names collapse to one
/// segment surface as a duplicate-name violation when the document is validated.
pub fn operation_name(tool: &str) -> String {
    let mut segment: String = tool
        .chars()
        .map(|c| match c.to_ascii_lowercase() {
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' => c,
            _ => '_',
        })
        .collect();
    if !segment.starts_with(|c: char| c.is_ascii_lowercase()) {
        segment.insert_str(0, "t_");
    }
    format!("{TOOLSET}.{segment}")
}

impl ResolvedProvider {
    /// One `build-test.<tool>` operation per resolved tool, in declaration order.
    pub fn operations(&self) -> Vec<Operation> {
        self.tools.iter().map(|tool| tool.operation(self)).collect()
    }

    /// The tool whose derived operation is `operation`. Both the published `build-test.<op>`
    /// and its bare `<op>` segment are accepted; either is matched against [`operation_name`]
    /// of each tool's own name. A tool's own name is not a published name when it differs from
    /// its segment (`Build` is found as `build`, not as `Build`); [`ResolvedProvider::plan`] is
    /// the route by tool name. Two tools whose names collapse to one segment are refused by the
    /// validator; here the first in declaration order is found.
    pub fn tool_for(&self, operation: &str) -> Option<&ResolvedTool> {
        let published = if operation
            .strip_prefix(TOOLSET)
            .is_some_and(|rest| rest.starts_with('.'))
        {
            operation.to_owned()
        } else {
            format!("{TOOLSET}.{operation}")
        };
        self.tools
            .iter()
            .find(|tool| operation_name(&tool.name) == published)
    }

    /// The plan of the tool whose derived operation is `operation`, as [`ResolvedProvider::plan`]
    /// answers it for that tool: `None` when no tool derives the name, else the tool's own plan
    /// or refusal for `arguments`.
    pub fn plan_operation(
        &self,
        operation: &str,
        arguments: &Value,
    ) -> Option<Result<CommandPlan, String>> {
        self.tool_for(operation).map(|tool| tool.plan(arguments))
    }
}

impl ResolvedTool {
    /// The `build-test.<tool>` operation this tool publishes under `provider`'s sandbox.
    pub fn operation(&self, provider: &ResolvedProvider) -> Operation {
        let name = operation_name(&self.name);
        let several = self.several_processes();
        let mut outcomes = vec![
            Outcome {
                name: "ran".to_owned(),
                summary: Some(
                    if several {
                        "Each process of the plan ran to its end or its timeout; their exit \
                         statuses and output are the result, whatever the statuses."
                    } else {
                        "The program ran to its end or its timeout; its exit status and output \
                         are the result, whatever the status."
                    }
                    .to_owned(),
                ),
                error: None,
            },
            Outcome {
                name: "refused".to_owned(),
                summary: Some(
                    "Nothing was started, or a started program could not be reported on."
                        .to_owned(),
                ),
                error: Some(REFUSED.to_owned()),
            },
        ];
        let mut guidance = None;
        if self.empty_stdout {
            outcomes.push(Outcome {
                name: "check-failed".to_owned(),
                summary: Some("The program ran and wrote to standard output.".to_owned()),
                error: Some(CHECK_FAILED.to_owned()),
            });
            guidance = Some(
                "A check whose standard output is expected to be empty: anything the program \
                 writes there is answered as the check-failed outcome carrying that output, \
                 whatever the exit status."
                    .to_owned(),
            );
        }
        Operation {
            summary: self.description.clone(),
            guidance,
            renderings: BTreeMap::new(),
            input: self
                .parameters
                .iter()
                .map(|(parameter, spec)| self.field(parameter, spec))
                .collect(),
            output: if several {
                vec![runs_field()]
            } else {
                run_fields()
            },
            outcomes,
            profile: self.profile(provider, &name),
            name,
        }
    }

    /// Whether the plan can start more than one process: a command list longer than one, or a
    /// command that runs once per item of a list parameter. The harness this crate came from
    /// answers one object for one process and an array for several.
    pub(crate) fn several_processes(&self) -> bool {
        self.commands.len() > 1
            || self
                .commands
                .iter()
                .any(|command| command.for_each.is_some())
    }

    /// The profile of what the executor ([`crate::executor`]) does with the tool: it starts the
    /// declared program on the caller's own machine, with the caller's `HOME` and caches, and
    /// confines nothing but the environment (cleared to an allow-list), the time, the output and
    /// the paths a call names as written. So every operation is `local` and declares every
    /// effect and consequence such a runner allows, at `high` risk, as `process.run` does,
    /// whether or not the tool declares `workspace-writes` (coordinator decision D7, wave w3).
    /// The sandbox's programs, environment names and roots stay under `requires`, and each
    /// parameter named in `writes` is a `path` subject.
    fn profile(&self, provider: &ResolvedProvider, name: &str) -> Profile {
        Profile {
            effects: vec![
                Effect::Process,
                Effect::Write,
                Effect::Network,
                Effect::Secret,
            ],
            consequences: vec![
                Consequence::Money,
                Consequence::Delete,
                Consequence::SendExternal,
                Consequence::Irreversible,
                Consequence::SensitiveData,
            ],
            risk: Risk::High,
            idempotency: Idempotency::NonIdempotent,
            preconditions: Vec::new(),
            determinism: Determinism::Nondeterministic,
            bounds: Bounds {
                timeout_ms: Some(Bound {
                    default: Some(TIMEOUT_MS),
                    max: Some(TIMEOUT_MS),
                }),
                output_bytes: Some(Bound {
                    default: Some(OUTPUT_BYTES),
                    max: Some(OUTPUT_BYTES),
                }),
                ..Bounds::default()
            },
            paging: None,
            requires: Requires {
                binaries: provider.programs.clone(),
                env: provider.env.keys().cloned().collect(),
                settings: provider
                    .roots
                    .iter()
                    .map(|root| root.name.clone())
                    .collect(),
                secrets: Vec::new(),
            },
            subjects: self
                .writes
                .iter()
                .map(|field| Subject {
                    field: field.clone(),
                    kind: SubjectKind::Path,
                    pattern: Some(format!("{name}({{{field}}})")),
                })
                .collect(),
            placement: Placement::Local,
            capability: CAPABILITY.to_owned(),
            output: Some(OutputMarks {
                untrusted: true,
                failure_as_result: true,
            }),
        }
    }

    /// One input field for a parameter: its kind's type, `Optional` unless required. The
    /// description carries what `Field` has no slot for, `;`-separated: the parameter's own
    /// description, the containment a path kind is held to (so a provider that turns a path
    /// into a plain string derives a different operation), an enum's values as resolved for
    /// this workspace, its default, and the item bounds of a list.
    fn field(&self, name: &str, spec: &ParameterSpec) -> Field {
        let base = match spec.kind {
            ParameterKind::String | ParameterKind::WorkspacePath | ParameterKind::Enum => "String",
            ParameterKind::Boolean => "Boolean",
            ParameterKind::StringList | ParameterKind::WorkspacePathList => "List<String>",
        };
        let ty = if spec.required {
            base.to_owned()
        } else {
            format!("Optional<{base}>")
        };
        let mut parts: Vec<String> = Vec::new();
        if let Some(text) = &spec.description {
            parts.push(text.clone());
        }
        match spec.kind {
            ParameterKind::WorkspacePath => parts.push(
                "a path relative to the workspace root, refused when it leaves the workspace"
                    .to_owned(),
            ),
            ParameterKind::WorkspacePathList => parts.push(
                "paths relative to the workspace root, each refused when it leaves the workspace"
                    .to_owned(),
            ),
            _ => {}
        }
        if spec.kind == ParameterKind::Enum
            && let Some(items) = spec
                .values
                .as_ref()
                .and_then(|source| self.values.get(source))
            && !items.is_empty()
        {
            parts.push(format!("one of: {}", items.join(", ")));
        }
        if let Some(default) = &spec.default {
            parts.push(format!("default {}", literal(default)));
        }
        match (spec.min_items, spec.max_items) {
            (Some(min), Some(max)) => parts.push(format!("{min} to {max} items")),
            (Some(min), None) => parts.push(format!("at least {min} items")),
            (None, Some(max)) => parts.push(format!("up to {max} items")),
            (None, None) => {}
        }
        Field {
            name: name.to_owned(),
            ty,
            description: (!parts.is_empty()).then(|| parts.join("; ")),
        }
    }
}

/// A default as a reader would type it: a string bare, anything else as JSON.
fn literal(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// What one process answers, as the harness this crate came from answers it (`exec` in its
/// `local.rs`): the argv it started, the exit status, the two streams each cut to the output
/// bound with a marker where bytes were dropped, and how the bounds were met.
fn run_fields() -> Vec<Field> {
    [
        ("argv", "List<String>", "The argv that was started."),
        (
            "exit",
            "Optional<Integer>",
            "The exit status; null when the program was ended by a signal, including at the timeout.",
        ),
        (
            "stdout",
            "String",
            "What the program wrote to standard output, cut to the output bound with a marker where bytes were dropped.",
        ),
        (
            "stderr",
            "String",
            "What the program wrote to standard error, cut the same way.",
        ),
        (
            "truncated",
            "Boolean",
            "Whether either stream was cut to the output bound.",
        ),
        (
            "omitted_bytes",
            "Integer",
            "How many bytes were dropped across both streams.",
        ),
        (
            "timed_out",
            "Boolean",
            "Whether the program was ended at the timeout.",
        ),
        (
            "timeout_ms",
            "Integer",
            "The wall time the run was given, in milliseconds.",
        ),
    ]
    .into_iter()
    .map(|(name, ty, description)| Field {
        name: name.to_owned(),
        ty: ty.to_owned(),
        description: Some(description.to_owned()),
    })
    .collect()
}

/// The output of an operation whose plan can start several processes: one run per process.
fn runs_field() -> Field {
    Field {
        name: "runs".to_owned(),
        ty: runs_type(),
        description: Some(
            "One run per process the plan started, in the order they ran.".to_owned(),
        ),
    }
}

fn runs_type() -> String {
    format!("List<{RUN_TYPE}>")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use serde_json::json;
    use tool_contract::{
        Bound, Consequence, Determinism, Effect, Field, Idempotency, Operation, OutputMarks,
        Placement, Risk, Subject, SubjectKind, TypeKind, validate,
    };

    use super::{
        CHECK_FAILED, REFUSED, RUN_TYPE, TOOLSET, VERSION, contract, operation_name, refused,
        run_fields, run_type,
    };
    use crate::{
        DeclaredProvider, Registry, ResolvedProvider, ResolvedRoot, expand_env, project_matches,
        resolve_tool, resolve_values,
    };

    /// One fixture workspace per built-in provider, under one temporary directory: a Cargo
    /// workspace, a Go module, a Taskfile, a package with a `package-lock.json` and one with a
    /// `yarn.lock`. The root is canonical because value sources check containment against it.
    fn fixtures() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonical fixture root");
        let write = |provider: &str, file: &str, text: &str| {
            let path = root.join(provider).join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture dir");
            std::fs::write(path, text).expect("fixture file");
        };
        write("rust", "Cargo.toml", "[workspace]\nmembers = []\n");
        write("go", "go.mod", "module fixture\n\ngo 1.22\n");
        write(
            "taskfile",
            "Taskfile.yml",
            "version: '3'\ntasks:\n  build: {}\n  test: {}\n",
        );
        write(
            "npm",
            "package.json",
            r#"{"scripts":{"test":"node test.js"}}"#,
        );
        write("npm", "package-lock.json", "{}");
        write(
            "yarn",
            "package.json",
            r#"{"scripts":{"test":"node test.js"}}"#,
        );
        write("yarn", "yarn.lock", "# yarn lockfile v1\n");
        (dir, root)
    }

    /// A provider resolved against a fixture workspace without touching the machine. The project
    /// probe and the value sources run as `Registry::resolve` runs them; roots are taken as
    /// declared and programs are not looked up, because `resolve` needs a rustup root and `go`,
    /// `task`, `npm` and `yarn` on this machine's `PATH`, which a fixture cannot provide.
    fn resolve_offline(workspace: &Path, declared: &DeclaredProvider) -> ResolvedProvider {
        let spec = &declared.spec;
        assert!(
            project_matches(workspace, &spec.project).expect("a project probe"),
            "the {} fixture matches its provider",
            spec.name
        );
        let roots: Vec<ResolvedRoot> = spec
            .roots
            .iter()
            .map(|root| ResolvedRoot {
                name: root.name.clone(),
                host: workspace.to_path_buf(),
                mount: format!("/toolchain/{}", root.mount),
            })
            .collect();
        let root_map: BTreeMap<&str, &ResolvedRoot> = roots
            .iter()
            .map(|root| (root.name.as_str(), root))
            .collect();
        let mut values = BTreeMap::new();
        let mut bound_values = BTreeMap::new();
        for source in &spec.values {
            let resolved = resolve_values(workspace, source).expect("a value source");
            if let Some(bound) = resolved.bound {
                bound_values.insert(source.name.clone(), bound);
            }
            values.insert(source.name.clone(), resolved.items);
        }
        let tools = spec
            .tools
            .iter()
            .map(|tool| resolve_tool(tool, &values, &bound_values))
            .collect();
        let env = spec
            .sandbox
            .env
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    expand_env(value, &root_map).expect("a sandbox value"),
                )
            })
            .collect();
        ResolvedProvider {
            name: spec.name.clone(),
            description: spec.description.clone(),
            roots,
            env,
            programs: spec.sandbox.programs.clone(),
            facts: Vec::new(),
            tools,
            provenance: declared.provenance.clone(),
        }
    }

    /// The five built-in providers, each resolved against its fixture.
    fn providers() -> (tempfile::TempDir, Vec<ResolvedProvider>) {
        let (dir, root) = fixtures();
        let registry = Registry::builtins().expect("built-ins");
        let providers = registry
            .providers
            .values()
            .map(|declared| resolve_offline(&root.join(&declared.spec.name), declared))
            .collect();
        (dir, providers)
    }

    fn operation(providers: &[ResolvedProvider], tool: &str) -> Operation {
        providers
            .iter()
            .find_map(|provider| {
                provider
                    .tools
                    .iter()
                    .find(|candidate| candidate.name == tool)
                    .map(|candidate| candidate.operation(provider))
            })
            .unwrap_or_else(|| panic!("{tool} is a built-in tool"))
    }

    fn typed(fields: &[Field]) -> Vec<(&str, &str)> {
        fields
            .iter()
            .map(|field| (field.name.as_str(), field.ty.as_str()))
            .collect()
    }

    fn described(fields: &[Field], name: &str) -> Option<String> {
        fields
            .iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("{name} is a field"))
            .description
            .clone()
    }

    fn lines(violations: &[tool_contract::Violation]) -> Vec<String> {
        violations.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn every_builtin_provider_derives_operations_the_profile_accepts() {
        let (_dir, providers) = providers();
        let contract = contract(&providers);
        assert_eq!(contract.toolset, TOOLSET);
        assert_eq!(contract.version, VERSION);
        let names: Vec<&str> = contract
            .operations
            .iter()
            .map(|op| op.name.as_str())
            .collect();
        assert_eq!(names.len(), 20, "{names:?}");
        for expected in [
            "build-test.go_build",
            "build-test.npm_run",
            "build-test.rust_check",
            "build-test.taskfile_run",
            "build-test.yarn_run",
        ] {
            assert!(names.contains(&expected), "{names:?}");
        }
        if let Err(violations) = validate(&contract) {
            panic!("{}", lines(&violations).join("\n"));
        }
    }

    #[test]
    fn a_document_over_no_providers_declares_nothing_it_does_not_name() {
        let contract = contract(&[]);
        assert_eq!(contract.operations, Vec::new());
        assert_eq!(contract.errors, Vec::new());
        assert_eq!(contract.types, Vec::new());
        assert_eq!(validate(&contract), Ok(()));
    }

    #[test]
    fn a_tool_name_outside_the_profile_grammar_is_brought_into_it() {
        assert_eq!(operation_name("rust_check"), "build-test.rust_check");
        assert_eq!(operation_name("go-fmt"), "build-test.go-fmt");
        assert_eq!(operation_name("Build"), "build-test.build");
        assert_eq!(operation_name("9build"), "build-test.t_9build");
        assert_eq!(operation_name("-build"), "build-test.t_-build");
        assert_eq!(operation_name("_build"), "build-test.t__build");
        assert_eq!(operation_name("a.b"), "build-test.a_b");
    }

    #[test]
    fn two_tools_that_collapse_to_one_segment_are_a_duplicate_name_violation() {
        let (_dir, root) = fixtures();
        let mut registry = Registry::default();
        registry
            .add_document(
                "version: 1\n\
                 toolchains:\n\
                 \x20 - name: probe\n\
                 \x20   description: Two tool names one segment apart.\n\
                 \x20   sandbox:\n\
                 \x20     programs: [sh]\n\
                 \x20   tools:\n\
                 \x20     - name: Build\n\
                 \x20       description: Upper.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n\
                 \x20     - name: build\n\
                 \x20       description: Lower.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n",
                "probe.yaml",
            )
            .expect("a document the registry accepts");
        let declared = registry.providers.get("probe").expect("probe");
        let provider = resolve_offline(&root, declared);
        let violations = validate(&contract(&[provider])).expect_err("a collision is refused");
        assert_eq!(
            lines(&violations),
            ["build-test.build: name: duplicate operation name"]
        );
    }

    /// Coordinator decision D7 (wave w3, correction round 1): the executor runs the program on
    /// the caller's machine, confined to nothing but the cleared environment, the bounds and the
    /// written-path check, so every operation declares what such a runner allows, as
    /// `process.run` does, whatever the tool says it writes.
    #[test]
    fn every_operation_declares_every_effect_and_consequence_an_unconfined_runner_allows() {
        let (_dir, providers) = providers();
        let derived = contract(&providers);
        assert_eq!(derived.operations.len(), 20);
        for op in &derived.operations {
            assert_eq!(
                op.profile.effects,
                [
                    Effect::Process,
                    Effect::Write,
                    Effect::Network,
                    Effect::Secret
                ],
                "{}",
                op.name
            );
            assert_eq!(
                op.profile.consequences,
                [
                    Consequence::Money,
                    Consequence::Delete,
                    Consequence::SendExternal,
                    Consequence::Irreversible,
                    Consequence::SensitiveData,
                ],
                "{}",
                op.name
            );
            assert_eq!(op.profile.risk, Risk::High, "{}", op.name);
            assert_eq!(op.profile.placement, Placement::Local, "{}", op.name);
        }
    }

    /// Coordinator decision on F6 (wave w3, correction round 1): a path parameter's containment
    /// is part of what the operation guarantees, so its field says so and a provider that turns
    /// it into a plain string derives a different operation.
    #[test]
    fn a_path_parameter_names_its_containment_and_a_string_one_does_not() {
        let (_dir, root) = fixtures();
        let mut registry = Registry::default();
        registry
            .add_document(
                "version: 1\n\
                 toolchains:\n\
                 \x20 - name: probe\n\
                 \x20   description: One parameter of each string kind.\n\
                 \x20   sandbox:\n\
                 \x20     programs: [sh]\n\
                 \x20   tools:\n\
                 \x20     - name: probe_kinds\n\
                 \x20       description: Takes every kind.\n\
                 \x20       parameters:\n\
                 \x20         file: { kind: workspace-path }\n\
                 \x20         files: { kind: workspace-path-list, max-items: 4 }\n\
                 \x20         name: { kind: string }\n\
                 \x20         names: { kind: string-list }\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n",
                "probe.yaml",
            )
            .expect("a document the registry accepts");
        let declared = registry.providers.get("probe").expect("probe");
        let provider = resolve_offline(&root, declared);
        let op = &provider.operations()[0];
        assert_eq!(
            described(&op.input, "file").as_deref(),
            Some("a path relative to the workspace root, refused when it leaves the workspace")
        );
        assert_eq!(
            described(&op.input, "files").as_deref(),
            Some(
                "paths relative to the workspace root, each refused when it leaves the \
                 workspace; up to 4 items"
            )
        );
        assert_eq!(described(&op.input, "name"), None);
        assert_eq!(described(&op.input, "names"), None);
    }

    #[test]
    fn a_written_path_list_parameter_is_a_path_subject() {
        let (_dir, providers) = providers();
        let op = operation(&providers, "rust_fmt");
        assert_eq!(
            op.profile.subjects,
            [Subject {
                field: "paths".to_owned(),
                kind: SubjectKind::Path,
                pattern: Some("build-test.rust_fmt({paths})".to_owned()),
            }]
        );
        assert_eq!(typed(&op.input), [("paths", "List<String>")]);
    }

    #[test]
    fn parameters_become_typed_input_fields_optional_unless_required() {
        let (_dir, providers) = providers();
        let op = operation(&providers, "rust_run");
        assert_eq!(
            typed(&op.input),
            [
                ("args", "Optional<List<String>>"),
                ("bin", "String"),
                ("package", "Optional<String>"),
                ("release", "Optional<Boolean>"),
            ]
        );
    }

    #[test]
    fn an_enum_parameter_is_a_string_that_names_its_resolved_values() {
        let (_dir, providers) = providers();
        let op = operation(&providers, "taskfile_run");
        let task = op
            .input
            .iter()
            .find(|field| field.name == "task")
            .expect("the task parameter");
        assert_eq!(task.ty, "String");
        assert_eq!(task.description.as_deref(), Some("one of: build, test"));
    }

    #[test]
    fn a_parameters_default_and_item_bounds_reach_its_description() {
        let (_dir, providers) = providers();
        let go_build = operation(&providers, "go_build");
        assert_eq!(
            described(&go_build.input, "package").as_deref(),
            Some("default ./...")
        );
        let rust_run = operation(&providers, "rust_run");
        assert_eq!(
            described(&rust_run.input, "args").as_deref(),
            Some("up to 128 items")
        );
        assert_eq!(described(&rust_run.input, "package"), None);
        let rust_fmt = operation(&providers, "rust_fmt");
        assert_eq!(
            described(&rust_fmt.input, "paths").as_deref(),
            Some(
                "paths relative to the workspace root, each refused when it leaves the \
                 workspace; 1 to 128 items"
            )
        );
        let taskfile_run = operation(&providers, "taskfile_run");
        assert_eq!(
            described(&taskfile_run.input, "args").as_deref(),
            Some("up to 128 items")
        );
    }

    #[test]
    fn the_output_is_what_a_run_answers() {
        let (_dir, providers) = providers();
        let op = operation(&providers, "go_test");
        assert_eq!(
            typed(&op.output),
            [
                ("argv", "List<String>"),
                ("exit", "Optional<Integer>"),
                ("stdout", "String"),
                ("stderr", "String"),
                ("truncated", "Boolean"),
                ("omitted_bytes", "Integer"),
                ("timed_out", "Boolean"),
                ("timeout_ms", "Integer"),
            ]
        );
        assert_eq!(op.output, run_fields());
        assert_eq!(
            op.profile.output,
            Some(OutputMarks {
                untrusted: true,
                failure_as_result: true,
            })
        );
    }

    #[test]
    fn a_for_each_tool_answers_a_list_of_runs_and_the_document_declares_the_type() {
        let (_dir, providers) = providers();
        let rust_fmt = operation(&providers, "rust_fmt");
        assert_eq!(typed(&rust_fmt.output), [("runs", "List<toolchain.Run>")]);
        let go_fmt = operation(&providers, "go_fmt");
        assert_eq!(
            go_fmt.output,
            run_fields(),
            "`each` expands every path into one argv, so one process"
        );
        let whole = contract(&providers);
        assert_eq!(whole.types, [run_type()]);
        assert_eq!(whole.types[0].name, RUN_TYPE);
        assert_eq!(whole.types[0].kind, TypeKind::Struct);
        assert_eq!(whole.types[0].fields, run_fields());
        let plain: Vec<ResolvedProvider> = providers
            .iter()
            .filter(|provider| provider.name == "npm")
            .cloned()
            .collect();
        assert_eq!(contract(&plain).types, Vec::new());
    }

    #[test]
    fn an_empty_stdout_check_answers_check_failed_and_says_so_in_its_guidance() {
        let (_dir, providers) = providers();
        let check = operation(&providers, "go_fmt_check");
        let outcomes: Vec<(&str, Option<&str>)> = check
            .outcomes
            .iter()
            .map(|outcome| (outcome.name.as_str(), outcome.error.as_deref()))
            .collect();
        assert_eq!(
            outcomes,
            [
                ("ran", None),
                ("refused", Some(REFUSED)),
                ("check-failed", Some(CHECK_FAILED)),
            ]
        );
        let guidance = check.guidance.expect("a check carries guidance");
        assert!(guidance.contains("standard output"), "{guidance}");
        let run = operation(&providers, "go_test");
        assert_eq!(run.guidance, None);
        assert_eq!(run.outcomes.len(), 2);
        let whole = contract(&providers);
        let errors: Vec<&str> = whole.errors.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(errors, [REFUSED, CHECK_FAILED]);
        let plain: Vec<ResolvedProvider> = providers
            .iter()
            .filter(|provider| provider.name == "npm")
            .cloned()
            .collect();
        let plain = contract(&plain);
        let errors: Vec<&str> = plain.errors.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(errors, [REFUSED]);
    }

    #[test]
    fn requires_comes_from_the_providers_sandbox_and_placement_is_local() {
        let (_dir, providers) = providers();
        let rust = operation(&providers, "rust_test");
        assert_eq!(
            rust.profile.requires.binaries,
            ["cargo", "rustc", "rustfmt"]
        );
        assert_eq!(
            rust.profile.requires.env,
            [
                "CARGO_HOME",
                "CARGO_TARGET_DIR",
                "HOME",
                "PATH",
                "RUSTUP_HOME"
            ]
        );
        assert_eq!(rust.profile.requires.settings, ["rustup"]);
        assert_eq!(rust.profile.requires.secrets, Vec::<String>::new());
        assert_eq!(rust.profile.placement, Placement::Local);
        let npm = operation(&providers, "npm_run");
        assert_eq!(npm.profile.requires.binaries, ["npm"]);
        assert_eq!(npm.profile.requires.settings, Vec::<String>::new());
        assert_eq!(npm.profile.placement, Placement::Local);
    }

    #[test]
    fn the_bounds_are_the_run_ceilings_and_the_outcomes_name_the_refusal() {
        let (_dir, providers) = providers();
        let op = operation(&providers, "go_fmt");
        assert_eq!(
            op.profile.bounds.timeout_ms,
            Some(Bound {
                default: Some(600_000),
                max: Some(600_000),
            })
        );
        assert_eq!(
            op.profile.bounds.output_bytes,
            Some(Bound {
                default: Some(65_536),
                max: Some(65_536),
            })
        );
        assert_eq!(op.profile.idempotency, Idempotency::NonIdempotent);
        assert_eq!(op.profile.determinism, Determinism::Nondeterministic);
        assert_eq!(op.profile.capability, "build.run");
        assert_eq!(op.profile.consequences.len(), 5);
        let outcomes: Vec<(&str, Option<&str>)> = op
            .outcomes
            .iter()
            .map(|outcome| (outcome.name.as_str(), outcome.error.as_deref()))
            .collect();
        assert_eq!(outcomes, [("ran", None), ("refused", Some(REFUSED))]);
        let declared = refused();
        assert_eq!(declared.name, REFUSED);
        assert_eq!(typed(&declared.fields), [("message", "String")]);
    }

    #[test]
    fn a_published_name_routes_back_to_the_tool_that_derives_it() {
        let (_dir, root) = fixtures();
        let mut registry = Registry::default();
        registry
            .add_document(
                "version: 1\n\
                 toolchains:\n\
                 \x20 - name: probe\n\
                 \x20   description: Tool names the registry admits and the grammar does not.\n\
                 \x20   sandbox:\n\
                 \x20     programs: [sh]\n\
                 \x20   tools:\n\
                 \x20     - name: Build\n\
                 \x20       description: An uppercase initial.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n\
                 \x20     - name: \"-\"\n\
                 \x20       description: A dash alone.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n\
                 \x20     - name: \"9\"\n\
                 \x20       description: A digit alone.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n\
                 \x20     - name: probe_plain\n\
                 \x20       description: A name already in the grammar.\n\
                 \x20       commands:\n\
                 \x20         - argv: [sh]\n",
                "probe.yaml",
            )
            .expect("a document the registry accepts");
        let declared = registry.providers.get("probe").expect("probe");
        let provider = resolve_offline(&root, declared);
        for (operation, tool) in [
            ("build-test.build", "Build"),
            ("build", "Build"),
            ("build-test.t_-", "-"),
            ("t_-", "-"),
            ("build-test.t_9", "9"),
            ("t_9", "9"),
            ("build-test.probe_plain", "probe_plain"),
            ("probe_plain", "probe_plain"),
        ] {
            let found = provider
                .tool_for(operation)
                .unwrap_or_else(|| panic!("{operation} routes to a tool"));
            assert_eq!(found.name, tool, "{operation}");
        }
        let own_name = provider.tool_for("Build").map(|tool| tool.name.as_str());
        assert_eq!(
            own_name, None,
            "a tool's own name outside the grammar is not published"
        );
        assert_eq!(
            provider
                .tool_for("build-test.missing")
                .map(|tool| tool.name.as_str()),
            None
        );
        for op in provider.operations() {
            let planned = provider
                .plan_operation(&op.name, &json!({}))
                .unwrap_or_else(|| panic!("{} plans", op.name))
                .expect("no parameters, so nothing to refuse");
            assert_eq!(planned.argv, [["sh"]], "{}", op.name);
        }
    }

    #[test]
    fn plan_operation_answers_what_plan_answers_for_the_tool() {
        let (_dir, providers) = providers();
        let rust = providers
            .iter()
            .find(|provider| provider.name == "rust")
            .expect("rust");
        let arguments = json!({"paths": ["src/lib.rs", "src/main.rs"]});
        let by_operation = rust
            .plan_operation("build-test.rust_fmt", &arguments)
            .expect("rust_fmt derives it")
            .expect("two contained paths");
        let by_tool = rust
            .plan("rust_fmt", &arguments)
            .expect("rust_fmt is a tool")
            .expect("two contained paths");
        assert_eq!(by_operation, by_tool);
        assert_eq!(
            by_operation.argv,
            [["rustfmt", "src/lib.rs"], ["rustfmt", "src/main.rs"]]
        );
        assert_eq!(by_operation.writes, ["src/lib.rs", "src/main.rs"]);
        let bad = json!({"release": "yes"});
        let refused = rust
            .plan_operation("rust_check", &bad)
            .expect("rust_check derives it")
            .expect_err("a boolean parameter refuses a string");
        let same = rust
            .plan("rust_check", &bad)
            .expect("rust_check is a tool")
            .expect_err("the same refusal");
        assert_eq!(refused, same);
        assert_eq!(rust.plan_operation("build-test.npm_run", &json!({})), None);
    }

    /// The input fields of `tool` whose parameter is an enum: their values are the workspace's
    /// own, resolved per workspace, so the static document says where they come from instead of
    /// listing them.
    fn enum_parameters(tool: &crate::ResolvedTool) -> Vec<String> {
        tool.parameters
            .iter()
            .filter(|(_, spec)| spec.kind == crate::ParameterKind::Enum)
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn without_descriptions(op: &Operation, fields: &[String]) -> Operation {
        let mut op = op.clone();
        for field in &mut op.input {
            if fields.contains(&field.name) {
                field.description = None;
            }
        }
        op
    }

    #[test]
    fn the_static_document_equals_what_the_builtins_derive_operation_for_operation() {
        let (_dir, providers) = providers();
        let derived = contract(&providers);
        let yaml = serde_yaml_ng::to_string(&derived).expect("a contract serialises");
        println!("the document the five built-in providers derive:\n{yaml}");
        let document = crate::document();
        assert_eq!(document.format, tool_contract::document::FORMAT);
        assert_eq!(document.toolset, TOOLSET);
        assert_eq!(document.version, VERSION);
        assert_eq!(validate(document), Ok(()));
        let names = |of: &tool_contract::Contract| -> Vec<String> {
            of.operations.iter().map(|op| op.name.clone()).collect()
        };
        assert_eq!(names(document), names(&derived));
        let mut compared = 0;
        for provider in &providers {
            for tool in &provider.tools {
                let name = operation_name(&tool.name);
                let fixed = document.operation(&name).expect("named above");
                let resolved = derived.operation(&name).expect("named above");
                let enums = enum_parameters(tool);
                assert_eq!(
                    without_descriptions(fixed, &enums),
                    without_descriptions(resolved, &enums),
                    "{name}"
                );
                for field in &enums {
                    let fixed = described(&fixed.input, field).unwrap_or_default();
                    assert!(
                        fixed.contains("resolved per workspace"),
                        "{name}.{field}: {fixed:?}"
                    );
                    let resolved = described(&resolved.input, field).unwrap_or_default();
                    assert!(
                        resolved.starts_with("one of: "),
                        "{name}.{field}: {resolved:?}"
                    );
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 20);
        assert_eq!(document.errors, derived.errors);
        assert_eq!(document.types, derived.types);
    }
}
