//! Every released contract document in the repository, compiled in: each
//! `contracts/<toolset>/<version>/contract.yaml` as text ([`all`], [`get`], one
//! `<TOOLSET>_<VERSION>` constant per document such as [`FS_V1ALPHA1`]) and parsed on first use
//! ([`Document::contract`]).
//!
//! The documents live here, not in the crates that implement them, because a contract names no
//! implementation. An adopter that links only this Apache-2.0 crate reads each toolset's command
//! names and input schemas from it without depending on any implementation crate. The build script
//! finds the documents, so a toolset added under `contracts/` is embedded with no edit to this
//! crate.

use std::sync::OnceLock;

use crate::document::Contract;

/// One contract document as compiled in: `contracts/<toolset>/<version>/contract.yaml`.
///
/// Only this crate makes one, so every `Document` is one of [`all`].
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Document {
    /// The toolset, the first directory under `contracts/`, such as `fs`.
    pub toolset: &'static str,
    /// The version, the directory under the toolset, such as `v1alpha1`.
    pub version: &'static str,
    /// The document's text, byte for byte as the file holds it.
    pub yaml: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/documents.rs"));

static CONTRACTS: [OnceLock<Contract>; COUNT] = [const { OnceLock::new() }; COUNT];

/// Every document, sorted by the byte order of the toolset directory name and then of the version
/// directory name. This is not version precedence: `v1 < v10 < v1alpha1 < v2`.
pub fn all() -> &'static [Document] {
    &DOCUMENTS
}

/// The document of this toolset version, if one is compiled in.
pub fn get(toolset: &str, version: &str) -> Option<&'static Document> {
    DOCUMENTS
        .iter()
        .find(|doc| doc.toolset == toolset && doc.version == version)
}

impl Document {
    /// The parsed document, parsed once.
    ///
    /// # Panics
    ///
    /// The document compiled in does not parse; the panic names its toolset and version.
    /// `tool-contract validate contracts/` refuses such a document before it gets here.
    pub fn contract(&self) -> &'static Contract {
        let index = DOCUMENTS
            .iter()
            .position(|doc| doc.toolset == self.toolset && doc.version == self.version)
            .unwrap_or_else(|| {
                panic!(
                    "{}/{} is not a compiled-in document",
                    self.toolset, self.version
                )
            });
        let doc = &DOCUMENTS[index];
        CONTRACTS[index].get_or_init(|| {
            Contract::from_yaml(doc.yaml).unwrap_or_else(|err| {
                panic!(
                    "the compiled-in {}/{} contract document does not parse: {err}",
                    doc.toolset, doc.version
                )
            })
        })
    }
}
