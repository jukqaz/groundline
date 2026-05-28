# Runtime Support

GroundLine targets three current agent runtimes.

## Codex

Codex uses `.codex-plugin/plugin.json` as the plugin manifest.

Expected shape:

```text
groundline/
├── .codex-plugin/plugin.json
└── skills/<skill-name>/SKILL.md
```

Starter prompt:

```text
Use GroundLine to reconcile current state before acting, then close with evidence.
```

## Claude Code

Claude Code uses `.claude-plugin/plugin.json` at the plugin root.

Expected shape:

```text
groundline/
├── .claude-plugin/plugin.json
└── skills/<skill-name>/SKILL.md
```

Local smoke:

```bash
claude --plugin-dir ./groundline
```

Invoke a skill such as:

```text
/groundline:reconcile-current-state
```

## Antigravity

Antigravity uses a root `plugin.json` marker file and can bundle skills in
`skills/`.

Expected shape:

```text
groundline/
├── plugin.json
└── skills/<skill-name>/SKILL.md
```

Starter prompt:

```text
Use GroundLine skills. Reconcile current state before acting, and separate read-only checks from mutation.
```

## Principles

- Keep skill bodies portable Markdown.
- Keep runtime-specific manifests small.
- Keep MCP, hooks, agents, and tool setup optional unless reviewed.
- Keep source configuration separate from runtime state.
