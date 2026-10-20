//! The configuration the `build-test` v1alpha1 suite is written against, shared by the
//! conformance run and the mutant run (`contracts/build-test/v1alpha1/semantics.md`, § The
//! suite's configuration): the five built-in providers resolved in each scenario's workspace as
//! a harness resolves them, a write scope that refuses every path under `protected/`, and no time
//! left to share except in the one scenario [`TIME_LEFT`] names.

#![allow(
    dead_code,
    reason = "each test binary that includes this module uses its own part of it"
)]

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use tool_contract::conformance::{Report, Suite, Toolset, run};
use tool_toolchain::Registry;
use tool_toolchain::executor::{Executor, Mutation};

/// The scenarios a caller's run has time left in, and how much, in milliseconds. Every other
/// scenario runs with none, so its processes get the contract's bound.
pub const TIME_LEFT: [(&str, u64); 1] = [("a-run-gets-only-the-time-its-caller-has-left", 1000)];

pub fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/build-test/v1alpha1")
}

pub fn suite() -> Suite {
    Suite::read_dir(contract_dir().join("scenarios")).expect("the build-test suite parses")
}

/// A scratch root under this build's own `target/`, never the system's temporary directory.
pub fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

/// The write scope the suite runs under: every path whose first component is `protected` is
/// refused, with a sentence naming it.
pub fn protected(path: &str) -> Option<String> {
    let first = Path::new(path)
        .components()
        .find(|part| !matches!(part, Component::CurDir));
    (first == Some(Component::Normal("protected".as_ref())))
        .then(|| format!("`{path}` is outside what this run may change"))
}

/// Every program a built-in provider declares that is not an executable file on `PATH`.
pub fn missing_programs() -> Vec<String> {
    let registry = Registry::builtins().expect("the built-ins load");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut missing = Vec::new();
    for (spec, _) in registry.definitions() {
        for program in &spec.sandbox.programs {
            let found = std::env::split_paths(&path).any(|dir| dir.join(program).is_file());
            if !found && !missing.contains(program) {
                missing.push(program.clone());
            }
        }
    }
    missing
}

/// The toolset over one scenario workspace: the built-ins resolved there, the write scope, and
/// the time left [`TIME_LEFT`] gives this scenario, the workspace's own name.
///
/// # Panics
///
/// The built-ins do not resolve in the workspace, which names the program or root that is
/// missing: a scenario whose program this machine lacks fails, and never passes silently.
pub fn make(root: &Path, mutation: Option<Mutation>) -> Box<dyn Toolset> {
    let providers = Registry::builtins()
        .expect("the built-ins load")
        .resolve(root, None)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()));
    let executor = match mutation {
        None => Executor::new(root, providers),
        Some(mutation) => Executor::mutated(root, providers, mutation),
    }
    .expect("the scenario workspace opens")
    .refusing_writes(protected);
    let name = root.file_name().and_then(|name| name.to_str());
    let executor = match TIME_LEFT
        .iter()
        .find(|(scenario, _)| Some(*scenario) == name)
    {
        Some((_, ms)) => executor.within(Duration::from_millis(*ms)),
        None => executor,
    };
    Box::new(executor)
}

pub fn run_with(suite: &Suite, mutation: Option<Mutation>) -> Report {
    let scratch = scratch();
    run(
        suite,
        tool_toolchain::document(),
        scratch.path(),
        |root: &Path| make(root, mutation),
    )
}

pub fn failed(report: &Report) -> Vec<&str> {
    report.failed().map(|s| s.name.as_str()).collect()
}
