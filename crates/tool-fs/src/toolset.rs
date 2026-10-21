//! [`Fs`] as a [`Toolset`]: the contract it carries, the schemas derived from its types, and the
//! dispatch from an operation name and a JSON input to the typed operation.

use std::sync::OnceLock;

use schemars::{Schema, schema_for};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Toolset};

use crate::Fs;
use crate::error::FsError;
use crate::types::{
    GlobInput, GlobOutput, ListInput, ListOutput, ReadInput, ReadOutput, StatInput, StatOutput,
};

/// The `fs` v1alpha2 contract document, compiled in so the binary carries the contract it claims.
pub const CONTRACT_YAML: &str = tool_contract::documents::FS_V1ALPHA2;

/// The parsed `fs` v1alpha2 contract this crate implements, parsed once.
///
/// # Panics
///
/// The document compiled into the crate does not parse. `tool-contract validate contracts/`
/// refuses such a document before it gets here.
pub fn contract() -> &'static Contract {
    static CONTRACT: OnceLock<Contract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        Contract::from_yaml(CONTRACT_YAML).expect("the fs contract the crate carries parses")
    })
}

impl Toolset for Fs {
    fn contract(&self) -> &Contract {
        contract()
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        vec![
            (
                "fs.read".to_owned(),
                schema_for!(ReadInput),
                schema_for!(ReadOutput),
            ),
            (
                "fs.list".to_owned(),
                schema_for!(ListInput),
                schema_for!(ListOutput),
            ),
            (
                "fs.glob".to_owned(),
                schema_for!(GlobInput),
                schema_for!(GlobOutput),
            ),
            (
                "fs.stat".to_owned(),
                schema_for!(StatInput),
                schema_for!(StatOutput),
            ),
        ]
    }

    /// Dispatches by operation name. An input that is not the operation's input is answered
    /// with `fs.InvalidInput`; an operation the contract does not declare with
    /// `fs.UnknownOperation`, which no contract declares because there is no operation to
    /// declare it on.
    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        match operation {
            "fs.read" => dispatch(input, |input| self.read(input)),
            "fs.list" => dispatch(input, |input| self.list(input)),
            "fs.glob" => dispatch(input, |input| self.glob(input)),
            "fs.stat" => dispatch(input, |input| self.stat(input)),
            other => Outcome::Error {
                name: "fs.UnknownOperation".to_owned(),
                message: format!("{other} is not an operation of fs"),
                fields: json!({}),
            },
        }
    }
}

fn dispatch<I: DeserializeOwned, O: Serialize>(
    input: &Value,
    call: impl FnOnce(&I) -> Result<O, FsError>,
) -> Outcome {
    let input: I = match serde_json::from_value(input.clone()) {
        Ok(input) => input,
        Err(error) => {
            return Outcome::Error {
                name: "fs.InvalidInput".to_owned(),
                message: error.to_string(),
                fields: json!({}),
            };
        }
    };
    match call(&input) {
        Ok(output) => match serde_json::to_value(output) {
            Ok(value) => Outcome::Ok(value),
            Err(error) => Outcome::Error {
                name: "fs.Io".to_owned(),
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
