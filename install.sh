#!/usr/bin/env bash
# Run from the reviewed binary-bearing stable distribution. No global Git edits.
set -uo pipefail
INSTALL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_HOME="${CODEX_HOME:-$HOME/.codex}"
INSTALL_CODEX=""
INSTALL_PROFILE=core
INSTALL_PRESET=preserve
INSTALL_SETUP=(--apply)
INSTALL_INSIGHTS=(--verify)
INSTALL_FAILED=0
INSTALL_PENDING=0
INSTALL_STAGES=()

usage() {
  cat <<'HELP'
Usage: install.sh [--codex PATH] [--profile core|insights|both]
  --preset preserve|astra     Keep existing/native choices (default), or opt into Astra/xhigh/Fast off
  --model MODEL --effort EFFORT --service-tier default|fast
  --restore-native-context   Explicitly remove existing root context overrides
  --insights-profile FILE    Owner-private connection profile, or use the next two options
  --insights-endpoint URL --enrollment-token-file FILE
  --enable-insights          Consent to aggregate uploads to the selected owner service
Insights verification runs after explicit enablement or with existing active consent.
Run the same command again after resolving a reported action; existing state is rechecked.
HELP
}
stage() {
  INSTALL_STAGES+=("\"$1\":{\"status\":\"$2\",\"exit_code\":$3}")
  case "$2" in FAIL) INSTALL_FAILED=1 ;; ACTION_REQUIRED) INSTALL_PENDING=1 ;; esac
}
finish() {
  local code=$? status=PASS
  if [[ $code != 0 && $INSTALL_FAILED == 0 && $INSTALL_PENDING == 0 ]]; then stage interrupted FAIL "$code"; fi
  if [[ $INSTALL_FAILED == 1 ]]; then status=FAIL; code=1
  elif [[ $INSTALL_PENDING == 1 ]]; then status=ACTION_REQUIRED; code=2; fi
  local joined; joined=$(IFS=,; printf '%s' "${INSTALL_STAGES[*]}")
  printf '\n{"kind":"groundline-installation","schema":1,"status":"%s","profile":"%s","stages":{%s},"resume":"rerun_same_installer_after_resolving_reported_actions"}\n' "$status" "$INSTALL_PROFILE" "$joined"
  trap - EXIT
  exit "$code"
}
checked() {
  local name=$1 code; shift
  if "$@"; then stage "$name" PASS 0; return 0
  else code=$?; stage "$name" FAIL "$code"; return 1; fi
}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    --codex|--profile|--preset|--model|--effort|--service-tier|--insights-profile|--insights-endpoint|--enrollment-token-file)
      [[ $# -ge 2 ]] || { usage >&2; exit 1; }
      case "$1" in
        --codex) INSTALL_CODEX=$2 ;;
        --profile) INSTALL_PROFILE=$2 ;;
        --preset) INSTALL_PRESET=$2 ;;
        --model|--effort|--service-tier) INSTALL_SETUP+=("$1" "$2") ;;
        --insights-profile) INSTALL_INSIGHTS+=(--input "$2") ;;
        --insights-endpoint) INSTALL_INSIGHTS+=(--endpoint "$2") ;;
        --enrollment-token-file) INSTALL_INSIGHTS+=("$1" "$2") ;;
      esac
      shift 2 ;;
    --restore-native-context) INSTALL_SETUP+=("$1"); shift ;;
    --enable-insights) INSTALL_INSIGHTS+=(--enable); shift ;;
    *) usage >&2; exit 1 ;;
  esac
done
case "$INSTALL_PROFILE/$INSTALL_PRESET" in core/preserve|core/astra|insights/preserve|both/preserve|both/astra) ;; *) usage >&2; exit 1 ;; esac
if [[ $INSTALL_PROFILE == core && ${#INSTALL_INSIGHTS[@]} -gt 1 ]] || [[ $INSTALL_PROFILE == insights && ${#INSTALL_SETUP[@]} -gt 1 ]]; then usage >&2; exit 1; fi
trap finish EXIT

if [[ -z "$INSTALL_CODEX" ]]; then
  if [[ -x /Applications/ChatGPT.app/Contents/Resources/codex ]]; then INSTALL_CODEX=/Applications/ChatGPT.app/Contents/Resources/codex
  else INSTALL_CODEX="$(command -v codex || true)"; fi
fi
if ! command -v git >/dev/null || [[ -z "$INSTALL_CODEX" ]] ||
   ! "$INSTALL_CODEX" plugin add --help >/dev/null ||
   ! "$INSTALL_CODEX" debug models --help >/dev/null ||
   ! "$INSTALL_CODEX" doctor --help >/dev/null; then
  echo 'Install Git and a Codex runtime with plugin, debug models, and doctor support; use --codex to select it.' >&2
  stage preflight FAIL 1; exit 1
fi
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) INSTALL_TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64)
    if [[ "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == 1 ]]; then INSTALL_TARGET=aarch64-apple-darwin
    else INSTALL_TARGET=x86_64-apple-darwin; fi ;;
  Linux/aarch64|Linux/arm64) INSTALL_TARGET=aarch64-unknown-linux-musl ;;
  Linux/x86_64) INSTALL_TARGET=x86_64-unknown-linux-musl ;;
  *) stage preflight FAIL 1; exit 1 ;;
esac
stage preflight PASS 0
INSTALL_PRODUCTS=(groundline)
case "$INSTALL_PROFILE" in insights) INSTALL_PRODUCTS=(groundline-insights) ;; both) INSTALL_PRODUCTS+=(groundline-insights) ;; esac
INSTALL_VERSION=""
for product in "${INSTALL_PRODUCTS[@]}"; do
  source_root="$INSTALL_ROOT/plugins/$product"
  binary="$source_root/bin/$INSTALL_TARGET/$product"
  checked "distribution_$product" "$binary" provider-smoke --plugin-root "$source_root" --require-installed --json || exit 1
  version=$("$binary" --version) || exit 1
  version=${version#"$product "}
  if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || [[ -n "$INSTALL_VERSION" && "$INSTALL_VERSION" != "$version" ]]; then stage distribution_version FAIL 1; exit 1; fi
  INSTALL_VERSION=$version
done
checked marketplace_add "$INSTALL_CODEX" plugin marketplace add https://github.com/jukqaz/groundline.git --ref stable --json || exit 1
checked marketplace_refresh "$INSTALL_CODEX" plugin marketplace upgrade groundline --json || exit 1
for product in "${INSTALL_PRODUCTS[@]}"; do
  checked "install_$product" "$INSTALL_CODEX" plugin add "$product@groundline" --json || exit 1
  binary="$INSTALL_HOME/plugins/cache/groundline/$product/$INSTALL_VERSION/bin/$INSTALL_TARGET/$product"
  if ! cmp -s "$INSTALL_ROOT/plugins/$product/bin/$INSTALL_TARGET/$product" "$binary"; then
    echo 'Installed artifact differs from this distribution. Obtain the complete current stable distribution and retry.' >&2
    stage "verify_$product" FAIL 1; exit 1
  fi
  checked "verify_$product" "$binary" provider-smoke --plugin-root "$INSTALL_HOME/plugins/cache/groundline/$product/$INSTALL_VERSION" --require-installed --json || exit 1
done
if [[ $INSTALL_PROFILE != insights ]]; then
  if INSTALL_CATALOG=$("$INSTALL_CODEX" debug models); then
    stage catalog PASS 0
    binary="$INSTALL_HOME/plugins/cache/groundline/groundline/$INSTALL_VERSION/bin/$INSTALL_TARGET/groundline"
    if printf '%s' "$INSTALL_CATALOG" | "$binary" setup --catalog - --preset "$INSTALL_PRESET" "${INSTALL_SETUP[@]}"; then stage settings PASS 0
    else code=$?; if [[ $code == 2 ]]; then stage settings ACTION_REQUIRED "$code"; else stage settings FAIL "$code"; fi; fi
    unset INSTALL_CATALOG
  else code=$?; stage catalog FAIL "$code"; stage settings NOT_RUN 0; fi
else stage settings NOT_SELECTED 0; fi
if "$INSTALL_CODEX" --strict-config doctor --summary --no-color --ascii; then stage native_doctor PASS 0
else code=$?; stage native_doctor ACTION_REQUIRED "$code"; fi
if [[ $INSTALL_PROFILE != core ]]; then
  binary="$INSTALL_HOME/plugins/cache/groundline/groundline-insights/$INSTALL_VERSION/bin/$INSTALL_TARGET/groundline-insights"
  # A terminal launching the App's CLI must consent for that App source. Keep
  # any explicit runtime/originator choice, including remote automation.
  insights_setup() (
    if [[ -z ${GROUNDLINE_RUNTIME_FAMILY+x} && -z ${CODEX_INTERNAL_ORIGINATOR_OVERRIDE+x} ]]; then
      case "$INSTALL_CODEX" in */ChatGPT.app/Contents/Resources/codex) export GROUNDLINE_RUNTIME_FAMILY=codex_app ;; esac
    fi
    "$binary" setup "${INSTALL_INSIGHTS[@]}"
  )
  if insights_setup; then stage insights_setup PASS 0
  else code=$?; if [[ $code == 2 ]]; then stage insights_setup ACTION_REQUIRED "$code"; else stage insights_setup FAIL "$code"; fi; fi
else stage insights_setup NOT_SELECTED 0; fi
