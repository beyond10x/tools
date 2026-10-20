//! [`Process`] as a [`Toolset`]: the contract it carries, the schemas derived from its types, and
//! the dispatch from an operation name and a JSON input to [`Process::run`].

use std::sync::OnceLock;

use schemars::{Schema, schema_for};
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Toolset};

use crate::Process;
use crate::error::ProcessError;
use crate::types::{RunInput, RunOutput};

/// The `process` v1alpha1 contract document, compiled in so the binary carries the contract it
/// claims.
pub const CONTRACT_YAML: &str = tool_contract::documents::PROCESS_V1ALPHA1;

/// The parsed `process` v1alpha1 contract this crate implements, parsed once.
///
/// # Panics
///
/// The document compiled into the crate does not parse. `tool-contract validate contracts/`
/// refuses such a document before it gets here.
pub fn contract() -> &'static Contract {
    static CONTRACT: OnceLock<Contract> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        Contract::from_yaml(CONTRACT_YAML).expect("the process contract the crate carries parses")
    })
}

impl Toolset for Process {
    fn contract(&self) -> &Contract {
        contract()
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        vec![(
            "process.run".to_owned(),
            schema_for!(RunInput),
            schema_for!(RunOutput),
        )]
    }

    /// Dispatches by operation name, with no deadline of the caller's: the runner has none to
    /// pass. An input that is not the operation's input is `process.InvalidInput`; an operation
    /// the contract does not declare is `process.UnknownOperation`, which no contract declares
    /// because there is no operation to declare it on.
    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        if operation != "process.run" {
            return Outcome::Error {
                name: "process.UnknownOperation".to_owned(),
                message: format!("{operation} is not an operation of process"),
                fields: json!({}),
            };
        }
        let answer = serde_json::from_value::<RunInput>(input.clone())
            .map_err(ProcessError::InvalidInput)
            .and_then(|input| self.run(&input, None));
        match answer {
            Ok(output) => {
                Outcome::Ok(serde_json::to_value(output).expect(
                    "a RunOutput is strings, integers and booleans, which always serialise",
                ))
            }
            Err(error) => refusal(&error),
        }
    }
}

fn refusal(error: &ProcessError) -> Outcome {
    Outcome::Error {
        name: error.name().to_owned(),
        message: error.to_string(),
        fields: error.fields(),
    }
}
