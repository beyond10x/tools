//! Adversary cases against the `build-test` executor and the document it claims, pass 1.
//!
//! The two `adversary-*.yaml` scenario files are run here alone, each over the suite's own
//! configuration (`common::make`), so a red one is seen without the rest of the suite. The other
//! cases drive the executor through its public API: a custom provider that starts only `sh`, a
//! Taskfile written into a scratch workspace under this build's `target/`, and a copy of the
//! built-in `go` provider with one parameter kind changed.

mod common;

use std::time::Duration;

use serde_json::{Value, json};
use tool_contract::conformance::{Suite, declared_properties, run};
use tool_contract::{Operation, Placement};
use tool_toolchain::contract::contract;
use tool_toolchain::executor::{Executor, RunError};
use tool_toolchain::{Registry, document};

use common::{contract_dir, make, scratch};

/// Runs one scenario file alone. The runner refuses a suite that does not hold every declared
/// property of every operation, so the contract is cut to the operations the file calls and,
/// in memory only, each scenario is counted as holding its operation's properties. Only the
/// scenarios' own results are asserted.
fn scenario_file(file: &str) {
    let mut suite = Suite::read(contract_dir().join("scenarios").join(file)).expect("it parses");
    let mut contract = document().clone();
    contract.operations.retain(|op| {
        suite
            .scenarios
            .iter()
            .any(|scenario| scenario.call.operation == op.name)
    });
    for scenario in &mut suite.scenarios {
        let op = op(&contract, &scenario.call.operation);
        scenario.holds = declared_properties(&op.profile)
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();
    }
    let root = scratch();
    let report = run(&suite, &contract, root.path(), |workspace| {
        make(workspace, None)
    });
    let results: Vec<String> = report
        .scenarios
        .iter()
        .map(|result| format!("{} {}: {}", result.passed, result.name, result.detail))
        .collect();
    println!("{results:#?}");
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(report.scenarios.len(), suite.scenarios.len());
    assert!(report.scenarios.iter().all(|s| s.passed), "{results:#?}");
}

/// K1. `go_fmt` writes the paths it is given, and the write scope is asked about each path as
/// spelled. `alias.go` is a link to `protected/kept.go`, which the scope refuses; `gofmt -w`
/// writes through the link.
#[test]
fn the_link_scenario_passes_alone() {
    scenario_file("adversary-links.yaml");
}

/// `semantics.md`: "nothing a caller passes becomes a program, a flag that was not declared, or
/// text a shell takes apart". `package` is a plain `string` placed after `go build`, so `-n` (a
/// dry run, harmless) is read by `go` as a flag.
#[test]
fn the_argument_scenario_passes_alone() {
    scenario_file("adversary-arguments.yaml");
}

/// K2. The document says every operation runs `placement: confined`, and the provider's sandbox
/// gives `HOME` the workspace. The executor applies no sandbox, so the program reads the
/// caller's own `HOME`. Either the profile or the run is wrong; the assertion holds them to each
/// other and goes green when either changes.
#[test]
fn an_operation_declared_confined_does_not_run_with_the_callers_home() {
    let root = scratch();
    let workspace = root.path().join("home");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        workspace.join("Taskfile.yml"),
        "version: '3'\n\ntasks:\n  home:\n    cmds:\n      - printf '%s' \"$HOME\"\n",
    )
    .expect("Taskfile");
    let providers = Registry::builtins()
        .expect("built-ins")
        .resolve(&workspace, None)
        .expect("the taskfile provider resolves");
    let executor = Executor::new(&workspace, providers).expect("the workspace opens");
    let answer = executor
        .run("build-test.taskfile_run", &json!({"task": "home"}), None)
        .expect("taskfile_run plans")
        .expect("task runs");
    let seen = answer["stdout"].as_str().expect("stdout").to_owned();
    let callers = std::env::var("HOME").unwrap_or_default();
    let placement = document()
        .operation("build-test.taskfile_run")
        .expect("declared")
        .profile
        .placement;
    assert!(
        !(placement == Placement::Confined && !callers.is_empty() && seen == callers),
        "build-test.taskfile_run declares placement confined, and its program read the caller's \
         own HOME (the provider's sandbox gives it the workspace)"
    );
}

const TWO_COMMANDS: &str = r#"version: 1
toolchains:
  - name: adversary
    description: One provider whose plan starts two sh processes.
    sandbox:
      programs: [sh]
    tools:
      - name: adversary_slow_then_quick
        description: A command that outlives the call, then a quick one.
        commands:
          - argv: [sh, -c, "sleep 2"]
          - argv: [sh, -c, "printf two"]
"#;

/// `executor.rs`: "A deadline is taken when the run starts and each argv gets what is left of
/// it, so the plan as a whole ends by it." Once the first argv used the whole of it, what is left
/// is zero, and the second argv is still started, with a bound of 0 ms, and killed. A process
/// started after the call's deadline is one the caller's time did not cover.
#[test]
fn no_argv_starts_after_the_calls_deadline_has_passed() {
    let root = scratch();
    let spec = root.path().join("two.yaml");
    std::fs::write(&spec, TWO_COMMANDS).expect("spec");
    let workspace = root.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let mut registry = Registry::default();
    registry.load_file(&spec).expect("the registry accepts it");
    let providers = registry
        .resolve(&workspace, Some(&["adversary".to_owned()]))
        .expect("sh is on PATH");
    let executor = Executor::new(&workspace, providers).expect("the workspace opens");
    let answer = executor.run(
        "adversary_slow_then_quick",
        &json!({}),
        Some(Duration::from_millis(300)),
    );
    match answer.expect("it plans") {
        Err(RunError::Refused(_)) => {}
        Err(other) => panic!("unexpected {other:?}"),
        Ok(answer) => {
            let runs = answer["runs"].as_array().expect("a list of runs");
            let started_late: Vec<&Value> = runs
                .iter()
                .filter(|run| run["timeout_ms"] == json!(0))
                .collect();
            assert!(
                started_late.is_empty(),
                "an argv was started after the call's deadline had passed: {answer}"
            );
        }
    }
}

/// `go_fmt_check` is declared `effects: [process]`: it lists files and writes none, so it names
/// no subject and the write scope is never asked about its paths. A `paths` item is checked only
/// as a relative spelling, and `-w` is one, so it reaches `gofmt -l` as gofmt's own write flag.
#[test]
fn a_check_declared_without_write_leaves_the_named_file_as_it_was() {
    let root = scratch();
    let workspace = root.path().join("check");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        workspace.join("go.mod"),
        "module example.com/x\n\ngo 1.22\n",
    )
    .expect("go.mod");
    let unformatted = "package main\n\nfunc main() {\nprintln(\"x\")\n}\n";
    std::fs::write(workspace.join("main.go"), unformatted).expect("main.go");
    let providers = Registry::builtins()
        .expect("built-ins")
        .resolve(&workspace, None)
        .expect("the go provider resolves");
    let executor = Executor::new(&workspace, providers)
        .expect("the workspace opens")
        .refusing_writes(|path| Some(format!("`{path}` may not be written by this run")));
    let declared = &op(document(), "build-test.go_fmt_check").profile.effects;
    let answer = executor
        .run(
            "build-test.go_fmt_check",
            &json!({"paths": ["-w", "main.go"]}),
            None,
        )
        .expect("go_fmt_check plans");
    let after = std::fs::read_to_string(workspace.join("main.go")).expect("main.go");
    assert_eq!(
        after, unformatted,
        "go_fmt_check (effects {declared:?}) rewrote main.go under a scope that refuses every \
         write; it answered {answer:?}"
    );
}

fn op<'a>(of: &'a tool_contract::Contract, name: &str) -> &'a Operation {
    of.operation(name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

/// `semantics.md`: "an implementation crate holds the two equal, operation for operation, so that
/// a provider changed without a new contract version fails its tests", and a path parameter that
/// climbs out of the workspace is refused. A `go` provider whose `go_fmt_check.paths` is a
/// `string-list` instead of a `workspace-path-list` no longer refuses `../outside.go`, and derives
/// an operation equal to the frozen one, so the drift test cannot see the guarantee go.
#[test]
fn a_provider_that_stops_containing_a_path_derives_a_different_operation() {
    let builtin = include_str!("../builtins/go.yaml");
    let changed = builtin
        .replacen(
            "paths: { kind: workspace-path-list,",
            "paths: { kind: string-list,",
            1,
        )
        .replacen(
            "parameters: *path_parameters",
            "parameters:\n          paths: { kind: workspace-path-list, required: true, min-items: 1, max-items: 128 }",
            1,
        );
    assert_ne!(builtin, changed, "the copy differs from the built-in");
    let root = scratch();
    let spec = root.path().join("go.yaml");
    std::fs::write(&spec, &changed).expect("spec");
    let workspace = root.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        workspace.join("go.mod"),
        "module example.com/x\n\ngo 1.22\n",
    )
    .expect("go.mod");
    let mut registry = Registry::default();
    registry
        .load_file(&spec)
        .expect("the changed provider loads");
    let providers = registry
        .resolve(&workspace, Some(&["go".to_owned()]))
        .expect("go resolves");
    let check = providers[0]
        .tools
        .iter()
        .find(|tool| tool.name == "go_fmt_check")
        .expect("go_fmt_check");
    assert!(
        check.plan(&json!({"paths": ["../outside.go"]})).is_ok(),
        "the changed provider no longer refuses a path that leaves the workspace"
    );
    let derived = contract(&providers);
    assert_ne!(
        op(&derived, "build-test.go_fmt_check"),
        op(document(), "build-test.go_fmt_check"),
        "dropping path containment from go_fmt_check leaves its operation equal to the frozen \
         one, so the drift test passes"
    );
}

/// Pass 2. `contract.yaml` describes `go_fmt_check.paths` as "paths relative to the workspace
/// root, each refused when it leaves the workspace". `up` is a link to the workspace's parent,
/// so `up/outside.go` leaves it while its spelling stays inside. The assertion holds the call to
/// the description and goes green when either the call or the description changes.
#[test]
fn a_check_path_that_leaves_through_a_link_is_refused_as_its_field_says() {
    let root = scratch();
    let outer = root.path().join("outer");
    let workspace = outer.join("ws");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        outer.join("outside.go"),
        "package outside\n\nfunc f() {\nreturn\n}\n",
    )
    .expect("outside.go");
    std::fs::write(
        workspace.join("go.mod"),
        "module example.com/x\n\ngo 1.22\n",
    )
    .expect("go.mod");
    std::os::unix::fs::symlink("..", workspace.join("up")).expect("link");
    let providers = Registry::builtins()
        .expect("built-ins")
        .resolve(&workspace, None)
        .expect("the go provider resolves");
    let executor = Executor::new(&workspace, providers).expect("the workspace opens");
    let field = op(document(), "build-test.go_fmt_check")
        .input
        .iter()
        .find(|field| field.name == "paths")
        .and_then(|field| field.description.clone())
        .unwrap_or_default();
    let answer = executor
        .run(
            "build-test.go_fmt_check",
            &json!({"paths": ["up/outside.go"]}),
            None,
        )
        .expect("go_fmt_check plans");
    if field.contains("refused when it leaves the workspace") {
        assert!(
            matches!(answer, Err(RunError::Refused(_))),
            "go_fmt_check.paths is described as {field:?}, and `up/outside.go`, which leaves \
             the workspace through a link, answered {answer:?}"
        );
    }
}

const REST: &str = r#"version: 1
toolchains:
  - name: adversary
    description: One provider whose tools pass a list after the program, with and without a separator.
    sandbox:
      programs: [sh]
    tools:
      - name: adversary_rest_bare
        description: A list placed with no separator.
        parameters:
          args: { kind: string-list, max-items: 4 }
        commands:
          - argv: [sh, -c, "true", { rest: { arg: args } }]
      - name: adversary_rest_separated
        description: A list placed after a declared separator.
        parameters:
          args: { kind: string-list, max-items: 4 }
        commands:
          - argv: [sh, -c, "true", { rest: { arg: args, separator: "--" } }]
      - name: adversary_slow_then_quick
        description: A command that outlives the call, then a quick one.
        commands:
          - argv: [sh, -c, "sleep 2"]
          - argv: [sh, -c, "printf two"]
"#;

/// Pass 2, boundaries of the corrections that held: an item that is exactly `-` or `--` is
/// refused; the `rest` exemption needs a declared separator; a call with no time left starts
/// nothing.
#[test]
fn dash_items_the_separator_exemption_and_a_zero_deadline_hold() {
    let root = scratch();
    let workspace = root.path().join("ws");
    std::fs::create_dir_all(&workspace).expect("workspace");
    std::fs::write(
        workspace.join("go.mod"),
        "module example.com/x\n\ngo 1.22\n",
    )
    .expect("go.mod");
    let go = Registry::builtins()
        .expect("built-ins")
        .resolve(&workspace, None)
        .expect("go resolves");
    let check = go[0]
        .tools
        .iter()
        .find(|tool| tool.name == "go_fmt_check")
        .expect("go_fmt_check");
    for paths in [json!(["-"]), json!(["--", "main.go"])] {
        assert!(
            check.plan(&json!({ "paths": paths })).is_err(),
            "{paths} is planned"
        );
    }
    let spec = root.path().join("rest.yaml");
    std::fs::write(&spec, REST).expect("spec");
    let mut registry = Registry::default();
    registry.load_file(&spec).expect("the registry accepts it");
    let custom = registry
        .resolve(&workspace, Some(&["adversary".to_owned()]))
        .expect("sh is on PATH");
    let tool = |name: &str| {
        custom[0]
            .tools
            .iter()
            .find(|tool| tool.name == name)
            .expect("declared")
    };
    assert!(
        tool("adversary_rest_bare")
            .plan(&json!({"args": ["-x"]}))
            .is_err(),
        "a dash item with no separator before it is planned"
    );
    assert!(
        tool("adversary_rest_separated")
            .plan(&json!({"args": ["-x"]}))
            .is_ok(),
        "a dash item after the separator is refused"
    );
    let executor = Executor::new(&workspace, custom).expect("the workspace opens");
    let answer = executor
        .run(
            "adversary_slow_then_quick",
            &json!({}),
            Some(Duration::ZERO),
        )
        .expect("it plans");
    assert!(
        matches!(answer, Err(RunError::Refused(_))),
        "no time left answered {answer:?}"
    );
}
