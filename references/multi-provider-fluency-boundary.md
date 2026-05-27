# Multi-Provider Fluency Boundary

GroundLine separates two assessment modes so agents do not claim automatic
conversation scoring when only artifact review was performed.

## Assessment Modes

`artifact-backed maturity` is the current GroundLine mode. It reviews durable
work artifacts such as repository diffs, docs, tests, validation logs, release
notes, handoff packets, automation configs, issue summaries, and explicit user
goals. It can include Codex, Claude Code, and Antigravity when those providers
produce artifacts or summaries. It does not require raw provider sessions.

`collector-backed fluency` is a future analyzer mode. It may inspect local
provider histories only after explicit approval, then convert source material
into a redacted evidence packet before scoring. It is the right mode for CEFR
style scoring, 24 behaviors, 12 sub-competencies, heuristic counts, and
questionnaire-backed evidence.

## Provider Coverage

- Codex: use local summaries, task packets, memory indexes, repository changes,
  and redacted session metadata before raw logs.
- Claude Code: use project sessions, Claude.ai exports, and repository evidence
  only when the user explicitly provides or approves the paths.
- Antigravity: use plugin, skill, task, and repository evidence first. Add
  history roots only after the local layout is confirmed.

Every report must say which providers were included, excluded, or not found.

## Raw Transcript Boundary

The raw transcript boundary is strict:

1. no automatic collection by default.
2. Build an inventory before opening history content.
3. Redact secrets, auth material, prompt logs, personal paths, and private
   provider state before scoring.
4. Store only a redacted evidence packet when possible.
5. Keep source excerpts short and task-relevant.
6. Require explicit approval before scanning provider homes or writing reports.

GroundLine must never treat a full raw message scan as the default assessment
path. Start with inventory-only discovery, then ask for approval before any
content review. Prefer a redacted summary unless a narrow approved excerpt is
needed.

## Provider Evidence Packet

A Provider Evidence Packet is the highest-detail input GroundLine should
request before any collector-backed fluency review. It is provider-neutral,
redacted, and scoped to the user's approved time window.

```text
Provider Evidence Packet:
- provider:
- time window:
- projects touched:
- task types:
- autonomy level:
- tools used:
- verification evidence:
- handoff evidence:
- repeated failure patterns:
- approved excerpts: none|redacted
- privacy boundary:
```

Packet rules:

- Use no secret values.
- Use `approved excerpts: none` unless the user explicitly approves a short,
  task-relevant redacted excerpt.
- Record whether the packet came from inventory-only metadata, artifacts, or
  approved content review.
- Prefer a redacted summary over source text.
- Keep one packet per provider so coverage gaps stay visible.

## Behavior Extraction

The behavior extraction step starts only after source material has been reduced
to a provider-neutral, redacted evidence packet.

Behavior extraction should turn provider-specific records into provider-neutral
facts:

- task framing signals
- delegation and handoff signals
- format, audience, tone, and constraint signals
- revision, pushback, or error-catching signals
- verification, safety, and accountability signals
- automation and reuse signals

Counts are not enough. A high count of a behavior should be checked against
quality, range, and whether the behavior improved outcomes.

## Output Rule

If no collector ran, say `evidence mode: artifact-backed maturity` and do not
claim CEFR scoring. If a collector ran with user approval, say `evidence mode:
collector-backed fluency`, list provider coverage, and attach the redacted
evidence packet path or summary.
