# Provider Packaging

GroundLine ships one canonical source tree and three provider-facing install
surfaces:

- Codex marketplace package: `.agents/plugins/marketplace.json` points to
  `plugins/groundline`.
- Claude Code marketplace package: `.claude-plugin/marketplace.json` points to
  `plugins/groundline`.
- Antigravity plugin package: the repository root and `plugins/groundline` both
  expose `plugin.json` with the same package identity.

## Codex

Codex discovers installable plugins from configured marketplace snapshots. Use
the public repository as a marketplace source:

```bash
codex plugin marketplace add jukqaz/groundline --ref main
codex plugin list --marketplace groundline
codex plugin add groundline@groundline
```

For local development, add the checkout path instead:

```bash
codex plugin marketplace add .
codex plugin add groundline@groundline
```

The Codex package must keep `.codex-plugin/plugin.json`, `skills/`, `docs/`,
`references/`, `scripts/`, and `assets/` inside `plugins/groundline`.

## Claude Code

Claude Code supports direct plugin testing and marketplace installation.

Local session test:

```bash
claude --plugin-dir ./plugins/groundline
claude plugin validate ./plugins/groundline --strict
```

Marketplace install:

```bash
claude plugin marketplace add jukqaz/groundline
claude plugin install groundline@groundline
```

Inside the interactive UI, the same flow is available through `/plugin
marketplace add` and `/plugin install`.

Community directory submission is separate from direct install. Run
`claude plugin validate ./plugins/groundline --strict` before submitting.
Anthropic controls official and verified placement.

## Antigravity

Antigravity plugins are directories with `plugin.json` plus optional `skills/`,
`rules/`, MCP config, and hooks. GroundLine keeps `plugin.json` at the repository
root for direct install and in `plugins/groundline` for packaged installs.

Local validation:

```bash
agy plugin validate .
agy plugin validate ./plugins/groundline
```

Install options:

```bash
agy plugin install https://github.com/jukqaz/groundline
agy plugin install ./plugins/groundline
```

Use `agy plugin list` after install to confirm the package is imported.

## Publishing Rules

- Do not claim official listing until the provider catalog actually lists it.
- Keep provider manifests on the same version.
- Keep plugin paths relative to the plugin root.
- Keep all runtime references inside the packaged plugin directory.
- Do not add hooks or MCP servers by default; document them as opt-in work.
- Run the provider checks before release:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_pack.py --json
claude plugin validate ./plugins/groundline --strict
agy plugin validate ./plugins/groundline
```
