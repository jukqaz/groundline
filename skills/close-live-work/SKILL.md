---
name: close-live-work
description: Use when local tests or CI appear complete but dev, review, staging, production, app store, worker, queue, scheduler, or browser-visible runtime state must be proven before saying deployment or release work is done.
---

# Close Live Work

## Purpose

Use this skill when local checks or CI success are not enough. The task is only closed when the target runtime or externally visible surface proves that the intended version or behavior is live.

## Trigger Examples

- "배포 끝났냐?"
- "CI 됐으면 개발서버도 반영됐는지 봐."
- "마무리해, live까지 확인해."
- "PR merge됐는데 앱/서버가 실제로 바뀌었는지 확인해."
- "workflow success 말고 endpoint 기준으로 봐."

## Workflow

1. Identify the target surface: local, dev, review, staging, prod, app store build, remote host, worker, queue, scheduled job, or browser-visible URL.
2. Establish the expected version or artifact: commit SHA, tag, image digest, package version, migration version, app build number, or API schema version.
3. Check the pipeline state: relevant CI jobs, release jobs, deployment jobs, artifacts, logs, and pending queues.
4. Probe the live runtime:
   - health endpoint
   - version endpoint or docs artifact
   - systemd or process state
   - worker or queue state
   - smoke command
   - browser or mobile smoke when the regression is user-facing
5. Distinguish pipeline success from live success. A successful workflow does not prove the target endpoint has changed.
6. Report `PASS`, `PARTIAL`, or `FAIL` with exact evidence and the next blocking item.

## Closure Matrix

| Target | Minimum evidence |
| --- | --- |
| API/server | health endpoint plus version or behavior probe |
| Web app | deployed asset/version plus browser smoke for user-facing regressions |
| Worker/queue | deployed revision plus queue processing or worker log evidence |
| Scheduled job | unit/timer/cron state plus latest run evidence |
| Mobile release | build number plus install/simulator/device or store-track evidence |
| Systemd service | `systemctl show` result plus logs or smoke command |

## PASS/PARTIAL/FAIL Rules

- `PASS`: Expected artifact is live and the relevant smoke passes.
- `PARTIAL`: Some pipeline or runtime layer is good, but a live proof is missing or stale.
- `FAIL`: The target runtime serves the wrong artifact, smoke fails, or deployment did not happen.

## With Superpowers

Use `superpowers:verification-before-completion` for the final status claim. This skill supplies the live/runtime proof that the final verification should cite.

## Common Mistakes

- Saying "deployed" because CI passed.
- Missing delayed release workflows that publish after CI.
- Treating a `systemctl status` inactive oneshot as failure without checking `Result` and `ExecMainStatus`.
- Verifying the backend only when the user-facing regression crosses backend and frontend.

## Output Contract

```text
Status: PASS / PARTIAL / FAIL
Expected artifact:
- ...
Evidence:
- CI/release: ...
- Runtime: ...
- Endpoint/smoke: ...
Gaps:
- ...
Next action:
- ...
```

Use Superpowers verification-before-completion for general success claims. Use this skill for live or runtime closure specifically.
