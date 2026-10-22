//! The `search` v1alpha2 conformance suite runs against the intact [`Search`] and passes,
//! including the check that the schemas derived from the crate's types equal the contract's field
//! lists; and the contract the crate carries is the one under `contracts/` and validates.

use std::path::{Path, PathBuf};

use tool_contract::conformance::{Suite, run};
use tool_contract::{Contract, validate};
use tool_search::{Search, contract};

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/search/v1alpha2")
}

fn suite() -> Suite {
    Suite::read_dir(contract_dir().join("scenarios")).expect("the search suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

#[test]
fn the_suite_passes_against_the_intact_toolset() {
    let suite = suite();
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root| {
        Box::new(Search::new(root).expect("the scenario workspace opens"))
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
        report.scenarios.len() >= 40,
        "the suite is the one in contracts/, not a stub: {report}"
    );
}

#[test]
fn the_contract_the_crate_carries_is_the_one_under_contracts_and_validates() {
    let on_disk =
        Contract::read(contract_dir().join("contract.yaml")).expect("the contract parses");
    assert_eq!(contract(), &on_disk);
    assert_eq!(validate(contract()), Ok(()));
    assert_eq!(
        tool_contract::documents::get("search", "v1alpha2")
            .expect("the search document is compiled in")
            .contract(),
        contract()
    );
}
