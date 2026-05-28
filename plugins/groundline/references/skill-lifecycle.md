# Skill Lifecycle

GroundLine keeps skill metadata in two layers:

- Human-readable docs explain why a skill exists and how a person should read
  the portfolio.
- LLM-readable JSON gives agents stable fields for selection, curation, and
  validation.

Do not overload `SKILL.md` frontmatter with portfolio metadata. Keep skill
frontmatter focused on `name` and `description`; use
`references/skill-index.json` for taxonomy.

## Taxonomy

### workflow_stage

- `orient`: prove current state before acting.
- `research`: gather evidence and source-backed candidates.
- `compare`: score alternatives.
- `decide`: choose a boundary, recommendation, or next move.
- `act`: guide controlled execution.
- `verify`: prove quality, release readiness, or runtime behavior.
- `handoff`: preserve useful state for the next agent or person.
- `maintain`: keep GroundLine itself coherent over time.

### artifact_type

- `skill`
- `reference`
- `script`
- `agent`
- `hook`
- `mcp-recommendation`
- `docs`

### risk_level

- `read-only`: inspection or recommendation only.
- `local-write`: changes files in the active repo.
- `provider-home-write`: changes local provider configuration.
- `remote-write`: changes remotes, issues, pull requests, releases, or hosted
  state.
- `production`: touches deployed systems.
- `access-billing`: touches permissions, money, quotas, or account state.
- `secret-sensitive`: may encounter credentials or private content.

### provider_scope

- `provider-neutral`
- `codex`
- `claude-code`
- `antigravity`

### lifecycle

- `candidate`: proposed but not yet part of the active surface.
- `active`: supported and expected to work.
- `experimental`: usable but still being shaped.
- `deprecated`: retained for transition notes only.
- `merged`: folded into another skill or reference.
- `rejected`: intentionally kept out.

## Curation Actions

- `create`: add a focused skill for a repeatable judgment workflow.
- `adapt`: borrow a pattern but keep GroundLine lighter.
- `merge`: combine skills with the same trigger, stage, and output contract.
- `split`: separate a skill that mixes unrelated stages or risk levels.
- `deprecate`: keep a visible transition note before removing a surface.
- `reject`: leave a candidate out because it adds context load, unsafe setup,
  or unsupported provider scope.

## Human-readable Rules

- Put short intent, stage, risk, and lifecycle in `docs/skill-portfolio.md`.
- Use language a maintainer can scan without knowing provider internals.
- Explain why a skill exists, not just what file path contains it.
- Keep release-impact notes near the portfolio table.

## LLM-readable Rules

- Put exact `workflow_stage`, `artifact_type`, `risk_level`, `provider_scope`,
  and `lifecycle` fields in `references/skill-index.json`.
- Keep `llm_trigger` aligned with the skill frontmatter description.
- Keep `human_summary` short enough for compact reports.
- Update tests when adding or removing a skill.

## Merge, Split, And Deprecate Signals

- merge when two skills trigger on the same user wording and produce the same
  output contract.
- split when one skill requires unrelated evidence, tools, or risk boundaries.
- deprecate when a skill is replaced but existing release notes or users may
  still refer to it.
- reject when a candidate should remain a source registry item, reference note,
  or optional MCP recommendation.
