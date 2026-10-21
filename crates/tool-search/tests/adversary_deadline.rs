//! The adversary's cases for `story:declared-timeouts-are-enforced` (`search` v1alpha2), written
//! against `contracts/search/v1alpha2/semantics.md` § The deadline before anything was run.
//!
//! The scenario cases are `contracts/search/v1alpha2/scenarios/adversary-deadline.yaml`; a test
//! here runs the whole suite and reads the one result it is about.
//!
//! The direct case drives a real clock through the public API: the unit's own cases stop the
//! search with `Deadline::after_checks` or `timeout_ms: 0`, so none of them would notice a
//! deadline that is never reached for any `timeout_ms` above 0.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use tool_contract::conformance::{Suite, run};
use tool_search::{GrepInput, Search, contract};

/// The most a call named `timeout_ms: 1` may take here: far above 1 ms so a loaded machine does
/// not fail it, and below the 1 s a seconds-for-milliseconds slip would give the call.
const NEAR_ITS_BOUND: Duration = Duration::from_millis(500);
/// The least the uncut search must take for a 1 ms deadline to be one it has to reach.
const SLOWER_THAN_THE_BOUND: Duration = Duration::from_millis(5);

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn passes(name: &str) {
    let suite = Suite::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha2/scenarios"),
    )
    .expect("the search suite parses");
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root: &Path| {
        Box::new(Search::new(root).expect("the scenario workspace opens"))
    });
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let result = report
        .scenarios
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} is not a scenario of the suite"));
    assert!(result.passed, "FAIL {name}: {}", result.detail);
}

// semantics.md § The deadline: "The pattern, the glob and the path are judged first, so a call
// that would be refused is refused whatever its deadline."
#[test]
fn a_path_that_climbs_out_is_refused_for_the_path_whatever_its_deadline_asks() {
    passes("adversary-a-path-that-climbs-out-is-refused-for-the-path-whatever-its-deadline-asks");
}

// The same sentence, for the pattern.
#[test]
fn a_broken_expression_is_refused_for_the_pattern_whatever_its_deadline_asks() {
    passes("adversary-a-broken-expression-is-refused-for-the-pattern-whatever-its-deadline-asks");
}

// contract.yaml, `timeout_ms`: `Optional<Integer>`.
#[test]
fn a_negative_deadline_is_not_the_operations_input() {
    passes("adversary-a-negative-deadline-is-not-the-operations-input");
}

// semantics.md § The deadline: "checked ... before every line of a file being searched, so ... a
// slow search stops near its bound."
#[test]
fn a_search_with_a_deadline_of_1_ms_stops_near_it_on_a_tree_that_takes_longer() {
    let dir = scratch();
    let body = "haystack line\n".repeat(5_000);
    for n in 0..200 {
        fs::write(dir.path().join(format!("f{n:03}.txt")), &body).expect("written");
    }
    let search = Search::new(dir.path()).expect("the workspace opens");
    let ask = |timeout_ms| GrepInput {
        timeout_ms: Some(timeout_ms),
        ..GrepInput::new("needle")
    };

    let started = Instant::now();
    let whole = search.grep(&ask(60_000)).expect("the search answers");
    let took = started.elapsed();
    assert!(!whole.deadline_reached, "the uncut search finished");
    assert!(!whole.truncated);
    assert!(took >= SLOWER_THAN_THE_BOUND, "the workload took {took:?}");

    let started = Instant::now();
    let cut = search.grep(&ask(1)).expect("the search answers");
    let took = started.elapsed();
    assert!(cut.deadline_reached, "timeout_ms: 1 on a {took:?} search");
    assert!(took < NEAR_ITS_BOUND, "timeout_ms: 1 took {took:?}");
    assert!(cut.truncated);
    assert!(
        cut.note.as_deref().is_some_and(|n| n.contains("deadline")),
        "{:?}",
        cut.note
    );
}

// Pass 2. The correction round moved the `timeout_ms` refusal after the pattern, the glob and the
// path (wave w4 D3). `adversary-deadline-order.yaml` holds each of those refusals with
// `timeout_ms` 60001 and with 0.
#[test]
fn every_input_refusal_wins_over_the_deadline_at_60001_and_at_0() {
    let suite = Suite::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha2/scenarios"),
    )
    .expect("the search suite parses");
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root: &Path| {
        Box::new(Search::new(root).expect("the scenario workspace opens"))
    });
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let ours: Vec<_> = report
        .scenarios
        .iter()
        .filter(|s| s.name.starts_with("adversary2-order-"))
        .collect();
    assert_eq!(ours.len(), 12, "every order scenario ran");
    let failed: Vec<String> = ours
        .iter()
        .filter(|s| !s.passed)
        .map(|s| format!("{}: {}", s.name, s.detail))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
}
