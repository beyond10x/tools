//! The `tool-contract/1` profile: document parser ([`document`]), validator ([`validate`]), the
//! ESS type grammar and schema shape check ([`types`]), the `tool-conformance/1` runner
//! ([`conformance`]) and the mutant check ([`mutants`]). The profile is written down in
//! `contracts/profile/tool-contract-1.md`; the design is `docs/design/tool-contracts.md` § T1 in
//! detail.

pub mod conformance;
pub mod digest;
pub mod document;
pub mod mutants;
pub mod types;
pub mod validate;

pub use conformance::Toolset;
pub use document::{
    Bound, Bounds, Consequence, Contract, Determinism, Effect, Error, ErrorDecl, Field,
    Idempotency, Operation, Outcome, OutputMarks, Paging, Placement, Profile, Requires, Risk,
    Subject, SubjectKind, TypeDecl, TypeKind,
};
pub use validate::{Violation, validate};
