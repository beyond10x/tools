//! [`Executor`] as the `build-test` v1alpha1 [`Toolset`]: the document it claims, the schemas
//! derived from the built-in providers and the run types, and the dispatch from an operation name
//! and a JSON input to [`Executor::run`].

use std::collections::BTreeMap;
use std::sync::OnceLock;

use schemars::{Schema, schema_for};
use serde_json::Value;
use tool_contract::Contract;
use tool_contract::conformance::{Outcome, Toolset};
use tool_process::RunOutput;

use crate::contract::operation_name;
use crate::executor::{Executor, RunError, Runs};
use crate::{ParameterKind, Registry, resolve_tool};

/// The `build-test` v1alpha1 contract document, `contracts/build-test/v1alpha1/contract.yaml`,
/// compiled in so the crate carries the contract it claims.
pub const DOCUMENT_YAML: &str = tool_contract::documents::BUILD_TEST_V1ALPHA1;

/// The parsed `build-test` v1alpha1 document: the operations the five built-in providers derive,
/// frozen ([`crate::contract`] derives the same per workspace).
///
/// # Panics
///
/// The document compiled into the crate does not parse. `tool-contract validate contracts/`
/// refuses such a document before it gets here.
pub fn document() -> &'static Contract {
    static DOCUMENT: OnceLock<Contract> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        Contract::from_yaml(DOCUMENT_YAML)
            .expect("the build-test document the crate carries parses")
    })
}

/// Per built-in tool: its operation name, the registry's input schema with an enum parameter's
/// values left out, and the schema of what it answers.
///
/// The input schema is the one the registry publishes for a resolved tool (`resolve_tool`),
/// resolved against no workspace. An enum parameter's values are the workspace's own, so the
/// document declares the string they are drawn from and the schema drops the empty value list
/// it would otherwise carry. The output is [`RunOutput`] for a tool whose plan starts one process
/// and [`Runs`] for one whose plan can start several.
fn builtin_schemas() -> Vec<(String, Schema, Schema)> {
    let registry = Registry::builtins().expect("the built-in providers load");
    let mut schemas = Vec::new();
    for (spec, _) in registry.definitions() {
        for tool_spec in &spec.tools {
            let tool = resolve_tool(tool_spec, &BTreeMap::new(), &BTreeMap::new());
            let mut input = tool.input_schema.clone();
            for (name, parameter) in &tool_spec.parameters {
                if parameter.kind == ParameterKind::Enum
                    && let Some(property) = input
                        .get_mut("properties")
                        .and_then(|properties| properties.get_mut(name))
                        .and_then(Value::as_object_mut)
                {
                    property.remove("enum");
                }
            }
            let input = Schema::try_from(input).expect("the registry's input schema is an object");
            let output = if tool.several_processes() {
                schema_for!(Runs)
            } else {
                schema_for!(RunOutput)
            };
            schemas.push((operation_name(&tool.name), input, output));
        }
    }
    schemas
}

impl Toolset for Executor {
    fn contract(&self) -> &Contract {
        document()
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        builtin_schemas()
    }

    /// Runs a declared operation under the time left [`Executor::within`] set, if any. An
    /// operation the document does not declare, or one no provider the workspace resolves
    /// plans, is `build-test.Refused` and starts nothing.
    fn invoke(&self, operation: &str, input: &Value) -> Outcome {
        let answer = if document().operation(operation).is_none() {
            Err(RunError::Refused(format!(
                "`{operation}` is not an operation of build-test v1alpha1; nothing was started"
            )))
        } else {
            self.run(operation, input, self.time_left())
                .unwrap_or_else(|| Err(RunError::Refused(self.not_resolved(operation))))
        };
        match answer {
            Ok(output) => Outcome::Ok(output),
            Err(error) => Outcome::Error {
                name: error.name().to_owned(),
                message: error.to_string(),
                fields: error.fields(),
            },
        }
    }
}

impl Executor {
    fn not_resolved(&self, operation: &str) -> String {
        let resolved: Vec<&str> = self
            .providers()
            .iter()
            .map(|provider| provider.name.as_str())
            .collect();
        format!(
            "`{operation}` is not an operation of a toolchain provider this workspace resolves \
             ({}); nothing was started",
            if resolved.is_empty() {
                "none".to_owned()
            } else {
                resolved.join(", ")
            }
        )
    }
}
