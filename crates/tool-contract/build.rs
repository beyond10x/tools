//! Embeds every contract document in the repository: each
//! `contracts/<toolset>/<version>/contract.yaml`, exactly that depth below the workspace root, no
//! symlink followed, sorted by toolset then version. Writes `$OUT_DIR/documents.rs`, which
//! `src/documents.rs` includes: one `<TOOLSET>_<VERSION>` constant per document and the table
//! `documents::all` answers from.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let contracts = manifest_dir.join("../../contracts");
    println!("cargo::rerun-if-changed={}", contracts.display());

    let found = find(&contracts);
    let mut seen = BTreeMap::new();
    let mut constants = String::new();
    let mut table = String::new();
    for (toolset, version) in &found {
        let document = contracts.join(toolset).join(version).join("contract.yaml");
        println!("cargo::rerun-if-changed={}", document.display());
        let constant = format!("{toolset}_{version}")
            .to_uppercase()
            .replace('-', "_");
        if let Some(other) = seen.insert(constant.clone(), format!("{toolset}/{version}")) {
            panic!("{toolset}/{version} and {other} both name the constant {constant}");
        }
        writeln!(
            constants,
            "/// The `{toolset}` `{version}` contract document, \
             `contracts/{toolset}/{version}/contract.yaml`, as text.\n\
             pub const {constant}: &str = include_str!(concat!(\n    \
             env!(\"CARGO_MANIFEST_DIR\"),\n    \
             \"/../../contracts/{toolset}/{version}/contract.yaml\"\n));\n"
        )
        .expect("writing to a String");
        writeln!(
            table,
            "    Document {{ toolset: \"{toolset}\", version: \"{version}\", yaml: {constant} }},"
        )
        .expect("writing to a String");
    }
    let count = found.len();
    let out = format!(
        "{constants}const COUNT: usize = {count};\n\n\
         static DOCUMENTS: [Document; COUNT] = [\n{table}];\n"
    );

    let target = PathBuf::from(env::var("OUT_DIR").expect("cargo sets it")).join("documents.rs");
    fs::write(&target, out).unwrap_or_else(|err| panic!("{}: {err}", target.display()));
}

/// Every `(toolset, version)` whose `contracts/<toolset>/<version>/contract.yaml` is a regular
/// file, sorted. A symlink at any of the three levels is skipped, not followed.
fn find(contracts: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for toolset in directories(contracts) {
        for version in directories(&contracts.join(&toolset)) {
            let document = contracts
                .join(&toolset)
                .join(&version)
                .join("contract.yaml");
            if fs::symlink_metadata(&document).is_ok_and(|meta| meta.is_file()) {
                check_name(&toolset, &document);
                check_name(&version, &document);
                found.push((toolset.clone(), version));
            }
        }
    }
    found.sort();
    found
}

/// The names of the directories directly in `dir`; a symlink is not a directory here.
fn directories(dir: &Path) -> Vec<String> {
    let read = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    let mut names = Vec::new();
    for entry in read {
        let entry = entry.unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
        let kind = entry
            .file_type()
            .unwrap_or_else(|err| panic!("{}: {err}", entry.path().display()));
        if kind.is_dir() {
            let name = entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| panic!("{}: the name is not UTF-8", entry.path().display()));
            names.push(name);
        }
    }
    names
}

/// A toolset or version becomes part of a Rust identifier and of a string literal, so it is held
/// to `[a-z][a-z0-9-]*`.
fn check_name(name: &str, document: &Path) {
    let mut chars = name.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    assert!(
        ok,
        "{}: {name:?} is not [a-z][a-z0-9-]*, so it cannot name a constant",
        document.display()
    );
}
