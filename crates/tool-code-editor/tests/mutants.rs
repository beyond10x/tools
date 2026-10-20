//! Each mutant of the editor (one property switched off) fails at least one scenario of the
//! `code-editor` v1alpha1 suite that the intact editor passes, so the suite is evidence for that
//! property. Every comparison is against the intact run, so a scenario that is red for an
//! unrelated reason cannot stand in for a mutant's failure.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tool_code_editor::{Editor, Mutant};
use tool_contract::Contract;
use tool_contract::conformance::{Report, Suite, run};
use tool_contract::mutants::each_mutant_fails;

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/code-editor/v1alpha1")
}

fn suite_and_contract() -> (Suite, Contract) {
    let contract =
        Contract::read(contract_dir().join("contract.yaml")).expect("the contract parses");
    let suite = Suite::read_dir(contract_dir().join("scenarios")).expect("the suite parses");
    (suite, contract)
}

fn scratch(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("mutants")
        .join(name)
}

fn run_intact(suite: &Suite, contract: &Contract, name: &str) -> Report {
    let report = run(suite, contract, &scratch(name), |root| {
        Box::new(Editor::new(root).expect("the workspace opens"))
    });
    assert!(report.refused.is_empty(), "intact: {report}");
    println!("intact ({name}):\n{report}");
    report
}

fn run_mutant(suite: &Suite, contract: &Contract, name: &str, mutant: Mutant) -> Report {
    let report = run(suite, contract, &scratch(name), move |root| {
        Box::new(Editor::with_mutant(root, mutant).expect("the workspace opens"))
    });
    assert!(report.refused.is_empty(), "{name}: {report}");
    println!("{name}:\n{report}");
    report
}

fn failures(report: &Report) -> BTreeSet<String> {
    report.failed().map(|s| s.name.clone()).collect()
}

/// The scenarios a mutant fails that the intact editor passes.
fn beyond_intact(mutant: &Report, intact: &Report) -> BTreeSet<String> {
    failures(mutant)
        .difference(&failures(intact))
        .cloned()
        .collect()
}

#[test]
fn each_mutant_fails_at_least_one_scenario_the_intact_editor_passes() {
    let (suite, contract) = suite_and_contract();
    let intact = run_intact(&suite, &contract, "intact");
    let reports = [
        (
            "ignore-precondition",
            run_mutant(
                &suite,
                &contract,
                "ignore-precondition",
                Mutant::IgnorePrecondition,
            ),
        ),
        (
            "sequential-patch",
            run_mutant(
                &suite,
                &contract,
                "sequential-patch",
                Mutant::SequentialPatch,
            ),
        ),
        (
            "no-diff",
            run_mutant(&suite, &contract, "no-diff", Mutant::NoDiff),
        ),
    ];
    for (name, report) in &reports {
        let extra = beyond_intact(report, &intact);
        println!("{name} fails beyond the intact run: {extra:?}");
        assert!(
            !extra.is_empty(),
            "{name} fails nothing the intact editor passes:\n{report}"
        );
    }
    assert_eq!(each_mutant_fails(&reports), Ok(()));
}

#[test]
fn the_ignored_precondition_fails_exactly_the_scenarios_that_expect_a_precondition_failure() {
    let (suite, contract) = suite_and_contract();
    let intact = run_intact(&suite, &contract, "intact-for-precondition");
    let mutant = run_mutant(
        &suite,
        &contract,
        "ignore-precondition-only",
        Mutant::IgnorePrecondition,
    );
    let expected: BTreeSet<String> = suite
        .scenarios
        .iter()
        .filter(|s| s.expect.error.as_deref() == Some("code.PreconditionFailed"))
        .map(|s| s.name.clone())
        .collect();
    assert!(!expected.is_empty(), "the suite pins the precondition");
    assert_eq!(
        beyond_intact(&mutant, &intact),
        expected,
        "the set difference against the intact run must be exactly the scenarios that expect \
         code.PreconditionFailed"
    );
}
