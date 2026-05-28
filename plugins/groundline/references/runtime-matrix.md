# Runtime Matrix

| Runtime | Manifest | Skills root | Notes |
| --- | --- | --- | --- |
| Codex | `.codex-plugin/plugin.json` | `skills/` | Plugin manifest points to `./skills/`. |
| Claude Code | `.claude-plugin/plugin.json` | `skills/` | Skills are invoked through the plugin namespace. |
| Antigravity | `plugin.json` | `skills/` | Root marker plus shared skills tree. |

GroundLine only targets these runtimes.

Keep runtime-specific manifests small and keep tool setup optional.

## Provider-Native Boundary

GroundLine should treat provider-native features as a capability boundary:

- do not reimplement runtime-specific affordances such as built-in goal modes,
  subagent runners, hook engines, plugin installers, MCP launchers, browser
  annotations, app snapshots, marketplace flows, or provider memory.
- express shared behavior as provider-neutral contracts: task packets, release
  cuts, side-effect boundaries, verification checklists, source registries, and
  skill curation decisions.
- document and verify whether a runtime can support the behavior, but keep the
  execution path owned by that provider.
- add setup recommendation text when a provider-native feature would help, not
  code that simulates the provider feature.

## Capability Classification

| Capability | GroundLine role | Provider role |
| --- | --- | --- |
| Skills | Provide shared SKILL.md content and output contracts. | Discover, load, namespace, and execute skills. |
| Plugins | Provide small manifests and package boundaries. | Install, share, update, and enforce plugin policy. |
| Subagents | Define task packets and handoff contracts. | Spawn, isolate, route, and summarize subagent work. |
| Hooks | Document reviewed hook policy and safe candidates. | Trigger hook engines and enforce provider permissions. |
| MCP | Recommend optional tool profiles. | Connect, authorize, scope, and run MCP servers. |
| Goal or long-running mode | Define goal, success criteria, and verification. | Continue execution across turns, hosts, or sessions. |
| Browser or desktop control | Define evidence needed and safety boundary. | Operate UI, annotate, screenshot, and enforce approvals. |
