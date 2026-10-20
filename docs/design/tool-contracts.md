# Tool contracts

Status: draft, 2026-10-19. Decided by the owner on 2026-10-19: tool contracts and their default
implementations live in this repository (`contracts/`, `crates/tool-<toolset>`); atlas ADR 0095
records it.

## Problem

Each agent harness in the organization re-implements the tool layer. The inventory
([../inventory.md](../inventory.md)) counted, across the beyond10x repositories and earlier private
agent projects: 14 independent file readers, 15 shell or process runners, 13 delegation layers, 9
tool meta layers, 8 web fetchers, 6 git layers, 16 vocabularies for effects, risk and approval, and 5
hand-written MCP JSON-RPC servers. No contract file is consumed by two programs. The same operation
has five names (`file_read`, `file.read`, `Read`, `show`, `code_read`), and the properties a governor
needs (what a call may touch, whether it can be retried, how long it may run) are declared in some
layers, inferred in others and absent in most.

## Model

A **tool contract** is an ESS command plus a `tool-contract/1` profile.

The ESS command gives the operation's input, its output, its outcomes and its errors. The profile
gives what a harness, a governor and a reviewer need to know about the operation without running
it. Each property below is taken from the best existing form the inventory found:

| property | meaning | taken from |
|---|---|---|
| `effects` | read, write, process, network, secret, external, human-visible | P1 `Effect`; platform module-core |
| `consequences` | money, delete, send-external, irreversible, sensitive-data | P1 semantic effects; P2 semantics |
| `risk` | none, low, medium, high, critical; checked against `effects` by the profile validator | P1 tool-spec invariants |
| `idempotency` | idempotent; keyed by a caller `operation_id`, with a reconcile operation; non-idempotent | substrate; platform module-core |
| `preconditions` | `expected_sha256`, `expected_revision` | workspace; entity-runtime |
| `determinism` | deterministic or not, for replay and caching | P2 semantics |
| `bounds` | timeout default and maximum, output bytes, processes, cpu, memory; required and capped | substrate; P3 timeout middleware |
| `paging` | offset, limit, digest of the whole, `truncated` | substrate; epistemic-knowledge-runtime |
| `requires` | binaries, environment variables, settings, secrets | declared nowhere today |
| `subjects` | the input fields that name paths, URLs or argv, and the permission patterns they suggest | P4; P5; P1 |
| `placement` | local, confined, container | P1 placement; substrate |
| `capability` | the Canon capability the operation serves (ADR 0083) | platform provider driver; takt protocols |
| output | typed result blocks; canonical content separate from the model's view; `untrusted` marking; failure returned as a result | P1; P3; epistemic-knowledge-runtime; platform |
| presentation | guidance separate from description; tags; aliases; group activation | P3; P6; P1 |

Names are dotted family ids: `fs.read`, `fs.list`, `fs.glob`, `code.write`, `code.edit`,
`code.patch`, `search.grep`, `process.run`, `process.start`, `web.fetch`, `web.search`, `git.diff`,
`agent.spawn`, `tool.search`. Claude, Codex and MCP names are renderings generated from a table in
the contract, never the source.

## Toolsets

| toolset | operations (first version) |
|---|---|
| `fs` | read (paged, with whole-file digest), list, glob, stat |
| `code-editor` | write, edit with a preimage digest, patch (several edits resolved against the original), move, delete |
| `process` | run (argv only, bounded; the default implementation confines no network or file access, so `process.run` v1alpha1 declares those effects, and a confined run is a narrower contract), shell (gated), start, output, wait, signal |
| `search` | grep, code search |
| `web` | fetch (SSRF guard, body cap), search, crawl |
| `git` | status, diff, log, stage, commit, branch, worktree |
| `forge` | pull request open and read, CI read, merge |
| `build-test` | toolchain runs (cargo, go, npm, pytest, task) |
| `browser` | open, snapshot, act, close |
| `code-intel` | definitions, references, outline, diagnostics |
| `delegation` | spawn, await, message |
| `memory` | remember, recall, forget |
| `interaction` | ask the user, notify |
| `tool-meta` | search, describe, invoke, activate |
| `control` | turn done, plan and todo |

`fs` and `code-editor` come first: every harness has them, and they exercise most profile
properties (subjects, preconditions, paging, idempotency).

## Conformance

Each toolset version ships scenarios and fixtures. "Implements `fs` v1alpha1" means passing them.
Every profile property has at least one scenario that fails when an implementation ignores it (a
mutant that drops the bound, the precondition or the truncation flag fails a scenario). An
implementation derives its schemas from its types; the suite checks the derived schema equals the
contract's.

## Default implementations

`crates/tool-<toolset>`, Rust. Seeds, best first per toolset: the P1 tools (fs, code
editing, process, git, web), substrate (confined process and file operations), harness-tools. A
toolset digest over the sorted operations and a frozen-contract digest per published workflow make
drift visible.

## How harnesses adopt it

- A protocol action grants a Canon capability; a harness offers exactly the tools whose contracts
  serve granted capabilities ("offered" vs "admitted" kept apart). Commission binds and invokes
  (ADR 0082, 0083).
- Grants can select tools by declared facts (maximum risk, effects within a set) instead of names.
- First adopters: harness (fs and code-editor from this repository), metaharness (vendor renderings
  from the contracts), takt (`--allowedTools` derived from capabilities). Then loom, conductor,
  agentide, agent-platform, devcenter and substrate map to the contracts in their own waves.

## Phases

| phase | work |
|---|---|
| T0 | this repository, its store, atlas ADR 0095 and catalog entry |
| T1 | the `tool-contract/1` profile and its validator; `fs` and `code-editor` contracts with suites; `crates/tool-fs`, `crates/tool-code-editor` passing them; `crates/tool-toolchain` from harness-toolchain |
| T2 | harness, metaharness and takt adopt T1 |
| T3 | the remaining toolsets in the order above; the other adopters |

## T1 in detail (2026-10-20)

Decided by the owner on 2026-10-20 (atlas ADR 0097): harness is kept, and `harness-tools`
(6,049 lines) and `harness-toolchain` (1,952 lines) move into this repository as crates under the
`tool-contract/1` profile; harness and metaharness then consume them. T1 therefore starts from
harness's code, not from empty crates. Every crate records the harness file it came from in its
crate documentation and in the table below.

### The contract document

`contracts/<toolset>/<version>/contract.yaml` is one `tool-contract/1` document. Its commands are
ESS-shaped (the same `input`, `outcomes`, `errors` and type vocabulary an ESS domain uses) so that a
later version can be lifted into an ESS domain unchanged; see *ESS validation* below for why it is
not an ESS domain today.

```yaml
format: tool-contract/1
toolset: fs                       # the family; every operation is named <toolset>.<op>
version: v1alpha1
summary: one paragraph
types:                            # ESS-shaped named types, used by input, output and errors
  - name: fs.Kind
    kind: enum
    variants: [file, directory, symlink, other]
errors:                           # ESS-shaped: a name and typed fields
  - name: fs.NotFound
    summary: ...
    fields: [{name: path, type: String}]
operations:
  - name: fs.read
    summary: one line a reader chooses between operations by
    guidance: how to call it well; separate from summary (presentation)
    renderings: {claude: Read, codex: read_file, mcp: fs_read}   # generated names, never the source
    input:                        # ESS field list; types: String, Integer, Boolean, Bytes,
      - name: path                #   Optional<T>, List<T>, Map<K, V>, a named type
        type: String
        description: ...
    output:                       # the same field list shape
      - name: text
        type: String
    outcomes:                     # every error an operation can return is an outcome naming it
      - name: read
        summary: ...
      - name: not-found
        error: fs.NotFound
    profile:
      effects: [read]                                     # read write process network secret external human-visible
      consequences: []                                    # money delete send-external irreversible sensitive-data
      risk: low                                           # none low medium high critical; never under the floor its effects set
      idempotency: idempotent                             # idempotent | keyed | non-idempotent
      preconditions: []                                   # field names of the input that are preconditions
      determinism: deterministic                          # deterministic | nondeterministic
      bounds:                                             # every bound names default and max; max is required
        timeout_ms: {default: 10000, max: 60000}
        output_bytes: {default: 65536, max: 262144}
      paging: {offset: offset, limit: limit, truncated: truncated, digest: sha256}   # input, input, output, output fields
      requires: {binaries: [], env: [], settings: [], secrets: []}
      subjects:                                           # input fields that name what a call touches
        - {field: path, kind: path, pattern: "fs.read({path})"}
      placement: local                                    # local | confined | container
      capability: workspace.read                          # a Canon capability id: dotted lowercase segments
      output: {untrusted: true, failure_as_result: true}
```

The profile's rules, which the validator enforces and `contracts/profile/tool-contract-1.md`
states in full:

| rule | refused when |
|---|---|
| risk floor | `risk` is under the floor set by effects and consequences: read→none, network→low, write→low, human-visible→low, process→medium, secret→medium, external→high; delete→medium, sensitive-data→medium, irreversible→high, money→high, send-external→high |
| bounds | a bound lacks `max`, or `default` exceeds `max`, or `timeout_ms` is missing |
| subjects | a subject names an input field that does not exist |
| preconditions, paging | a named field does not exist on the side (input or output) the property reads |
| capability | not `[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)+` |
| names | an operation is not `<toolset>.<op>`, or two operations share a name, or a rendering is reused |
| outcomes | an error an outcome names is not declared, or a declared error no outcome names |
| types | a field names a type that is neither primitive nor declared |

Each rule names the operation and the property in its refusal.

### The conformance suite

`contracts/<toolset>/<version>/scenarios/*.yaml` are `tool-conformance/1` documents. A scenario
lays out a fixture tree, calls one operation and checks the result and the tree afterwards:

```yaml
format: tool-conformance/1
toolset: fs
version: v1alpha1
scenarios:
  - name: a-window-is-answered-with-the-whole-files-digest
    holds: [paging, bounds]            # the profile properties this scenario is evidence for
    fixture:
      files: {a.txt: "one\ntwo\nthree\n"}   # or dir: fixtures/<name>
    call: {operation: fs.read, input: {path: a.txt, offset: 2, limit: 1}}
    expect:
      outcome: ok                          # or error: fs.NotFound
      output:                              # subset match; matchers: {$sha256_of: a.txt}, {$any: true},
        lines: {from: 2, to: 2, total: 3}  #   {$contains: text}, {$min_len: n}, {$absent: true}
        truncated: true
        sha256: {$sha256_of: a.txt}
      files: {a.txt: unchanged}            # unchanged | absent | {equals: text} | {sha256: hex}
```

Every behavioural profile property an operation declares (`effects`, `idempotency`,
`determinism`, `bounds`, `paging`, `preconditions`, `subjects`, `output`) has at least one
scenario whose `holds` names it; the runner refuses a suite where one has none. The declarative
properties (`risk`, `consequences`, `requires`, `placement`, `capability`) are the validator's
alone. Each implementation crate also runs the suite against
its own **mutants** (a build of the implementation with one property switched off, behind
`#[cfg(any(test, feature = "mutants"))]`) and asserts that each mutant fails at least one
scenario. That is how "a scenario that fails when an implementation ignores it" is a test rather
than a sentence.

### The crates and the runner

| crate | package | holds |
|---|---|---|
| `crates/tool-contract` | `b10x-tool-contract` | the profile types, the document parser, the validator, the `tool-conformance/1` runner, the schema shape check, the `tool-contract` binary (`validate`; suites run inside each toolset crate's tests) |
| `crates/tool-fs` | `b10x-tool-fs` | `fs.read`, `fs.list`, `fs.glob`, `fs.stat` over one workspace root |
| `crates/tool-code-editor` | `b10x-tool-code-editor` | `code.write`, `code.edit`, `code.patch`, `code.move`, `code.delete` over one workspace root |
| `crates/tool-toolchain` | `b10x-tool-toolchain` | the declarative toolchain registry (rust, go, taskfile, npm, yarn) and the `build-test` operations it derives |

An implementation exposes typed input and output structs (`serde` + `schemars`) and one
`tool_contract::Toolset` implementation:

```rust
pub trait Toolset {
    fn contract(&self) -> &Contract;                         // the parsed document it claims
    fn schemas(&self) -> Vec<(String, Schema, Schema)>;      // per operation: derived input, output
    fn invoke(&self, operation: &str, input: &Value) -> Outcome;   // Ok(Value) | Error { name, message, fields }
}
```

The runner (`tool_contract::conformance::run(&suite, &contract, scratch_root, |workspace| make_toolset(workspace))`;
`contracts/profile/tool-contract-1.md` is the authority for the exact signature) creates one
workspace per scenario under the caller's scratch root, lays out the fixture, invokes, matches, and
reports one line per scenario plus a summary with exit status. The schema check maps
both the contract's field lists and the `schemars`-derived schemas to one canonical shape
(`String`, `Integer`, `Boolean`, `Optional`, `List`, `Map`, `Struct`, `Enum`) and refuses on the
first difference, naming the operation, the side and the field.

### Operation sets

| operation | input | output | from harness |
|---|---|---|---|
| `fs.read` | `path`, `offset?`, `limit?`, `max_bytes?` | `path`, `bytes`, `text` (numbered lines), `lines{from,to,total?}`, `truncated`, `truncated_lines`, `sha256?`, `lines_counted_to?`, `note?` | `local.rs` `read`, plus the whole-file digest |
| `fs.list` | `path?` | `path`, `entries[{name,kind,bytes?}]`, `truncated` | `local.rs` `list` |
| `fs.glob` | `glob`, `path?`, `max_results?` | `paths`, `truncated`, `depth_bound_reached` | `local.rs` `find_paths`, `walk`, `PathGlob` |
| `fs.stat` | `path` | `path`, `kind`, `bytes?`, `sha256?` | new |
| `code.write` | `path`, `text`, `expected_sha256?` | `path`, `bytes`, `sha256` | `local.rs` `write`, `resolve_new` |
| `code.edit` | `path`, `old`, `new`, `expected_sha256?` | `path`, `bytes`, `sha256`, `diff` | `local.rs` `edit` (exactly one match) |
| `code.patch` | `path`, `edits[{old,new}]`, `expected_sha256?` | `path`, `bytes`, `sha256`, `diff`, `applied` | new; every edit resolved against the original, overlaps refused |
| `code.move` | `from`, `to` | `from`, `to` | new |
| `code.delete` | `path`, `expected_sha256?` | `path` | new |

`sha256` is the digest of the **whole** file; on a read whose line scan stopped at the 16 MiB
counting bound it is `null`, like `lines.total`. A precondition that does not match refuses with
`code.PreconditionFailed{expected, actual}` and the file unchanged.

### Provenance

| harness file | lines | goes to |
|---|---|---|
| `crates/harness-tools/src/local.rs` | read, list, find, walk, line reader, containment | `tool-fs` |
| `crates/harness-tools/src/local.rs` | write, edit, `resolve_new` | `tool-code-editor` |
| `crates/harness-tools/src/local.rs` at harness 0.14.1 | `exec`, `terminate`, `drain`, `read_ends`, `keep_both_ends`, output caps, `INHERITED_ENV` | `tool-process` (wave w2) |
| `crates/harness-tools/src/local.rs` at harness 0.14.1 | `grep`, `search_expression`, `around`, `cut_to`, the search constants | `tool-search` (wave w3), walking with `tool-fs`'s public walk |
| `crates/harness-tools/src/toolchain.rs` at harness 0.14.1 | `programs`, `entry_names`, `plan` | `tool-toolchain` `executor` (wave w3) |
| `crates/harness-tools/src/catalogue.rs` at harness 0.14.1 | `invoke_toolchain` | `tool-toolchain` `executor::Executor::run` (wave w3) |
| `crates/harness-tools/src/catalogue.rs`, `verbs.rs`, `flat.rs`, `scope.rs`, `toolchain.rs`, `operations.rs` | the publication over `harness-wire` | stays in harness; T2 rewires it over these crates |
| `crates/harness-toolchain/src/lib.rs`, `builtins/*.yaml` | all | `tool-toolchain` |

### ESS validation

`ess specify validate` (ess 0.60.0) refuses a command whose success outcome emits no event
(`ESS-COMMAND-007 empty_change`) and a command whose outcomes carry no `when` (`ESS-COMMAND-004
conflicting_declaration`); both were observed on 2026-10-20 against a one-command probe domain. A
tool operation's outcome is decided by the filesystem, not by its input, and emits no domain event,
so the T1 contract documents are validated by `tool-contract validate`, which reads the ESS field
and type grammar itself. Lifting the documents into ESS domains, with events per operation, is a
separate story once ESS can express an outcome decided outside the input.
