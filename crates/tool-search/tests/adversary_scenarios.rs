//! The adversary's scenarios for `story:toolset-search` (wave w3), one test each, so that each can
//! be run and read alone. The scenarios themselves are `contracts/search/v1alpha2/scenarios/
//! adversary-held.yaml` and `adversary-red.yaml`, part of the suite the conformance test runs; a
//! filtered suite would be refused by the coverage rule, so each test runs the whole suite and
//! reads the one result it is about.
//!
//! `adversary-held.yaml` holds edges the suite did not: a line of exactly 400 characters and one of
//! 401, in characters of two and four bytes; a file over 1 MiB that the glob does not name; links
//! below a `path` that land elsewhere inside the workspace. `adversary-red.yaml` holds two sentences
//! of the contract and its semantics: the guidance's "the walk skips .git", met by a worktree's
//! `.git` file, and semantics.md's "a binary file among them, is not searched".

use std::path::Path;

use tool_contract::conformance::{Outcome, Suite, Toolset, run};
use tool_search::{Search, contract};

fn suite() -> Suite {
    let scenarios =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha2/scenarios");
    Suite::read_dir(scenarios).expect("the search suite parses")
}

/// Runs the whole suite against the intact toolset and asserts that `name` passed. On a failure
/// the message also carries the answer: the scenario's call made again on the workspace the
/// runner laid out for it, because the runner's detail names only the first difference.
fn passes(name: &str) {
    let suite = suite();
    let scratch = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .expect("a scratch directory under target/");
    let report = run(&suite, contract(), scratch.path(), |root| {
        Box::new(Search::new(root).expect("the scenario workspace opens"))
    });
    assert!(report.refused.is_empty(), "{report}");
    let result = report
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the suite"));
    if result.passed {
        return;
    }
    let call = &suite
        .scenarios
        .iter()
        .find(|scenario| scenario.name == name)
        .expect("the scenario is in the suite")
        .call;
    let answer = match Search::new(scratch.path().join(name))
        .expect("the scenario workspace opens")
        .invoke(&call.operation, &call.input)
    {
        Outcome::Ok(output) => output.to_string(),
        Outcome::Error { name, message, .. } => format!("{name}: {message}"),
    };
    panic!("{name}: {}\nanswered: {answer}", result.detail);
}

#[test]
fn a_line_of_400_characters_is_whole_and_one_of_401_is_cut_on_a_character() {
    passes("adversary-a-line-of-400-characters-is-whole-and-one-of-401-is-cut-on-a-character");
}

#[test]
fn a_file_over_one_mebibyte_the_glob_does_not_name_is_neither_read_nor_counted() {
    passes("adversary-a-file-over-one-mebibyte-the-glob-does-not-name-is-neither-read-nor-counted");
}

#[test]
fn links_below_the_start_that_land_elsewhere_inside_are_followed() {
    passes("adversary-links-below-the-start-that-land-elsewhere-inside-are-followed");
}

#[test]
fn a_git_file_is_skipped_as_the_guidance_says() {
    passes("adversary-a-git-file-is-skipped-as-the-guidance-says");
}

#[test]
fn a_binary_file_that_is_valid_utf_8_is_not_searched() {
    passes("adversary-a-binary-file-that-is-valid-utf-8-is-not-searched");
}
