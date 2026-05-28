# Language Policy

English is the default and canonical language for GroundLine documentation.

## Canonical English Surfaces

Keep these surfaces in English:

- `README.md`
- `docs/*.md` English originals
- `references/*.md`
- `references/skill-index.json`
- `skills/*/SKILL.md`
- provider manifests
- scripts, tests, and machine-readable output contracts

These files are read by LLM agents, validation scripts, provider tooling, or
release automation. Keeping them in one default language reduces drift.

## Korean Companion Docs

Korean docs are provided for human-facing reading and onboarding:

- `README.ko.md`
- `docs/ko/index.md`
- `docs/ko/human-guide.md`
- `docs/ko/install.md`
- `docs/ko/update.md`
- `docs/ko/examples.md`
- `docs/ko/skill-portfolio.md`
- `docs/ko/privacy.md`
- `docs/ko/release-checklist.md`
- `docs/ko/next-version.md`

Korean companion docs should explain the same operational intent as the English
docs, but they do not need to duplicate every line. Prefer concise guidance that
helps a human choose the right workflow quickly.

## Update Rule

When changing a human-facing English doc, check whether the Korean companion
needs one of these updates:

- link update
- command update
- changed safety boundary
- changed release or install step
- changed skill name or workflow order

LLM-readable references remain English unless a future release explicitly adds
structured multilingual metadata.
