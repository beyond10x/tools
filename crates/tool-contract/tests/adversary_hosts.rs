//! Adversary cases for the runner's `host`, `serve`, `calls`, `{$lines: n}` and `Missing` keys:
//! the documented grammar driven against the runner, and the mutants the unit's own lane misses.

use std::collections::BTreeMap;
use std::fmt::Write as _;
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

/// The contract (`Missing::Setting`: "a setting the toolset reads (an environment variable, a
/// host key)") lets a factory report a host key it needs. Every scenario below provides it, so
/// nothing is missing and the run passes. The schema check must not build a toolset with an
/// empty host and count that as the run missing the setting.
#[test]
fn a_factory_needing_a_host_key_every_scenario_provides_passes() {
    let suite = suite(&format!(
        "  - name: allow-is-given
    {HOLDS}
    host: {{allow: example}}
    call: {{operation: probe.host}}
    expect:
      outcome: ok
      output: {{host: {{allow: example}}}}
"
    ));
    let contract = contract_for(&suite);
    let scratch = scratch();
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let report = run_with(
        &suite,
        &contract,
        scratch.path(),
        |context: &ScenarioContext<'_>| {
            if !context.host.contains_key("allow") {
                return Err(Missing::Setting("allow".to_string()));
            }
            Ok(
                Box::new(Probe::new(context, contract.clone(), statuses.clone()))
                    as Box<dyn Toolset>,
            )
        },
    );
    assert!(report.passed(), "{report}");
}

/// `/a/b` and `/a_b` both map to `.serve/_a_b.jsonl`, so `{$lines: n}` on that log counts the
/// requests of two routes. A suite naming both must be refused at parse.
#[test]
fn two_routes_sharing_one_request_log_are_refused_at_parse() {
    let parsed = Suite::from_yaml(&format!(
        "{HEAD}  - name: colliding-logs
    {HOLDS}
    fixture:
      serve:
        /a/b: {{body: one}}
        /a_b: {{body: two}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
"
    ));
    assert!(
        parsed.is_err(),
        "routes /a/b and /a_b both log to .serve/_a_b.jsonl and the suite parsed"
    );
}

/// A chunked request body is either logged (`body` is `hello`) or refused with `501`; it is not
/// answered `200` and logged as an empty body.
#[test]
fn a_chunked_request_body_is_not_silently_logged_empty() {
    let request = "POST /up HTTP/1.1\\r\\nHost: x\\r\\nTransfer-Encoding: chunked\\r\\nConnection: close\\r\\n\\r\\n5\\r\\nhello\\r\\n0\\r\\n\\r\\n";
    let suite = suite(&format!(
        "  - name: chunked
    {HOLDS}
    fixture:
      serve:
        /up: {{body: ok}}
    call: {{operation: probe.send, input: {{url: \"{{{{serve}}}}/up\", request: \"{request}\", timeout_ms: 2000}}}}
    expect: {{outcome: ok}}
"
    ));
    let scratch = scratch();
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let report = run_probe(&suite, scratch.path(), &statuses);
    assert!(report.passed(), "{report}");
    let status = statuses.lock().expect("not poisoned")[0];
    let log =
        fs::read_to_string(scratch.path().join("chunked/.serve/_up.jsonl")).unwrap_or_default();
    let body = log
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<Value>(line).ok())
        .map(|entry| entry["body"].clone());
    assert!(
        status == 501 || body == Some(Value::String("hello".to_string())),
        "answered {status}, logged body {body:?}"
    );
}

/// Catches the mutant that drops the `not counted` pass in `run_with`: one scenario's factory call
/// reports a missing program, the other's succeeds, and still no scenario counts as passed.
#[test]
fn a_program_missing_for_one_scenario_leaves_no_other_scenario_counted() {
    let suite = suite(&format!(
        "  - name: needs-nothing
    {HOLDS}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
  - name: needs-probe-bin
    {HOLDS}
    host: {{needs: probe-bin}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
"
    ));
    let contract = contract_for(&suite);
    let scratch = scratch();
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let report = run_with(
        &suite,
        &contract,
        scratch.path(),
        |context: &ScenarioContext<'_>| {
            if let Some(Value::String(program)) = context.host.get("needs") {
                return Err(Missing::Program(program.clone()));
            }
            Ok(
                Box::new(Probe::new(context, contract.clone(), statuses.clone()))
                    as Box<dyn Toolset>,
            )
        },
    );
    assert!(!report.passed(), "{report}");
    assert!(report.scenarios.iter().all(|s| !s.passed), "{report}");
    assert!(report.scenarios[0].detail.contains("probe-bin"), "{report}");
    assert!(
        report.to_string().contains("0 of 2 scenarios passed"),
        "{report}"
    );
}

/// `{$lines: n}` at its boundaries: an empty file holds 0 lines, a last line without a newline
/// counts, and one line is not two (catches `==` mutated to `<=` or `>=`).
#[test]
fn the_lines_matcher_counts_exactly_at_its_boundaries() {
    let cases: [(&str, &str, u64, bool); 6] = [
        ("empty-is-0", "", 0, true),
        ("empty-is-not-1", "", 1, false),
        ("no-final-newline-counts", "a\nb", 2, true),
        ("one-is-not-2", "a\n", 2, false),
        ("three-is-not-2", "a\nb\nc", 2, false),
        ("a-blank-line-counts", "a\n\n", 2, true),
    ];
    let mut text = String::new();
    for (name, content, lines, _) in &cases {
        write!(
            text,
            "  - name: {name}
    {HOLDS}
    fixture:
      files: {{log.jsonl: {content:?}}}
    call: {{operation: probe.host}}
    expect:
      outcome: ok
      files: {{log.jsonl: {{$lines: {lines}}}}}
"
        )
        .unwrap();
    }
    let suite = suite(&text);
    let scratch = scratch();
    let report = run_probe(&suite, scratch.path(), &Arc::new(Mutex::new(Vec::new())));
    for ((name, _, _, passes), result) in cases.iter().zip(&report.scenarios) {
        assert_eq!(result.name, *name, "{report}");
        assert_eq!(result.passed, *passes, "{name}: {report}");
    }
}

/// Catches the mutant that drops the stop check in a delayed answer: the client gives up after
/// 200 ms on a route delayed 30 s, and the scenario ends long before the delay would.
#[test]
fn a_delayed_answer_does_not_hold_its_scenario_past_the_end() {
    let suite = suite(&format!(
        "  - name: given-up
    {HOLDS}
    fixture:
      serve:
        /slow: {{body: late, delay_ms: 30000}}
    call: {{operation: probe.send, input: {{url: \"{{{{serve}}}}/slow\", request: \"GET /slow HTTP/1.1\\r\\nHost: x\\r\\n\\r\\n\", timeout_ms: 200}}}}
    expect: {{outcome: ok, output: {{status: 0}}}}
"
    ));
    let scratch = scratch();
    let started = Instant::now();
    let report = run_probe(&suite, scratch.path(), &Arc::new(Mutex::new(Vec::new())));
    let took = started.elapsed();
    assert!(report.passed(), "{report}");
    assert!(took < Duration::from_secs(5), "the scenario took {took:?}");
}
