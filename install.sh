#!/usr/bin/env bash
# Run from the reviewed binary-bearing stable distribution. No global Git edits.
set -uo pipefail
INSTALL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_HOME="${CODEX_HOME:-$HOME/.codex}"
INSTALL_CODEX=""
INSTALL_PROFILE=core
INSTALL_SETUP=(--apply)
INSTALL_INSIGHTS=(--verify)
INSTALL_FAILED=0
INSTALL_PENDING=0
INSTALL_STAGES=()
INSTALL_URL=https://github.com/jukqaz/groundline.git
INSTALL_COMMIT=""
INSTALL_PREVIOUS_COMMIT=""
INSTALL_PREVIOUS_PLUGINS='[]'
INSTALL_ADDED_PRODUCTS=()
INSTALL_ROLLBACK_ARMED=0
INSTALL_ROLLBACK=not_needed

usage() {
  cat <<'HELP'
Usage: install.sh [--codex PATH] [--profile core|insights|both]
  Existing/native model, effort, and service tier remain unchanged unless selected below.
  Profiles select explicit setup; native refresh can also update other installed marketplace plugins.
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
  if [[ $INSTALL_ROLLBACK_ARMED == 1 ]]; then
    INSTALL_ROLLBACK_ARMED=0
    if restore_source; then stage source_rollback PASS 0
    else INSTALL_ROLLBACK=failed; stage source_rollback FAIL 1; fi
    code=1
  fi
  if [[ $code != 0 && $INSTALL_FAILED == 0 && $INSTALL_PENDING == 0 ]]; then stage interrupted FAIL "$code"; fi
  if [[ $INSTALL_FAILED == 1 ]]; then status=FAIL; code=1
  elif [[ $INSTALL_PENDING == 1 ]]; then status=ACTION_REQUIRED; code=2; fi
  local joined; joined=$(IFS=,; printf '%s' "${INSTALL_STAGES[*]}")
  printf '\n{"kind":"groundline-installation","schema":1,"status":"%s","profile":"%s","stages":{%s},"source_commit":"%s","previous_commit":"%s","rollback":"%s","resume":"rerun_same_installer_after_resolving_reported_actions"}\n' "$status" "$INSTALL_PROFILE" "$joined" "$INSTALL_COMMIT" "$INSTALL_PREVIOUS_COMMIT" "$INSTALL_ROLLBACK"
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
    --codex|--profile|--model|--effort|--service-tier|--insights-profile|--insights-endpoint|--enrollment-token-file)
      [[ $# -ge 2 ]] || { usage >&2; exit 1; }
      case "$1" in
        --codex) INSTALL_CODEX=$2 ;;
        --profile) INSTALL_PROFILE=$2 ;;
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
case "$INSTALL_PROFILE" in core|insights|both) ;; *) usage >&2; exit 1 ;; esac
if [[ $INSTALL_PROFILE == core && ${#INSTALL_INSIGHTS[@]} -gt 1 ]] || [[ $INSTALL_PROFILE == insights && ${#INSTALL_SETUP[@]} -gt 1 ]]; then usage >&2; exit 1; fi
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Reject unsupported hardware before invoking Codex or changing installation state.
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) INSTALL_TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64)
    if [[ "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == 1 ]]; then INSTALL_TARGET=aarch64-apple-darwin
    else
      echo 'Intel Macs are unsupported; use Apple Silicon macOS or Linux ARM64/x86-64.' >&2
      stage preflight FAIL 1; exit 1
    fi ;;
  Linux/aarch64|Linux/arm64) INSTALL_TARGET=aarch64-unknown-linux-musl ;;
  Linux/x86_64) INSTALL_TARGET=x86_64-unknown-linux-musl ;;
  *) echo 'Unsupported platform; use Apple Silicon macOS or Linux ARM64/x86-64.' >&2; stage preflight FAIL 1; exit 1 ;;
esac

# Native JSON is parsed with jq, never shell text matching or config.toml edits.
marketplace() {
  local value
  value=$("$INSTALL_CODEX" plugin marketplace list --json) || return 1
  printf '%s' "$value" | jq -ce '
    [.marketplaces[] | select(.name == "groundline")] |
    if length <= 1 and all(.[]; .marketplaceSource.sourceType == "git" and
      (.marketplaceSource.source == "https://github.com/jukqaz/groundline.git" or
       .marketplaceSource.source == "git@github.com:jukqaz/groundline.git" or
       .marketplaceSource.source == "ssh://git@github.com/jukqaz/groundline.git") and (.root | type) == "string")
    then . else error("unsupported_marketplace_source") end'
}
installed_plugins() {
  local value
  value=$("$INSTALL_CODEX" plugin list --json) || return 1
  printf '%s' "$value" | jq -ce '
    [.installed[] | select(.marketplaceName == "groundline")] |
    if all(.[]; (.name == "groundline" or .name == "groundline-insights") and
      .pluginId == (.name + "@groundline") and .installed == true and
      (.enabled | type) == "boolean" and (.version | type) == "string") and
      (map(.name) | unique | length) == length
    then map({name,version,enabled}) | sort_by(.name) else error("unsupported_plugin_state") end'
}
clean_head() {
  local root=$1 native=${2:-0} top changes head metadata
  [[ -d "$root" && ! -L "$root" && -O "$root/.git" ]] || return 1
  top=$(git -C "$root" rev-parse --show-toplevel) || return 1
  [[ "$top" == "$(cd -- "$root" && pwd -P)" ]] || return 1
  changes=$(git -c core.fsmonitor=false -C "$root" status --porcelain --untracked-files=all) || return 1
  head=$(git -C "$root" rev-parse --verify 'HEAD^{commit}') || return 1
  [[ "$head" =~ ^[0-9a-f]{40}$ ]] || return 1
  if [[ "$native" == 1 && "$changes" == '?? .codex-marketplace-install.json' ]]; then
    metadata="$root/.codex-marketplace-install.json"
    [[ -f "$metadata" && ! -L "$metadata" && -O "$metadata" ]] || return 1
    [[ $(wc -c < "$metadata") -le 16384 ]] || return 1
    jq -e --arg url "$INSTALL_URL" --arg head "$head" '
      type == "object" and (keys | sort) == ["ref_name","revision","source","source_type","sparse_paths"] and
      .source_type == "git" and .source == $url and .revision == $head and .sparse_paths == [] and
      (.ref_name | type == "string" and length > 0)' "$metadata" >/dev/null || return 1
  elif [[ -n "$changes" ]]; then return 1; fi
  printf '%s' "$head"
}
snapshot_matches() {
  local listing root head
  listing=$(marketplace) || return 1
  root=$(printf '%s' "$listing" | jq -er 'if length == 1 then .[0].root else error("missing_marketplace") end') || return 1
  head=$(clean_head "$root" 1) || return 1
  [[ "$head" == "$1" ]]
}
restore_source() {
  local listing count product restored root version file source cache
  listing=$(marketplace) || return 1
  count=$(printf '%s' "$listing" | jq -r length) || return 1
  if [[ "$count" == 1 ]] && ! snapshot_matches "$INSTALL_COMMIT" &&
     { [[ -z "$INSTALL_PREVIOUS_COMMIT" ]] || ! snapshot_matches "$INSTALL_PREVIOUS_COMMIT"; }; then return 1; fi
  # Remove only newly selected installations, never a pre-existing package.
  restored=$(installed_plugins) || return 1
  # Bash 3.2 treats an empty array as unset under nounset; expand zero arguments.
  for product in ${INSTALL_ADDED_PRODUCTS[@]+"${INSTALL_ADDED_PRODUCTS[@]}"}; do
    if printf '%s' "$restored" | jq -e --arg name "$product" 'any(.[]; .name == $name)' >/dev/null; then
      "$INSTALL_CODEX" plugin remove "$product@groundline" --json || return 1
    fi
  done
  if [[ "$count" == 1 ]]; then
    "$INSTALL_CODEX" plugin marketplace remove groundline --json || return 1
  fi
  if [[ -z "$INSTALL_PREVIOUS_COMMIT" ]]; then
    [[ $(marketplace) == '[]' && $(installed_plugins) == '[]' ]] || return 1
    INSTALL_ROLLBACK=fresh_registration_removed
    return 0
  fi
  "$INSTALL_CODEX" plugin marketplace add "$INSTALL_URL" --ref "$INSTALL_PREVIOUS_COMMIT" --json || return 1
  "$INSTALL_CODEX" plugin marketplace upgrade groundline --json || return 1
  snapshot_matches "$INSTALL_PREVIOUS_COMMIT" || return 1
  restored=$(installed_plugins) || return 1
  [[ "$restored" == "$INSTALL_PREVIOUS_PLUGINS" ]] || return 1
  listing=$(marketplace) || return 1
  root=$(printf '%s' "$listing" | jq -er '.[0].root') || return 1
  for product in groundline groundline-insights; do
    version=$(printf '%s' "$restored" | jq -r --arg name "$product" '.[] | select(.name == $name) | .version') || return 1
    [[ -n "$version" ]] || continue
    source="$root/plugins/$product"
    cache="$INSTALL_HOME/plugins/cache/groundline/$product/$version"
    for file in ".codex-plugin/plugin.json" "bin/$INSTALL_TARGET/$product" "bin/$INSTALL_TARGET/$product.sha256" "bin/$INSTALL_TARGET/manifest.json"; do
      cmp -s "$source/$file" "$cache/$file" || return 1
    done
  done
  INSTALL_ROLLBACK=previous_commit_pinned
}

app_codex_bundle() {
  local app plist bundle_id
  case "$1" in
    */Contents/Resources/codex-cli/bin/codex) app=${1%/Contents/Resources/codex-cli/bin/codex} ;;
    */Contents/Resources/codex) app=${1%/Contents/Resources/codex} ;;
    *) return 1 ;;
  esac
  plist="$app/Contents/Info.plist"
  [[ -x "$1" && -f "$plist" ]] || return 1
  bundle_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist" 2>/dev/null) || return 1
  [[ "$bundle_id" == com.openai.codex ]]
}
find_app_codex() {
  local resource_path applications app candidate
  for resource_path in codex-cli/bin/codex codex; do
    for applications in /Applications "$HOME/Applications"; do
      for app in "$applications"/*.app; do
        [[ -d "$app" ]] || continue
        candidate="$app/Contents/Resources/$resource_path"
        if app_codex_bundle "$candidate"; then printf '%s\n' "$candidate"; return 0; fi
      done
    done
  done
  return 1
}
if [[ -z "$INSTALL_CODEX" ]]; then
  if [[ $(uname -s) == Darwin ]]; then INSTALL_CODEX=$(find_app_codex) || true; fi
  if [[ -z "$INSTALL_CODEX" ]]; then INSTALL_CODEX="$(command -v codex || true)"; fi
fi
INSTALL_APP_BUNDLED=0
if [[ $(uname -s) == Darwin ]] && app_codex_bundle "$INSTALL_CODEX"; then INSTALL_APP_BUNDLED=1; fi
if ! command -v git >/dev/null || ! command -v jq >/dev/null || [[ -z "$INSTALL_CODEX" ]] ||
   ! "$INSTALL_CODEX" plugin add --help >/dev/null ||
   ! "$INSTALL_CODEX" plugin marketplace upgrade --help >/dev/null ||
   ! "$INSTALL_CODEX" debug models --help >/dev/null ||
   ! "$INSTALL_CODEX" doctor --help >/dev/null; then
  echo 'Install Git, jq, and a Codex runtime with plugin, debug models, and doctor support; use --codex to select it.' >&2
  stage preflight FAIL 1; exit 1
fi
stage preflight PASS 0
INSTALL_MARKETPLACE=$(marketplace) || { stage native_source FAIL 1; exit 1; }
INSTALL_PREVIOUS_PLUGINS=$(installed_plugins) || { stage native_source FAIL 1; exit 1; }
if [[ $(printf '%s' "$INSTALL_MARKETPLACE" | jq -r length) == 1 ]]; then
  INSTALL_URL=$(printf '%s' "$INSTALL_MARKETPLACE" | jq -er '.[0].marketplaceSource.source') || exit 1
  previous_root=$(printf '%s' "$INSTALL_MARKETPLACE" | jq -er '.[0].root') || exit 1
  INSTALL_PREVIOUS_COMMIT=$(clean_head "$previous_root" 1) || { stage native_source FAIL 1; exit 1; }
  if ! "$INSTALL_CODEX" plugin list --json | jq -e --arg root "$previous_root" --arg url "$INSTALL_URL" '
    all(.installed[] | select(.marketplaceName == "groundline");
      .source.source == "local" and .source.path == ($root + "/plugins/" + .name) and
      .marketplaceSource.sourceType == "git" and .marketplaceSource.source == $url)' >/dev/null; then stage native_source FAIL 1; exit 1; fi
  for product in groundline groundline-insights; do
    previous_version=$(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -r --arg name "$product" '.[] | select(.name == $name) | .version') || exit 1
    if [[ -n "$previous_version" ]]; then
      source_version=$(jq -er '.version' "$previous_root/plugins/$product/.codex-plugin/plugin.json") || { stage native_source FAIL 1; exit 1; }
      [[ "$source_version" == "$previous_version" ]] || { stage native_source FAIL 1; exit 1; }
    fi
  done
elif [[ "$INSTALL_PREVIOUS_PLUGINS" != '[]' ]]; then stage native_source FAIL 1; exit 1; fi
stage native_source PASS 0
INSTALL_PRODUCTS=(groundline)
case "$INSTALL_PROFILE" in insights) INSTALL_PRODUCTS=(groundline-insights) ;; both) INSTALL_PRODUCTS+=(groundline-insights) ;; esac
INSTALL_VERIFY_PRODUCTS=("${INSTALL_PRODUCTS[@]}")
for product in groundline groundline-insights; do
  if printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -e --arg name "$product" 'any(.[]; .name == $name)' >/dev/null; then
    if [[ " ${INSTALL_VERIFY_PRODUCTS[*]} " != *" $product "* ]]; then INSTALL_VERIFY_PRODUCTS+=("$product"); fi
  fi
done
INSTALL_CHECK_INSIGHTS=0
# Native refresh can advance an installed collector even for --profile core.
# Let the candidate's existing profile validator decide compatibility without
# changing consent, enrollment, collection state, or Codex plugin metadata.
if [[ -e "$INSTALL_HOME/groundline/insights" || -L "$INSTALL_HOME/groundline/insights" ]] ||
   printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -e 'any(.[]; .name == "groundline-insights")' >/dev/null; then
  INSTALL_CHECK_INSIGHTS=1
  if [[ " ${INSTALL_VERIFY_PRODUCTS[*]} " != *" groundline-insights "* ]]; then INSTALL_VERIFY_PRODUCTS+=(groundline-insights); fi
fi
INSTALL_VERSION=""
for product in "${INSTALL_VERIFY_PRODUCTS[@]}"; do
  source_root="$INSTALL_ROOT/plugins/$product"
  binary="$source_root/bin/$INSTALL_TARGET/$product"
  checked "distribution_$product" "$binary" provider-smoke --plugin-root "$source_root" --require-installed --json || exit 1
  version=$("$binary" --version) || exit 1
  version=${version#"$product "}
  if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || [[ -n "$INSTALL_VERSION" && "$INSTALL_VERSION" != "$version" ]]; then stage distribution_version FAIL 1; exit 1; fi
  INSTALL_VERSION=$version
done
if ! printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -e --arg version "$INSTALL_VERSION" '
  ($version | split(".") | map(tonumber)) as $candidate |
  all(.[]; (.version | test("^[0-9]+\\.[0-9]+\\.[0-9]+$")) and
    (.version | split(".") | map(tonumber)) <= $candidate)' >/dev/null; then stage distribution_version FAIL 1; exit 1; fi
INSTALL_COMMIT=$(clean_head "$INSTALL_ROOT") || { stage distribution_revision FAIL 1; exit 1; }
for product in "${INSTALL_VERIFY_PRODUCTS[@]}"; do
  git -C "$INSTALL_ROOT" ls-files --error-unmatch -- "plugins/$product/.codex-plugin/plugin.json" \
    "plugins/$product/bin/$INSTALL_TARGET/$product" "plugins/$product/bin/$INSTALL_TARGET/$product.sha256" \
    "plugins/$product/bin/$INSTALL_TARGET/manifest.json" >/dev/null || { stage distribution_revision FAIL 1; exit 1; }
done
stage distribution_revision PASS 0
if [[ $INSTALL_CHECK_INSIGHTS == 1 ]]; then
  checked insights_server_compatibility "$INSTALL_ROOT/plugins/groundline-insights/bin/$INSTALL_TARGET/groundline-insights" worker check-server --json || exit 1
else stage insights_server_compatibility NOT_CONFIGURED 0; fi
INSTALL_ROLLBACK_ARMED=1
if [[ -n "$INSTALL_PREVIOUS_COMMIT" ]]; then
  checked marketplace_remove "$INSTALL_CODEX" plugin marketplace remove groundline --json || exit 1
fi
checked marketplace_add "$INSTALL_CODEX" plugin marketplace add "$INSTALL_URL" --ref "$INSTALL_COMMIT" --json || exit 1
checked marketplace_refresh "$INSTALL_CODEX" plugin marketplace upgrade groundline --json || exit 1
checked snapshot_revision snapshot_matches "$INSTALL_COMMIT" || exit 1
for product in "${INSTALL_PRODUCTS[@]}"; do
  if printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -e --arg name "$product" 'any(.[]; .name == $name)' >/dev/null; then
    stage "install_$product" PASS 0
  else
    INSTALL_ADDED_PRODUCTS+=("$product")
    checked "install_$product" "$INSTALL_CODEX" plugin add "$product@groundline" --json || exit 1
  fi
done
for product in "${INSTALL_VERIFY_PRODUCTS[@]}"; do
  # A profile can exist without an installed collector: validate its candidate,
  # but do not install or demand a cache for an unselected, absent product.
  if [[ " ${INSTALL_PRODUCTS[*]} " != *" $product "* ]] &&
     ! printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -e --arg name "$product" 'any(.[]; .name == $name)' >/dev/null; then continue; fi
  binary="$INSTALL_HOME/plugins/cache/groundline/$product/$INSTALL_VERSION/bin/$INSTALL_TARGET/$product"
  if ! cmp -s "$INSTALL_ROOT/plugins/$product/bin/$INSTALL_TARGET/$product" "$binary"; then
    echo 'Installed artifact differs from this distribution. Obtain the complete current stable distribution and retry.' >&2
    stage "verify_$product" FAIL 1; exit 1
  fi
  checked "verify_$product" "$binary" provider-smoke --plugin-root "$INSTALL_HOME/plugins/cache/groundline/$product/$INSTALL_VERSION" --require-installed --json || exit 1
done
current_plugins=$(installed_plugins) || { stage installed_state FAIL 1; exit 1; }
selected_products=$(printf '%s\n' "${INSTALL_PRODUCTS[@]}" | jq -Rsc 'split("\n") | map(select(length > 0))') || exit 1
expected_plugins=$(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -ce --argjson selected "$selected_products" --arg version "$INSTALL_VERSION" '
  reduce $selected[] as $name (. ; if any(.[]; .name == $name) then . else . + [{name:$name,enabled:true}] end) |
  map({name,version:$version,enabled}) | sort_by(.name)') || exit 1
if [[ "$current_plugins" != "$expected_plugins" ]]; then
  stage installed_state FAIL 1; exit 1
fi
stage installed_state PASS 0
INSTALL_ROLLBACK_ARMED=0
if [[ $INSTALL_PROFILE != insights ]]; then
  if INSTALL_CATALOG=$("$INSTALL_CODEX" debug models); then
    stage catalog PASS 0
    binary="$INSTALL_HOME/plugins/cache/groundline/groundline/$INSTALL_VERSION/bin/$INSTALL_TARGET/groundline"
    if printf '%s' "$INSTALL_CATALOG" | "$binary" setup --catalog - "${INSTALL_SETUP[@]}"; then stage settings PASS 0
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
      if [[ $INSTALL_APP_BUNDLED == 1 ]]; then export GROUNDLINE_RUNTIME_FAMILY=codex_app; fi
    fi
    "$binary" setup "${INSTALL_INSIGHTS[@]}"
  )
  if insights_setup; then stage insights_setup PASS 0
  else code=$?; if [[ $code == 2 ]]; then stage insights_setup ACTION_REQUIRED "$code"; else stage insights_setup FAIL "$code"; fi; fi
else stage insights_setup NOT_SELECTED 0; fi
