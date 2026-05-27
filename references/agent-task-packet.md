# Agent Task Packet

Use a task packet when a request needs to survive handoff, resume, or
multi-agent execution without relying on hidden context.

## Required Fields

- `current conclusion`: what is currently true.
- `goal`: the actual outcome requested by the user.
- `context`: only the facts and files needed for this task.
- `constraints`: platform, provider, safety, style, and timing limits.
- `non-goals`: work that should not be done in this packet.
- `mutation boundary`: read-only, local write, provider-home write, external
  mutation, or production boundary.
- `success criteria`: observable results that prove the task is handled.
- `verification`: commands, checks, runtime probes, or review needed.
- `handoff`: exact next prompt or continuation note.

## Packet Rules

- Keep context smaller than the task.
- Prefer file paths, commands, and current evidence over narrative memory.
- Split broad requests into ordered packets when one packet would mix unrelated
  risk levels.
- State assumptions explicitly.
- Never include raw transcripts, credentials, tokens, or secret values.

## Quality Check

A good task packet lets another LLM answer:

- what should change
- what must not change
- what evidence proves completion
- where to continue if interrupted
