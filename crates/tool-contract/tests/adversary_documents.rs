//! Claims of `tool_contract::documents` the `documents` lane does not pin on its own: each named
//! constant against its file read by a literal path, `get` against every entry of `all` and against
//! keys that are not one, the strict order of `all`, and the parse cache across documents and
//! across threads making the first call at once.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use tool_contract::Contract;
use tool_contract::documents;

fn file(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts")
        .join(rel)
}

#[test]
fn each_named_constant_is_its_file_read_by_a_literal_path() {
    for (constant, rel) in [
        (documents::FS_V1ALPHA1, "fs/v1alpha1/contract.yaml"),
        (
            documents::CODE_EDITOR_V1ALPHA1,
            "code-editor/v1alpha1/contract.yaml",
        ),
    ] {
        let on_disk = fs::read(file(rel)).expect("the file is readable");
        assert!(
            constant.as_bytes() == on_disk.as_slice(),
            "{rel}: the constant differs from the file"
        );
    }
}

#[test]
fn get_answers_every_entry_of_all_and_no_key_that_is_not_one() {
    for doc in documents::all() {
        let found = documents::get(doc.toolset, doc.version).expect("an entry of all answers");
        assert!(
            std::ptr::eq(found, doc),
            "{}/{}: get answers another entry",
            doc.toolset,
            doc.version
        );
        let upper = doc.toolset.to_uppercase();
        for (toolset, version) in [
            (doc.version, doc.toolset),
            (doc.toolset, ""),
            ("", doc.version),
            (upper.as_str(), doc.version),
            (&doc.toolset[..doc.toolset.len() - 1], doc.version),
        ] {
            assert!(
                documents::get(toolset, version).is_none(),
                "get({toolset:?}, {version:?}) answers"
            );
        }
    }
}

#[test]
fn all_is_strictly_ordered_by_toolset_then_version() {
    let keys: Vec<(&str, &str)> = documents::all()
        .iter()
        .map(|doc| (doc.toolset, doc.version))
        .collect();
    assert!(keys.len() >= 2, "{keys:?}");
    for pair in keys.windows(2) {
        assert!(pair[0] < pair[1], "{keys:?} is not strictly ordered");
    }
}

#[test]
fn each_document_is_cached_as_its_own_contract_and_equals_a_fresh_parse_of_its_file() {
    let all = documents::all();
    for (i, doc) in all.iter().enumerate() {
        let cached = doc.contract();
        let fresh = Contract::read(file(&format!(
            "{}/{}/contract.yaml",
            doc.toolset, doc.version
        )))
        .expect("the file parses");
        assert_eq!(cached, &fresh, "{}/{}", doc.toolset, doc.version);
        for other in &all[i + 1..] {
            assert!(
                !std::ptr::eq(cached, other.contract()),
                "{}/{} and {}/{} share one cache entry",
                doc.toolset,
                doc.version,
                other.toolset,
                other.version
            );
        }
    }
}

#[test]
fn threads_that_parse_at_once_all_get_the_one_cached_contract() {
    let addresses: Vec<Vec<usize>> = thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|n| {
                scope.spawn(move || {
                    let mut seen: Vec<(usize, usize)> = documents::all()
                        .iter()
                        .enumerate()
                        .rev()
                        .map(|(i, doc)| (i, std::ptr::from_ref(doc.contract()).addr()))
                        .collect();
                    if n % 2 == 0 {
                        seen.reverse();
                    }
                    seen.sort_unstable();
                    seen.into_iter().map(|(_, addr)| addr).collect()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a thread did not panic"))
            .collect()
    });
    for seen in &addresses[1..] {
        assert_eq!(seen, &addresses[0]);
    }
    let after: Vec<usize> = documents::all()
        .iter()
        .map(|doc| std::ptr::from_ref(doc.contract()).addr())
        .collect();
    assert_eq!(after, addresses[0]);
}
