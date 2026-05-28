# Workflow Modes

## companion-superpowers

Use when Superpowers is installed or explicitly selected.

GroundLine opens and closes the work. Superpowers handles planning, TDD,
debugging, review, and completion verification.

## standalone-groundline

Use when Superpowers is not present or not wanted.

```text
orient -> bound -> act -> prove -> handoff
```

## external-stack

Use when another workflow tool should receive the next step. GroundLine emits a
handoff packet with verified state, open risks, side-effect boundary, and the
next prompt.
