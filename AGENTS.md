# GroundLine Codex Project Guidance

- Prove the current checkout, runtime, and configuration before changing them.
- Preserve user-owned WIP and use bounded edits.
- Maintain one current GroundLine contract. Do not add backward-compatibility
  adapters, deprecated aliases, or silent downgrades for retired formats.
  Reject unsupported state explicitly without rewriting or deleting it. Keep
  data-loss protections, bounded delivery retries, and current native input
  variants; these are not legacy support. A state reset needs separate approval.
- Use `$groundline:reconcile-current-state` for stale or resumed work and before
  non-trivial broad, ambiguous, current-fact-dependent, or high-impact changes.
  Use `$groundline:close-live-work` when deployed or user-visible proof matters.
- Treat Codex App and its bundled CLI as the primary runtime. Use PATH CLI for
  packaging, automation, and secondary-channel evidence.
- GroundLine ships Core and Insights plugins/CLIs plus the Insights API. Keep
  connection and collection in the shared runtime; no separate Desktop GUI.
- Use a light preflight for clear bounded work. Before writing for broader work,
  collect targeted evidence, synthesize once, and settle scope, mutation boundary,
  success criteria, and verification. Proceed when the next action is clear;
  these checks are not an extra approval gate or mandatory report template. Do not
  require a Goal unless the user explicitly requests one. Defer non-blocking
  later ideas instead of restarting implementation. Incorporate explicit user
  steering into the same task when its outcome and authority remain compatible.
- Reviews are read-only unless a fix is requested. Existing approval covers
  routine work inside its stated boundary; do not ask again for each local step.
  Ask for new authority only when scope or external effects materially change.
- Use other GroundLine skills only when their explicit workflow matches the
  request; ordinary planning and implementation stay Codex-native.
- Spawn subagents only when the user explicitly asks for delegation, parallel
  work, or subagents. Give each agent one independent lane and a compact return
  contract.
- Use Codex built-in agents and any user-owned specialist agents directly;
  GroundLine does not install or override custom agents.
- Leave model and reasoning effort unpinned unless the user explicitly requests
  a setting. The documented `setup` preset is a separate opt-in configuration
  operation, not a prerequisite for ordinary guidance review or repair.
- Chronicle may provide redacted behavior-boundary counts when the user allows
  it. Do not change Chronicle state, automation, or experiment ledgers.
- Run the smallest credible verification and report source, package, install,
  and live state separately.
- Repeat passed checks only after relevant changes or unresolved risk. Diagnose
  unchanged failures before retrying; distinguish code and infrastructure errors.
- Use `cargo test --locked -p <affected-crate>` for bounded Rust behavior changes;
  `cargo run --locked -p xtask -- verify-source --root . --json` checks plugin
  metadata, source boundaries, and CI. Use `docs/release-checklist.md` only for
  release qualification. Instruction semantics need separate behavioral evidence.
- 문자열 파서를 새로 만들거나 수정할 때 현재 표준 라이브러리와 유지보수되는
  crate를 검토한다. 실제 결함·호환성·필요한 feature·보안 검증을 근거로 선택하고,
  회귀 입력과 적용 이유를 남긴다. 기준은 `docs/research/string-processing-crates.ko.md`다.
- Skill guidelines support user intent under higher-priority instructions and
  permissions. Name the exact skill instruction if it causes a pause or changes
  direction. Omit empty process fields from user-facing reports.
