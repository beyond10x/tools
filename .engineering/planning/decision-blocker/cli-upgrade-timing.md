---
format: aep.planning-md/3
id: decision-blocker:cli-upgrade-timing
kind: decision-blocker
status: cleared
title: When this machine's aep, plugins and connectors move to their newest releases
summary: The upgrade runs in a gap between running waves, on the orchestrator's signal
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-23T11:41:11Z", actor: "human:timo", revision: 3}
---
## Question

On 2026-10-23 this machine ran `aep` 0.74.0 against a newest release 0.76.0 and the Beyond10x
plugins 0.23.2 against 0.24.0, and `connectors` was behind as well. When does this repository's
session upgrade them?

## The two answers on the table

| option | what it means for this repository |
|---|---|
| A | upgrade at once |
| B | upgrade in a gap between running waves |

## Who answers

The owner's orchestrating session, `b10x-chef-orchestrator`, which owns the machine's shared tools.

## Answer

Option B, decided by `b10x-chef-orchestrator` on 2026-10-23. `aep`, the plugins and `connectors`
live in `PATH` directories that waves running in other repositories call, so an upgrade during a
wave changes the tool under it. The orchestrator sends "upgrade now" in a gap between waves; this
session then runs the upgrade and reports each tool's old and new version, with `aep --version`
and `connectors --version` read back. ESS stays on 0.68.0 until 0.69.0 ships, because 0.68.0 is
the newest release.
