#!/usr/bin/env bash
# Install one reviewed product distribution through native Codex only.
set -uo pipefail
umask 077
INSTALL_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
INSTALL_HOME="${CODEX_HOME:-$HOME/.codex}"
INSTALL_CODEX=""
INSTALL_PROFILE=""
INSTALL_SETUP=(--apply)
INSTALL_INSIGHTS=(--verify)
INSTALL_STAGES=()
INSTALL_FAILED=0
INSTALL_PENDING=0
INSTALL_ARMED=0
INSTALL_ROLLBACK=not_needed
INSTALL_PREVIOUS_COMMIT=""
INSTALL_COMMIT=""
INSTALL_URL=git@github.com:jukqaz/groundline.git
INSTALL_PREVIOUS_PLUGINS='[]'
INSTALL_PREVIOUS_SELECTED=null
INSTALL_LEGACY_CORE=null
INSTALL_LEGACY_REMOVED=0
INSTALL_TEMP=""

usage() {
  cat <<'HELP'
Usage: install.sh [--codex PATH] [--profile core|insights]
The reviewed distribution selects exactly one product and independent channel.
Existing preferences, disabled flags, records and collection consent are preserved.
  --model MODEL --effort EFFORT --service-tier default|fast
  --restore-native-context    Core only; explicit settings operation
  --insights-profile FILE     Insights only; owner-private connection input
  --insights-endpoint URL --enrollment-token-file FILE
  --enable-insights           Explicit collection consent
Default installation does not configure models, enroll or enable collection.
HELP
}
stage() {
  INSTALL_STAGES+=("\"$1\":{\"status\":\"$2\",\"exit_code\":$3}")
  case "$2" in FAIL) INSTALL_FAILED=1 ;; ACTION_REQUIRED) INSTALL_PENDING=1 ;; esac
}
checked() {
  local name=$1 code; shift
  if "$@"; then stage "$name" PASS 0; return 0
  else code=$?; stage "$name" FAIL "$code"; return 1; fi
}
marketplace() {
  "$INSTALL_CODEX" plugin marketplace list --json | jq -ce --arg name "$1" '
    [.marketplaces[] | select(.name == $name)] |
    if length <= 1 and all(.[]; .marketplaceSource.sourceType == "git" and
      (.marketplaceSource.source == "https://github.com/jukqaz/groundline.git" or
       .marketplaceSource.source == "git@github.com:jukqaz/groundline.git") and (.root|type)=="string")
    then . else error("unsupported_marketplace_source") end'
}
plugins() {
  "$INSTALL_CODEX" plugin list --json | jq -ce '
    [.installed[] | {pluginId,name,marketplaceName,version,enabled}] | sort_by(.pluginId) |
    if all(.[]; (.pluginId|type)=="string" and (.name|type)=="string" and
      (.marketplaceName|type)=="string" and (.version|type)=="string" and (.enabled|type)=="boolean")
      and (map(.pluginId)|unique|length)==length then . else error("unsupported_plugin_state") end'
}
clean_head() {
  local root=$1 native=${2:-0} changes head top metadata
  [[ -d "$root" && ! -L "$root" && -O "$root/.git" ]] || return 1
  top=$(git -C "$root" rev-parse --show-toplevel) || return 1
  [[ "$top" == "$(cd -- "$root" && pwd -P)" ]] || return 1
  head=$(git -C "$root" rev-parse --verify 'HEAD^{commit}') || return 1
  [[ "$head" =~ ^[0-9a-f]{40}$ ]] || return 1
  changes=$(git -c core.fsmonitor=false -C "$root" status --porcelain --untracked-files=all) || return 1
  if [[ "$native" == 1 && "$changes" == '?? .codex-marketplace-install.json' ]]; then
    metadata="$root/.codex-marketplace-install.json"
    [[ -f "$metadata" && ! -L "$metadata" && -O "$metadata" && $(wc -c < "$metadata") -le 16384 ]] || return 1
    jq -e --arg head "$head" '
      type=="object" and (keys|sort)==["ref_name","revision","source","source_type","sparse_paths"] and
      .source_type=="git" and .revision==$head and .sparse_paths==[] and
      (.source=="https://github.com/jukqaz/groundline.git" or .source=="git@github.com:jukqaz/groundline.git") and
      (.ref_name|type=="string" and length>0)' "$metadata" >/dev/null || return 1
  elif [[ -n "$changes" ]]; then return 1; fi
  printf '%s' "$head"
}
snapshot_matches() {
  local row root head
  row=$(marketplace "$INSTALL_MARKET") || return 1
  root=$(printf '%s' "$row" | jq -er 'if length==1 then .[0].root else error("missing_source") end') || return 1
  head=$(clean_head "$root" 1) || return 1
  [[ "$head" == "$1" ]]
}
cache_matches() {
  local source=$1 name=$2 version=$3 cache file relative count=0
  cache="$INSTALL_HOME/plugins/cache/$4/$name/$version"
  [[ -d "$cache" && ! -L "$cache" && -O "$cache" ]] || return 1
  [[ -z $(find "$cache" -type l -print) ]] || return 1
  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    relative=${file#"plugins/$name/"}
    [[ -f "$cache/$relative" && ! -L "$cache/$relative" ]] || return 1
    cmp -s "$source/$file" "$cache/$relative" || return 1
    count=$((count+1))
  done < <(git -C "$source" ls-tree -r --name-only HEAD -- "plugins/$name")
  [[ "$count" -gt 0 && $(find "$cache" -type f -print | wc -l | tr -d ' ') == "$count" ]]
}
restore_source() {
  local current row old_source
  current=$(plugins) || return 1
  if [[ "$INSTALL_PREVIOUS_SELECTED" == null ]] &&
     printf '%s' "$current" | jq -e --arg id "$INSTALL_ID" 'any(.[];.pluginId==$id)' >/dev/null; then
    "$INSTALL_CODEX" plugin remove "$INSTALL_ID" --json || return 1
  fi
  row=$(marketplace "$INSTALL_MARKET") || return 1
  if [[ $(printf '%s' "$row" | jq -r length) == 1 ]]; then
    if ! snapshot_matches "$INSTALL_COMMIT" &&
       { [[ -z "$INSTALL_PREVIOUS_COMMIT" ]] || ! snapshot_matches "$INSTALL_PREVIOUS_COMMIT"; }; then return 1; fi
    "$INSTALL_CODEX" plugin marketplace remove "$INSTALL_MARKET" --json || return 1
  fi
  if [[ -n "$INSTALL_PREVIOUS_COMMIT" ]]; then
    "$INSTALL_CODEX" plugin marketplace add "$INSTALL_URL" --ref "$INSTALL_PREVIOUS_COMMIT" --json || return 1
    "$INSTALL_CODEX" plugin marketplace upgrade "$INSTALL_MARKET" --json || return 1
    snapshot_matches "$INSTALL_PREVIOUS_COMMIT" || return 1
  fi
  if [[ "$INSTALL_LEGACY_REMOVED" == 1 ]]; then
    row=$(marketplace groundline) || return 1
    old_source=$(printf '%s' "$row" | jq -er '.[0].root') || return 1
    [[ $(clean_head "$old_source" 1) == "$INSTALL_LEGACY_COMMIT" ]] || return 1
    current=$(plugins) || return 1
    if ! printf '%s' "$current" | jq -e 'any(.[];.pluginId=="groundline@groundline")' >/dev/null; then
      "$INSTALL_CODEX" plugin add groundline@groundline --json || return 1
    fi
  fi
  [[ $(plugins) == "$INSTALL_PREVIOUS_PLUGINS" ]] || return 1
  if [[ "$INSTALL_PREVIOUS_SELECTED" != null ]]; then
    row=$(marketplace "$INSTALL_MARKET") || return 1
    old_source=$(printf '%s' "$row" | jq -er '.[0].root') || return 1
    cache_matches "$old_source" "$INSTALL_PLUGIN" "$(printf '%s' "$INSTALL_PREVIOUS_SELECTED" | jq -r .version)" "$INSTALL_MARKET" || return 1
  fi
  if [[ "$INSTALL_LEGACY_REMOVED" == 1 ]]; then
    row=$(marketplace groundline) || return 1
    old_source=$(printf '%s' "$row" | jq -er '.[0].root') || return 1
    cache_matches "$old_source" groundline "$(printf '%s' "$INSTALL_LEGACY_CORE" | jq -r .version)" groundline || return 1
  fi
  INSTALL_ROLLBACK=previous_product_registration_restored
}
finish() {
  local code=$? status=PASS joined
  if [[ "$INSTALL_ARMED" == 1 ]]; then
    INSTALL_ARMED=0
    if restore_source; then stage source_rollback PASS 0; else INSTALL_ROLLBACK=failed; stage source_rollback FAIL 1; fi
    code=1
  fi
  [[ -z "$INSTALL_TEMP" ]] || rm -rf -- "$INSTALL_TEMP"
  if [[ "$code" != 0 && "$INSTALL_FAILED" == 0 && "$INSTALL_PENDING" == 0 ]]; then stage interrupted FAIL "$code"; fi
  if [[ "$INSTALL_FAILED" == 1 ]]; then status=FAIL; code=1
  elif [[ "$INSTALL_PENDING" == 1 ]]; then status=ACTION_REQUIRED; code=2; fi
  joined=$(IFS=,; printf '%s' "${INSTALL_STAGES[*]}")
  printf '\n{"kind":"groundline-installation","schema":2,"status":"%s","profile":"%s","stages":{%s},"source_commit":"%s","previous_commit":"%s","rollback":"%s","resume":"rerun_reviewed_product_installer"}\n' "$status" "$INSTALL_PROFILE" "$joined" "$INSTALL_COMMIT" "$INSTALL_PREVIOUS_COMMIT" "$INSTALL_ROLLBACK"
  trap - EXIT
  exit "$code"
}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h) usage; exit 0 ;;
    --codex|--profile|--model|--effort|--service-tier|--insights-profile|--insights-endpoint|--enrollment-token-file)
      [[ $# -ge 2 ]] || { usage >&2; exit 1; }
      case "$1" in
        --codex) INSTALL_CODEX=$2 ;; --profile) INSTALL_PROFILE=$2 ;;
        --model|--effort|--service-tier) INSTALL_SETUP+=("$1" "$2") ;;
        --insights-profile) INSTALL_INSIGHTS+=(--input "$2") ;;
        --insights-endpoint) INSTALL_INSIGHTS+=(--endpoint "$2") ;;
        --enrollment-token-file) INSTALL_INSIGHTS+=("$1" "$2") ;;
      esac; shift 2 ;;
    --restore-native-context) INSTALL_SETUP+=("$1"); shift ;;
    --enable-insights) INSTALL_INSIGHTS+=(--enable); shift ;;
    *) usage >&2; exit 1 ;;
  esac
done
case "$INSTALL_PROFILE" in ''|core|insights) ;; *) usage >&2; exit 1 ;; esac
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
command -v git >/dev/null && command -v jq >/dev/null || { stage preflight FAIL 1; exit 1; }
descriptor="$INSTALL_ROOT/distribution.json"
[[ -f "$descriptor" && ! -L "$descriptor" && -O "$descriptor" && $(wc -c < "$descriptor") -le 4096 ]] || { stage distribution FAIL 1; exit 1; }
jq -e '
  type=="object" and (keys|sort)==["channel","kind","marketplace","plugin","product","release_tag","schema","source_sha","version"] and
  .kind=="groundline-plugin-distribution" and .schema==1 and
  (.source_sha|test("^[0-9a-f]{40}$")) and (.version|test("^[0-9]{4}\\.[1-9][0-9]{2,3}\\.[1-9][0-9]*$")) and
  ((.product=="core" and .plugin=="groundline" and .marketplace=="groundline-core" and .channel=="core-stable") or
   (.product=="insights" and .plugin=="groundline-insights" and .marketplace=="groundline" and .channel=="stable")) and
  .release_tag==(.product+"-v"+.version)' "$descriptor" >/dev/null || { stage distribution FAIL 1; exit 1; }
product=$(jq -r .product "$descriptor")
[[ -z "$INSTALL_PROFILE" || "$INSTALL_PROFILE" == "$product" ]] || { stage product_selection FAIL 1; exit 1; }
INSTALL_PROFILE=$product
INSTALL_PLUGIN=$(jq -r .plugin "$descriptor")
INSTALL_MARKET=$(jq -r .marketplace "$descriptor")
INSTALL_VERSION=$(jq -r .version "$descriptor")
INSTALL_ID="$INSTALL_PLUGIN@$INSTALL_MARKET"
if [[ "$product" == core && ${#INSTALL_INSIGHTS[@]} -gt 1 ]] || [[ "$product" == insights && ${#INSTALL_SETUP[@]} -gt 1 ]]; then stage product_selection FAIL 1; exit 1; fi
INSTALL_COMMIT=$(clean_head "$INSTALL_ROOT") || { stage distribution_revision FAIL 1; exit 1; }
[[ $(git -C "$INSTALL_ROOT" rev-parse HEAD^) == $(jq -r .source_sha "$descriptor") ]] || { stage distribution_revision FAIL 1; exit 1; }
jq -e --arg market "$INSTALL_MARKET" --arg name "$INSTALL_PLUGIN" '
  .name==$market and (.plugins|length)==1 and .plugins[0].name==$name and
  .plugins[0].source=={"source":"local","path":("./plugins/"+$name)}' "$INSTALL_ROOT/.agents/plugins/marketplace.json" >/dev/null || { stage distribution_catalog FAIL 1; exit 1; }
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) INSTALL_TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64) if [[ "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == 1 ]]; then INSTALL_TARGET=aarch64-apple-darwin; else stage platform FAIL 1; exit 1; fi ;;
  Linux/aarch64|Linux/arm64) INSTALL_TARGET=aarch64-unknown-linux-musl ;;
  Linux/x86_64) INSTALL_TARGET=x86_64-unknown-linux-musl ;;
  *) stage platform FAIL 1; exit 1 ;;
esac
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
[[ -n "$INSTALL_CODEX" ]] && "$INSTALL_CODEX" plugin marketplace upgrade --help >/dev/null || { stage native_support FAIL 1; exit 1; }
stage preflight PASS 0
binary="$INSTALL_ROOT/plugins/$INSTALL_PLUGIN/bin/$INSTALL_TARGET/$INSTALL_PLUGIN"
checked distribution_package "$binary" provider-smoke --plugin-root "$INSTALL_ROOT/plugins/$INSTALL_PLUGIN" --require-installed --json || exit 1
[[ $("$binary" --version) == "$INSTALL_PLUGIN $INSTALL_VERSION" ]] || { stage distribution_version FAIL 1; exit 1; }
INSTALL_PREVIOUS_PLUGINS=$(plugins) || { stage native_source FAIL 1; exit 1; }
INSTALL_PREVIOUS_SELECTED=$(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -c --arg id "$INSTALL_ID" '[.[]|select(.pluginId==$id)]|if length==0 then null elif length==1 then .[0] else error("duplicate") end') || exit 1
[[ $(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq --arg market "$INSTALL_MARKET" --arg id "$INSTALL_ID" '[.[]|select(.marketplaceName==$market and .pluginId!=$id)]|length') == 0 ]] || { stage other_product_migration_required FAIL 1; exit 1; }
selected_source=$(marketplace "$INSTALL_MARKET") || { stage native_source FAIL 1; exit 1; }
if [[ $(printf '%s' "$selected_source" | jq -r length) == 1 ]]; then
  INSTALL_URL=$(printf '%s' "$selected_source" | jq -r '.[0].marketplaceSource.source')
  old_root=$(printf '%s' "$selected_source" | jq -r '.[0].root')
  INSTALL_PREVIOUS_COMMIT=$(clean_head "$old_root" 1) || { stage native_source FAIL 1; exit 1; }
  if [[ "$INSTALL_PREVIOUS_SELECTED" != null ]]; then
    previous_version=$(printf '%s' "$INSTALL_PREVIOUS_SELECTED" | jq -r .version)
    [[ $(jq -r .version "$old_root/plugins/$INSTALL_PLUGIN/.codex-plugin/plugin.json") == "$previous_version" ]] &&
      cache_matches "$old_root" "$INSTALL_PLUGIN" "$previous_version" "$INSTALL_MARKET" || { stage existing_package_changed FAIL 1; exit 1; }
    jq -en --arg old "$previous_version" --arg new "$INSTALL_VERSION" '($old|split(".")|map(tonumber))<=($new|split(".")|map(tonumber))' >/dev/null || { stage downgrade FAIL 1; exit 1; }
  fi
elif [[ "$INSTALL_PREVIOUS_SELECTED" != null ]]; then stage native_source FAIL 1; exit 1; fi
if [[ "$product" == core ]]; then
  INSTALL_LEGACY_CORE=$(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -c '[.[]|select(.pluginId=="groundline@groundline")]|if length==0 then null else .[0] end')
  if [[ "$INSTALL_LEGACY_CORE" != null ]]; then
    [[ "$INSTALL_PREVIOUS_SELECTED" == null && $(printf '%s' "$INSTALL_LEGACY_CORE" | jq -r .enabled) == true ]] || { stage legacy_core_requires_review FAIL 1; exit 1; }
    legacy_source=$(marketplace groundline) || { stage native_source FAIL 1; exit 1; }
    legacy_root=$(printf '%s' "$legacy_source" | jq -er '.[0].root') || exit 1
    INSTALL_LEGACY_COMMIT=$(clean_head "$legacy_root" 1) || { stage native_source FAIL 1; exit 1; }
    legacy_version=$(printf '%s' "$INSTALL_LEGACY_CORE" | jq -r .version)
    cache_matches "$legacy_root" groundline "$legacy_version" groundline || { stage existing_package_changed FAIL 1; exit 1; }
    INSTALL_URL=$(printf '%s' "$legacy_source" | jq -r '.[0].marketplaceSource.source')
  fi
  if [[ -e "$INSTALL_HOME/groundline/learning/profile.json" || -L "$INSTALL_HOME/groundline/learning/profile.json" ]]; then
    jq -e '.enabled==false' "$INSTALL_HOME/groundline/learning/profile.json" >/dev/null || { stage learning_profile_requires_review FAIL 1; exit 1; }
  fi
else
  checked insights_server_compatibility "$binary" worker check-server --json || exit 1
fi
# Back up the actual config privately, before any native registration mutation.
[[ -d "$INSTALL_HOME" && ! -L "$INSTALL_HOME" && -O "$INSTALL_HOME" ]] || { stage owner_home FAIL 1; exit 1; }
for dir in "$INSTALL_HOME/groundline" "$INSTALL_HOME/groundline/install-backups"; do
  if [[ -e "$dir" || -L "$dir" ]]; then [[ -d "$dir" && ! -L "$dir" && -O "$dir" ]] || { stage backup FAIL 1; exit 1; }
  else mkdir "$dir" || { stage backup FAIL 1; exit 1; }; fi
done
backup=$(mktemp -d "$INSTALL_HOME/groundline/install-backups/$product.XXXXXXXX") || exit 1
if [[ -e "$INSTALL_HOME/config.toml" || -L "$INSTALL_HOME/config.toml" ]]; then
  [[ -f "$INSTALL_HOME/config.toml" && ! -L "$INSTALL_HOME/config.toml" && -O "$INSTALL_HOME/config.toml" ]] || { stage backup FAIL 1; exit 1; }
  cp -p "$INSTALL_HOME/config.toml" "$backup/config.toml" || exit 1
  chmod 0600 "$backup/config.toml" || exit 1
fi
INSTALL_ARMED=1
if [[ -n "$INSTALL_PREVIOUS_COMMIT" ]]; then checked marketplace_remove "$INSTALL_CODEX" plugin marketplace remove "$INSTALL_MARKET" --json || exit 1; fi
checked marketplace_add "$INSTALL_CODEX" plugin marketplace add "$INSTALL_URL" --ref "$INSTALL_COMMIT" --json || exit 1
checked marketplace_refresh "$INSTALL_CODEX" plugin marketplace upgrade "$INSTALL_MARKET" --json || exit 1
checked snapshot_revision snapshot_matches "$INSTALL_COMMIT" || exit 1
if [[ "$INSTALL_LEGACY_CORE" != null ]]; then
  INSTALL_LEGACY_REMOVED=1
  checked migrate_legacy_core "$INSTALL_CODEX" plugin remove groundline@groundline --json || exit 1
fi
if [[ "$INSTALL_PREVIOUS_SELECTED" == null ]]; then
  checked install_product "$INSTALL_CODEX" plugin add "$INSTALL_ID" --json || exit 1
else stage install_product PRESERVED 0; fi
selected_root=$(marketplace "$INSTALL_MARKET" | jq -er '.[0].root') || exit 1
cache="$INSTALL_HOME/plugins/cache/$INSTALL_MARKET/$INSTALL_PLUGIN/$INSTALL_VERSION"
checked installed_bytes cache_matches "$selected_root" "$INSTALL_PLUGIN" "$INSTALL_VERSION" "$INSTALL_MARKET" || exit 1
checked installed_package "$cache/bin/$INSTALL_TARGET/$INSTALL_PLUGIN" provider-smoke --plugin-root "$cache" --require-installed --json || exit 1
expected=$(printf '%s' "$INSTALL_PREVIOUS_PLUGINS" | jq -c --arg id "$INSTALL_ID" --arg plugin "$INSTALL_PLUGIN" --arg market "$INSTALL_MARKET" --arg version "$INSTALL_VERSION" --argjson legacy "$([[ "$INSTALL_LEGACY_CORE" != null ]] && printf true || printf false)" '
  ([.[]|select(.pluginId==$id)]|if length==0 then true else .[0].enabled end) as $enabled |
  [.[]|select(.pluginId!=$id and (if $legacy then .pluginId!="groundline@groundline" else true end))]+
    [{pluginId:$id,name:$plugin,marketplaceName:$market,version:$version,enabled:$enabled}] | sort_by(.pluginId)') || exit 1
[[ $(plugins) == "$expected" ]] || { stage plugin_choices FAIL 1; exit 1; }
stage plugin_choices PASS 0
INSTALL_ARMED=0
if [[ "$product" == core && ${#INSTALL_SETUP[@]} -gt 1 ]]; then
  INSTALL_TEMP=$(mktemp -d) || exit 1
  "$INSTALL_CODEX" debug models > "$INSTALL_TEMP/models.json" || { stage catalog FAIL 1; exit 1; }
  checked settings "$cache/bin/$INSTALL_TARGET/groundline" setup --catalog "$INSTALL_TEMP/models.json" "${INSTALL_SETUP[@]}" || exit 1
else stage settings PRESERVED 0; fi
if [[ "$product" == insights && ${#INSTALL_INSIGHTS[@]} -gt 1 ]]; then
  if [[ "$INSTALL_APP_BUNDLED" == 1 && -z ${GROUNDLINE_RUNTIME_FAMILY+x} && -z ${CODEX_INTERNAL_ORIGINATOR_OVERRIDE+x} ]]; then
    export GROUNDLINE_RUNTIME_FAMILY=codex_app
  fi
  checked insights_setup "$cache/bin/$INSTALL_TARGET/groundline-insights" setup "${INSTALL_INSIGHTS[@]}" || exit 1
else stage insights_setup PRESERVED 0; fi
if "$INSTALL_CODEX" --strict-config doctor --summary --no-color --ascii; then stage native_doctor PASS 0
else stage native_doctor ACTION_REQUIRED "$?"; fi
