# Git History Privacy

current tree scans are not enough before a private repository becomes public.
Old commits can still expose author metadata, old license text, removed files,
local paths, or earlier secrets.

## Check Current Tree

```bash
PRIVACY_PATTERN='(replace-with-personal-name|replace-with-local-home-prefix|replace-with-token-pattern|replace-with-private-key-header)'
rg -n --hidden -g '!.git/**' -g '!tests/**' -i "$PRIVACY_PATTERN" .
find . -maxdepth 4 -type f \( -name '*.env' -o -name '.env*' -o -name '*secret*' -o -name '*token*' -o -name 'auth.json' -o -name 'config.toml' \) -print
```

## Check Commit Metadata

```bash
git log --all --format='%H %an %ae %cn %ce %s'
```

Look for personal names, company email addresses, private hostnames, or
anything that should not be part of a public project.

## Check Historical File Contents

```bash
PRIVACY_PATTERN='(replace-with-personal-name|replace-with-local-home-prefix|replace-with-token-pattern|replace-with-private-key-header)'
git grep -n -I -i -E "$PRIVACY_PATTERN" $(git rev-list --all) -- . ':!tests/**'
```

Treat matches in old commits as public exposure if the repository visibility is
changed without rewriting history.

## Publishing Options

Preferred low-risk option:

- create a fresh public repository
- copy the current sanitized tree
- make one clean initial commit with public-safe author metadata
- tag the first public release from that clean history

Higher-risk option:

- rewrite the existing private repository history
- replace author and committer metadata
- remove old personal file contents
- force-push rewritten refs only after explicit approval

Do not rewrite shared history or force-push without a clear approval and backup.
