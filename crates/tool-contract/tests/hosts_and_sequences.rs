//! The runner's host keys over an in-crate probe toolset: a scenario's `host` reaches the toolset
//! factory and never the operation, `fixture.serve` routes answer on loopback and log what they
//! receive, `{{serve}}` and `{{workspace}}` are substituted, `calls` runs several calls on one
//! toolset and reads earlier answers, `{$lines: n}` counts a file's lines, and a factory that
//! reports a missing program fails the run naming it.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tool_contract::Contract;
use tool_contract::conformance::{
    Missing, Outcome, Report, ScenarioContext, Suite, Toolset, run, run_with,
};
use tool_contract::mutants::each_mutant_fails;

const CONTRACT: &str = "\
format: tool-contract/1
toolset: probe
version: v1alpha1
summary: A fixture toolset for the runner's host, serve and calls keys.
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
  - name: probe.get
    summary: GETs a URL over plain HTTP/1.1.
    input:
      - {name: url, type: String}
      - {name: header, type: Optional<String>}
    output:
      - {name: status, type: Integer}
      - {name: body, type: String}
      - {name: elapsed_ms, type: Integer}
    outcomes:
      - {name: answered, summary: The response.}
      - {name: failed, error: probe.Failed}
    profile: {effects: [], risk: low, idempotency: idempotent, determinism: deterministic, placement: local, capability: workspace.read}
  - name: probe.make
    summary: Keeps a value under a new id.
    input:
      - {name: value, type: String}
    output:
      - {name: id, type: String}
    outcomes:
      - {name: made, summary: The id.}
    profile: {effects: [], risk: low, idempotency: idempotent, determinism: deterministic, placement: local, capability: workspace.read}
  - name: probe.fetch
    summary: Answers the value kept under an id by this toolset instance.
    input:
      - {name: id, type: String}
    output:
      - {name: id, type: String}
      - {name: value, type: String}
    outcomes:
      - {name: fetched, summary: The value.}
      - {name: failed, error: probe.Failed}
    profile: {effects: [], risk: low, idempotency: idempotent, determinism: deterministic, placement: local, capability: workspace.read}
";

const HEAD: &str = "format: tool-conformance/1\ntoolset: probe\nversion: v1alpha1\nscenarios:\n";

fn suite(scenarios: &str) -> Suite {
    Suite::from_yaml(&format!("{HEAD}{scenarios}")).expect("the suite parses")
}

fn refusal(scenarios: &str) -> String {
    Suite::from_yaml(&format!("{HEAD}{scenarios}"))
        .expect_err("the suite is refused at parse")
        .to_string()
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

/// The probe contract cut down to the operations the suite calls, so a suite of one scenario
/// meets the coverage rule.
fn contract_for(suite: &Suite) -> Contract {
    let mut contract = Contract::from_yaml(CONTRACT).expect("the probe contract parses");
    contract.operations.retain(|op| {
        suite.scenarios.iter().any(|scenario| {
            scenario.call.operation == op.name
                || scenario.calls.iter().any(|step| step.operation == op.name)
        })
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
struct GetInput {
    url: String,
    header: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct GetOutput {
    status: u64,
    body: String,
    elapsed_ms: u64,
}

#[derive(Deserialize, JsonSchema)]
struct MakeInput {
    value: String,
}

#[derive(Serialize, JsonSchema)]
struct MakeOutput {
    id: String,
}

#[derive(Deserialize, JsonSchema)]
struct FetchInput {
    id: String,
}

#[derive(Serialize, JsonSchema)]
struct FetchOutput {
    id: String,
    value: String,
}

/// What a test reads back from the toolsets a run built.
#[derive(Clone, Default)]
struct Seen {
    elapsed_ms: Arc<Mutex<Vec<u64>>>,
    served: Arc<Mutex<Vec<String>>>,
}

struct Probe {
    contract: Contract,
    host: BTreeMap<String, String>,
    items: RefCell<BTreeMap<String, String>>,
    next: Cell<u128>,
    seen: Seen,
}

impl Probe {
    fn new(context: &ScenarioContext<'_>, contract: Contract, seen: Seen) -> Self {
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
        if let Some(served) = context.served {
            seen.served
                .lock()
                .expect("not poisoned")
                .push(served.to_string());
        }
        let first_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_nanos();
        Self {
            contract,
            host,
            items: RefCell::new(BTreeMap::new()),
            next: Cell::new(first_id),
            seen,
        }
    }

    fn get(&self, input: &GetInput) -> Outcome {
        let started = Instant::now();
        match http_get(&input.url, input.header.as_deref()) {
            Ok((status, body)) => {
                let elapsed_ms = u64::try_from(started.elapsed().as_millis()).expect("fits");
                self.seen
                    .elapsed_ms
                    .lock()
                    .expect("not poisoned")
                    .push(elapsed_ms);
                ok(&GetOutput {
                    status,
                    body,
                    elapsed_ms,
                })
            }
            Err(reason) => failed(&reason),
        }
    }
}

fn ok(output: &impl Serialize) -> Outcome {
    Outcome::Ok(serde_json::to_value(output).expect("serialises"))
}

fn failed(reason: &str) -> Outcome {
    Outcome::Error {
        name: "probe.Failed".to_string(),
        message: reason.to_string(),
        fields: json!({ "reason": reason }),
    }
}

fn input<T: for<'de> Deserialize<'de>>(input: &Value) -> Result<T, Outcome> {
    serde_json::from_value(input.clone()).map_err(|err| failed(&err.to_string()))
}

/// One GET over a fresh connection: `(status, body)`.
fn http_get(url: &str, header: Option<&str>) -> Result<(u64, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("{url} is not an http URL"))?;
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let path = if path.is_empty() { "/" } else { path };
    let mut stream = TcpStream::connect(authority).map_err(|e| format!("{authority}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    let extra = header.map(|h| format!("{h}\r\n")).unwrap_or_default();
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\n{extra}Connection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;
    let response = String::from_utf8_lossy(&response);
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| format!("no header end in {response:?}"))?;
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| format!("no status in {head:?}"))?;
    Ok((status, body.to_string()))
}

impl Toolset for Probe {
    fn contract(&self) -> &Contract {
        &self.contract
    }

    fn schemas(&self) -> Vec<(String, Schema, Schema)> {
        [
            ("probe.host", schema_for!(NoInput), schema_for!(HostOutput)),
            ("probe.get", schema_for!(GetInput), schema_for!(GetOutput)),
            (
                "probe.make",
                schema_for!(MakeInput),
                schema_for!(MakeOutput),
            ),
            (
                "probe.fetch",
                schema_for!(FetchInput),
                schema_for!(FetchOutput),
            ),
        ]
        .into_iter()
        .filter(|(name, _, _)| self.contract.operation(name).is_some())
        .map(|(name, input, output)| (name.to_string(), input, output))
        .collect()
    }

    fn invoke(&self, operation: &str, raw: &Value) -> Outcome {
        let answer = match operation {
            "probe.host" => match raw.as_object() {
                Some(fields) if fields.is_empty() => Ok(ok(&HostOutput {
                    host: self.host.clone(),
                })),
                _ => Ok(failed(&format!("probe.host takes no input, got {raw}"))),
            },
            "probe.get" => input::<GetInput>(raw).map(|input| self.get(&input)),
            "probe.make" => input::<MakeInput>(raw).map(|input| {
                let id = format!("item-{}", self.next.get());
                self.next.set(self.next.get() + 1);
                self.items.borrow_mut().insert(id.clone(), input.value);
                ok(&MakeOutput { id })
            }),
            "probe.fetch" => {
                input::<FetchInput>(raw).map(|input| match self.items.borrow().get(&input.id) {
                    Some(value) => ok(&FetchOutput {
                        id: input.id.clone(),
                        value: value.clone(),
                    }),
                    None => failed(&format!("no item {}", input.id)),
                })
            }
            other => Ok(failed(&format!("no operation {other}"))),
        };
        answer.unwrap_or_else(|refused| refused)
    }
}

/// How the run hands the probe what the runner gave the factory.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Runner {
    Intact,
    /// The runner's `host` never reaches the factory.
    DropsHost,
}

fn run_probe(suite: &Suite, scratch: &Path, runner: Runner, seen: &Seen) -> Report {
    let contract = contract_for(suite);
    let empty = Map::new();
    run_with(
        suite,
        &contract,
        scratch,
        |context: &ScenarioContext<'_>| {
            let context = match runner {
                Runner::Intact => *context,
                Runner::DropsHost => ScenarioContext {
                    host: &empty,
                    ..*context
                },
            };
            Ok(Box::new(Probe::new(&context, contract.clone(), seen.clone())) as Box<dyn Toolset>)
        },
    )
}

fn run_intact(suite: &Suite) -> (Report, tempfile::TempDir, Seen) {
    let scratch = scratch();
    let seen = Seen::default();
    let report = run_probe(suite, scratch.path(), Runner::Intact, &seen);
    (report, scratch, seen)
}

const HOLDS: &str = "holds: [idempotency, determinism]";

fn host_scenario() -> String {
    format!(
        "  - name: the-host-map-reaches-the-toolset
    {HOLDS}
    host: {{region: north, root: \"{{{{workspace}}}}\"}}
    call: {{operation: probe.host}}
    expect:
      outcome: ok
      output:
        host:
          region: north
          root: {{$contains: the-host-map-reaches-the-toolset}}
"
    )
}

#[test]
fn a_host_value_reaches_the_toolset_and_dropping_host_fails_the_scenario() {
    let suite = suite(&host_scenario());
    let (report, _scratch, _) = run_intact(&suite);
    assert!(report.passed(), "{report}");
    assert_eq!(report.scenarios.len(), 1, "{report}");

    let scratch = scratch();
    let dropped = run_probe(&suite, scratch.path(), Runner::DropsHost, &Seen::default());
    assert!(!dropped.passed(), "{dropped}");
    assert!(dropped.scenarios[0].detail.contains("region"), "{dropped}");
    assert_eq!(each_mutant_fails(&[("drops-host", dropped)]), Ok(()));
}

#[test]
fn host_is_handed_to_the_factory_never_to_the_operation() {
    let suite = suite(&format!(
        "  - name: host-is-not-input
    {HOLDS}
    host: {{region: north}}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
"
    ));
    let (report, _scratch, _) = run_intact(&suite);
    assert!(
        report.passed(),
        "probe.host refuses any input key, so host was not merged into it: {report}"
    );
}

fn serve_scenario(name: &str, route: &str, header: &str) -> String {
    format!(
        "  - name: {name}
    {HOLDS}
    fixture:
      serve:
        /a: {route}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/a\"{header}}}}}
    expect:
      outcome: ok
      output: {{status: 200, body: x}}
"
    )
}

#[test]
fn a_served_route_answers_its_body_on_loopback() {
    let suite = suite(&serve_scenario(
        "a-served-route-answers",
        "{status: 200, body: \"x\"}",
        "",
    ));
    let (report, _scratch, seen) = run_intact(&suite);
    assert!(report.passed(), "{report}");
    let served = seen.served.lock().expect("not poisoned").clone();
    assert_eq!(served.len(), 1, "{served:?}");
    assert!(served[0].starts_with("http://127.0.0.1:"), "{served:?}");
}

#[test]
fn a_request_with_a_header_leaves_one_line_naming_it_in_the_route_log() {
    let suite = suite(&serve_scenario(
        "a-request-is-logged",
        "{status: 200, body: \"x\"}",
        ", header: \"X-T: 1\"",
    ));
    let (report, scratch, _) = run_intact(&suite);
    assert!(report.passed(), "{report}");
    let log = fs::read_to_string(scratch.path().join("a-request-is-logged/.serve/_a.jsonl"))
        .expect("the route log exists");
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 1, "{log}");
    assert!(lines[0].contains("X-T"), "{log}");
    let entry: Value = serde_json::from_str(lines[0]).expect("a JSON line");
    assert_eq!(entry["method"], "GET", "{entry}");
    assert_eq!(entry["path"], "/a", "{entry}");
    assert!(entry["headers"].is_object(), "{entry}");
    assert_eq!(entry["body"], "", "{entry}");
}

#[test]
fn a_route_with_a_delay_answers_after_at_least_the_delay() {
    let suite = suite(&serve_scenario(
        "a-delayed-route",
        "{status: 200, body: \"x\", delay_ms: 2000}",
        "",
    ));
    let (report, _scratch, seen) = run_intact(&suite);
    assert!(report.passed(), "{report}");
    let elapsed = seen.elapsed_ms.lock().expect("not poisoned").clone();
    assert_eq!(elapsed.len(), 1, "{elapsed:?}");
    assert!(elapsed[0] >= 2000, "answered after {elapsed:?} ms");
}

#[test]
fn the_server_stops_when_its_scenario_ends() {
    let suite = suite(&serve_scenario(
        "the-server-stops",
        "{status: 200, body: \"x\"}",
        "",
    ));
    let (report, _scratch, seen) = run_intact(&suite);
    assert!(report.passed(), "{report}");
    let served = seen.served.lock().expect("not poisoned").clone();
    let authority = served[0].trim_start_matches("http://");
    assert!(
        TcpStream::connect(authority).is_err(),
        "{authority} still accepts after the run"
    );
}

#[test]
fn a_route_status_redirect_and_body_file_are_served() {
    let root = scratch();
    let dir = root.path().join("contract");
    fs::create_dir_all(dir.join("scenarios")).expect("mkdir");
    fs::create_dir_all(dir.join("fixtures")).expect("mkdir");
    fs::write(dir.join("fixtures/page.txt"), "from a file").expect("write");
    fs::write(
        dir.join("scenarios/s.yaml"),
        format!(
            "{HEAD}  - name: body-file
    {HOLDS}
    fixture:
      serve:
        /page: {{status: 201, body_file: fixtures/page.txt}}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/page\"}}}}
    expect:
      outcome: ok
      output: {{status: 201, body: from a file}}
  - name: redirect
    {HOLDS}
    fixture:
      serve:
        /old: {{redirect: /new}}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/old\"}}}}
    expect:
      outcome: ok
      output: {{status: 302}}
  - name: unknown-path
    {HOLDS}
    fixture:
      serve:
        /a: {{body: x}}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/b\"}}}}
    expect:
      outcome: ok
      output: {{status: 404}}
"
        ),
    )
    .expect("write");
    let suite = Suite::read_dir(dir.join("scenarios")).expect("the suite parses");
    let (report, _scratch, _) = run_intact(&suite);
    assert!(report.passed(), "{report}");
}

#[test]
fn a_served_route_whose_path_does_not_start_with_a_slash_is_refused_at_parse() {
    let refused = refusal(&format!(
        "  - name: relative-route
    {HOLDS}
    fixture:
      serve:
        a: {{body: x}}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/a\"}}}}
    expect: {{outcome: ok}}
"
    ));
    assert!(
        refused.contains("relative-route") && refused.contains("must start with /"),
        "{refused}"
    );
}

#[test]
fn a_route_with_both_body_and_body_file_is_refused_at_parse() {
    let refused = refusal(&format!(
        "  - name: two-bodies
    {HOLDS}
    fixture:
      serve:
        /a: {{body: x, body_file: fixtures/x.txt}}
    call: {{operation: probe.get, input: {{url: \"{{{{serve}}}}/a\"}}}}
    expect: {{outcome: ok}}
"
    ));
    assert!(refused.contains("body_file"), "{refused}");
}

fn calls_scenario(name: &str, id: &str) -> String {
    format!(
        "  - name: {name}
    calls:
      - operation: probe.make
        {HOLDS}
        input: {{value: kept}}
        expect: {{outcome: ok, output: {{id: {{$min_len: 1}}}}}}
      - operation: probe.fetch
        {HOLDS}
        input: {{id: \"{id}\"}}
        expect: {{outcome: ok, output: {{value: kept}}}}
"
    )
}

#[test]
fn a_second_call_passes_only_when_it_reads_the_first_calls_output() {
    let reads = suite(&calls_scenario(
        "the-second-call-reads-the-first",
        "{{calls.0.output.id}}",
    ));
    let (report, _scratch, _) = run_intact(&reads);
    assert!(report.passed(), "{report}");

    let guesses = suite(&calls_scenario("the-second-call-guesses", "item-0"));
    let (report, _scratch, _) = run_intact(&guesses);
    assert!(!report.passed(), "{report}");
    let detail = &report.scenarios[0].detail;
    assert!(
        detail.contains("calls[1]") && detail.contains("probe.Failed"),
        "{detail}"
    );
}

#[test]
fn a_reference_to_a_call_not_yet_made_fails_the_scenario() {
    let suite = suite(&calls_scenario(
        "a-forward-reference",
        "{{calls.1.output.id}}",
    ));
    let (report, _scratch, _) = run_intact(&suite);
    assert!(!report.passed(), "{report}");
    assert!(report.scenarios[0].detail.contains("calls.1"), "{report}");
}

#[test]
fn each_entrys_holds_counts_toward_coverage() {
    let suite = suite(
        &calls_scenario("entries-hold", "{{calls.0.output.id}}").replacen(
            &format!("        {HOLDS}\n"),
            "",
            1,
        ),
    );
    let (report, _scratch, _) = run_intact(&suite);
    assert!(
        report
            .refused
            .iter()
            .any(|line| line.contains("probe.make") && line.contains("idempotency")),
        "{report}"
    );
    assert!(
        !report
            .refused
            .iter()
            .any(|line| line.contains("probe.fetch")),
        "{report}"
    );
}

#[test]
fn a_scenario_with_both_call_and_calls_or_neither_is_refused_at_parse() {
    let both = refusal(&format!(
        "  - name: both
    {HOLDS}
    call: {{operation: probe.host}}
    calls:
      - {{operation: probe.host, expect: {{outcome: ok}}}}
    expect: {{outcome: ok}}
"
    ));
    assert!(both.contains("both") && both.contains("calls"), "{both}");
    let neither = refusal(&format!(
        "  - name: neither
    {HOLDS}
    expect: {{outcome: ok}}
"
    ));
    assert!(
        neither.contains("neither") && neither.contains("calls"),
        "{neither}"
    );
}

fn lines_scenario(text: &str) -> String {
    format!(
        "  - name: a-port-log
    {HOLDS}
    fixture:
      files: {{.ports/fixture.jsonl: {text:?}}}
    call: {{operation: probe.host}}
    expect:
      outcome: ok
      files: {{.ports/fixture.jsonl: {{$lines: 2}}}}
"
    )
}

#[test]
fn the_lines_matcher_passes_on_two_lines_and_fails_on_three() {
    let two = suite(&lines_scenario("{\"n\":1}\n{\"n\":2}\n"));
    let (report, _scratch, _) = run_intact(&two);
    assert!(report.passed(), "{report}");

    let three = suite(&lines_scenario("{\"n\":1}\n{\"n\":2}\n{\"n\":3}\n"));
    let (report, _scratch, _) = run_intact(&three);
    assert!(!report.passed(), "{report}");
    let detail = &report.scenarios[0].detail;
    assert!(
        detail.contains(".ports/fixture.jsonl") && detail.contains('3'),
        "{detail}"
    );
}

#[test]
fn a_factory_reporting_a_missing_program_fails_the_run_naming_it() {
    let suite = suite(&format!(
        "{}  - name: second
    {HOLDS}
    call: {{operation: probe.host}}
    expect: {{outcome: ok}}
",
        host_scenario()
    ));
    let contract = contract_for(&suite);
    let scratch = scratch();
    let report = run_with(
        &suite,
        &contract,
        scratch.path(),
        |_: &ScenarioContext<'_>| Err(Missing::Program("probe-bin".to_string())),
    );
    assert!(!report.passed(), "{report}");
    assert!(report.scenarios.iter().all(|s| !s.passed), "{report}");
    let shown = report.to_string();
    assert!(
        shown.lines().any(|line| line.contains("probe-bin")),
        "{shown}"
    );
    assert!(shown.contains("0 of 2 scenarios passed"), "{shown}");
}

#[test]
fn run_still_takes_a_workspace_factory() {
    let suite = suite(&format!(
        "  - name: plain
    {HOLDS}
    call: {{operation: probe.host}}
    expect: {{outcome: ok, output: {{host: {{}}}}}}
"
    ));
    let contract = contract_for(&suite);
    let scratch = scratch();
    let empty = Map::new();
    let report = run(&suite, &contract, scratch.path(), |workspace: &Path| {
        let context = ScenarioContext {
            workspace,
            host: &empty,
            served: None,
        };
        Box::new(Probe::new(&context, contract.clone(), Seen::default())) as Box<dyn Toolset>
    });
    assert!(report.passed(), "{report}");
}
