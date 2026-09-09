#!/usr/bin/env bash
# Run from a reviewed, binary-bearing stable distribution; requires Git and Codex.
# Installs Core and applies its declared defaults. No Insights enrollment or hooks.
set -euo pipefail
INSTALL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_HOME="${CODEX_HOME:-$HOME/.codex}"
INSTALL_CODEX="${1:-}"
if [[ -z "$INSTALL_CODEX" ]]; then
  if [[ -x /Applications/ChatGPT.app/Contents/Resources/codex ]]; then
    INSTALL_CODEX=/Applications/ChatGPT.app/Contents/Resources/codex
  else
    INSTALL_CODEX="$(command -v codex)"
  fi
fi
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) INSTALL_TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64)
    if [[ "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == 1 ]]; then
      INSTALL_TARGET=aarch64-apple-darwin
    else INSTALL_TARGET=x86_64-apple-darwin; fi ;;
  Linux/aarch64|Linux/arm64) INSTALL_TARGET=aarch64-unknown-linux-musl ;;
  Linux/x86_64) INSTALL_TARGET=x86_64-unknown-linux-musl ;;
  *) echo 'Unsupported host architecture.' >&2; exit 1 ;;
esac
INSTALL_SOURCE="$INSTALL_ROOT/plugins/groundline"
INSTALL_BINARY="$INSTALL_SOURCE/bin/$INSTALL_TARGET/groundline"
if [[ ! -x "$INSTALL_BINARY" ]]; then
  echo 'Use the binary-bearing stable distribution, not a source checkout.' >&2
  exit 1
fi
"$INSTALL_BINARY" provider-smoke --plugin-root "$INSTALL_SOURCE" --require-installed --json
INSTALL_VERSION="$("$INSTALL_BINARY" --version)"
INSTALL_VERSION="${INSTALL_VERSION#groundline }"
[[ "$INSTALL_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || exit 1
"$INSTALL_CODEX" plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json
"$INSTALL_CODEX" plugin marketplace upgrade groundline --json
"$INSTALL_CODEX" plugin add groundline@groundline --json
INSTALL_CACHE="$INSTALL_HOME/plugins/cache/groundline/groundline/$INSTALL_VERSION"
INSTALL_EXEC="$INSTALL_CACHE/bin/$INSTALL_TARGET/groundline"
# If stable moved during installation, stop and obtain its complete distribution.
# Do not run an old or unrelated cached executable as a fallback.
cmp -s "$INSTALL_BINARY" "$INSTALL_EXEC" || { echo 'Installed artifact differs from this distribution.' >&2; exit 1; }
"$INSTALL_EXEC" provider-smoke --plugin-root "$INSTALL_CACHE" --require-installed --json
INSTALL_CATALOG="$("$INSTALL_CODEX" debug models)"
printf '%s' "$INSTALL_CATALOG" | "$INSTALL_EXEC" setup --catalog - --apply
"$INSTALL_CODEX" --strict-config doctor --summary --no-color --ascii
