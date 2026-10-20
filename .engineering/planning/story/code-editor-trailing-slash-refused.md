---
format: aep.planning-md/3
id: story:code-editor-trailing-slash-refused
kind: story
status: rejected
title: A trailing slash on a path that is not a directory is refused by the code-editor toolset (not planned)
relations:
- decomposes: epic:tool-contracts
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T14:20:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "rejected", at: "2026-10-20T14:20:13Z", actor: "human:timo", revision: 3}
---
## Outcome

Not planned. Asked for: a trailing `/` or `/.` on a path that is not a directory answers
`code.NotAFile` for every code-editor operation. Decided on 2026-10-20 by the approving session
(`b10x-chef-orchestrator`, under the owner's standing grant): no change to `tools`, nothing tagged.

## What was reported

A probe through harness 0.15.0 against harness 0.13.3: `code.write` to `link-in/` (a link to a file
inside the workspace, spelled with a trailing `/`) writes the target; `code.edit` of `plain.txt/`
and of `link-in/` is applied; 0.13.3 refused all three. Every target stayed inside the workspace.

## Why it is not built

- It is what the released contract specifies. `contracts/code-editor/v1alpha1/semantics.md:14-17`:
  a link that lands inside the workspace is followed, and "a trailing `/` or `/.` on a path names
  the same entry as the path without it (`link/` is `link`)".
  `contracts/code-editor/v1alpha1/scenarios/adversary-trailing-slash.yaml` requires `code.edit`,
  `code.patch` and `code.move` of `link/` on a link to a file inside to succeed.
- Harness 0.13.3's refusals were the operating system answering "Not a directory (os error 20)"
  to the spelling, not a rule of the toolset; wave t1 normalised the spelling to close a
  containment bypass the same spelling opened.
- Changing it fails the released suite, so it would be a new contract version beside `v1alpha1`
  (`AGENTS.md` § Rules), which costs every adopter a switch and buys no containment: the targets
  are inside the workspace either way.

## Revisit when

Somebody wants a trailing `/` refused as a rule. Then it is `code-editor` `v1alpha2`, with the
three cases above as its scenarios.
