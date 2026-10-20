//! The `build-test` v1alpha1 conformance suite runs against the executor over the five built-in
//! providers and passes, the check that the schemas the executor derives equal the contract's
//! field lists included; and the document the crate carries is the one under `contracts/` and
//! validates.
//!
//! Every scenario starts a real toolchain program on a fixture workspace (`cargo`, `rustfmt`,
//! `go`, `gofmt`, `task`, `npm`, `yarn`), offline. A machine without one of them fails here,
//! naming it, rather than passing a suite that never ran.

mod common;

use tool_contract::conformance::run;
use tool_contract::{Contract, validate};
use tool_toolchain::document;

use common::{contract_dir, make, missing_programs, scratch, suite};

#[test]
fn every_program_a_built_in_provider_declares_is_on_this_machine() {
    let missing = missing_programs();
    assert!(
        missing.is_empty(),
        "the build-test suite starts every built-in provider's programs, and this machine has \
         none of: {missing:?}"
    );
}

#[test]
fn the_suite_passes_against_the_executor() {
    let suite = suite();
    let scratch = scratch();
    let report = run(&suite, document(), scratch.path(), |root| make(root, None));
    println!("{report}");
    assert!(report.passed(), "{report}");
    assert!(report.refused.is_empty(), "{report}");
    assert!(report.schema_mismatches.is_empty(), "{report}");
    assert_eq!(report.scenarios.len(), suite.scenarios.len(), "{report}");
    let operations: std::collections::BTreeSet<&str> = suite
        .scenarios
        .iter()
        .map(|scenario| scenario.call.operation.as_str())
        .collect();
    assert_eq!(
        operations.len(),
        document().operations.len(),
        "every operation is called: {operations:?}"
    );
}

#[test]
fn the_document_the_crate_carries_is_the_one_under_contracts_and_validates() {
    let on_disk =
        Contract::read(contract_dir().join("contract.yaml")).expect("the contract parses");
    assert_eq!(document(), &on_disk);
    assert_eq!(validate(document()), Ok(()));
    let text =
        std::fs::read_to_string(contract_dir().join("contract.yaml")).expect("the contract reads");
    assert_eq!(tool_toolchain::DOCUMENT_YAML, text, "byte for byte");
}
