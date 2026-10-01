# Development contracts

Use [Contributing](../CONTRIBUTING.md) for build checks and
[architecture](architecture.md) for ownership. Dependency versions and features
belong in Cargo.toml/Cargo.lock; historical investigations are not current setup
instructions. Support macOS and Linux on ARM64 and x86_64.

## Parsing and dependencies

Reproduce the failing input before changing a parser. Prefer standard-library
operations for a small fixed grammar and maintained format parsers for structured
input. Add a dependency only for a demonstrated requirement; verify supported
features, target compatibility, licenses, and advisories. Keep parsers out of
runtime features that do not need them.

| Input | Existing implementation | Boundary |
| --- | --- | --- |
| JSON | Serde, `serde_json`, `serde_path_to_error` | Typed validation; errors expose known field categories, not input values |
| TOML | `toml_edit` | Preserve unrelated keys/comments and verify allowed semantic changes |
| YAML | `serde-saphyr` | Explicit metadata and duplicate-key validation |
| Markdown | `pulldown-cmark`, `percent-encoding` | Decode real links/images; ignore code examples; reject package escapes and symlinks |
| Rust source | `syn`, `proc-macro2` | Exact spans for narrowly allowed source-scanner exceptions; malformed input gets no exception |
| JavaScript tool calls | Oxc AST | Bounded static calls and literal arguments, never execution or general control-flow inference |
| Shell command classification | `shlex` plus a narrow prefix grammar | Classify executable position, not command-like text inside search arguments |
| Secret storage | `secrecy`, `zeroize`, private file helpers | No secret-bearing values in errors or reports |

Checksums have a fixed byte format: expected SHA-256, two spaces, exact executable
name, and at most one LF/CRLF ending. Compare that format directly; reject added
lines, spaces, BOMs, names, and hashes rather than introducing a general parser.

Keep license exceptions package/version scoped in `deny.toml`. Do not broaden
license or advisory policy to make a dependency pass. Preserve required transitive
dependencies even when they target an unsupported operating system.

## Configuration and local files

`config-repair` previews by default and requires the preview's source-bound plan
for `--apply`. `setup` also previews without `--apply`, but an authorized direct
`setup --apply` needs no separate preview or repair plan. Existing-file changes
use a private original backup and a last-moment source check; new configuration
uses exclusive creation. Both paths are idempotent. Preserve user comments and
unrelated values; update existing TOML values without replacing key formatting.
Refuse unsupported profiles/providers/catalogs without fallback. Explicit
model/effort choices must match the current native catalog; setup preserves choices
by default. See [configuration review](../plugins/groundline/references/codex-configuration.md).

Reuse one validated catalog during an operation. Error paths must not print model
names, dynamic map keys, original parser errors, credentials, or private paths.
Exercise quoted keys, comments, CRLF, malformed input, backup byte equality,
concurrent edits, symlinks, and a second no-op application.

Use the shared Unix file helpers for bounded regular-file reads, owner/private
mode checks, exclusive creation, fsync, and rename. Do not weaken permissions or
replace a bounded failure with unlimited retries.

## Audit classification

Bound scanning and retained memory independently. Stream large tool outputs with
`RawValue`/`DeserializeSeed`; skip bodies that are not needed for classification.
Preserve unknown, incomplete, and lower-bound observations rather than inventing
zero usage or successful termination.

Interpret native status/exit metadata and bounded native text headers, not words
such as “failed” or “timeout” in stdout. Link polls only through explicit handles
within the same rollout and observation window. Static JavaScript output count and
order must match the result envelope; missing, conflicting, or dynamic results
remain unresolved. Never expose internal handles or infer completion from a later
unrelated tool result.

Shell classification allows only its explicit literal grammar. Cargo-specific
`CARGO_HOME`/`CARGO_TARGET_DIR` prefixes are checked before quote information is
lost. Substitutions, pipelines, conditional execution, dynamic IDs, and unsupported
batch forms stay unclassified. Test original/projected record equivalence and
search strings that look like executable test commands.

The current limits and parser cases live in the contracts' audit modules and
runtime rollout reader. [Weekly audit](../plugins/groundline/references/weekly-usage-audit.md)
defines public coverage semantics. Whole-store diagnostics are a separate request;
audit changes must not rewrite native history, consent, cursors, or server events.

## Performance evidence

Use synthetic local inputs for parser benchmarks:

```console
cargo bench --locked -p groundline-cli --bench config_catalog -- --sample-count 50 --sample-size 1
cargo run --locked -p groundline-contracts --example audit_benchmark
```

Compare the same machine, build profile, input, and aggregate fingerprint. Record
allocations and timing separately. These benchmarks do not measure provider tokens,
billing, model quality, installed-plugin latency, or production deployment.
Use [guidance validation](guidance-validation.md) for behavioral evidence and the
[release checklist](release-checklist.md) only when qualifying a release.

Model IDs use bounded standard-library byte checks for local selection syntax.
Regression inputs include `gpt-6.1-sol`, unknown 6.x tiers, future-generation Sol,
and unconfirmed snapshot suffixes. Generation checks precede historical tier
checks so newly observed models cannot enter legacy Sol cohorts. The exact
optimization allowlist is separate from syntax and native catalog availability;
no snapshot suffix is removed to infer availability.
