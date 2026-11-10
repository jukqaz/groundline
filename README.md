# GroundLine

GroundLine helps reduce repeated Codex mistakes: review relevant conversation
context, make a small justified guidance or skill change, and check later natural
work. Native Codex owns execution, permissions, settings and agents. Useful
defaults live in native AGENTS layers; project requirements remain project-local.

[한국어](README.ko.md) · [Documentation](docs/index.md)

| Product | Purpose | Default |
| --- | --- | --- |
| [Core](plugins/groundline/README.md) | One workflow-improvement skill, compact private changes and contextual follow-up | Offline; no hooks |
| [Insights](plugins/groundline-insights/README.md) | Optional operational usage collection and dashboards | Separate opt-in and consent |

## Use

Use `$groundline:optimize-codex-workflow` for an evidenced repeated friction.
Follow [one compact path](plugins/groundline/references/conversation-improvement.md):
capture the actual target, edit and verify through native Codex, record changed
bytes, then observe independent later work. No justified change is a normal
result. Usage audits are optional diagnosis, not a recurring prerequisite.
Core does not route models, manage device baselines or run continuous learning.

Existing conversation records remain readable and unchanged. A contextual
criterion being met is separate from causal quality or cost improvement;
installation and test success do not prove either. See [validation](docs/guidance-validation.md).

## Install and maintain

With Git, Codex, Bash and `jq`, review a complete binary-bearing stable distribution:

```console
git clone --branch stable --single-branch https://github.com/jukqaz/groundline.git groundline-install
bash groundline-install/install.sh
```

The default is Core. Insights is installed independently with `--profile insights`
or `--profile both`; it is not needed for local improvement. Existing model,
effort, permissions and disabled plugins are preserved unless explicitly changed.
The installer pins the reviewed distribution. Source changes are not installed
updates. See [installation](docs/installation.md) and [versioning](docs/versioning.md).
Supported hosts: Apple Silicon macOS and Linux ARM64/x86_64.

[Contributing](CONTRIBUTING.md) · [Architecture](docs/architecture.md) ·
[Privacy](docs/privacy.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md)
