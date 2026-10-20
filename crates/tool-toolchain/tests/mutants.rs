//! Each mutant of the executor (one property switched off) fails at least one scenario of the
//! `build-test` v1alpha1 suite, so the suite is evidence for the property; and the scenario
//! written to catch each one does, while passing against the intact executor.

mod common;

use tool_contract::conformance::Report;
use tool_contract::mutants::each_mutant_fails;
use tool_toolchain::executor::Mutation;

use common::{failed, run_with, suite};

/// The scenario written to catch each mutant.
fn killer(mutation: Mutation) -> &'static str {
    match mutation {
        Mutation::NoEmptyStdoutCheck => "an-unformatted-go-file-fails-the-go-formatting-check",
        Mutation::NoDeclaredPrograms => "go-build-compiles-the-module",
        Mutation::NoTimeLeft => "a-run-gets-only-the-time-its-caller-has-left",
        Mutation::NoWriteScope => "a-path-the-write-scope-refuses-is-refused-before-rustfmt-starts",
        Mutation::NoLanding => {
            "a-written-path-that-lands-outside-the-workspace-is-refused-before-gofmt-starts"
        }
        Mutation::NoReadLanding => {
            "a-check-path-that-leaves-through-a-link-is-refused-before-gofmt-starts"
        }
    }
}

#[test]
fn each_mutant_fails_at_least_one_scenario_and_the_one_written_for_it() {
    let suite = suite();
    let (intact, reports): (Report, Vec<(Mutation, Report)>) = std::thread::scope(|scope| {
        let intact = scope.spawn(|| run_with(&suite, None));
        let mutants: Vec<_> = Mutation::ALL
            .into_iter()
            .map(|mutation| {
                let suite = &suite;
                (
                    mutation,
                    scope.spawn(move || run_with(suite, Some(mutation))),
                )
            })
            .collect();
        (
            intact.join().expect("the intact run"),
            mutants
                .into_iter()
                .map(|(mutation, run)| (mutation, run.join().expect("a mutant run")))
                .collect(),
        )
    });
    assert!(
        intact.passed(),
        "the intact executor must pass before a mutant's failure means anything:\n{intact}"
    );
    for (mutation, report) in &reports {
        let killed = failed(report);
        println!(
            "{}: fails {} of {}: {killed:?}",
            mutation.name(),
            killed.len(),
            report.scenarios.len()
        );
        assert!(
            killed.contains(&killer(*mutation)),
            "{}: expected {} to fail:\n{report}",
            mutation.name(),
            killer(*mutation)
        );
    }
    let named: Vec<(&str, Report)> = reports
        .into_iter()
        .map(|(mutation, report)| (mutation.name(), report))
        .collect();
    assert_eq!(each_mutant_fails(&named), Ok(()));
}
