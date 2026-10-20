//! Each mutant of [`Search`] (one property switched off) fails at least one scenario of the
//! `search` v1alpha1 suite, so the suite is evidence for every property it claims to hold; and the
//! scenario that catches each one is the one written to.

use std::path::Path;

use tool_contract::conformance::{Report, Suite, run};
use tool_contract::mutants::each_mutant_fails;
use tool_search::{Mutation, Search, contract};

fn suite() -> Suite {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha1/scenarios");
    Suite::read_dir(scenarios).expect("the search suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn run_with(suite: &Suite, mutation: Option<Mutation>) -> Report {
    let scratch = scratch();
    run(suite, contract(), scratch.path(), |root: &Path| {
        let toolset = match mutation {
            None => Search::new(root),
            Some(mutation) => Search::mutated(root, mutation),
        };
        Box::new(toolset.expect("the scenario workspace opens"))
    })
}

fn failed(report: &Report) -> Vec<&str> {
    report.failed().map(|s| s.name.as_str()).collect()
}

/// Each of `scenarios` passes against the intact toolset and fails against `mutation`: a scenario
/// that fails either way is no evidence about the property the mutant drops.
fn killing(mutation: Mutation, scenarios: &[&str]) {
    let suite = suite();
    let intact = run_with(&suite, None);
    let report = run_with(&suite, Some(mutation));
    println!("{}:\n{report}", mutation.name());
    for scenario in scenarios {
        assert!(
            intact.scenarios.iter().any(|s| s.name == *scenario),
            "{scenario} is not in the suite"
        );
        assert!(
            !failed(&intact).contains(scenario),
            "{scenario} fails against the intact toolset too:\n{intact}"
        );
        assert!(
            failed(&report).contains(scenario),
            "{}: expected {scenario} to fail, failed: {:?}",
            mutation.name(),
            failed(&report)
        );
    }
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
fn no_result_cap_is_caught_by_more_than_200_matches_and_by_the_larger_ask() {
    killing(
        Mutation::NoResultCap,
        &[
            "more-than-200-matches-are-answered-as-200-and-truncated",
            "a-larger-ask-than-200-is-capped-at-200",
        ],
    );
}

#[test]
fn no_truncated_flag_is_caught_by_the_cut_list_and_by_the_large_file() {
    killing(
        Mutation::NoTruncatedFlag,
        &[
            "a-cut-list-says-it-was-cut",
            "a-file-over-one-mebibyte-is-not-searched-and-is-named",
        ],
    );
}

#[test]
fn no_large_file_bound_is_caught_by_the_file_over_one_mebibyte() {
    killing(
        Mutation::NoLargeFileBound,
        &["a-file-over-one-mebibyte-is-not-searched-and-is-named"],
    );
}

#[test]
fn no_containment_is_caught_by_the_path_that_climbs_out_and_by_the_link_out() {
    killing(
        Mutation::NoContainment,
        &[
            "a-path-that-climbs-out-is-refused-with-nothing-read",
            "a-search-started-in-a-link-out-of-the-workspace-is-refused",
            "a-link-out-of-the-workspace-is-not-searched",
        ],
    );
}

#[test]
fn no_match_cut_is_caught_by_the_long_line_and_by_its_context() {
    killing(
        Mutation::NoMatchCut,
        &[
            "a-match-on-a-long-line-is-cut-at-400-characters-and-says-so",
            "a-line-of-context-is-cut-the-same-way",
        ],
    );
}

#[test]
fn no_skipped_file_names_is_caught_by_the_worktrees_git_file() {
    killing(
        Mutation::NoSkippedFileNames,
        &["adversary-a-git-file-is-skipped-as-the-guidance-says"],
    );
}

#[test]
fn nul_is_text_is_caught_by_the_binary_file_that_is_valid_utf_8() {
    killing(
        Mutation::NulIsText,
        &["adversary-a-binary-file-that-is-valid-utf-8-is-not-searched"],
    );
}

#[test]
fn no_context_cap_is_caught_by_the_ask_above_five() {
    killing(
        Mutation::NoContextCap,
        &["a-context-above-five-is-capped-and-the-reply-says-so"],
    );
}
