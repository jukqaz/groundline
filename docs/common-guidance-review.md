# Shared guidance review and Codex alignment

Reviewed 2026-10-11. This is a source review record, not another task workflow or
an instruction to install external plugins. Native model behavior remains untested.

## Scope and boundary

The review covered all three Core skill entries and all fifteen Core reference
documents, their UI metadata and plugin entry points. GroundLine does not ship a
fixed global AGENTS template: environment baselines hold owner-selected registered
content. The repository's own `AGENTS.md` remains project-local.

| Surface | Decision |
| --- | --- |
| Shared execution | Keep user intent, scoped authority, existing work, independent ownership, valid verification reuse and evidence-based completion |
| Project behavior | Keep features, architecture, compatibility, test commands, release policy and presentation conventions in the owning project |
| Model and delegation | Remove dated recommendation tables, model/effort-by-role examples, fixed agent counts and visual-production sequencing; use current native support and authorized task judgment |
| Instruction loading | Keep entries focused on triggers and route selection; read the selected entry and only necessary references; use the native instruction chain |
| Adaptation | Keep observations and learned changes within their applicable project/target; a correction from one project is not a cross-project default |
| Verification | Choose the project's appropriate check and inspect the actual result; distinguish invocation/environment/product failures and test a focused hypothesis |
| Product contracts | Preserve CLI schemas, empirical comparison limits, platform artifacts, installation/recovery, privacy and Insights consent contracts for their named operations |

No framework, language, test-first methodology, release mode, fixed model pair,
new scheduler or generic execution controller was adopted. Unchanged Core
references retain conditional native capabilities and GroundLine product contracts.
The Insights metadata hook count was corrected to the existing five-hook contract;
no hook definition or runtime behavior changed.

## External patterns considered

These are original adaptations of design principles, not copied skill text or
third-party code. Sources were read on 2026-10-11; remote default branches can move.

| Primary source | Adopted principle | Kept within its original domain |
| --- | --- | --- |
| [Official Codex skill authoring](https://learn.chatgpt.com/docs/build-skills) | Clear early triggers, focused entries, conditional reference loading and instruction-first workflows | Native discovery and explicit/implicit invocation remain host-owned |
| [Official Codex AGENTS discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md#how-codex-discovers-guidance), [AGENTS.md project](https://github.com/agentsmd/agents.md) | Keep guidance in its owning scope and follow native precedence | Repository example package-manager commands and PR rules stay examples |
| [OpenAI plugin examples](https://github.com/openai/plugins), [macOS build/run skill](https://github.com/openai/plugins/blob/main/plugins/build-macos-apps/skills/build-run-debug/SKILL.md), [frontend testing skill](https://github.com/openai/plugins/blob/main/plugins/build-web-apps/skills/frontend-testing-debugging/SKILL.md) | One authoritative reference for a contract; verify the result actually requested on the actual native surface | Build scripts, browser preference, viewport checks and report templates are specialist/project choices |
| [Superpowers writing skills](https://github.com/obra/superpowers/blob/main/skills/writing-skills/SKILL.md) | Conditional wording, concise entry points and relevant positive/negative behavior cases | Mandatory test-first skill authoring, subagent scenarios and workflow-wide gates were not adopted |
| [Superpowers verification](https://github.com/obra/superpowers/blob/main/skills/verification-before-completion/SKILL.md), [systematic debugging](https://github.com/obra/superpowers/blob/main/skills/systematic-debugging/SKILL.md) | Bind claims to the appropriate evidence and investigate a focused cause before speculative repair | Mandatory full fresh reruns for every message and fixed retry/architecture gates were not adopted |

The [old OpenAI skills catalog](https://github.com/openai/skills) marks itself
deprecated and points to the plugin examples repository. It was used to locate
the current source, not selected as an installation target.

## Verification boundary

Use the existing guidance structural suite and source verifier for metadata,
links and product boundaries. The review and those checks do not establish skill
trigger behavior, model quality, savings, installed activation or release readiness.
The project-neutral cases in [guidance validation](guidance-validation.md) retain
those behavioral acceptance requirements without creating new tasks or paid runs.
