#!/usr/bin/env bash
# Emit the AI contribution evidence manifest required by alpha-spec.md 8.12.
#
# Model and harness identity are READ FROM ai-toolchain.lock at runtime, never hard-coded:
# 1.3 requires the model be referenced through a configurable alias rather than an assumed
# public API identifier embedded in source.
#
# Usage: generate-evidence-manifest.sh <WORK_PACKAGE_ID> [--base <ref>]
#
# Fields the operator must supply are emitted as explicit nulls or empty arrays rather than
# invented. A manifest that guesses its reviewer or its verification results is worse than
# one that admits it does not know.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

WP_ID="${1:-}"
if [[ -z "$WP_ID" ]]; then
    echo "usage: $0 <WORK_PACKAGE_ID> [--base <ref>]" >&2
    exit 2
fi
shift
# The ID becomes a directory below (.ai/evidence/<WP_ID>), so it is held to the work-package ID
# scheme first: P<phase>-<NN>, e.g. P0-01 or P1-00 (DR-A5). check-evidence-claims.sh applies the
# same rule, so a record this writes is always one that checker can select.
if [[ ! "$WP_ID" =~ ^P[0-9]-[0-9]{2}$ ]]; then
    echo "generate-evidence-manifest: '$WP_ID' is not a work-package ID (P<phase>-<NN>)" >&2
    exit 2
fi

BASE="${BASE_REF:-origin/main}"
INPUT=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --base) BASE="${2:?--base needs a ref}"; shift 2 ;;
        --input) INPUT="${2:?--input needs a file}"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

command -v jq >/dev/null || { echo "generate-evidence-manifest: jq is required" >&2; exit 1; }

# Newline-list on stdin -> JSON array. Emits [] when empty.
# Deliberately NOT `... | jq -s . || echo '[]'`: under `set -o pipefail` a filtered-empty
# pipeline still emits [] AND then the fallback appends a second [], producing "[]\n[]".
to_json_array() {
    local input; input="$(cat)"
    if [[ -z "${input//[[:space:]]/}" ]]; then printf '[]'; else printf '%s\n' "$input" | jq -R . | jq -s .; fi
}

lock_value() {  # read a top-level scalar from ai-toolchain.lock without a YAML parser
    [[ -f ai-toolchain.lock ]] || { echo ""; return; }
    sed -n "s/^$1:[[:space:]]*//p" ai-toolchain.lock | head -1 | tr -d '\r' | sed 's/^"//; s/"$//'
}

SHA="$(git rev-parse HEAD)"
PENDING="$( { git diff --name-only; git diff --cached --name-only
              git ls-files --others --exclude-standard; } | sort -u | grep -vE '^$' | wc -l)"
if [[ "$PENDING" -gt 0 ]]; then
    COMMIT_STATE="uncommitted: $PENDING path(s) not yet in $SHA"
else
    COMMIT_STATE="clean"
fi
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Whole commit series plus uncommitted work, not just HEAD~1..HEAD.
if git rev-parse --verify --quiet "$BASE" >/dev/null; then
    RANGE="$BASE...HEAD"
    FILES="$( { git diff --name-only "$BASE"...HEAD
                git diff --name-only; git diff --cached --name-only
                git ls-files --others --exclude-standard; } | sort -u | grep -vE '^$' || true)"
else
    RANGE="(base '$BASE' unavailable)"
    FILES="$(git ls-files | sort -u)"
fi

WP_FILE="$(ls docs/01-work-packages/*"$(tr '[:upper:]' '[:lower:]' <<< "$WP_ID")"* 2>/dev/null | head -1 || true)"
ADRS="$(ls docs/02-adr/[0-9]*.md 2>/dev/null || true)"
# Listed the same way as the ADRs: every document of the kind on disk, README excluded. These
# were hard-coded [] while the directories were empty; P0-01 is the first package with any.
CONTRACTS="$(ls docs/03-contracts/*.md 2>/dev/null | grep -v '/README\.md$' || true)"
MODEL_SPECS="$(ls docs/05-model-specs/*.md 2>/dev/null | grep -v '/README\.md$' || true)"
MIGRATIONS="$( { git diff --name-only "$BASE"...HEAD -- migrations 2>/dev/null || true
                 git diff --name-only -- migrations 2>/dev/null || true
                 git diff --cached --name-only -- migrations 2>/dev/null || true
                 git ls-files --others --exclude-standard -- migrations 2>/dev/null || true
               } | sort -u | grep -vE '^$' || true)"

OUT_DIR=".ai/evidence/$WP_ID"
mkdir -p "$OUT_DIR"

# `just evidence <WP>` passes no --input. Without this, re-running the documented command
# would regenerate an empty skeleton over real recorded results. Auto-detect the conventional
# location so the command is idempotent.
if [[ -z "$INPUT" && -f "$OUT_DIR/verification-input.json" ]]; then
    INPUT="$OUT_DIR/verification-input.json"
fi

jq -n \
  --arg wp "$WP_ID" \
  --arg wp_file "$WP_FILE" \
  --arg commit "$SHA" \
  --arg branch "$BRANCH" \
  --arg ts "$TIMESTAMP" \
  --arg range "$RANGE" \
  --arg commit_state "$COMMIT_STATE" \
  --arg alias "$(lock_value project_alias)" \
  --arg model "$(lock_value model_public_id)" \
  --arg harness "$(lock_value harness)" \
  --arg harness_version "$(lock_value harness_version)" \
  --arg profile "$(lock_value permission_profile)" \
  --arg retention "$(lock_value retention)" \
  --arg os "$(uname -srm)" \
  --arg rustc "$(rustc --version 2>/dev/null || echo unknown)" \
  --arg cargo "$(cargo --version 2>/dev/null || echo unknown)" \
  --argjson files "$(printf '%s' "$FILES" | to_json_array)" \
  --argjson adrs "$(printf '%s' "$ADRS" | to_json_array)" \
  --argjson contracts "$(printf '%s' "$CONTRACTS" | to_json_array)" \
  --argjson model_specs "$(printf '%s' "$MODEL_SPECS" | to_json_array)" \
  --argjson migrations "$(printf '%s' "$MIGRATIONS" | to_json_array)" \
'{
  schema: "grid-alpha/evidence-manifest/1",
  work_package_id: $wp,
  commit: $commit,
  branch: $branch,
  model: $alias,
  work_package: { id: $wp, file: (if $wp_file == "" then null else $wp_file end) },
  commit_info: { sha: $commit, branch: $branch, range: $range, generated_at: $ts, state: $commit_state },
  model_and_harness: {
    project_alias: $alias, provider_model_id: $model,
    harness: $harness, harness_version: $harness_version,
    permission_profile: $profile, retention_policy: $retention,
    source: "ai-toolchain.lock"
  },
  environment: { os: $os, rustc: $rustc, cargo: $cargo },
  files_changed: $files,
  contracts_and_adrs_referenced: { adrs: $adrs, contracts: $contracts, model_specs: $model_specs },
  migrations_changed: $migrations,
  verification: {
    commands: [],
    note: "Populate from an actual run. A command that was not executed is never recorded as passing (alpha-spec.md 8.11)."
  },
  review: { reviewer_type: null, reviewer: null, findings: [], resolved: [] },
  human_approvals: { required: [], obtained: [] },
  known_limitations: [],
  follow_up: []
}' > "$OUT_DIR/manifest.json"

# Operator-supplied results (verification runs, reviewer, approvals, limitations) are merged
# in from a file rather than hand-edited into the manifest afterwards: a manifest that someone
# typed into is not evidence. The file itself is committed alongside, so the merge is auditable.
if [[ -n "$INPUT" ]]; then
    [[ -f "$INPUT" ]] || { echo "generate-evidence-manifest: no such input file: $INPUT" >&2; exit 1; }
    jq empty "$INPUT" || { echo "generate-evidence-manifest: $INPUT is not valid JSON" >&2; exit 1; }
    tmp="$(mktemp)"
    jq -s '.[0] * .[1]' "$OUT_DIR/manifest.json" "$INPUT" > "$tmp"
    mv "$tmp" "$OUT_DIR/manifest.json"
    echo "generate-evidence-manifest: merged operator input from $INPUT"
fi

# verification_status is DERIVED from the recorded exit statuses (alpha-spec.md 8.11), never
# asserted. An earlier revision returned a flat "passed" whenever the canonical command passed,
# even with failures and un-run checks counted right beneath it -- a green headline over a
# failure, which is what the counts were supposed to prevent. Review finding "minor 1".
# "passed_with_exceptions" now surfaces that in the headline itself.
tmp="$(mktemp)"
jq '
  (.verification.commands // []) as $c
  | ($c | map(select(.canonical == true)) | first) as $canon
  | .verification_status = (
      if   $canon == null                                          then "unknown"
      elif $canon.exit_status != 0                                 then "failed"
      elif ($c | any(.exit_status != null and .exit_status != 0))  then "passed_with_exceptions"
      elif ($c | any(.exit_status == null))                        then "passed_with_exceptions"
      else "passed" end)
  | .verification_summary = {
      canonical_command: ($canon.command // null),
      canonical_exit_status: ($canon.exit_status // null),
      total_recorded: ($c | length),
      passed:  ($c | map(select(.exit_status == 0)) | length),
      failed:  ($c | map(select(.exit_status != null and .exit_status != 0)) | length),
      not_run: ($c | map(select(.exit_status == null)) | length)
    }' "$OUT_DIR/manifest.json" > "$tmp"
mv "$tmp" "$OUT_DIR/manifest.json"

jq empty "$OUT_DIR/manifest.json"
echo "generate-evidence-manifest: wrote $OUT_DIR/manifest.json ($(jq '.files_changed | length' "$OUT_DIR/manifest.json") files, range $RANGE)"
echo "  Verification results, reviewer, approvals and limitations must be filled in from a real run."
