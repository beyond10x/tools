//! The `process` v1alpha1 conformance suite runs against the intact [`Process`] and passes,
//! including the check that the schemas derived from the crate's types equal the contract's field
//! lists; and the contract the crate carries is the one under `contracts/` and validates.
//!
//! The suite's configuration is the one `semantics.md` names: the declared set is exactly
//! [`DECLARED`], nothing is added to the allow-list, and the caller's environment holds a variable
//! off the allow-list (cargo's `CARGO_MANIFEST_DIR`).

use std::path::{Path, PathBuf};

use tool_contract::conformance::{Suite, run};
use tool_contract::{Contract, validate};
use tool_process::{INHERITED_ENV, Process, contract};

/// The declared set the suite is written against.
const DECLARED: [&str; 3] = ["sh", "printf", "b10x-conformance-absent-program"];

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/process/v1alpha1")
}

fn suite() -> Suite {
    Suite::read_dir(contract_dir().join("scenarios")).expect("the process suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn declared() -> Vec<String> {
    DECLARED.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn the_suite_passes_against_the_intact_toolset() {
    assert!(
        std::env::vars_os().any(|(name, _)| {
            !INHERITED_ENV
                .iter()
                .any(|allowed| name.as_os_str() == *allowed)
        }),
        "the environment scenario needs a variable off the allow-list to keep out"
    );
    let suite = suite();
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root| {
        Box::new(Process::new(root, declared()).expect("the scenario workspace opens"))
    });
    println!("{report}");
    assert!(report.passed(), "{report}");
    assert!(
        report.refused.is_empty(),
        "every declared property has a scenario: {report}"
    );
    assert!(report.schema_mismatches.is_empty(), "{report}");
    assert_eq!(report.scenarios.len(), suite.scenarios.len(), "{report}");
    assert!(
        report.scenarios.len() >= 20,
        "the suite is the one in contracts/, not a stub: {report}"
    );
}

#[test]
fn the_contract_the_crate_carries_is_the_one_under_contracts_and_validates() {
    let on_disk =
        Contract::read(contract_dir().join("contract.yaml")).expect("the contract parses");
    assert_eq!(contract(), &on_disk);
    assert_eq!(validate(contract()), Ok(()));
}
