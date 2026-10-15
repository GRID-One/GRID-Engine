#!/usr/bin/env bash
# Scan for secret material.
#
# A security control MUST FAIL CLOSED. Every git invocation below aborts the run on error
# rather than reporting a clean scan over content that was never examined.
#
# Regression (found by adversarial review of PR #1): the base fallback used an empty-tree
# sentinel with `git diff <tree>...HEAD`. `...` requires two COMMITS, so git aborted with
# "Invalid symmetric difference expression"; a `|| true` swallowed the fatal and the guard
# printed OK with exit 0 while a committed ghp_ token sat in the tree. When no base commit
# is available the script now scans the ENTIRE tracked tree instead.
#
# Patterns live in scripts/secret-patterns.txt. That file and this one are excluded from the
# diff/tree/untracked passes so the detector cannot trigger on its own definitions -- but they
# are NOT unscanned. They were, and that made them the one place in the repository where a
# secret was never looked for (second adversarial review, minor 4). They now get a dedicated
# pass of their own, in full, on every run and in every mode. Inside that pass only, a line
# carrying the marker `# nosecret` is skipped, so a pattern that legitimately matches itself
# can be exempted line by line instead of exempting two whole files.
#
# The marker has no effect anywhere else: it is applied only to these two paths, so it cannot
# become a way to smuggle a secret past the scanner from ordinary source.
#
# Usage: check-secrets.sh [base-ref]     (default: origin/main)
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATTERNS="scripts/secret-patterns.txt"
[[ -f "$PATTERNS" ]] || { echo "check-secrets: FAIL missing $PATTERNS" >&2; exit 1; }

BASE="${1:-${BASE_REF:-origin/main}}"
SELF=(':(exclude)scripts/check-secrets.sh' ':(exclude)scripts/secret-patterns.txt')

die() { echo "check-secrets: FAIL $*" >&2; exit 1; }
plus() { grep -E '^\+' | grep -vE '^\+\+\+' || true; }   # added lines only; no match is fine

added=""
scanned=0
# Scratch file for an index blob (see scan_files). Removed on every exit path, including die.
BLOB="$(mktemp)" || die "mktemp failed"
trap 'rm -f "$BLOB"' EXIT

# Append every readable text file in a newline-separated list to the scan corpus.
#
# THE EMPTY-SCAN GUARD LIVES HERE, INSIDE THE FUNCTION, NOT AT THE CALL SITES.
#
# That placement is the whole point, and it was learned the hard way. Round 2 found scan_files
# opening zero of the files handed to it on Windows, and the fix added a guard -- to the one
# call site that had failed. Round 3 then found the OTHER call site (untracked files) unguarded
# in both arms of the if below: diff mode never checked it at all, and in full-tree mode the
# call-site guard runs BEFORE the untracked scan, so it only ever counted the tracked pass.
# Same defect, third instance, because each fix was applied to the instance and not to the shape.
#
# A guard at a call site protects that one call. A guard inside the function protects every
# call, including ones nobody has written yet.
#
# `f="${f%$'\r'}"`: a carriage return on a path silently fails the -f test, so the file is
# skipped and the scan reports a clean corpus over content it never opened. That is the exact
# mechanism that blinded the Windows merge gate.
scan_files() {
    local label="$1" list="$2" f src unres=""
    local listed=0 excluded=0 skipped=0 opened=0
    while IFS= read -r f; do
        f="${f%$'\r'}"
        [[ -z "$f" ]] && continue
        listed=$((listed + 1))
        case "$f" in
            scripts/check-secrets.sh|scripts/secret-patterns.txt)
                excluded=$((excluded + 1)); continue ;;   # scanned separately, in full
        esac
        # A TRACKED path deleted in the worktree (an unstaged `rm`) is not unresolvable: the index
        # still holds the content a commit of this tree would carry, so that is what is scanned.
        # Only the full-tree pass lists tracked paths; an untracked path has no index entry, so a
        # vanished or mangled untracked path still falls through to the unresolvable branch.
        # Before R4-1 this case was skipped silently; after it, without this, it would be fatal.
        src="$f"
        if [[ ! -e "$f" ]] && git cat-file -e ":$f" 2>/dev/null; then
            git cat-file blob ":$f" > "$BLOB" || die "$label: could not read the index copy of $f"
            src="$BLOB"
        fi
        [[ -f "$src" ]] || { unres+="$f"$'\n'; continue; }  # UNRESOLVABLE -- counted below; fatal (R4-1)
        if ! grep -Iq . "$src" 2>/dev/null; then
            skipped=$((skipped + 1)); continue             # binary or empty: deliberate
        fi
        added+=$'\n'"$(sed 's/^/+/' "$src")"
        opened=$((opened + 1))
        scanned=$((scanned + 1))
    done <<< "$list"

    # Every listed path must be ACCOUNTED FOR: opened, or a deliberate skip -- the detector's own
    # two files, or a binary/empty file. A path that simply would not resolve is neither, and the
    # scanner could not reach content it was asked to examine.
    #
    # R4-1 (round-4 adversarial review, major). This guard used to fire only when NOTHING was
    # opened (`listed > 0 && opened == 0 && accounted < listed`), so a PARTIAL failure passed
    # silently: two clean untracked files with one made unresolvable printed "OK ... 1 whole
    # file(s)" and exited 0 with a path never read. A subset failing is the same shape as all of
    # them failing -- a filename the loop mishandles, a file removed between listing and reading,
    # a permission error -- so the guard is now on the count of unresolved paths, not on zero.
    # The values were already in hand.
    #
    # Still stated as "accounted for" rather than "scanned > 0" so an untracked set that is
    # genuinely all-binary passes, while the CR-on-every-path signature still dies.
    local accounted=$((excluded + skipped)) unresolved=$((listed - excluded - skipped - opened))
    if [[ "$unresolved" -gt 0 ]]; then
        die "$label: $unresolved of $listed path(s) could not be opened and are not deliberate
       skips ($opened opened, $accounted accounted for as deliberate skips$([[ "$opened" -eq 0 ]] && echo ", ZERO opened")).
       Refusing to report OK over content that was never examined. First unopened path(s):
$(printf '%s' "$unres" | head -5 | sed 's/^/         /')"
    fi
}

if git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
    mode="diff vs $BASE"
    d="$(git diff "$BASE"...HEAD --unified=0 -- . "${SELF[@]}")" \
        || die "git diff ${BASE}...HEAD failed — refusing to report a clean scan"
    diff_lines="$(printf '%s\n' "$d" | plus)"
    added+=$'\n'"$diff_lines"
    hunk_lines=$(printf '%s\n' "$diff_lines" | grep -c . || true)

    # The same guard the full-tree branch has, on the branch CI actually takes. The full-tree
    # empty-scan check was added after the Windows fail-open; this `if`'s other arm went without
    # one for a round, which is the same defect class sitting untouched next to its own fix.
    #
    # Deliberately compares against CHANGED FILES, not against the diff text: an empty diff is a
    # legitimate clean result, but files changed with no added lines collected means the scanner
    # parsed nothing out of a real change set.
    changed="$(git diff --name-only "$BASE"...HEAD -- . "${SELF[@]}")" \
        || die "git diff --name-only failed — refusing to report a clean scan"
    changed_n=$(printf '%s\n' "$changed" | grep -c . || true)
    if [[ "$changed_n" -gt 0 && "$hunk_lines" -eq 0 ]]; then
        die "the diff against $BASE touches $changed_n file(s) but ZERO added lines were
       collected. The scanner parsed nothing out of a real change set; that is not a clean
       diff. Refusing to report OK."
    fi
else
    # No usable base: fresh clone, shallow checkout, or an unborn branch. Scan everything
    # tracked. Scanning less than the diff would be a silent downgrade of the control.
    mode="FULL tracked tree (base '$BASE' unresolvable)"
    files="$(git ls-files)" || die "git ls-files failed — refusing to report a clean scan"
    # No call-site empty-scan check here any more: scan_files enforces it for every caller.
    # This site used to carry the only copy, which is precisely why the untracked call went
    # unguarded for two rounds. One guard, in one place, covering both.
    scan_files "tracked tree" "$files"
fi

# Staged and unstaged work, so a secret is caught before it is ever committed.
u="$(git diff --unified=0 -- . "${SELF[@]}")"        || die "git diff (unstaged) failed"
s="$(git diff --cached --unified=0 -- . "${SELF[@]}")" || die "git diff --cached failed"
added+=$'\n'"$(printf '%s\n' "$u" | plus)"
added+=$'\n'"$(printf '%s\n' "$s" | plus)"

# The detector's own two files, always and in full. `# nosecret` exempts a single line here.
for f in scripts/check-secrets.sh "$PATTERNS"; do
    [[ -f "$f" ]] || continue
    added+=$'\n'"$(grep -v '# nosecret' "$f" | sed 's/^/+/')"
done

# Untracked files are in no diff, but a new file full of secrets is exactly this guard's job.
# This call runs in BOTH modes and was the unguarded one: in diff mode nothing checked it, and
# in full-tree mode the old call-site guard had already run, several lines above, counting only
# the tracked pass. scan_files now guards itself, so this site is covered by construction.
untracked="$(git ls-files --others --exclude-standard)" || die "git ls-files --others failed"
scan_files "untracked files" "$untracked"

# Two tiers, separated by the #WARN# line in the pattern file. Deny fails; warn reports.
# The warn tier exists because the deny patterns require realistic token lengths, so a
# truncated or short test token slips through -- narrower than the acceptance criterion's
# literal "patterns matching ghp_, sk-ant-". Reporting it is better than either gating on a
# noisy pattern or pretending the gap is not there.
status=0
warned=0
tier="deny"
while IFS= read -r pattern; do
    pattern="${pattern%$'\r'}"   # a CRLF pattern file would append \r to every regex
    if [[ "$pattern" == "#WARN#" ]]; then tier="warn"; continue; fi
    [[ -z "$pattern" || "$pattern" == \#* ]] && continue
    if hits="$(printf '%s\n' "$added" | grep -nE -e "$pattern" || true)"; [[ -n "$hits" ]]; then
        if [[ "$tier" == "deny" ]]; then
            echo "check-secrets: FAIL pattern matched: $pattern" >&2
            printf '%s\n' "$hits" | head -3 | sed 's/^/    /' >&2
            status=1
        else
            echo "check-secrets: WARN low-confidence match (not failing): $pattern" >&2
            printf '%s\n' "$hits" | head -3 | sed 's/^/    /' >&2
            warned=$((warned+1))
        fi
    fi
done < "$PATTERNS"

if [[ "$status" -eq 0 ]]; then
    # The file count is part of the verdict: "OK" over an empty corpus is what the Windows
    # fail-open looked like, and an unqualified OK gives a reader no way to notice.
    # The size of the corpus is part of the verdict. An unqualified OK is what the Windows
    # fail-open looked like, and each mode measures a different thing: diff mode reads added
    # lines out of hunks, full-tree mode opens whole files. Say which, and how many.
    if [[ "${hunk_lines:-}" != "" ]]; then
        size="${hunk_lines} added line(s) from ${changed_n} changed file(s), plus $scanned whole file(s)"
    else
        size="$scanned whole file(s) read"
    fi
    echo "check-secrets: OK no secret patterns found ($mode; $size)$([[ "$warned" -gt 0 ]] && echo ", $warned warn-tier match(es) above")"
else
    echo "check-secrets: secrets must never be committed (alpha-spec.md 14.1)." >&2
    echo "  If this is a false positive, narrow the pattern in $PATTERNS — never delete the check." >&2
fi
exit "$status"
