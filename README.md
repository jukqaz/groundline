# GroundLine

GroundLine connects Codex usage patterns, actual work outcomes and current
official model guidance to improve a user's workflow and common Codex environment.
Codex owns execution, permissions, settings and agents. Historical observations
remain intact; environment changes follow the user's explicit choices and scope.

[한국어](README.ko.md) · [Documentation](docs/index.md)

| Product | Purpose | Default |
| --- | --- | --- |
| [Core](plugins/groundline/README.md) | Local audits, delivery evidence, and workflow recommendations | Offline; no hooks |
| [Insights](plugins/groundline-insights/README.md) | Optional aggregate collection, ClickHouse, and Grafana | Collection off until configured and consented |

The plugins install independently. Insights connects to the owner's service;
installing it does not enroll anyone in the maintainer's infrastructure.
Supported hosts: **Apple Silicon macOS (ARM64) and Linux (ARM64 or x86_64)**.

## Install and update

With Git, Codex, Bash, and `jq` installed, review a complete binary-bearing
`stable` distribution and run its installer:

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

The default is Core. Use `--profile insights` or `--profile both` as needed.
Existing model, effort, permissions, and disabled plugins are preserved unless
explicitly changed. Insights needs a compatible owner API before collector upgrades.

The installer pins the reviewed commit, so App Refresh stays there. For a newer
release, review its complete `stable` distribution and rerun that installer.
`main` and version tags contain source, not the generated binary trees.
See [installation and recovery](docs/installation.md) for App/CLI selection,
package-only commands, private connection inputs, consent, and partial retries.
Release names use a date and daily sequence; see [versioning](docs/versioning.md).

## Use and evaluate

- `$groundline:align-agent-home`: requested installation and configuration alignment.
- `$groundline:audit-agent-history`: explicit history inspection.
- `$groundline:optimize-codex-workflow`: task-scoped model/effort choices and workflow review.

The evidence loop is **audit → delivery → route**. Start with the
[CLI examples](docs/examples.md); record failed and incomplete work as well as
successes. Weekly samples do not establish whole-history coverage, and aggregate
usage alone cannot identify the best model or prove improvement.

The [adaptive environment design](docs/adaptive-environment-design.md) extends
this loop to a personal baseline, scoped application, recovery and later outcomes.
The [local implementation](docs/adaptive-environment-implementation.md) connects
registered guidance plans, recoverable application and private outcome sidecars.
Installed-plugin activation and later workflow effects require separate evidence.

Core guidance and model-led analysis consume input tokens. Insights' native
collector does not call a language model. Compare quality, rework, user effort,
time, and total observed resources on matched tasks; token savings are not
guaranteed. See [behavioral validation](docs/guidance-validation.md).

## Develop and operate

[Contributing](CONTRIBUTING.md) owns development checks;
[architecture](docs/architecture.md) defines code responsibilities;
[self-hosting](docs/self-hosting.md) covers the public-preview server;
[operations](docs/insights-operations.md) covers diagnosis and live evidence.
Keep credentials, raw history, private endpoints, and deployment receipts outside Git.

[Privacy](docs/privacy.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md)
