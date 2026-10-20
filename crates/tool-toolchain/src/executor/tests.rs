//! The executor over two custom providers whose tools start only `sh` and `printf`, so every
//! case runs on any unix machine. Ported from harness 0.14.1 by name where a harness test exists
//! (`generic_roles_require_namespaced_provider_arguments`); the rest pin what
//! `invoke_toolchain` does and where this crate differs from it.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{CHECK_FAILED, Executor, Mutation, REFUSED, RunError, entry_names, plan, programs};
use crate::{Registry, ResolvedProvider};

const PROVIDERS: &str = r#"version: 1
toolchains:
  - name: alpha
    description: A provider whose tools start sh and printf.
    sandbox:
      programs: [sh, printf]
    tools:
      - name: alpha_check
        description: Print the provider's name.
        role: check
        commands:
          - argv: [printf, alpha]
      - name: alpha_quiet
        description: A check whose standard output is whitespace.
        role: fmt-check
        empty-stdout: true
        commands:
          - argv: [sh, -c, "printf ' \n'"]
      - name: alpha_loud
        description: A check that names a file it would change.
        empty-stdout: true
        commands:
          - argv: [printf, "src/a.rs\n"]
      - name: alpha_loud_then_marker
        description: A check that prints, then a command that leaves a marker.
        empty-stdout: true
        workspace-writes: true
        commands:
          - argv: [printf, unformatted]
          - argv: [sh, -c, "printf x > marker.txt"]
      - name: alpha_each
        description: One process per named path, each appending to it.
        workspace-writes: true
        parameters:
          paths: { kind: workspace-path-list, required: true, min-items: 1, max-items: 8 }
        writes: [paths]
        commands:
          - for-each: paths
            argv: [sh, -c, "printf x >> \"$0\"", { item: true }]
      - name: alpha_two
        description: Two commands that each take most of a second.
        commands:
          - argv: [sh, -c, "sleep 0.6; printf one"]
          - argv: [sh, -c, "sleep 0.6; printf two"]
  - name: beta
    description: A second provider, so that a role spans two.
    sandbox:
      programs: [printf]
    tools:
      - name: beta_check
        description: Print the provider's name.
        role: check
        commands:
          - argv: [printf, beta]
  - name: gamma
    description: Tools that place one argument in each argv position.
    sandbox:
      programs: [printf]
    tools:
      - name: gamma_word
        description: A string as a positional argument.
        parameters:
          word: { kind: string, required: true }
        commands:
          - argv: [printf, "%s", { arg: word }]
      - name: gamma_option
        description: A string as an option's value.
        parameters:
          word: { kind: string, required: true }
        commands:
          - argv: [printf, "%s %s", { option: { name: --opt, arg: word } }]
      - name: gamma_file
        description: A workspace path as a positional argument.
        parameters:
          file: { kind: workspace-path, required: true }
        commands:
          - argv: [printf, "%s", { arg: file }]
      - name: gamma_files
        description: Workspace paths spread into the argv.
        parameters:
          files: { kind: workspace-path-list, required: true, min-items: 1, max-items: 4 }
        commands:
          - argv: [printf, "%s ", { each: files }]
      - name: gamma_loose
        description: A string list spread into the argv with no separator before it.
        parameters:
          words: { kind: string-list, required: true, max-items: 4 }
        commands:
          - argv: [printf, "%s ", { each: words }]
      - name: gamma_rest
        description: A string list after a declared separator.
        parameters:
          words: { kind: string-list, max-items: 4 }
        commands:
          - argv: [printf, "%s ", { rest: { arg: words, separator: "--" } }]
"#;

/// The named providers of [`PROVIDERS`], resolved against an empty workspace.
fn providers(names: &[&str]) -> (tempfile::TempDir, Vec<ResolvedProvider>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut registry = Registry::default();
    registry
        .add_document(PROVIDERS, "executor-tests.yaml")
        .expect("a document the registry accepts");
    let selected: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    let resolved = registry
        .resolve(dir.path(), Some(&selected))
        .expect("sh and printf are on PATH and the providers have no roots");
    (dir, resolved)
}

fn executor(names: &[&str]) -> (tempfile::TempDir, Executor) {
    let (dir, active) = providers(names);
    let executor = Executor::new(dir.path(), active).expect("the workspace opens");
    (dir, executor)
}

fn run(executor: &Executor, name: &str, arguments: &Value) -> Result<Value, RunError> {
    executor
        .run(name, arguments, None)
        .unwrap_or_else(|| panic!("{name} is a name the executor plans"))
}

fn refused(executor: &Executor, name: &str, arguments: &Value) -> String {
    match run(executor, name, arguments) {
        Err(RunError::Refused(message)) => message,
        other => panic!("{name}: expected a refusal, got {other:?}"),
    }
}

fn protected(path: &str) -> Option<String> {
    path.starts_with("protected/")
        .then(|| format!("`{path}` is outside what this run may change"))
}

#[test]
fn the_declared_program_set_is_every_active_providers_programs_sorted_once() {
    let (_dir, active) = providers(&["alpha", "beta"]);
    assert_eq!(programs(&active), ["printf", "sh"]);
    let (_dir, active) = providers(&["beta"]);
    assert_eq!(programs(&active), ["printf"]);
    assert_eq!(programs(&[]), Vec::<String>::new());
    let (_dir, executor) = executor(&["alpha", "beta"]);
    assert_eq!(executor.programs(), ["printf", "sh"]);
}

#[test]
fn entry_names_add_a_role_only_when_every_active_provider_implements_it() {
    let (_dir, active) = providers(&["alpha", "beta"]);
    let names = entry_names(&active);
    assert!(names.contains(&"toolchain_check".to_owned()), "{names:?}");
    assert!(
        !names.contains(&"toolchain_fmt_check".to_owned()),
        "beta has no fmt-check tool: {names:?}"
    );
    assert_eq!(names.first().map(String::as_str), Some("alpha_check"));
    assert!(names.contains(&"beta_check".to_owned()), "{names:?}");
    assert_eq!(entry_names(&[]), Vec::<String>::new());
}

#[test]
fn generic_roles_require_namespaced_provider_arguments() {
    let (_dir, active) = providers(&["alpha", "beta"]);
    let planned = plan(
        "toolchain_check",
        &json!({"alpha": {}, "beta": {}}),
        &active,
    )
    .expect("a role name plans")
    .expect("both providers' arguments are given");
    let providers: Vec<&str> = planned
        .groups
        .iter()
        .map(|group| group.provider.as_str())
        .collect();
    assert_eq!(providers, ["alpha", "beta"]);
    let missing = plan("toolchain_check", &json!({"alpha": {}}), &active)
        .expect("a role name plans")
        .expect_err("beta's arguments are missing");
    assert_eq!(
        missing,
        "`beta` arguments are required by `toolchain_check`"
    );
    let incomplete = plan("toolchain_fmt_check", &json!({"alpha": {}}), &active)
        .expect("a role name plans")
        .expect_err("beta has no fmt-check tool");
    assert_eq!(
        incomplete,
        "provider `beta` has no `fmt_check` role implementation"
    );
    assert!(plan("toolchain_lint", &json!({}), &active).is_none());
    assert!(plan("gamma_check", &json!({}), &active).is_none());
}

#[test]
fn one_process_answers_one_run_object() {
    let (_dir, executor) = executor(&["alpha"]);
    let answer = run(&executor, "alpha_check", &json!({})).expect("printf runs");
    assert_eq!(answer["argv"], json!(["printf", "alpha"]));
    assert_eq!(answer["exit"], json!(0));
    assert_eq!(answer["stdout"], json!("alpha"));
    assert_eq!(answer["timed_out"], json!(false));
    assert_eq!(answer["timeout_ms"], json!(600_000));
    let published = run(&executor, "build-test.alpha_check", &json!({})).expect("printf runs");
    assert_eq!(published, answer, "the published name runs the same tool");
}

#[test]
fn a_tool_whose_plan_can_start_several_answers_a_list_of_runs_even_for_one() {
    let (dir, executor) = executor(&["alpha"]);
    let answer = run(&executor, "alpha_each", &json!({"paths": ["a.txt"]})).expect("sh runs");
    let runs = answer["runs"].as_array().expect("a list of runs");
    assert_eq!(runs.len(), 1, "{answer}");
    assert_eq!(runs[0]["exit"], json!(0));
    let two = run(
        &executor,
        "alpha_each",
        &json!({"paths": ["a.txt", "b.txt"]}),
    )
    .expect("runs");
    assert_eq!(two["runs"].as_array().map(Vec::len), Some(2), "{two}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).expect("a.txt"),
        "xx"
    );
}

#[test]
fn a_role_across_two_providers_answers_one_answer_per_provider_under_toolchains() {
    let (_dir, executor) = executor(&["alpha", "beta"]);
    let answer = run(
        &executor,
        "toolchain_check",
        &json!({"alpha": {}, "beta": {}}),
    )
    .expect("both run");
    let toolchains = answer["toolchains"].as_object().expect("a map by provider");
    assert_eq!(toolchains.len(), 2, "{answer}");
    assert_eq!(answer["toolchains"]["alpha"]["stdout"], json!("alpha"));
    assert_eq!(answer["toolchains"]["beta"]["stdout"], json!("beta"));
}

#[test]
fn a_role_with_one_active_provider_answers_that_providers_answer_alone() {
    let (_dir, executor) = executor(&["beta"]);
    let answer = run(&executor, "toolchain_check", &json!({"beta": {}})).expect("beta runs");
    assert_eq!(answer["stdout"], json!("beta"), "{answer}");
}

#[test]
fn a_role_over_no_active_provider_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let executor = Executor::new(dir.path(), Vec::new()).expect("the workspace opens");
    let message = refused(&executor, "toolchain_check", &json!({}));
    assert!(message.contains("toolchain_check"), "{message}");
}

#[test]
fn an_empty_stdout_check_that_prints_is_refused_naming_the_formatting_check() {
    let (_dir, executor) = executor(&["alpha"]);
    let error = run(&executor, "alpha_loud", &json!({})).expect_err("the check printed");
    assert_eq!(
        error,
        RunError::CheckFailed {
            stdout: "src/a.rs\n".to_owned()
        }
    );
    assert_eq!(error.name(), CHECK_FAILED);
    assert_eq!(
        error.to_string(),
        "formatting check failed: \"src/a.rs\\n\"",
        "harness's sentence, the output as a JSON string"
    );
    assert_eq!(error.fields(), json!({"stdout": "src/a.rs\n"}));
}

#[test]
fn a_check_that_prints_stops_the_plan_before_its_next_command() {
    let (dir, executor) = executor(&["alpha"]);
    let error = run(&executor, "alpha_loud_then_marker", &json!({})).expect_err("printed");
    assert_eq!(error.name(), CHECK_FAILED);
    assert!(
        !dir.path().join("marker.txt").exists(),
        "the second command never started"
    );
}

#[test]
fn whitespace_on_standard_output_passes_the_check() {
    let (_dir, executor) = executor(&["alpha"]);
    let answer = run(&executor, "alpha_quiet", &json!({})).expect("whitespace only");
    assert_eq!(answer["stdout"], json!(" \n"));
}

#[test]
fn a_written_path_the_scope_refuses_starts_nothing_and_names_the_path() {
    let (dir, executor) = executor(&["alpha"]);
    let executor = executor.refusing_writes(protected);
    let message = refused(
        &executor,
        "alpha_each",
        &json!({"paths": ["ok.txt", "protected/x.txt"]}),
    );
    assert_eq!(
        message,
        "`protected/x.txt` is outside what this run may change"
    );
    assert!(
        !dir.path().join("ok.txt").exists(),
        "the first path's process never started"
    );
    run(&executor, "alpha_each", &json!({"paths": ["ok.txt"]})).expect("an allowed path runs");
    assert!(dir.path().join("ok.txt").exists());
}

#[test]
fn the_time_left_is_shared_by_every_process_of_one_plan() {
    let (_dir, executor) = executor(&["alpha"]);
    let started = Instant::now();
    let answer = executor
        .run("alpha_two", &json!({}), Some(Duration::from_secs(1)))
        .expect("alpha_two plans")
        .expect("both start");
    let elapsed = started.elapsed();
    let runs = answer["runs"].as_array().expect("a list of runs");
    assert_eq!(runs.len(), 2, "{answer}");
    assert_eq!(runs[0]["stdout"], json!("one"), "{answer}");
    assert_eq!(runs[0]["timed_out"], json!(false), "{answer}");
    assert_eq!(
        runs[1]["timed_out"],
        json!(true),
        "the second process gets what the first left: {answer}"
    );
    let second = runs[1]["timeout_ms"].as_u64().expect("an integer");
    assert!(second < 600, "{answer}");
    assert!(elapsed < Duration::from_millis(1900), "{elapsed:?}");
}

#[test]
fn an_unknown_name_plans_nothing() {
    let (_dir, executor) = executor(&["alpha"]);
    assert!(executor.run("gamma_check", &json!({}), None).is_none());
    assert!(
        executor
            .run("build-test.beta_check", &json!({}), None)
            .is_none()
    );
}

#[test]
fn an_argument_the_registry_refuses_is_refused_with_its_sentence() {
    let (_dir, executor) = executor(&["alpha"]);
    assert_eq!(
        refused(&executor, "alpha_each", &json!({})),
        "`paths` is required by `alpha_each`"
    );
    assert_eq!(
        refused(&executor, "alpha_each", &json!({"paths": ["../out.txt"]})),
        "`../out.txt` is not a contained relative path"
    );
}

#[test]
fn a_refusal_is_the_refused_error_with_its_message_as_the_one_field() {
    let error = RunError::Refused("nothing was started".to_owned());
    assert_eq!(error.name(), REFUSED);
    assert_eq!(error.to_string(), "nothing was started");
    assert_eq!(error.fields(), json!({"message": "nothing was started"}));
}

#[test]
fn a_runner_without_the_declared_set_refuses_what_the_provider_declares() {
    let (dir, active) = providers(&["alpha"]);
    let mutant = Executor::mutated(dir.path(), active, Mutation::NoDeclaredPrograms)
        .expect("the workspace opens");
    let message = refused(&mutant, "alpha_check", &json!({}));
    assert!(
        message.contains("is not a program this run may start"),
        "{message}"
    );
}

#[test]
fn the_workspace_must_open() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("missing");
    assert!(Executor::new(Path::new(&missing), Vec::new()).is_err());
}

fn link(target: &str, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("a symlink");
}

/// F1 (correction round 1): the scope judges the file the bytes reach. `alias.txt` is spelled
/// outside `protected/` and lands inside it.
#[test]
fn a_written_path_is_judged_where_it_lands_and_not_only_as_spelled() {
    let (dir, executor) = executor(&["alpha"]);
    let executor = executor.refusing_writes(protected);
    std::fs::create_dir_all(dir.path().join("protected")).expect("protected/");
    std::fs::write(dir.path().join("protected/kept.txt"), "kept").expect("kept.txt");
    link("protected/kept.txt", &dir.path().join("alias.txt"));
    let message = refused(&executor, "alpha_each", &json!({"paths": ["alias.txt"]}));
    assert_eq!(
        message,
        "`alias.txt` leads to `protected/kept.txt`, and `protected/kept.txt` is outside what \
         this run may change"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("protected/kept.txt")).expect("kept.txt"),
        "kept"
    );
    std::fs::create_dir_all(dir.path().join("src")).expect("src/");
    link("src", &dir.path().join("sources"));
    run(
        &executor,
        "alpha_each",
        &json!({"paths": ["sources/new.txt"]}),
    )
    .expect("a link that lands where the scope allows is written through");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/new.txt")).expect("src/new.txt"),
        "x"
    );
}

/// F1, D6: a written path that lands outside the workspace is refused with no scope at all, and
/// so is one through a link that leads nowhere.
#[test]
fn a_written_path_that_lands_outside_the_workspace_or_nowhere_is_refused() {
    let outer = tempfile::tempdir().expect("tempdir");
    let (dir, active) = providers(&["alpha"]);
    let workspace = dir.path();
    link(
        outer.path().to_str().expect("a UTF-8 temporary directory"),
        &workspace.join("out"),
    );
    link("missing/deeper", &workspace.join("nowhere"));
    let executor = Executor::new(workspace, active).expect("the workspace opens");
    let escaped = refused(&executor, "alpha_each", &json!({"paths": ["out/x.txt"]}));
    assert_eq!(
        escaped,
        "`out/x.txt` lands outside the workspace; nothing was started"
    );
    assert!(
        !outer.path().join("x.txt").exists(),
        "nothing was written there"
    );
    let dangling = refused(&executor, "alpha_each", &json!({"paths": ["nowhere"]}));
    assert!(
        dangling.starts_with("`nowhere`: `nowhere` is a link that leads nowhere"),
        "{dangling}"
    );
    assert!(!workspace.join("missing").exists());
}

/// F2 and F3 (correction round 1), and the class they belong to: a caller's value that begins
/// with `-` reaches the program as an option wherever it lands in an argv, so it is refused for
/// every string-carrying kind, naming the argument, unless every place the parameter reaches is
/// after a declared `rest` separator.
#[test]
fn an_argument_spelled_as_an_option_is_refused_wherever_it_would_land() {
    let (_dir, executor) = executor(&["gamma"]);
    for (tool, arguments, expected) in [
        (
            "gamma_word",
            json!({"word": "-n"}),
            "`word` is `-n`, which begins with `-` and would reach `gamma_word`'s program as an \
             option",
        ),
        (
            "gamma_option",
            json!({"word": "--frozen"}),
            "`word` is `--frozen`, which begins with `-` and would reach `gamma_option`'s \
             program as an option",
        ),
        (
            "gamma_file",
            json!({"file": "-w"}),
            "`file` is `-w`, which begins with `-` and would reach `gamma_file`'s program as an \
             option",
        ),
        (
            "gamma_files",
            json!({"files": ["a.txt", "-w"]}),
            "`files` holds `-w`, which begins with `-` and would reach `gamma_files`'s program as \
             an option",
        ),
        (
            "gamma_loose",
            json!({"words": ["-x"]}),
            "`words` holds `-x`, which begins with `-` and would reach `gamma_loose`'s program as \
             an option",
        ),
    ] {
        assert_eq!(refused(&executor, tool, &arguments), expected, "{tool}");
    }
    let after = run(&executor, "gamma_rest", &json!({"words": ["-x", "--y"]}))
        .expect("after a declared separator a leading `-` is the program's argument");
    assert_eq!(after["argv"], json!(["printf", "%s ", "--", "-x", "--y"]));
    let plain = run(
        &executor,
        "gamma_files",
        &json!({"files": ["a-b.txt", "x/-y"]}),
    )
    .expect("a `-` that does not lead the value is no option");
    assert_eq!(plain["stdout"], json!("a-b.txt x/-y "));
}

/// F5 (correction round 1): a process is never started once the call's deadline has passed. A
/// call with no time left starts nothing and is refused; a plan whose deadline passes partway
/// answers the runs that started.
#[test]
fn no_process_starts_once_the_calls_deadline_has_passed() {
    let (_dir, executor) = executor(&["alpha"]);
    let message = match executor.run("alpha_check", &json!({}), Some(Duration::ZERO)) {
        Some(Err(RunError::Refused(message))) => message,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(
        message,
        "the call's deadline passed before `alpha_check` started a process; nothing was started"
    );
    let answer = executor
        .run("alpha_two", &json!({}), Some(Duration::from_millis(300)))
        .expect("alpha_two plans")
        .expect("the first process started");
    let runs = answer["runs"].as_array().expect("a list of runs");
    assert_eq!(runs.len(), 1, "the second process never started: {answer}");
    assert_eq!(runs[0]["timed_out"], json!(true), "{answer}");
}

/// P1 (correction round 2): a path-kind argument the call only reads is judged where it lands
/// too, before anything starts, as its field says; the write scope is asked about written paths
/// alone.
#[test]
fn a_path_the_call_only_reads_is_judged_where_it_lands_and_not_by_the_scope() {
    let outer = tempfile::tempdir().expect("tempdir");
    let (dir, active) = providers(&["gamma"]);
    let workspace = dir.path();
    link(
        outer.path().to_str().expect("a UTF-8 temporary directory"),
        &workspace.join("out"),
    );
    std::fs::create_dir_all(workspace.join("src")).expect("src/");
    link("src", &workspace.join("inner"));
    link("missing/deeper", &workspace.join("nowhere"));
    let executor = Executor::new(workspace, active)
        .expect("the workspace opens")
        .refusing_writes(|path| Some(format!("`{path}` may not be written by this run")));
    assert_eq!(
        refused(&executor, "gamma_file", &json!({"file": "out/a.go"})),
        "`out/a.go` lands outside the workspace; nothing was started"
    );
    assert_eq!(
        refused(
            &executor,
            "gamma_files",
            &json!({"files": ["src/a.go", "out/b.go"]})
        ),
        "`out/b.go` lands outside the workspace; nothing was started"
    );
    let dangling = refused(&executor, "gamma_file", &json!({"file": "nowhere"}));
    assert!(
        dangling.starts_with("`nowhere`: `nowhere` is a link that leads nowhere"),
        "{dangling}"
    );
    let inside = run(&executor, "gamma_files", &json!({"files": ["inner/a.go"]})).expect(
        "a read path that lands inside runs, and the scope that refuses every write is not asked",
    );
    assert_eq!(inside["stdout"], json!("inner/a.go "));
}
