#!/usr/bin/env bash
# Migrations are append-only.
#
# alpha-spec.md Appendix D #4 and final-build-spec.md 8.2: never rewrite, squash, or delete a
# historical migration to make a build pass. Only additions are permitted.
#
# Usage: check-migrations.sh [base-ref]     (default: origin/main)
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

DIR="migrations"
BASE="${1:-${BASE_REF:-origin/main}}"

if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
    # Append-only is a property OF A DIFF; with no base there is nothing to diff against.
    # This is a genuine gap in coverage, not a pass — say so rather than printing OK.
    #
    # In CI it is not a skip either, it is a failure. The second adversarial review found this
    # path running in both build jobs, which check out at fetch-depth 1 so origin/main does not
    # exist: the guard printed a stderr notice and exited 0, and the merge-authoritative green
    # therefore never included append-only verification. An exit 0 with a warning IS passing
    # silently. Locally, a shallow or detached checkout is a legitimate reason to skip.
    echo "check-migrations: SKIP (NOT VERIFIED) base ref '$BASE' unresolvable, so append-only" \
         "could not be checked. CI always has a base; a local skip is not evidence." >&2
    if [[ -n "${CI:-}" ]]; then
        echo "check-migrations: FAIL running under CI with no resolvable base ref." \
             "Give the job 'fetch-depth: 0' or pass an explicit base." >&2
        exit 1
    fi
    exit 0
fi

# D=deleted, M=modified, R=renamed. Additions (A) are the only permitted change.
die() { echo "check-migrations: FAIL $*" >&2; exit 1; }
# Fail closed: a git error must never be reported as "append-only satisfied".
raw="$(git diff --name-status --diff-filter=DMR "$BASE"...HEAD -- "$DIR")" \
    || die "git diff ${BASE}...HEAD failed — refusing to report append-only as satisfied"
v2="$(git diff --name-status --diff-filter=DMR -- "$DIR")"        || die "git diff (unstaged) failed"
v3="$(git diff --cached --name-status --diff-filter=DMR -- "$DIR")" || die "git diff --cached failed"
raw+=$'\n'"$v2"$'\n'"$v3"

# Append-only protects migrations that ALREADY EXIST IN THE BASE. A migration introduced by
# this branch is still a draft: amending it is not rewriting history, and blocking that would
# stop anyone from correcting their own unmerged migration. Only flag paths present in $BASE.
violations=""
while IFS=$'\t' read -r st path rest; do
    [[ -z "${path:-}" ]] && continue
    if git cat-file -e "${BASE}:${path}" 2>/dev/null; then
        violations+="$st"$'\t'"$path"$'\n'
    fi
done <<< "$raw"
violations="$(printf '%s\n' "$violations" | grep -vE '^[[:space:]]*$' || true)"

if [[ -n "$violations" ]]; then
    echo "check-migrations: FAIL migrations are append-only; these were modified or removed:" >&2
    printf '%s\n' "$violations" | sed 's/^/    /' >&2
    echo "  Add a new forward migration instead of editing history (alpha-spec.md Appendix D #4)." >&2
    exit 1
fi

count=$(find "$DIR" -name '*.sql' 2>/dev/null | wc -l)
echo "check-migrations: OK append-only satisfied ($count migration(s), base $BASE)"
