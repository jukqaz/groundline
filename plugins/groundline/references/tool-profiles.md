# Tool Profiles

GroundLine core has no hard MCP dependency and does not enable MCP servers
automatically.

## Core

- git
- rg
- local files
- Python 3

## Standard

- GitHub: PR, issue, CI, release, branch, and adoption evidence
- Context7: current library and runtime documentation lookup
- Exa: ecosystem discovery, naming checks, and research

## External Tool Probes

The Python scripts may call local tools when the operator opts in:

- `groundline_doctor.py --probe-tools`: read-only version probes for git, gh,
  docker, and curl
- `groundline_radar.py --command-sources`: local `command` sources in the
  registry, expressed as JSON arrays of command tokens

Rules:

- no shell string execution
- no network by default
- no mutation commands
- no secret-like output in reports

## Strict Local

- no external calls
- no network
- no secret access

Missing standard tools produce setup recommendations only.

## Provider-Native Tools

Provider-native tools stay outside GroundLine core. A provider-native tool is
documented as a runtime capability, not rebuilt in GroundLine. When Codex, Claude Code, or
Antigravity already provides a feature, GroundLine should emit a setup
recommendation or verification checklist instead of enabling or recreating it.

Examples:

- use the provider's plugin installer instead of writing one
- use the provider's hook engine instead of adding global hooks by default
- use the provider's subagent or long-running goal mode instead of building an
  orchestrator into GroundLine
- use the provider's MCP configuration after user approval instead of shipping
  always-on MCP servers

GroundLine may still probe local tools read-only, record missing capability, and
produce a provider-neutral task packet for the operator.
