//! The executor over a runner the caller supplies: a recording runner that starts nothing and
//! answers a run of its own making, so every case shows what reached the runner, what the call
//! answered from it, and that a refused call never reached it. One case runs `Executor::new` on
//! the default runner, `tool_process::Process`, which starts the real programs.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tool_toolchain::executor::{Executor, RunError};
use tool_toolchain::{Registry, ResolvedProvider, RunOutput, RunRequest, Runner};

use common::scratch;

const PROVIDERS: &str = r#"version: 1
toolchains:
  - name: alpha
    description: A provider whose tools start sh and printf.
    sandbox:
      programs: [sh, printf]
      env:
        HOME: "{workspace}"
        PATH: "{workspace}/bin:/usr/bin:/bin"
    tools:
      - name: alpha_check
        description: Print the provider's name.
        role: check
        commands:
          - argv: [printf, alpha]
      - name: alpha_two
        description: Two commands.
        commands:
          - argv: [printf, one]
          - argv: [sh, -c, "printf two"]
      - name: alpha_silent_check
        description: A check whose program prints nothing.
        empty-stdout: true
        commands:
          - argv: [sh, -c, "true"]
      - name: alpha_loud_check
        description: A check whose program prints.
        empty-stdout: true
        commands:
          - argv: [printf, unformatted]
      - name: alpha_each
        description: One process per named path, each appending to it.
        workspace-writes: true
        parameters:
          paths: { kind: workspace-path-list, required: true, min-items: 1, max-items: 8 }
        writes: [paths]
        commands:
          - for-each: paths
            argv: [sh, -c, "printf x >> \"$0\"", { item: true }]
      - name: alpha_read
        description: A workspace path the call only reads.
        parameters:
          file: { kind: workspace-path, required: true }
        commands:
          - argv: [printf, "%s", { arg: file }]
      - name: alpha_word
        description: A string as a positional argument.
        parameters:
          word: { kind: string, required: true }
        commands:
          - argv: [printf, "%s", { arg: word }]
  - name: beta
    description: A second provider, so that a role spans two.
    sandbox:
      programs: [printf]
      env:
        LANG: C.UTF-8
    tools:
      - name: beta_check
        description: Print the provider's name.
        role: check
        commands:
          - argv: [printf, beta]
"#;

/// What one call of the recording runner received.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Received {
    argv: Vec<String>,
    provider: String,
    programs: Vec<String>,
    env: Vec<(String, String)>,
    time_left: Option<Duration>,
}

/// The stdout the recording runner answers for one argv.
type Stdout = fn(&[String]) -> String;

/// Starts nothing: records each request, waits `takes`, and answers a run whose fields no real
/// process would produce, so the call's answer can be told apart from a process's.
struct Recording {
    root: PathBuf,
    received: Arc<Mutex<Vec<Received>>>,
    stdout: Stdout,
    takes: Duration,
    refuses: bool,
}

impl Recording {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            received: Arc::default(),
            stdout: |argv| format!("recorded {}", argv.join(" ")),
            takes: Duration::ZERO,
            refuses: false,
        }
    }

    fn answering(self, stdout: Stdout) -> Self {
        Self { stdout, ..self }
    }

    fn taking(self, takes: Duration) -> Self {
        Self { takes, ..self }
    }

    fn refusing(self) -> Self {
        Self {
            refuses: true,
            ..self
        }
    }

    fn output(&self, argv: &[String]) -> RunOutput {
        RunOutput {
            argv: argv.to_vec(),
            exit: Some(7),
            stdout: (self.stdout)(argv),
            stderr: "from the recording runner".to_owned(),
            truncated: false,
            omitted_bytes: 0,
            timed_out: false,
            timeout_ms: 4242,
        }
    }
}

impl Runner for Recording {
    type Error = String;

    fn root(&self) -> &Path {
        &self.root
    }

    fn run(&self, request: RunRequest<'_>) -> Result<RunOutput, String> {
        self.received.lock().expect("the record").push(Received {
            argv: request.argv.to_vec(),
            provider: request.provider.to_owned(),
            programs: request.sandbox.programs.clone(),
            env: request
                .sandbox
                .env
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            time_left: request.time_left,
        });
        std::thread::sleep(self.takes);
        if self.refuses {
            return Err(format!(
                "the recording runner refuses `{}`",
                request.argv[0]
            ));
        }
        Ok(self.output(request.argv))
    }
}

/// A scratch workspace and the named providers of [`PROVIDERS`] resolved in it.
fn providers(names: &[&str]) -> (tempfile::TempDir, PathBuf, Vec<ResolvedProvider>) {
    let root = scratch();
    let spec = root.path().join("runner.yaml");
    std::fs::write(&spec, PROVIDERS).expect("spec");
    let workspace = root.path().join("ws");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let mut registry = Registry::default();
    registry.load_file(&spec).expect("the registry accepts it");
    let selected: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
    let resolved = registry
        .resolve(&workspace, Some(&selected))
        .expect("sh and printf are on PATH");
    (root, workspace, resolved)
}

/// The executor over a recording runner rooted in the workspace, and the runner's record.
fn recorded(
    providers: Vec<ResolvedProvider>,
    runner: Recording,
) -> (Executor<Recording>, Arc<Mutex<Vec<Received>>>) {
    let record = Arc::clone(&runner.received);
    let executor = Executor::with_runner(runner, providers).expect("the workspace opens");
    (executor, record)
}

fn calls(record: &Arc<Mutex<Vec<Received>>>) -> Vec<Received> {
    record.lock().expect("the record").clone()
}

fn refused<R: Runner>(executor: &Executor<R>, name: &str, arguments: &Value) -> String {
    match executor.run(name, arguments, None) {
        Some(Err(RunError::Refused(message))) => message,
        other => panic!("{name}: expected a refusal, got {other:?}"),
    }
}

fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("a symlink");
}

#[test]
fn every_argv_of_a_plan_reaches_the_runner_with_its_provider_sandbox_and_a_shrinking_time_left() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let runner = Recording::new(&workspace).taking(Duration::from_millis(30));
    let (executor, record) = recorded(active, runner);
    let answer = executor
        .run("alpha_two", &json!({}), Some(Duration::from_secs(5)))
        .expect("alpha_two plans")
        .expect("the runner answers");
    let received = calls(&record);
    let argvs: Vec<&[String]> = received.iter().map(|call| call.argv.as_slice()).collect();
    assert_eq!(
        argvs,
        [&["printf", "one"][..], &["sh", "-c", "printf two"][..]],
        "{received:#?}"
    );
    for call in &received {
        assert_eq!(call.provider, "alpha");
        assert_eq!(call.programs, ["sh", "printf"], "the declared order");
        assert_eq!(
            call.env,
            [
                ("HOME".to_owned(), "{workspace}".to_owned()),
                (
                    "PATH".to_owned(),
                    "{workspace}/bin:/usr/bin:/bin".to_owned()
                ),
            ],
            "the declared environment, placeholders unexpanded"
        );
    }
    let first = received[0]
        .time_left
        .expect("the first argv is told the time left");
    let second = received[1]
        .time_left
        .expect("the second argv is told the time left");
    assert!(first <= Duration::from_secs(5), "{first:?}");
    assert!(
        second + Duration::from_millis(30) <= first,
        "the second argv gets what the first left: {first:?} then {second:?}"
    );
    assert_eq!(
        answer["runs"],
        json!([
            {
                "argv": ["printf", "one"],
                "exit": 7,
                "stdout": "recorded printf one",
                "stderr": "from the recording runner",
                "truncated": false,
                "omitted_bytes": 0,
                "timed_out": false,
                "timeout_ms": 4242
            },
            {
                "argv": ["sh", "-c", "printf two"],
                "exit": 7,
                "stdout": "recorded sh -c printf two",
                "stderr": "from the recording runner",
                "truncated": false,
                "omitted_bytes": 0,
                "timed_out": false,
                "timeout_ms": 4242
            }
        ]),
        "the call answers the runner's output: {answer}"
    );
}

#[test]
fn a_call_with_no_time_left_passes_none_and_one_process_answers_the_runners_run() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let (executor, record) = recorded(active, Recording::new(&workspace));
    let answer = executor
        .run("build-test.alpha_check", &json!({}), None)
        .expect("alpha_check plans")
        .expect("the runner answers");
    assert_eq!(answer["stdout"], json!("recorded printf alpha"), "{answer}");
    assert_eq!(answer["timeout_ms"], json!(4242), "{answer}");
    let received = calls(&record);
    assert_eq!(received.len(), 1, "{received:#?}");
    assert_eq!(received[0].time_left, None);
}

#[test]
fn a_role_across_two_providers_hands_each_argv_its_own_providers_name_and_sandbox() {
    let (_root, workspace, active) = providers(&["alpha", "beta"]);
    let (executor, record) = recorded(active, Recording::new(&workspace));
    let answer = executor
        .run("toolchain_check", &json!({"alpha": {}, "beta": {}}), None)
        .expect("a role plans")
        .expect("the runner answers");
    assert_eq!(
        answer["toolchains"]["alpha"]["stdout"],
        json!("recorded printf alpha")
    );
    assert_eq!(
        answer["toolchains"]["beta"]["stdout"],
        json!("recorded printf beta")
    );
    let received = calls(&record);
    assert_eq!(received.len(), 2, "{received:#?}");
    assert_eq!(received[0].provider, "alpha");
    assert_eq!(received[0].programs, ["sh", "printf"]);
    assert_eq!(received[1].provider, "beta");
    assert_eq!(received[1].programs, ["printf"]);
    assert_eq!(
        received[1].env,
        [("LANG".to_owned(), "C.UTF-8".to_owned())],
        "beta's own environment"
    );
}

#[test]
fn the_empty_stdout_check_judges_the_runners_stdout_and_not_the_programs() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let printing = Recording::new(&workspace).answering(|_| "would change src/a.rs\n".to_owned());
    let (executor, record) = recorded(active.clone(), printing);
    let error = executor
        .run("alpha_silent_check", &json!({}), None)
        .expect("it plans")
        .expect_err("the runner's stdout is not empty");
    assert_eq!(
        error,
        RunError::CheckFailed {
            stdout: "would change src/a.rs\n".to_owned()
        }
    );
    assert_eq!(calls(&record).len(), 1);

    let quiet = Recording::new(&workspace).answering(|_| " \n".to_owned());
    let (executor, _record) = recorded(active, quiet);
    let answer = executor
        .run("alpha_loud_check", &json!({}), None)
        .expect("it plans")
        .expect("the runner's stdout is whitespace, whatever the program would print");
    assert_eq!(answer["stdout"], json!(" \n"));
}

#[test]
fn a_program_outside_the_declared_set_is_refused_before_the_runner_is_called() {
    let (_root, workspace, mut active) = providers(&["alpha"]);
    active[0].programs.retain(|program| program != "sh");
    let (executor, record) = recorded(active, Recording::new(&workspace));
    assert_eq!(executor.programs(), ["printf"]);
    let message = refused(&executor, "alpha_two", &json!({}));
    assert_eq!(
        message,
        "`sh` is not a program this run may start; the declared programs are: printf. Nothing \
         was started."
    );
    assert_eq!(
        calls(&record),
        [],
        "not even the first argv, whose program is declared, reached the runner"
    );
}

#[test]
fn an_argument_spelled_as_an_option_is_refused_before_the_runner_is_called() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let (executor, record) = recorded(active, Recording::new(&workspace));
    assert_eq!(
        refused(&executor, "alpha_word", &json!({"word": "-n"})),
        "`word` is `-n`, which begins with `-` and would reach `alpha_word`'s program as an option"
    );
    assert_eq!(calls(&record), []);
}

#[test]
fn a_read_path_that_lands_outside_the_runners_root_is_refused_before_the_runner_is_called() {
    let outer = scratch();
    let (_root, workspace, active) = providers(&["alpha"]);
    link(outer.path(), &workspace.join("out"));
    let (executor, record) = recorded(active, Recording::new(&workspace));
    assert_eq!(
        executor.root(),
        workspace.canonicalize().expect("canonical")
    );
    assert_eq!(
        refused(&executor, "alpha_read", &json!({"file": "out/a.go"})),
        "`out/a.go` lands outside the workspace; nothing was started"
    );
    assert_eq!(calls(&record), []);
}

#[test]
fn a_written_path_that_lands_outside_the_runners_root_is_refused_before_the_runner_is_called() {
    let outer = scratch();
    let (_root, workspace, active) = providers(&["alpha"]);
    link(outer.path(), &workspace.join("out"));
    let (executor, record) = recorded(active, Recording::new(&workspace));
    assert_eq!(
        refused(
            &executor,
            "alpha_each",
            &json!({"paths": ["ok.txt", "out/x.txt"]})
        ),
        "`out/x.txt` lands outside the workspace; nothing was started"
    );
    assert_eq!(calls(&record), []);
}

#[test]
fn a_written_path_the_scope_refuses_is_refused_before_the_runner_is_called() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let runner = Recording::new(&workspace);
    let record = Arc::clone(&runner.received);
    let executor = Executor::with_runner(runner, active)
        .expect("the workspace opens")
        .refusing_writes(|path| {
            path.starts_with("protected/")
                .then(|| format!("`{path}` is outside what this run may change"))
        });
    assert_eq!(
        refused(
            &executor,
            "alpha_each",
            &json!({"paths": ["ok.txt", "protected/x.txt"]})
        ),
        "`protected/x.txt` is outside what this run may change"
    );
    assert_eq!(calls(&record), []);
}

#[test]
fn no_argv_reaches_the_runner_once_the_calls_deadline_has_passed() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let (executor, record) = recorded(active.clone(), Recording::new(&workspace));
    let message = match executor.run("alpha_two", &json!({}), Some(Duration::ZERO)) {
        Some(Err(RunError::Refused(message))) => message,
        other => panic!("expected a refusal, got {other:?}"),
    };
    assert_eq!(
        message,
        "the call's deadline passed before `alpha_two` started a process; nothing was started"
    );
    assert_eq!(calls(&record), []);

    let slow = Recording::new(&workspace).taking(Duration::from_millis(200));
    let (executor, record) = recorded(active, slow);
    let answer = executor
        .run("alpha_two", &json!({}), Some(Duration::from_millis(50)))
        .expect("it plans")
        .expect("the first argv ran");
    assert_eq!(answer["runs"].as_array().map(Vec::len), Some(1), "{answer}");
    assert_eq!(
        calls(&record).len(),
        1,
        "the second argv never reached the runner"
    );
}

#[test]
fn a_runners_own_refusal_is_the_refused_error_with_its_sentence() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let (executor, record) = recorded(active, Recording::new(&workspace).refusing());
    assert_eq!(
        refused(&executor, "alpha_two", &json!({})),
        "the recording runner refuses `printf`"
    );
    assert_eq!(calls(&record).len(), 1, "no further argv after a refusal");
}

#[test]
fn a_runner_whose_root_does_not_open_is_refused_at_construction() {
    let (root, _workspace, active) = providers(&["alpha"]);
    let missing = root.path().join("missing");
    assert!(Executor::with_runner(Recording::new(&missing), active).is_err());
}

#[test]
fn executor_new_still_runs_every_argv_on_the_process_runner() {
    let (_root, workspace, active) = providers(&["alpha"]);
    let executor: Executor<tool_process::Process> =
        Executor::new(&workspace, active).expect("the workspace opens");
    let answer = executor
        .run("alpha_two", &json!({}), None)
        .expect("alpha_two plans")
        .expect("printf and sh run");
    assert_eq!(answer["runs"][0]["stdout"], json!("one"), "{answer}");
    assert_eq!(answer["runs"][1]["stdout"], json!("two"), "{answer}");
    assert_eq!(answer["runs"][1]["exit"], json!(0), "{answer}");
}
