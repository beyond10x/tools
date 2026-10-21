//! Adversary pass 2 cases for the correction round: which context the schema check builds with,
//! and the 501 answer to a request carrying `Transfer-Encoding`.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{
    Missing, Outcome, Report, ScenarioContext, Suite, Toolset, run_with,
};

const CONTRACT: &str = "\
format: tool-contract/1
toolset: probe
version: v1alpha1
summary: A fixture toolset for adversary cases on the runner's host and serve keys.
errors:
  - name: probe.Failed
    summary: The probe could not answer.
    fields: [{name: reason, type: String}]
operations:
  - name: probe.host
    summary: Answers the host map the toolset was built with.
    input: []
    output:
      - {name: host, type: \"Map<String, String>\"}
    outcomes:
      - {name: answered, summary: The host map.}
    profile: {effects: [], risk: low, idempotency: idempotent, determinism: deterministic, placement: local, capability: workspace.read}
  - name: probe.send
    summary: Sends one raw HTTP/1.1 request and answers the status, 0 when none came in time.
    input:
      - {name: url, type: String}
      - {name: request, type: String}
      - {name: timeout_ms, type: Integer}
    output:
      - {name: status, type: Integer}
    outcomes:
      - {name: answered, summary: The status.}
    profile: {effects: [], risk: low, idempotency: idempotent, determinism: deterministic, placement: local, capability: workspace.read}
";

const HEAD: &str = "format: tool-conformance/1\ntoolset: probe\nversion: v1alpha1\nscenarios:\n";
const HOLDS: &str = "holds: [idempotency, determinism]";

fn suite(scenarios: &str) -> Suite {
    Suite::from_yaml(&format!("{HEAD}{scenarios}")).expect("the suite parses")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn contract_for(suite: &Suite) -> Contract {
    let mut contract = Contract::from_yaml(CONTRACT).expect("the probe contract parses");
    contract.operations.retain(|op| {
        suite
            .scenarios
            .iter()
            .any(|scenario| scenario.call.operation == op.name)
    });
    contract
}

#[allow(dead_code)]
#[derive(JsonSchema)]
struct NoInput {}

#[derive(Serialize, JsonSchema)]
struct HostOutput {
    host: BTreeMap<String, String>,
}

#[derive(Deserialize, JsonSchema)]
struct SendInput {
    url: String,
    request: String,
    timeout_ms: u64,
}

#[derive(Serialize, JsonSchema)]
struct SendOutput {
    status: u64,
}

struct Probe {
    contract: Contract,
    host: BTreeMap<String, String>,
    statuses: Arc<Mutex<Vec<u64>>>,
}

impl Probe {
    fn new(
        context: &ScenarioContext<'_>,
        contract: Contract,
        statuses: Arc<Mutex<Vec<u64>>>,
    ) -> Self {
        let host = context
            .host
            .iter()
            .map(|(key, value)| {
                let text = value
                    .as_str()
                    .map_or_else(|| value.to_string(), ToOwned::to_owned);
                (key.clone(), text)
            })
            .collect();
        Self {
            contract,
            host,
            statuses,
        }
    }
}

/// Sends `request` as written to the authority of `url`; the status, or 0 when no answer came
/// within `timeout_ms`.
fn send(input: &SendInput) -> Result<u64, String> {
    let authority = input
        .url
        .strip_prefix("http://")
        .ok_or_else(|| format!("{} is not an http URL", input.url))?;
    let authority = authority.split('/').next().unwrap_or(authority);
    let mut stream = TcpStream::connect(authority).map_err(|e| format!("{authority}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(input.timeout_ms)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(input.request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() && response.is_empty() {
        return Ok(0);
    }
    let response = String::from_utf8_lossy(&response);
    Ok(response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0))
}

impl Toolset for Probe {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        [
            ("probe.host", schema_for!(NoInput), schema_for!(HostOutput)),
            (
                "probe.send",
                schema_for!(SendInput),
                schema_for!(SendOutput),
            ),
        ]
        .into_iter()
        .filter(|(name, _, _)| self.contract.operation(name).is_some())
        .map(|(name, input, output)| (name.to_string(), input, output))
        .collect()
    }

    fn invoke(&self, operation: &str, raw: &Value) -> Outcome {
        let failed = |reason: String| Outcome::Error {
            name: "probe.Failed".to_string(),
            message: reason.clone(),
            fields: json!({ "reason": reason }),
        };
        match operation {
            "probe.host" => Outcome::Ok(json!({ "host": self.host })),
            "probe.send" => match serde_json::from_value::<SendInput>(raw.clone()) {
                Ok(input) => match send(&input) {
                    Ok(status) => {
                        self.statuses.lock().expect("not poisoned").push(status);
                        Outcome::Ok(serde_json::to_value(SendOutput { status }).expect("json"))
                    }
                    Err(reason) => failed(reason),
                },
                Err(err) => failed(err.to_string()),
            },
            other => failed(format!("no operation {other}")),
        }
    }
}

fn run_probe(suite: &Suite, scratch: &Path, statuses: &Arc<Mutex<Vec<u64>>>) -> Report {
    let contract = contract_for(suite);
    run_with(
        suite,
        &contract,
        scratch,
        |context: &ScenarioContext<'_>| {
            Ok(
                Box::new(Probe::new(context, contract.clone(), statuses.clone()))
                    as Box<dyn Toolset>,
            )
        },
    )
}

/// A factory that needs the host key `allow`, recording the host value and workspace of every
/// toolset it builds.
type Built = Arc<Mutex<Vec<(Option<String>, String, bool)>>>;

fn run_needing_allow(suite: &Suite, scratch: &Path, built: &Built) -> Report {
    let contract = contract_for(suite);
    let statuses = Arc::new(Mutex::new(Vec::new()));
    run_with(
        suite,
        &contract,
        scratch,
        |context: &ScenarioContext<'_>| {
            built.lock().expect("not poisoned").push((
                context
                    .host
                    .get("allow")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                context.workspace.display().to_string(),
                context.served.is_some(),
            ));
            if !context.host.contains_key("allow") {
                return Err(Missing::Setting("allow".to_string()));
            }
            Ok(
                Box::new(Probe::new(context, contract.clone(), statuses.clone()))
                    as Box<dyn Toolset>,
            )
        },
    )
}

/// Pass 2, red: the first scenario fails before its toolset is built (its `host` names
/// `{{serve}}` and it serves no route). Every scenario's `host` holds `allow`, so nothing is
/// missing; the schema check must not fall back to an empty `host`, report `setting allow` as
/// missing and turn the passing second scenario into `not counted`.
#[test]
fn a_first_scenario_failing_before_its_factory_does_not_make_the_run_miss_a_setting() {
    let suite = suite(&format!(
        "  - name: first-fails-in-host
    {HOLDS}
    host: {{allow: first, url: \"{{{{serve}}}}/x\"}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
  - name: second-passes
    {HOLDS}
    host: {{allow: second}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
"
    ));
    let scratch = scratch();
    let built = Built::default();
    let report = run_needing_allow(&suite, scratch.path(), &built);
    assert!(!report.scenarios[0].passed, "{report}");
    assert!(report.missing.is_empty(), "{report}");
    assert!(report.scenarios[1].passed, "{report}");
}

/// Pass 2, green: the schema check's toolset is built with the first scenario's substituted
/// `host` and workspace and no `served`, not the last scenario's (catches the mutant that keeps
/// every scenario's host).
#[test]
fn the_schema_check_builds_with_the_first_scenarios_host_and_workspace() {
    let suite = suite(&format!(
        "  - name: first-scenario
    {HOLDS}
    host: {{allow: first}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
  - name: second-scenario
    {HOLDS}
    host: {{allow: second}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
"
    ));
    let scratch = scratch();
    let built = Built::default();
    let report = run_needing_allow(&suite, scratch.path(), &built);
    assert!(report.passed(), "{report}");
    let built = built.lock().expect("not poisoned").clone();
    assert_eq!(built.len(), 3, "two scenarios and the schema check");
    let (allow, workspace, served) = &built[2];
    assert_eq!(allow.as_deref(), Some("first"));
    assert!(workspace.ends_with("first-scenario"), "{workspace}");
    assert!(!served, "the schema check runs no server");
}

fn encoded_scenario(name: &str, request: &str) -> String {
    format!(
        "  - name: {name}
    {HOLDS}
    fixture:
      serve:
        /up: {{body: ok}}
    call: {{operation: probe.send, input: {{url: \"{{{{serve}}}}/up\", request: \"{request}\", timeout_ms: 3000}}}}
    expect: {{outcome: ok, output: {{status: 501}}}}
"
    )
}

/// Pass 2, green: `Transfer-Encoding: identity` beside a `Content-Length` is still a request
/// carrying `Transfer-Encoding`: answered 501 and logged with `"body": null`, as the profile says.
#[test]
fn transfer_encoding_identity_is_answered_501_and_logged_with_a_null_body() {
    let request = "POST /up HTTP/1.1\\r\\nHost: x\\r\\nTransfer-Encoding: identity\\r\\nContent-Length: 5\\r\\nConnection: close\\r\\n\\r\\nhello";
    let suite = suite(&encoded_scenario("identity", request));
    let scratch = scratch();
    let report = run_probe(&suite, scratch.path(), &Arc::new(Mutex::new(Vec::new())));
    assert!(report.passed(), "{report}");
    let log = fs::read_to_string(scratch.path().join("identity/.serve/_up.jsonl"))
        .expect("the route log exists");
    let entry: Value = serde_json::from_str(log.lines().next().expect("one line")).expect("JSON");
    assert_eq!(entry["body"], Value::Null, "{entry}");
}

/// Pass 2, green: a 501 answer ends promptly; the client reads to the end and sees the close at
/// once (catches the mutant that drains without shutting the write side, which holds the client
/// until a read timeout).
#[test]
fn a_501_answer_closes_promptly_and_leaves_nothing_running() {
    let request = "POST /up HTTP/1.1\\r\\nHost: x\\r\\nTransfer-Encoding: chunked\\r\\n\\r\\n5\\r\\nhello\\r\\n0\\r\\n\\r\\n";
    let suite = suite(&encoded_scenario("prompt-501", request));
    let scratch = scratch();
    let started = Instant::now();
    let report = run_probe(&suite, scratch.path(), &Arc::new(Mutex::new(Vec::new())));
    let took = started.elapsed();
    assert!(report.passed(), "{report}");
    assert!(
        took < Duration::from_millis(1500),
        "the scenario took {took:?}"
    );
}
