---
format: aep.planning-md/3
id: story:fs-climb-through-missing-refused
kind: story
status: active
title: fs refuses a climb out of the workspace through a missing component as OutsideWorkspace
relations:
- decomposes: epic:tool-contracts
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-28T13:14:50Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-28T21:30:45Z", actor: "human:timo", revision: 4, executor: "agent:takt"}
---
## Outcome

`fs` refuses a path that climbs out of the workspace through a component that does not exist, such
as `missing/../../x`, as `OutsideWorkspace`, not `NotFound`, the same refusal `../x` gets; a
conformance scenario holds it and a mutant that keeps the lexical walk fails it.

## Context

Found by the 2026-10-21 cross-repository review (finding 5, `low`, `suspected`), confirmed by
reading `crates/tool-fs/src/lib.rs` (`resolve`, the `Err` arm): when `canonicalize` fails, the walk
to the deepest existing ancestor uses `Path::parent`, which is lexical. For `missing/../../x` every
ancestor under the root fails to canonicalize until the root itself, which is inside, so the answer
is `NotFound`. The real target lies outside the root. Nothing is read or written there, so only the
error class is wrong; a governor that counts escape attempts by `OutsideWorkspace` misses this one.
Not run yet: the claim rests on the code read.

## Scope

- `crates/tool-fs/src/lib.rs` (`resolve`)
- `contracts/fs/v1alpha2/scenarios/` (a new scenario; v1alpha1 stays unchanged)

## Acceptance

- A test in `crates/tool-fs/tests/` asks `fs` to read `missing/../../x`, where `x` exists beside the workspace root and `missing` does not exist, and gets `OutsideWorkspace`; `../x` still gets `OutsideWorkspace`, and `missing/x` inside the root still gets the not-found refusal it gets today.
- A new scenario under `contracts/fs/v1alpha2/scenarios/` holds the same refusal, and the conformance run of `crates/tool-fs/tests/conformance.rs` executes it.
