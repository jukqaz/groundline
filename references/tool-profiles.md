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
