# The `tool-contract/1` profile

Status: draft, 2026-10-20. This document states the profile in full: every property, its allowed
values, and the rules `tool-contract validate` applies. The rationale is in
[`docs/design/tool-contracts.md`](../../docs/design/tool-contracts.md) (§ Model for where each
property was taken from, § T1 in detail for the grammar); this document does not restate it. The
implementation is `crates/tool-contract` (package `b10x-tool-contract`, library `tool_contract`).

A contract is a specification: it names no implementation, no vendor tool name and no harness.
Harness names appear only under `renderings`, as generated output.

## The document

One file, `contracts/<toolset>/<version>/contract.yaml`. Unknown keys are refused at every level.

| key | required | value |
|---|---|---|
| `format` | yes | the literal `tool-contract/1`; another value is refused at parse |
| `toolset` | yes | the toolset, which is also the directory name under `contracts/` |
| `family` | no | the prefix every operation name carries, when it differs from `toolset` (the `code-editor` toolset's operations are `code.*`, so it sets `family: code`); defaults to `toolset` |
| `version` | yes | the toolset version, such as `v1alpha1`; a released version is never edited in place |
| `summary` | yes | one paragraph |
| `types` | no | named types ([§ Types](#types)) |
| `errors` | no | the errors operations can return ([§ Errors and outcomes](#errors-and-outcomes)) |
| `operations` | yes | the operations ([§ Operations](#operations)) |

## Types

Every field (`input`, `output`, an error's `fields`, a `struct` type's `fields`) has a `name`, a
`type` and an optional `description`. The `type` text is in this grammar:

| text | meaning |
|---|---|
| `String`, `Integer`, `Boolean`, `Bytes` | the primitives |
| `Optional<T>` | `T`, or absent, or `null` |
| `List<T>` | an ordered list of `T` |
| `Map<K, V>` | a map whose keys are `K` and values `V`; `K` must resolve to `String` |
| any other identifier | a type declared under `types`, such as `fs.Kind` |

Whitespace around `,` and inside `<…>` is allowed. In a YAML flow mapping (`{name: x, type: …}`)
a type text containing `,` must be quoted.

A declaration under `types` has `name`, `kind`, an optional `summary`, and the part its kind asks
for:

| `kind` | carries | shape |
|---|---|---|
| `enum` | `variants`: a non-empty list of strings | one of the variants; the order is not significant |
| `struct` | `fields`: a non-empty field list | named fields |
| `newtype` | `type`: one type text | the shape of that type |

Rule `types` (document-level): a declaration that does not fit its kind, a duplicate type name, or
a field whose type does not parse or names an undeclared type, is refused.

## Operations

| key | required | value |
|---|---|---|
| `name` | yes | `<family>.<op>`, with `<op>` in `[a-z][a-z0-9_-]*` |
| `summary` | yes | one line a reader chooses between operations by |
| `guidance` | no | how to call it well; presentation, kept apart from `summary` |
| `renderings` | no | harness → generated name ([§ Renderings](#renderings)) |
| `input` | no | the input field list |
| `output` | no | the output field list |
| `outcomes` | no | the outcomes ([§ Errors and outcomes](#errors-and-outcomes)) |
| `profile` | yes | the profile ([§ Profile](#profile)) |

Rule `name`: an operation whose name is not `<family>.<op>`, or whose name another operation
already carries, is refused.

Rule `input`, `output`: a duplicate field name, a field whose type does not parse, a field that
names an undeclared type, or a `Map` whose key is not `String`, is refused.

### Renderings

`renderings` maps a harness key to the name the operation is published under there. The source of
the name is the operation's `name`; the renderings are generated output and may be regenerated.

| key | meaning | example for `fs.read` |
|---|---|---|
| `claude` | the tool name a Claude harness offers | `Read` |
| `codex` | the tool name a Codex harness offers | `read_file` |
| `mcp` | the MCP tool name | `fs_read` |

Other keys are allowed. Rule `renderings`: within one harness key, a rendered name used by two
operations is refused.

### Errors and outcomes

An error under `errors` has a `name` (such as `fs.NotFound`), an optional `summary` and optional
`fields`. An outcome has a `name`, an optional `summary` and, for an error outcome, `error`: the
name of a declared error. Every error an operation can return is one of its outcomes.

Rule `outcomes`: an outcome that names an undeclared error, or a duplicate outcome name, is
refused. Rule `errors` (document-level): an error that no outcome in any operation names, or a
duplicate error name, is refused.

## Profile

| property | required | values | what the validator checks |
|---|---|---|---|
| `effects` | yes | a list of `read`, `write`, `process`, `network`, `secret`, `external`, `human-visible` | sets the risk floor |
| `consequences` | no | a list of `money`, `delete`, `send-external`, `irreversible`, `sensitive-data` | sets the risk floor |
| `risk` | yes | `none`, `low`, `medium`, `high`, `critical` | rule `risk`: never under the floor |
| `idempotency` | yes | `idempotent`, `keyed`, `non-idempotent` | nothing; `keyed` means the input carries a caller `operation_id` and the toolset has a reconcile operation, by convention and not yet checked |
| `preconditions` | no | input field names, such as `expected_sha256` | rule `preconditions`: each is an input field |
| `determinism` | yes | `deterministic`, `nondeterministic` | nothing |
| `bounds` | yes | a map of bound name → `{default, max}` | rule `bounds` ([§ Bounds](#bounds)) |
| `paging` | no | `{offset, limit, truncated, digest}` | rule `paging` ([§ Paging](#paging)) |
| `requires` | no | `{binaries, env, settings, secrets}`, each a list of strings | nothing |
| `subjects` | no | a list of `{field, kind, pattern}` | rule `subjects` ([§ Subjects](#subjects)) |
| `placement` | yes | `local`, `confined`, `container` | nothing |
| `capability` | yes | a Canon capability id | rule `capability`: `[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)+` |
| `output` | no | `{untrusted, failure_as_result}`, each a boolean, default `false` | nothing |

### Risk floor

The floor is the greatest of the floors below over the operation's effects and consequences;
`none` when it has neither. Rule `risk`: a `risk` under the floor is refused, naming the floor and
what set it.

| effect | floor | | consequence | floor |
|---|---|---|---|---|
| `read` | `none` | | `delete` | `medium` |
| `network` | `low` | | `sensitive-data` | `medium` |
| `write` | `low` | | `irreversible` | `high` |
| `human-visible` | `low` | | `money` | `high` |
| `process` | `medium` | | `send-external` | `high` |
| `secret` | `medium` | | | |
| `external` | `high` | | | |

### Bounds

The bound names are fixed: `timeout_ms`, `output_bytes`, `processes`, `cpu_ms`, `memory_bytes`.
Each is `{default, max}` with non-negative integers; `max` is required, `default` is the value a
call gets when it names none and is `max` when absent. Rule `bounds`: `timeout_ms` missing, a bound
without `max`, or a `default` above `max`, is refused.

### Paging

`paging` names the fields the operation pages with. Each of the four is optional, at least one
must be named. `offset` and `limit` name input fields; `truncated` and `digest` name output fields.
`digest` is the digest of the whole, not of the page. Rule `paging`: a named field that is not on
its side, or a `paging` that names nothing, is refused.

### Subjects

A subject is an input field that names what a call touches: `{field, kind, pattern}` with `kind`
one of `path`, `url`, `argv`, and an optional `pattern` such as `fs.read({path})`, the permission
pattern a governor can match a call against. Rule `subjects`: a `field` that is not an input field
is refused.

## Violations

Every refusal is `<operation>: <property>: <message>`; a rule over the document as a whole (types,
errors) carries `document` as the operation. `tool-contract validate <dir>` prints one line per
violation prefixed with the document path, `ok <path>` per accepted document, and exits 1 on any
violation or parse error. A directory with no `contract.yaml` under it prints
`no contract documents under <dir>` and exits 0. When a document has a sibling `scenarios/`
directory, `validate` also applies the coverage rule below.

## Conformance: `tool-conformance/1`

`contracts/<toolset>/<version>/scenarios/*.yaml` are `tool-conformance/1` documents; the runner
reads them in name order into one suite. Fixture `dir`s resolve against the contract directory,
the parent of `scenarios/`.

| key | required | value |
|---|---|---|
| `format` | yes | `tool-conformance/1` |
| `toolset`, `version` | yes | the contract's |
| `scenarios` | yes | the scenarios |

A scenario:

| key | required | value |
|---|---|---|
| `name` | yes | unique in the suite; also the workspace directory name, so no `/` and no leading `.` |
| `holds` | no | the profile properties this scenario is evidence for; each must be a profile property. Not allowed beside `calls`, whose entries carry their own |
| `host` | no | a map handed to the toolset factory, never to an operation: allow-lists, secret values, a grant ceiling, port fixture tables. Its strings are substituted (below) |
| `fixture` | no | `dir`: a directory to copy into the workspace first, symlinks recreated as symlinks with their target as written; `files`: workspace-relative path → text, written after `dir`; `serve`: HTTP routes served while the scenario runs (below) |
| `call` | one of `call`, `calls` | `{operation, input}`; `input` is the operation's input as JSON, `{}` when absent; every key must be an input field of the operation (a missing key is not checked). Its strings are substituted (below) |
| `calls` | one of `call`, `calls` | a non-empty ordered list of `{operation, input, holds, expect}`, made in order on one toolset; `operation` and `input` as in `call`, `holds` as the scenario's (on that entry's operation), `expect` as the scenario's (required) |
| `expect` | with `call` | exactly one of `outcome: ok` or `error: <name>`, where `<name>` is the `error` of one of the operation's outcomes; optional `output`; optional `files`. Not allowed beside `calls` |

A scenario with both `call` and `calls`, or neither, is refused when the suite is parsed.

`fixture.serve` maps a route path to its answer. The runner serves the routes on
`127.0.0.1:<ephemeral port>` over plain HTTP/1.1, one request per connection, from the moment the
fixture is laid out until the scenario ends; no thread or socket outlives the scenario. A path
must start with `/` and hold no whitespace, `?` or `#`, or the suite is refused when it is parsed.
A request is matched on its path without the query; a path with no route is answered `404`.

| route key | required | value |
|---|---|---|
| `status` | no | the status code, 100 to 599; `200`, or `302` when `redirect` is set |
| `headers` | no | response header name → value |
| `body` | no | the body text; empty when neither it nor `body_file` is given |
| `body_file` | no | a file whose bytes are the body, relative to the contract directory; not with `body` |
| `redirect` | no | the `Location` header value |
| `delay_ms` | no | how long to wait before answering |

Each request a route receives is appended to `.serve/<path with / as _>.jsonl` in the workspace
(`/a` → `.serve/_a.jsonl`) as one JSON line `{"method", "path", "headers", "body"}`. Two routes of
one scenario whose paths give the same log name (`/a/b` and `/a_b` both give `.serve/_a_b.jsonl`)
are refused at parse, naming both routes and the log file. `path` is the
request target as sent, query included; `headers` maps each name as sent to its value, repeated
names joined with `, `; `body` is the body read by `Content-Length`, as text. A request carrying
`Transfer-Encoding` (a chunked body, say) is not decoded: it is logged with `"body": null` and
answered `501 Not Implemented`, whatever the route says. A request log that cannot be written
fails the scenario.

Substitution, in every string of `call.input`, of each `calls` entry's `input` and of `host`:

| written | replaced with |
|---|---|
| `{{serve}}` | the served base URL, `http://127.0.0.1:<port>`, no trailing slash; a scenario without `fixture.serve` fails |
| `{{workspace}}` | the scenario's workspace path |
| `{{calls.<n>.output.<path>}}` | in an entry of `calls`, the answer of entry `<n>` (0-based, earlier than this one): its output, or its error's fields; `<path>` is keys and list indices joined by `.`. A string that is the reference alone takes the value with its JSON type; inside a longer string a non-string value is written as JSON. A reference to an entry not yet made, or a path the answer does not have, fails the scenario |

Any other `{{...}}` is left as written.

`expect.output` is matched as a subset: every key named must match, keys not named are not
looked at. Under `error`, it is matched against the error's fields. Lists match element by
element and must have the same length. A value that is an object with one key starting with `$`
is a matcher:

| matcher | holds when |
|---|---|
| `{$any: true}` | the key is present with a non-`null` value |
| `{$absent: true}` | the key is missing or `null` |
| `{$sha256_of: <path>}` | the value is the lowercase hex SHA-256 of the workspace file at `<path>`, read after the call |
| `{$contains: <text>}` | the value is a string containing `<text>`, or a list with an element equal to it |
| `{$min_len: <n>}` | the value is a string of at least `<n>` characters, or a list or object of at least `<n>` members |

`expect.files` maps a workspace-relative path to a post-condition, checked after the call:

| post-condition | holds when |
|---|---|
| `unchanged` | the entry exists as the fixture laid it out: a file with the same content, or a symlink with the same target |
| `absent` | there is no such file (nor a link, dangling or not) |
| `{equals: <text>}` | the file holds exactly `<text>`; a link is followed |
| `{sha256: <hex>}` | the file's SHA-256 is `<hex>`; a link is followed |
| `{$lines: <n>}` | the file holds `<n>` lines, a last line without a newline counted; a link is followed. A fixture port's call log (`.ports/<port>.jsonl`, one line per call) is checked with it |

In a `calls` scenario each entry's `expect.files` is checked after that entry's call; `unchanged`
compares with the tree as the fixture laid it out, before the first call.

### Coverage

The set of profile properties that must each have at least one scenario whose `holds` names them
is the behavioural set: `effects`, `idempotency`, `determinism`, `bounds`, `paging`,
`preconditions`, `subjects`, `output`; and only those of them that are present and non-empty in
the operation's profile. The declarative properties `risk`, `consequences`, `requires`,
`placement` and `capability` are checked by the validator alone and need no scenario.

The rule is applied per operation: for each operation and each behavioural property present in
its profile, some scenario whose `call.operation` is that operation must list the property in
`holds`, or some `calls` entry whose `operation` is that operation must list it in its own
`holds`. The runner refuses the suite before any scenario runs, with one line
`<operation>: <property>: no scenario holds it` per gap. It also refuses a `holds` entry that is
not a profile property, a call to an operation the contract does not declare, a `call.input` key
that is not an input field of the operation, an `expect.error` that is not the `error` of one of
the operation's outcomes, and a `toolset` or `version` that is not the contract's; each of these
applies to every entry of `calls` too, and its line names the entry as `calls[<i>]`. A scenario
is checked against the contract before it counts as evidence about an implementation.

### The runner

`tool_contract::conformance::run_with(suite, contract, scratch, make)`, where `make` takes a
`ScenarioContext { workspace, host, served }` (the workspace path, the substituted `host` map,
empty when the scenario names none, and the served base URL when it has routes) and answers a
toolset or `Missing::Program(name)` / `Missing::Setting(name)`.
`tool_contract::conformance::run(suite, contract, scratch, make)` with `make(workspace)` is
`run_with` over a factory that reads only the workspace.

1. applies the coverage rule; a refused suite runs nothing;
2. for each scenario, in order: clears and creates `<scratch>/<name>/`, copies `fixture.dir`,
   writes `fixture.files`, records every file's digest and every symlink's target, starts serving
   `fixture.serve`, calls `make` once for a toolset, invokes `call.operation` with the
   substituted `call.input` (or each `calls` entry in order on that toolset), checks the
   outcome, matches `expect.output`, checks `expect.files`, and stops the server. The first
   expectation that does not hold ends the scenario; its detail names the entry as
   `calls[<i>]`. A returned error whose name is not
   the `error` of one of the operation's outcomes fails the scenario whatever `expect` says: an
   implementation may only return the errors its contract declares for that operation. Its
   `fields` must be a JSON object whose keys are exactly the declared `fields` names of that
   error, in any order; a missing or an extra key fails the scenario naming the error and the
   key. Field values are not checked against their declared types (a returned value carries no
   schema; `expect.output` is where a scenario pins them);
3. once per run, after the scenarios, builds a toolset with the context of the first scenario
   whose factory was called (its workspace and its `host` as substituted for its run, `served`
   absent; a `Missing` from this build is the run's, as below) and checks every schema it
   derives against the contract ([§ Schema shapes](#schema-shapes)); an operation with no
   derived schema, or a derived schema for an undeclared operation, is a mismatch. When no
   scenario reached its factory, the toolset is built with an empty `host` over
   `<scratch>/.schemas/`, and a `Missing` from that build is not reported unless the suite has
   no scenario (every scenario has already failed, so the run fails anyway).

A program or setting the toolset needs and the machine or the scenario's `host` lacks fails the
run: when `make` answers `Missing`, the scenario fails naming it, the report records it once, and
every other scenario's result is turned into a failure (`not counted: the run is missing ...`),
so the summary counts no scenario as passed. There is no skipped state.

The `Report` holds the refusals, what was missing, one result per scenario (`name`, `passed`,
`detail`) and the schema mismatches; `passed()` is true only when all four are clean. Its text is
one line per refusal, `missing: <program|setting> <name>` per missing item, `ok   <name>` or
`FAIL <name>: <detail>` per scenario, `schema: <mismatch>` per mismatch, then `<passed> of
<total> scenarios passed`, followed by `, <n> missing` when anything was.

### Mutants

Each toolset crate builds mutants of itself (one property switched off, behind
`#[cfg(any(test, feature = "mutants"))]`), runs the suite against each and asserts with
`tool_contract::mutants::each_mutant_fails` that every mutant fails at least one scenario. A
mutant that passes every scenario shows the suite is not evidence for the property it drops.

## Schema shapes

An implementation exposes `schemars`-derived input and output schemas per operation. The check
maps the contract's field list and the derived schema to one canonical shape and refuses the first
difference as `<operation>: <input|output>: <field>: <detail>`, where `<field>` is a path such as
`lines.total` or `entries[].kind`.

| shape | from a field list | from a JSON Schema node |
|---|---|---|
| `String` | `String` | `type: string` |
| `Integer` | `Integer` | `type: integer`, any `format` |
| `Boolean` | `Boolean` | `type: boolean` |
| `Bytes` | `Bytes` | `type: array` of `uint8` integers, or `type: string` with `contentEncoding: base64` |
| `Optional<T>` | `Optional<T>` | a property missing from `required`, a `type` list containing `null`, or an `anyOf`/`oneOf` of one schema and `{type: null}`; `Optional<Optional<T>>` collapses to `Optional<T>` |
| `List<T>` | `List<T>` | `type: array` with `items` |
| `Map<String, V>` | `Map<K, V>` with `K` resolving to `String` | `type: object` with `additionalProperties` and no `properties` |
| `Struct` | a field list (an operation with no `input` or no `output` is the empty one), or a `struct` type | `type: object` with `properties`, or with neither `properties` nor an `additionalProperties` schema (an empty struct) |
| `Enum` | an `enum` type | `type: string` with `enum` (or `const`); a `null` in the `enum` list is nullability, not a variant; variants compared as a sorted set |

`$ref`s resolve into the root's `$defs`; a `newtype` resolves to the shape of its type. A node
outside this table (a `number`, a union of several non-`null` schemas, an object with
`additionalProperties: true` and no `properties`, whose values have no declared shape, a
recursive type) has no shape and is refused.
