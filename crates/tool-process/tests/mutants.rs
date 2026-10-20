//! Each mutant of [`Process`] (one property switched off) fails at least one scenario of the
//! `process` v1alpha1 suite, so the suite is evidence for every property it claims to hold; and
//! the scenario that catches each one is the one written to.

use std::path::Path;

use tool_contract::conformance::{Report, Suite, run};
use tool_contract::mutants::each_mutant_fails;
use tool_process::{Mutation, Process, contract};

const DECLARED: [&str; 3] = ["sh", "printf", "b10x-conformance-absent-program"];

fn suite() -> Suite {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/process/v1alpha1/scenarios");
    Suite::read_dir(scenarios).expect("the process suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn run_with(suite: &Suite, mutation: Option<Mutation>) -> Report {
    let scratch = scratch();
    let declared = || DECLARED.iter().map(|p| (*p).to_owned()).collect();
    run(suite, contract(), scratch.path(), |root: &Path| {
        let toolset = match mutation {
            None => Process::new(root, declared()),
            Some(mutation) => Process::mutated(root, declared(), mutation),
        };
        Box::new(toolset.expect("the scenario workspace opens"))
    })
}

fn failed(report: &Report) -> Vec<&str> {
    report.failed().map(|s| s.name.as_str()).collect()
}

/// `scenario` passes against the intact toolset and fails against `mutation`: a scenario that
/// fails either way is no evidence about the property the mutant drops.
fn killing(mutation: Mutation, scenario: &str) {
    let suite = suite();
    let intact = run_with(&suite, None);
    assert!(
        intact.scenarios.iter().any(|s| s.name == scenario),
        "{scenario} is not in the suite"
    );
    assert!(
        !failed(&intact).contains(&scenario),
        "{scenario} fails against the intact toolset too:\n{intact}"
    );
    let report = run_with(&suite, Some(mutation));
    println!("{}:\n{report}", mutation.name());
    assert!(!report.passed(), "{}: {report}", mutation.name());
    assert!(
        failed(&report).contains(&scenario),
        "{}: expected {scenario} to fail, failed: {:?}",
        mutation.name(),
        failed(&report)
    );
}

#[test]
fn each_mutant_fails_at_least_one_scenario() {
    let suite = suite();
    let intact = run_with(&suite, None);
    assert!(
        intact.passed(),
        "the intact toolset must pass before a mutant's failure means anything:\n{intact}"
    );
    let reports: Vec<(&str, Report)> = Mutation::ALL
        .iter()
        .map(|mutation| (mutation.name(), run_with(&suite, Some(*mutation))))
        .collect();
    for (name, report) in &reports {
        let killed: Vec<&str> = failed(report);
        println!(
            "{name}: fails {} of {}: {killed:?}",
            killed.len(),
            report.scenarios.len()
        );
    }
    assert_eq!(each_mutant_fails(&reports), Ok(()));
}

#[test]
fn no_program_check_is_caught_by_the_program_outside_the_declared_set() {
    killing(
        Mutation::NoProgramCheck,
        "a-program-outside-the-declared-set-is-refused-by-name-with-nothing-started",
    );
    killing(
        Mutation::NoProgramCheck,
        "a-declared-program-under-another-spelling-is-refused",
    );
}

#[test]
fn inherit_all_is_caught_by_the_variable_outside_the_allow_list() {
    killing(
        Mutation::InheritAll,
        "the-child-sees-no-variable-outside-the-allow-list",
    );
}

#[test]
fn no_kill_is_caught_by_the_descendant_that_outlives_the_bound() {
    killing(
        Mutation::NoKill,
        "a-run-over-its-bound-is-killed-with-its-descendants-and-says-so",
    );
}

/// A run killed at its bound stops waiting on its streams after a short settle, so a descendant
/// the kill missed no longer shows by holding them open; it shows by outliving its shell, which a
/// kill of the whole group never lets it see.
#[test]
fn no_kill_and_no_group_kill_are_caught_by_the_descendant_that_sees_its_shell_die() {
    for mutation in [Mutation::NoKill, Mutation::NoGroupKill] {
        killing(
            mutation,
            "a-descendant-in-the-group-dies-with-its-shell-at-the-bound",
        );
    }
}

/// The descendant writes after the bound and inside the settle, so only the kill keeps its word
/// out of the reply; the suite's own scenario for a shell that ended has its descendant write
/// after the settle, where a run that never killed it answers the same.
#[test]
fn kill_only_a_live_program_is_caught_by_the_descendant_left_by_a_shell_that_ended() {
    killing(
        Mutation::KillOnlyALiveProgram,
        "adversary-a-descendant-left-by-a-shell-that-ended-is-killed-at-the-bound-not-only-left-behind",
    );
}

#[test]
fn no_output_cap_is_caught_on_both_streams_and_at_the_character_boundary() {
    for scenario in [
        "output-over-the-cap-keeps-both-ends-and-names-what-was-omitted",
        "standard-error-is-capped-the-same-way-and-standard-output-is-not-touched",
        "a-cut-lands-between-characters-and-the-count-includes-the-bytes-it-moved",
    ] {
        killing(Mutation::NoOutputCap, scenario);
    }
}

#[test]
fn no_timeout_cap_is_caught_by_the_bound_above_the_maximum() {
    killing(
        Mutation::NoTimeoutCap,
        "a-bound-above-the-maximum-is-capped-at-the-maximum",
    );
}
