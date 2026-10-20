//! The `code-editor` v1alpha1 conformance suite, run against this crate's [`Editor`].

use std::path::{Path, PathBuf};

use tool_code_editor::Editor;
use tool_contract::Contract;
use tool_contract::conformance::{Suite, run};

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/code-editor/v1alpha1")
}

fn scratch(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

#[test]
fn the_suite_passes_against_the_editor() {
    let contract =
        Contract::read(contract_dir().join("contract.yaml")).expect("the contract parses");
    let suite = Suite::read_dir(contract_dir().join("scenarios")).expect("the suite parses");
    let report = run(&suite, &contract, &scratch("conformance"), |root| {
        Box::new(Editor::new(root).expect("the workspace opens"))
    });
    println!("{report}");
    assert!(report.refused.is_empty(), "{report}");
    assert!(report.schema_mismatches.is_empty(), "{report}");
    assert!(report.passed(), "{report}");
    assert_eq!(report.scenarios.len(), suite.scenarios.len(), "{report}");
}
