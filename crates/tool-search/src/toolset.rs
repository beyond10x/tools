//! [`Search`] as a [`Toolset`]: the contract it carries, the schemas derived from its types, and
//! the dispatch from an operation name and a JSON input to [`Search::grep`].

use std::sync::OnceLock;

use schemars::{Schema, schema_for};
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Toolset};

use crate::Search;
use crate::types::{GrepInput, GrepOutput};

/// The `search` v1alpha2 contract document, compiled in so the binary carries the contract it
/// claims.
pub const CONTRACT_YAML: &str = tool_contract::documents::SEARCH_V1ALPHA2;

/// The parsed `search` v1alpha2 contract this crate implements, parsed once.
///
/// # Panics
///
/// The document compiled into the crate does not parse. `tool-contract validate contracts/`
/// refuses such a document before it gets here.
pub fn contract() -> &'static Contract {
    static CONTRACT: OnceLock<Contract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        Contract::from_yaml(CONTRACT_YAML).expect("the search contract the crate carries parses")
    })
}

impl Toolset for Search {
    fn contract(&self) -> &Contract {
        contract()
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        vec![(
            "search.grep".to_owned(),
            schema_for!(GrepInput),
            schema_for!(GrepOutput),
        )]
    }

    /// Dispatches by operation name. An input that is not the operation's input is answered
    /// with `search.InvalidInput`; an operation the contract does not declare with
    /// `search.UnknownOperation`, which no contract declares because there is no operation to
    /// declare it on.
    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        if operation != "search.grep" {
            return Outcome::Error {
                name: "search.UnknownOperation".to_owned(),
                message: format!("{operation} is not an operation of search"),
                fields: json!({}),
            };
        }
        let input: GrepInput = match serde_json::from_value(input.clone()) {
            Ok(input) => input,
            Err(error) => {
                return Outcome::Error {
                    name: "search.InvalidInput".to_owned(),
                    message: error.to_string(),
                    fields: json!({}),
                };
            }
        };
        match self.grep(&input) {
            Ok(output) => match serde_json::to_value(output) {
                Ok(value) => Outcome::Ok(value),
                Err(error) => Outcome::Error {
                    name: "search.Io".to_owned(),
                    message: format!("the output could not be serialised: {error}"),
                    fields: json!({ "path": "" }),
                },
            },
            Err(error) => Outcome::Error {
                name: error.name().to_owned(),
                message: error.to_string(),
                fields: error.fields(),
            },
        }
    }
}
