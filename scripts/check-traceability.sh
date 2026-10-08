#!/usr/bin/env bash
# Every non-trivial change traces to an approved work package or ADR.
#
# alpha-spec.md 9.4: "Every merged non-trivial change is linked to an approved work package
# and evidence manifest." 12.8 requires the PR body to carry the work-package link.
#
# PER-FILE, not per-PR. An earlier revision passed the whole change set if a single token like
# "P1-00" appeared anywhere in the commit range -- a PR touching 200 unrelated files passed on
# one commit-subject prefix, while the manifest recorded the check as satisfied. Adversarial
# review of PR #1 caught that. Each non-trivial path must now be accounted for individually.
#
# A path is accounted for when it is named, or covered by a named directory prefix, in a
# referenced work package or ADR. Referenced documents are those cited in the commit range or
# $PR_BODY / $PR_BODY_FILE.
#
# Usage: check-traceability.sh [base-ref]     (default: origin/main)
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

BASE="${1:-${BASE_REF:-origin/main}}"
die() { echo "check-traceability: FAIL $*" >&2; exit 1; }

if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
    # Same fail-open-in-CI hazard as check-migrations: see the comment there. A shallow local
    # checkout may skip; a CI job with no base ref is a misconfigured gate, not a passing one.
    echo "check-traceability: SKIP (NOT VERIFIED) base ref '$BASE' unresolvable, so no change" \
         "set could be derived. CI always has a base; a local skip is not evidence." >&2
    if [[ -n "${CI:-}" ]]; then
        echo "check-traceability: FAIL running under CI with no resolvable base ref." \
             "Give the job 'fetch-depth: 0' or pass an explicit base." >&2
        exit 1
    fi
    exit 0
fi

# Trivial paths: excluded by the P1-00 acceptance criteria, plus generated artifacts.
is_trivial() {
    case "$1" in
        docs/99-templates/*|.github/*|*.lock|Cargo.lock|.sqlx/*|\
        .gitignore|.gitattributes|*/.gitkeep|.gitkeep|.ai/evidence/*) return 0 ;;
        *) return 1 ;;
    esac
}

changed="$( { git diff --name-only "$BASE"...HEAD || die "git diff failed"
              git diff --name-only        || die "git diff (unstaged) failed"
              git diff --cached --name-only || die "git diff --cached failed"
              git ls-files --others --exclude-standard || die "git ls-files --others failed"
            } | sort -u | grep -vE '^$' || true)"

nontrivial=()
while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    is_trivial "$f" || nontrivial+=("$f")
done <<< "$changed"

if [[ ${#nontrivial[@]} -eq 0 ]]; then
    echo "check-traceability: OK no non-trivial changes to trace (base $BASE)"
    exit 0
fi

# Which authority documents does this change set cite?
#
# Bare work-package IDs are P<phase>-<NN>: P0-01 (the engine consolidation), P1-NN, P2-NN. The
# pattern was P1-[0-9]{2} until P0-01, so the consolidation's own ID -- and every P2 package --
# was not a reference at all (DR-A5, ADR-011). Widening the ID syntax changes which documents a
# commit may CITE; it does not change what citing one buys. Coverage is still per path, against
# the text of the documents that actually exist, through covered() below, which is unchanged.
haystack="$(git log --format='%s%n%b' "$BASE"..HEAD 2>/dev/null || true)"
haystack+=$'\n'"${PR_BODY:-}"
[[ -n "${PR_BODY_FILE:-}" && -f "${PR_BODY_FILE:-}" ]] && haystack+=$'\n'"$(cat "$PR_BODY_FILE")"

refs="$(printf '%s' "$haystack" \
        | grep -oE 'docs/01-work-packages/[A-Za-z0-9._-]+\.md|docs/02-adr/[0-9]{3}[A-Za-z0-9._-]*\.md|P[0-9]-[0-9]{2}|ADR-[0-9]{3}' \
        | sort -u || true)"
[[ -n "$refs" ]] || die "no work-package or ADR reference found in the commit range or PR_BODY.
  Reference the governing document, e.g. \"P1-00: add authority index\".
  Required by alpha-spec.md 9.4 and 12.8."

# Resolve bare IDs (P0-01, P1-00, ADR-001) to the documents they name, then read them all.
docs=()
while IFS= read -r r; do
    [[ -z "$r" ]] && continue
    case "$r" in
        docs/*) [[ -f "$r" ]] && docs+=("$r") ;;
        P[0-9]-*) for f in docs/01-work-packages/*"$(tr '[:upper:]' '[:lower:]' <<< "$r")"*; do
                    [[ -f "$f" ]] && docs+=("$f"); done ;;
        ADR-*)  for f in docs/02-adr/"${r#ADR-}"*.md; do [[ -f "$f" ]] && docs+=("$f"); done ;;
    esac
done <<< "$refs"
[[ ${#docs[@]} -gt 0 ]] || die "referenced documents ($(tr '\n' ' ' <<< "$refs")) do not exist on disk."

corpus="$(cat "${docs[@]}" 2>/dev/null)" || die "could not read referenced documents"

# A path is covered if it is named exactly, or if an ancestor is named AS A DIRECTORY.
#
# The directory form must end at the slash — followed by whitespace, a closing delimiter, or a
# glob. Otherwise "src/" matches as a substring of "src/covered.rs" and every sibling in that
# directory is silently covered by one named file. tests/guards/run.sh caught exactly that.
covered() {
    local p="$1"
    grep -qF -- "$p" <<< "$corpus" && return 0
    local d; d="$(dirname "$p")"
    while [[ "$d" != "." && "$d" != "/" ]]; do
        grep -qE -- "(^|[^A-Za-z0-9._/-])${d}/([[:space:]*\`,\")]|$)" <<< "$corpus" && return 0
        d="$(dirname "$d")"
    done
    return 1
}

untraced=()
for f in "${nontrivial[@]}"; do covered "$f" || untraced+=("$f"); done

if [[ ${#untraced[@]} -gt 0 ]]; then
    echo "check-traceability: FAIL ${#untraced[@]} of ${#nontrivial[@]} non-trivial path(s) are not" \
         "named, and have no named parent directory, in any referenced document:" >&2
    printf '%s\n' "${untraced[@]}" | head -15 | sed 's/^/    /' >&2
    [[ ${#untraced[@]} -gt 15 ]] && echo "    ... and $(( ${#untraced[@]} - 15 )) more" >&2
    echo "  referenced: $(tr '\n' ' ' <<< "$refs")" >&2
    echo "  Add the path to the work package's Scope section, or record it in an ADR." >&2
    exit 1
fi

echo "check-traceability: OK all ${#nontrivial[@]} non-trivial path(s) traced to: $(tr '\n' ' ' <<< "$refs")"
