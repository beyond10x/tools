//! Adversary cases against the `build-test` derivation, reached only through the crate's public
//! API: `Registry::builtins`, `Registry::load_file`, `Registry::resolve`, `ResolvedProvider::
//! operations`, `ResolvedTool::plan`, `contract::contract` and `tool_contract::validate`.
//!
//! The custom providers here start `sh` and nothing else; no plan is ever executed.

use serde_json::json;
use tool_contract::validate;
use tool_toolchain::contract::contract;
use tool_toolchain::{Registry, ResolvedProvider};

/// A registry of the built-ins plus one custom document, resolved against an empty workspace
/// with the custom provider selected. The provider's project probe is empty, so it matches.
fn custom(text: &str, provider: &str) -> (tempfile::TempDir, Vec<ResolvedProvider>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec = dir.path().join("toolchain.yaml");
    std::fs::write(&spec, text).expect("spec file");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let mut registry = Registry::builtins().expect("built-ins");
    registry
        .load_file(&spec)
        .expect("a custom document the registry accepts");
    let providers = registry
        .resolve(&workspace, Some(&[provider.to_owned()]))
        .expect("the custom provider resolves: `sh` is on PATH and it has no roots");
    (dir, providers)
}

fn violations(providers: &[ResolvedProvider]) -> Vec<String> {
    validate(&contract(providers))
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Boundary: a workspace that carries no provider's project markers. `Registry::resolve` answers
/// no providers, and the document derived over none must still be a document the profile
/// accepts, because it is what every consumer of `contract(&resolved)` gets for such a workspace.
#[test]
fn a_workspace_matching_no_provider_derives_a_document_the_profile_accepts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry = Registry::builtins().expect("built-ins");
    let providers = registry
        .resolve(dir.path(), None)
        .expect("an empty workspace resolves to no providers");
    assert_eq!(providers.len(), 0, "no marker, no provider");
    let found = violations(&providers);
    assert_eq!(found, Vec::<String>::new(), "{found:?}");
}

/// Acceptance: the derivation claims one `build-test.<tool>` operation per resolved tool that the
/// profile accepts. The registry's own name rule (`legal_name`) admits ASCII letters of either
/// case, digits, `_`, `-` and `.` in any order; the profile's `<op>` grammar is
/// `[a-z][a-z0-9_-]*`. The derivation brings such a name into the grammar rather than refusing
/// it (coordinator decision, pass 2): lower-cased, every other character replaced by `_`, and
/// prefixed `t_` when the result does not start with a letter. The derived names are pinned here
/// and the document they form must pass the validator.
#[test]
fn every_tool_name_the_registry_admits_derives_an_operation_name_the_profile_accepts() {
    let (_dir, providers) = custom(
        "version: 1\n\
         toolchains:\n\
         \x20 - name: probe\n\
         \x20   description: A custom provider whose tool names the registry admits.\n\
         \x20   sandbox:\n\
         \x20     programs: [sh]\n\
         \x20   tools:\n\
         \x20     - name: Build\n\
         \x20       description: An uppercase initial.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n\
         \x20     - name: 9build\n\
         \x20       description: A digit initial.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n\
         \x20     - name: \"-build\"\n\
         \x20       description: A hyphen initial.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n",
        "probe",
    );
    let names: Vec<String> = providers
        .iter()
        .flat_map(ResolvedProvider::operations)
        .map(|op| op.name)
        .collect();
    assert_eq!(
        names,
        [
            "build-test.build",
            "build-test.t_9build",
            "build-test.t_-build"
        ],
        "the registry admitted all three and the derivation brought each into the grammar"
    );
    let found = violations(&providers);
    assert_eq!(found, Vec::<String>::new(), "{found:?}");
}

/// Contract drift: `output()` documents itself as "what a run answers, as the harness this crate
/// came from answers it". That harness's `exec` answers `argv`, `exit`, `stdout`, `stderr`,
/// `truncated`, `omitted_bytes`, `timed_out` and `timeout_ms`, and the profile's shape check
/// refuses a derived field the contract does not name. `timed_out` is the one that matters: the
/// contract folds a timeout into `exit: null`, which is also what a signal answers.
#[test]
fn the_output_names_every_field_the_harness_run_answers() {
    let (_dir, providers) = custom(
        "version: 1\n\
         toolchains:\n\
         \x20 - name: probe\n\
         \x20   description: A custom provider with one plain run.\n\
         \x20   sandbox:\n\
         \x20     programs: [sh]\n\
         \x20   tools:\n\
         \x20     - name: probe_run\n\
         \x20       description: One process.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n",
        "probe",
    );
    let op = &providers[0].operations()[0];
    let mut names: Vec<&str> = op.output.iter().map(|field| field.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "argv",
            "exit",
            "omitted_bytes",
            "stderr",
            "stdout",
            "timed_out",
            "timeout_ms",
            "truncated",
        ]
    );
}

/// Contract drift: a `for-each` command starts one process per list item, and the harness this
/// crate came from answers a JSON array of runs when a plan has more than one argv and a single
/// run when it has one. The derived operation declares the single-run output whatever the plan
/// does. `rust_fmt` (`builtins/rust.yaml`, `for-each: paths`) is the built-in instance; this
/// probe is its shape without a rustup root.
#[test]
fn a_for_each_tool_whose_plan_starts_several_processes_does_not_declare_a_single_runs_output() {
    let (_dir, providers) = custom(
        "version: 1\n\
         toolchains:\n\
         \x20 - name: probe\n\
         \x20   description: A custom provider with a for-each tool and a plain one.\n\
         \x20   sandbox:\n\
         \x20     programs: [sh]\n\
         \x20   tools:\n\
         \x20     - name: probe_each\n\
         \x20       description: One process per named path.\n\
         \x20       workspace-writes: true\n\
         \x20       parameters:\n\
         \x20         paths: { kind: workspace-path-list, required: true, min-items: 1, max-items: 128 }\n\
         \x20       writes: [paths]\n\
         \x20       commands:\n\
         \x20         - for-each: paths\n\
         \x20           argv: [sh, { item: true }]\n\
         \x20     - name: probe_once\n\
         \x20       description: One process.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n",
        "probe",
    );
    let provider = &providers[0];
    let plan = provider
        .plan("probe_each", &json!({"paths": ["a.txt", "b.txt"]}))
        .expect("probe_each is a tool of the provider")
        .expect("two contained relative paths are a valid argument");
    assert_eq!(
        plan.argv,
        [["sh", "a.txt"], ["sh", "b.txt"]],
        "the plan starts two processes"
    );
    let ops = provider.operations();
    let each = ops
        .iter()
        .find(|op| op.name == "build-test.probe_each")
        .expect("probe_each");
    let once = ops
        .iter()
        .find(|op| op.name == "build-test.probe_once")
        .expect("probe_once");
    assert_ne!(
        each.output, once.output,
        "a two-process plan and a one-process plan declare the same output: {:?}",
        each.output
    );
}

/// Contract drift: a tool declared `empty-stdout: true` is one the harness this crate came from
/// answers as an error when the program ran and wrote to stdout, which is neither the `ran`
/// outcome ("its exit status and output are the result, whatever the status") nor the `refused`
/// one ("Nothing was started"), and contradicts `failure_as_result: true`. The flag has to reach
/// the operation somewhere: an outcome, the guidance, the output or the profile. `go_fmt_check`
/// (`builtins/go.yaml`, `empty-stdout: true`) is the built-in instance.
#[test]
fn an_empty_stdout_check_derives_an_operation_that_differs_from_a_plain_run() {
    let (_dir, providers) = custom(
        "version: 1\n\
         toolchains:\n\
         \x20 - name: probe\n\
         \x20   description: A custom provider with an empty-stdout check and a plain run.\n\
         \x20   sandbox:\n\
         \x20     programs: [sh]\n\
         \x20   tools:\n\
         \x20     - name: probe_check\n\
         \x20       description: Fails when the program writes anything to stdout.\n\
         \x20       empty-stdout: true\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n\
         \x20     - name: probe_run\n\
         \x20       description: Fails when the program writes anything to stdout.\n\
         \x20       commands:\n\
         \x20         - argv: [sh]\n",
        "probe",
    );
    let ops = providers[0].operations();
    let check = ops
        .iter()
        .find(|op| op.name == "build-test.probe_check")
        .expect("probe_check");
    let run = ops
        .iter()
        .find(|op| op.name == "build-test.probe_run")
        .expect("probe_run");
    let shape = |op: &tool_contract::Operation| {
        (
            op.outcomes.clone(),
            op.output.clone(),
            op.guidance.clone(),
            op.profile.clone(),
        )
    };
    assert_ne!(
        shape(check),
        shape(run),
        "apart from its name, the empty-stdout check derives the same operation as a plain run"
    );
}

/// Pass 2. The derivation now brings a tool name into the profile's grammar instead of refusing
/// it, so an operation's name is no longer the tool's name. The name is the only handle the
/// contract gives a caller (`Operation` has no slot for the source tool), and the crate's only
/// call API, `ResolvedProvider::plan`, matches `tool.name` exactly. A dash-only and a digit-only
/// name are brought into the grammar and the document validates (the green half); the operations
/// they derive, and `Build`'s, cannot be planned by the name the contract publishes.
#[test]
fn every_derived_operation_name_routes_back_to_the_tool_that_plans_it() {
    let (_dir, providers) = custom(
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
        "probe",
    );
    let provider = &providers[0];
    let names: Vec<String> = provider
        .operations()
        .into_iter()
        .map(|op| op.name)
        .collect();
    assert_eq!(
        names,
        [
            "build-test.build",
            "build-test.t_-",
            "build-test.t_9",
            "build-test.probe_plain",
        ],
        "each name is brought into the grammar"
    );
    assert_eq!(
        violations(&providers),
        Vec::<String>::new(),
        "and the document validates"
    );
    let unroutable: Vec<&String> = names
        .iter()
        .filter(|name| {
            let segment = name.strip_prefix("build-test.").expect("the family prefix");
            provider.plan(segment, &json!({})).is_none()
        })
        .collect();
    assert_eq!(
        unroutable,
        Vec::<&String>::new(),
        "operations whose published name plans nothing"
    );
}
