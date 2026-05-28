# Release Stabilization

Use release stabilization when a growing change set needs a controlled path to
public release or a versioned package.

## Scope Lock

A scope lock states:

- included changes
- excluded changes
- allowed must-fix changes
- deferred ideas
- owner and review point

## Expansion Classifier

A new idea cannot enter release scope without classification.

```text
new idea
-> release blocker evidence exists? must fix
-> fixes broken in-scope behavior? adapt
-> promising but unproven? watch
-> useful but outside this cut? defer
-> unsafe, duplicated, too broad, or unsupported? reject
```

watch is the default for attractive ideas that lack failure evidence. The
classifier keeps the release cut from absorbing every good idea.

## Change Budget

The change budget should be small. Spend it only on:

- failing release gates
- public release blockers
- privacy or security findings
- broken install, provider smoke, or runtime checks
- documentation errors that would mislead a user or LLM

## Release Gates

Choose gates that match the changed surface:

- package validation
- lint and actionlint
- unit tests
- macOS local scenario
- Linux Docker scenario
- provider smoke
- privacy and git history review
- docs and output contract review
- regression check for recently fixed behavior

## Dogfood Evidence

`dogfood evidence` is different from scripted smoke output. It records whether a
real provider session can discover and use the skill or workflow in a realistic
task.

Good dogfood notes include:

- provider
- task prompt
- skill selected
- result quality
- failure or confusion
- follow-up fix

## Triage Labels

- `must fix`: blocks release readiness or user trust. `must fix requires`
  release blocker evidence such as a failing release gate, public release
  blocker, privacy or security finding, broken install, provider smoke failure,
  runtime failure, docs contract failure, or repeated dogfood failure.
- `adapt`: fixes broken in-scope behavior and has a verification path.
- `watch`: promising but unproven; keep as a tracked idea without implementation.
- `defer`: useful, but outside the current scope lock.
- `reject`: unsafe, too broad, duplicated, or outside supported scope.

## Ship Decision

The `ship decision` is one of:

- `ship`: gates pass and missing evidence is acceptable.
- `hold`: a must-fix item or required gate is missing.
- `continue`: scope is still moving and the release cut is not ready.
