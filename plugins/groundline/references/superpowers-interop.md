# Superpowers Interop

GroundLine does not replace Superpowers.

Use GroundLine to prove state, route the next step, guard side effects, and
close live work. Use Superpowers for planning, TDD, debugging, review, and final
verification gates.

Recommended flow:

```text
reconcile-current-state
-> Superpowers planning/debugging/TDD/review
-> guard-side-effects when mutation appears
-> close-live-work when runtime proof matters
-> Superpowers verification-before-completion
```

`close-live-work` supplies live evidence. `verification-before-completion`
guards the final success claim.
