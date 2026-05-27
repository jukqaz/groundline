# Runtime Matrix

| Runtime | Manifest | Skills root | Notes |
| --- | --- | --- | --- |
| Codex | `.codex-plugin/plugin.json` | `skills/` | Plugin manifest points to `./skills/`. |
| Claude Code | `.claude-plugin/plugin.json` | `skills/` | Skills are invoked through the plugin namespace. |
| Antigravity | `plugin.json` | `skills/` | Root marker plus shared skills tree. |

GroundLine only targets these runtimes.

Keep runtime-specific manifests small and keep tool setup optional.
