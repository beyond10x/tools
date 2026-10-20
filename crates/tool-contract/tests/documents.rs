//! `tool_contract::documents` carries every `contracts/<toolset>/<version>/contract.yaml` in the
//! repository, byte for byte, so an adopter that links only this crate reads the released contract
//! documents: their command names and their input schemas.

use std::fs;
use std::path::{Path, PathBuf};

use tool_contract::Contract;
use tool_contract::documents::{self, Document};
use tool_contract::types::Shape;

fn contracts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts")
}

fn file_of(toolset: &str, version: &str) -> PathBuf {
    contracts_dir()
        .join(toolset)
        .join(version)
        .join("contract.yaml")
}

/// Every `(toolset, version)` with a `contract.yaml` exactly two directories below `contracts/`,
/// no symlink followed, sorted.
fn on_disk() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for toolset in fs::read_dir(contracts_dir()).expect("contracts/ is readable") {
        let toolset = toolset.expect("a contracts/ entry");
        if !toolset.file_type().expect("its type").is_dir() {
            continue;
        }
        for version in fs::read_dir(toolset.path()).expect("a toolset directory is readable") {
            let version = version.expect("a toolset entry");
            if !version.file_type().expect("its type").is_dir() {
                continue;
            }
            let document = version.path().join("contract.yaml");
            if fs::symlink_metadata(&document).is_ok_and(|meta| meta.is_file()) {
                found.push((
                    toolset.file_name().into_string().expect("a UTF-8 name"),
                    version.file_name().into_string().expect("a UTF-8 name"),
                ));
            }
        }
    }
    found.sort();
    found
}

fn embedded() -> Vec<(String, String)> {
    documents::all()
        .iter()
        .map(|doc| (doc.toolset.to_owned(), doc.version.to_owned()))
        .collect()
}

fn command_names(contract: &Contract) -> Vec<&str> {
    contract
        .operations
        .iter()
        .map(|op| op.name.as_str())
        .collect()
}

fn document(toolset: &str, version: &str) -> &'static Document {
    documents::get(toolset, version).unwrap_or_else(|| panic!("{toolset}/{version} is embedded"))
}

#[test]
fn all_holds_the_released_fs_and_code_editor_documents() {
    let embedded = embedded();
    for (toolset, version) in [("fs", "v1alpha1"), ("code-editor", "v1alpha1")] {
        assert!(
            embedded.iter().any(|(t, v)| t == toolset && v == version),
            "{toolset}/{version} is missing from {embedded:?}"
        );
    }
}

#[test]
fn all_holds_every_document_on_disk_in_toolset_then_version_order() {
    assert_eq!(embedded(), on_disk());
}

#[test]
fn each_embedded_yaml_equals_its_file_on_disk_byte_for_byte() {
    for doc in documents::all() {
        let on_disk = fs::read(file_of(doc.toolset, doc.version)).expect("the file is readable");
        assert!(
            doc.yaml.as_bytes() == on_disk.as_slice(),
            "{}/{}: the embedded text differs from the file",
            doc.toolset,
            doc.version
        );
    }
}

#[test]
fn each_embedded_document_parses_and_names_its_own_directory() {
    for doc in documents::all() {
        let contract = doc.contract();
        assert_eq!(contract.toolset, doc.toolset);
        assert_eq!(contract.version, doc.version);
        assert!(
            std::ptr::eq(contract, doc.contract()),
            "{}/{} is parsed once",
            doc.toolset,
            doc.version
        );
    }
}

#[test]
fn the_fs_document_has_exactly_the_commands_of_its_file() {
    let contract = document("fs", "v1alpha1").contract();
    let from_file = Contract::read(file_of("fs", "v1alpha1")).expect("the file parses");
    assert_eq!(
        command_names(contract),
        ["fs.read", "fs.list", "fs.glob", "fs.stat"]
    );
    assert_eq!(command_names(contract), command_names(&from_file));
}

#[test]
fn the_code_editor_document_has_exactly_the_commands_of_its_file() {
    let contract = document("code-editor", "v1alpha1").contract();
    let from_file = Contract::read(file_of("code-editor", "v1alpha1")).expect("the file parses");
    assert_eq!(
        command_names(contract),
        [
            "code.write",
            "code.edit",
            "code.patch",
            "code.move",
            "code.delete"
        ]
    );
    assert_eq!(command_names(contract), command_names(&from_file));
}

#[test]
fn each_command_has_an_input_schema() {
    for doc in documents::all() {
        let contract = doc.contract();
        for op in &contract.operations {
            let shape = Shape::of_fields(&op.input, &contract.types)
                .unwrap_or_else(|err| panic!("{}: the input has no shape: {err}", op.name));
            let Shape::Struct(fields) = shape else {
                panic!("{}: the input is not a struct", op.name);
            };
            let names: Vec<&str> = fields.keys().map(String::as_str).collect();
            let mut declared: Vec<&str> = op.input.iter().map(|f| f.name.as_str()).collect();
            declared.sort_unstable();
            assert_eq!(names, declared, "{}", op.name);
        }
    }
    for (toolset, version) in [("fs", "v1alpha1"), ("code-editor", "v1alpha1")] {
        for op in &document(toolset, version).contract().operations {
            assert!(!op.input.is_empty(), "{}: no input fields", op.name);
        }
    }
}

#[test]
fn get_answers_an_embedded_document_and_nothing_else() {
    let fs = document("fs", "v1alpha1");
    assert_eq!((fs.toolset, fs.version), ("fs", "v1alpha1"));
    assert!(documents::get("fs", "v9").is_none());
    assert!(documents::get("nope", "v1alpha1").is_none());
}

#[test]
fn each_named_constant_is_its_document() {
    assert_eq!(documents::FS_V1ALPHA1, document("fs", "v1alpha1").yaml);
    assert_eq!(
        documents::CODE_EDITOR_V1ALPHA1,
        document("code-editor", "v1alpha1").yaml
    );
}
