//! Each mutant of [`Fs`] (one property switched off) fails at least one scenario of the `fs`
//! v1alpha2 suite, so the suite is evidence for every property it claims to hold; and the scenario
//! that catches each one is the one written to.

use std::path::Path;

use tool_contract::conformance::{Report, Suite, run};
use tool_contract::mutants::each_mutant_fails;
use tool_fs::{Fs, Mutation, contract};

fn suite() -> Suite {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/fs/v1alpha2/scenarios");
    Suite::read_dir(scenarios).expect("the fs suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn run_with(suite: &Suite, mutation: Option<Mutation>) -> Report {
    let scratch = scratch();
    run(suite, contract(), scratch.path(), |root: &Path| {
        let toolset = match mutation {
            None => Fs::new(root),
            Some(mutation) => Fs::mutated(root, mutation),
        };
        Box::new(toolset.expect("the scenario workspace opens"))
    })
}

fn failed(report: &Report) -> Vec<&str> {
    report.failed().map(|s| s.name.as_str()).collect()
}

fn killing(mutation: Mutation, scenario: &str) {
    let report = run_with(&suite(), Some(mutation));
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
fn no_read_cap_is_caught_by_the_file_over_the_byte_ceiling() {
    killing(
        Mutation::NoReadCap,
        "a-file-over-the-byte-ceiling-is-cut-at-the-ceiling",
    );
}

#[test]
fn no_truncated_flag_is_caught_by_the_window_in_the_middle() {
    killing(
        Mutation::NoTruncatedFlag,
        "a-window-in-the-middle-is-truncated-and-carries-the-whole-files-digest",
    );
}

#[test]
fn page_digest_is_caught_by_the_window_in_the_middle() {
    killing(
        Mutation::PageDigest,
        "a-window-in-the-middle-is-truncated-and-carries-the-whole-files-digest",
    );
}

#[test]
fn no_containment_is_caught_by_the_path_that_climbs_out_and_by_the_link_out() {
    killing(
        Mutation::NoContainment,
        "a-path-that-climbs-out-is-refused-with-nothing-read",
    );
    killing(
        Mutation::NoContainment,
        "a-link-to-a-directory-outside-the-workspace-is-not-listed",
    );
}

#[test]
fn no_ceiling_cap_is_caught_by_the_ask_above_the_maximum() {
    killing(
        Mutation::NoCeilingCap,
        "a-byte-ceiling-above-the-maximum-is-capped-at-the-maximum",
    );
}

#[test]
fn ignore_deadline_is_caught_by_the_deadline_of_0_on_every_operation() {
    let report = run_with(&suite(), Some(Mutation::IgnoreDeadline));
    println!("{}:\n{report}", Mutation::IgnoreDeadline.name());
    for scenario in [
        "a-read-with-a-deadline-of-0-stops-before-the-first-line",
        "a-listing-with-a-deadline-of-0-answers-no-entry-and-is-truncated",
        "a-glob-with-a-deadline-of-0-answers-no-path-and-is-truncated",
        "a-stat-with-a-deadline-of-0-answers-the-size-without-the-digest",
    ] {
        assert!(
            failed(&report).contains(&scenario),
            "{}: expected {scenario} to fail, failed: {:?}",
            Mutation::IgnoreDeadline.name(),
            failed(&report)
        );
    }
}
