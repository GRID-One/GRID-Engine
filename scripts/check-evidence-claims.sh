#!/usr/bin/env bash
# Re-derive every mechanically-checkable claim in the evidence record from the artifact it
# describes, and fail on any mismatch.
#
# WHY THIS EXISTS. Round 2's M2 was "six checkably-false statements in the completion record".
# The remedy was a claim-verification pass -- which worked, and caught several more over the
# following rounds. But it was a routine the implementer ran from memory over nine facts, so a
# tenth fact ("All 65 non-trivial paths traced", actually 66) sat permanently outside it and
# survived three rounds. A pass you have to remember to extend is not a control. Round 3, min-1.
#
# WHY IT IS NOT PART OF `just verify`. The manifest records the OUTPUT of `just verify`. If
# verify required the manifest to already be current, neither could ever converge: you would
# need the manifest to run verify, and verify's result to write the manifest. So this runs
# separately -- from `just evidence`, from CI's guards job, and by hand before committing
# evidence. Keeping it out of the frozen chain is deliberate, not an oversight, and it needs no
# D5 amendment for the same reason.
#
# WHAT IT DELIBERATELY DOES NOT CHECK. That the manifest's commit equals HEAD. A manifest cannot
# record its own SHA, so the evidence commit always follows the commit it attests to. Asserting
# equality here would encode a falsehood as a rule.
#
# WHICH RECORD. Until P0-01 this file hard-coded WP="P1-00" and the literal "/16", so it could
# only ever check the P1-00 record -- against whatever tree it ran on. On the engine-only pivot
# (ADR-011) that failed five claims that were true of the tree P1-00 attested to, while the
# pivot's OWN record could not be checked at all. Each work package's evidence is its own record
# (alpha-spec.md 8.12), so the record is now chosen, never assumed:
#   1. an explicit WP: the second argument, else $EVIDENCE_WP;
#   2. otherwise the ONE evidence record this change set adds or modifies (diff BASE...HEAD plus
#      staged, unstaged and untracked work). If several are in the change set -- a branch that
#      stacks an unmerged package's commits -- the newest wins: uncommitted work first, then the
#      latest commit touching .ai/evidence/. A tie at that point is ambiguous and FAILS;
#   3. otherwise the record most recently ADDED in history, so a change set that carries no
#      evidence of its own is held to the latest record and fails if it moved a counted fact;
#   4. otherwise P1-00, the original behaviour (which then fails on "missing").
# A record the BASE already holds is history: a change set that adds, modifies or deletes any
# file in it FAILS outright (alpha-spec.md 8.12; bootstrap inventory 3.6: never "fix" a check by
# rewriting an earlier record). Without that rule, step 2 would select a rewritten P1-00 record
# that had been made true of a later tree -- and pass it.
# Called with no arguments on the P1-00 tree, steps 2-4 all select P1-00: backward-compatible.
# This is not a relaxation. Whatever record is selected must be true, claim by claim, against
# the tree this runs on, and every claim is still re-derived from the artifact it describes.
#
# Usage: check-evidence-claims.sh [base-ref] [WP-ID]   (defaults: origin/main; selected as above)
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

BASE="${1:-${BASE_REF:-origin/main}}"
WP="${2:-${EVIDENCE_WP:-}}"

base_ok=0
git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null && base_ok=1

# Every evidence record (directory name) this change set touches: committed range, staged,
# unstaged and untracked. --no-renames so a renamed-away file still names its old record.
touched_records() {
    [[ "$base_ok" -eq 1 ]] || return 0
    { git diff --no-renames --name-only "$BASE"...HEAD -- .ai/evidence
      git diff --no-renames --name-only -- .ai/evidence
      git diff --no-renames --cached --name-only -- .ai/evidence
      git ls-files --others --exclude-standard -- .ai/evidence
    } | sed -n 's|^\.ai/evidence/\([^/]*\)/.*|\1|p' | sort -u
}

if [[ "$base_ok" -eq 1 ]]; then
    historical="$(touched_records | while IFS= read -r w; do
                      git cat-file -e "$BASE:.ai/evidence/$w" 2>/dev/null && echo "$w"; done || true)"
    if [[ -n "$historical" ]]; then
        echo "check-evidence-claims: FAIL this change set alters evidence the base already holds:" \
             "$(tr '\n' ' ' <<< "$historical")" >&2
        echo "  A merged work package's record attests to ITS final commit and is never rewritten" >&2
        echo "  (alpha-spec.md 8.12). Record later facts in this change set's own evidence." >&2
        exit 1
    fi
fi

select_wp() {
    local touched n newest
    touched="$(touched_records | while IFS= read -r w; do
                   [[ -f ".ai/evidence/$w/PR-BODY.md" ]] && echo "$w"; done || true)"
    n=$(printf '%s' "$touched" | grep -c . || true)
    if [[ "$n" -eq 1 ]]; then printf '%s\n' "$touched"; return; fi
    if [[ "$n" -gt 1 ]]; then
        newest="$( { git diff --no-renames --name-only -- .ai/evidence; git diff --no-renames --cached --name-only -- .ai/evidence
                    git ls-files --others --exclude-standard -- .ai/evidence; } \
                  | sed -n 's|^\.ai/evidence/\([^/]*\)/.*|\1|p' | sort -u | grep -Fxf <(printf '%s\n' "$touched") || true)"
        if [[ -z "$newest" ]]; then
            local c; c="$(git log -1 --format=%H "$BASE"..HEAD -- .ai/evidence)"
            newest="$(git show --no-renames --format= --name-only "$c" -- .ai/evidence \
                      | sed -n 's|^\.ai/evidence/\([^/]*\)/.*|\1|p' | sort -u | grep -Fxf <(printf '%s\n' "$touched") || true)"
        fi
        if [[ "$(printf '%s' "$newest" | grep -c . || true)" -ne 1 ]]; then
            echo "check-evidence-claims: FAIL the change set carries $n evidence records ($(tr '\n' ' ' <<< "$touched"))" >&2
            echo "  and the newest change does not single one out. Name it: $0 <base> <WP-ID>." >&2
            return 1
        fi
        echo "check-evidence-claims: NOTE $n evidence records in the change set; checking the newest," \
             "$newest. The others attest to earlier commits and are not re-derived here." >&2
        printf '%s\n' "$newest"; return
    fi
    newest="$(git log --diff-filter=A --format= --name-only -- '.ai/evidence/*/PR-BODY.md' 2>/dev/null \
              | sed -n 's|^\.ai/evidence/\([^/]*\)/PR-BODY\.md$|\1|p' \
              | while IFS= read -r w; do [[ -f ".ai/evidence/$w/PR-BODY.md" ]] && echo "$w"; done | head -1 || true)"
    printf '%s\n' "${newest:-P1-00}"
}

if [[ -z "$WP" ]]; then
    WP="$(select_wp)" || exit 1
fi
# The ID becomes a path below, so it is held to the work-package ID scheme (DR-A5) first.
[[ "$WP" =~ ^P[0-9]-[0-9]{2}$ ]] || { echo "check-evidence-claims: FAIL '$WP' is not a work-package ID (P<phase>-<NN>)" >&2; exit 1; }
echo "check-evidence-claims: checking the $WP record"
BODY=".ai/evidence/$WP/PR-BODY.md"
MANIFEST=".ai/evidence/$WP/manifest.json"

for f in "$BODY" "$MANIFEST"; do
    [[ -f "$f" ]] || { echo "check-evidence-claims: FAIL missing $f" >&2; exit 1; }
done

status=0
checked=0

# claim <label> <claimed> <actual> -- every comparison goes through here so the count of checks
# performed is itself reported. A pass that silently checked nothing is the failure mode this
# whole file is about.
claim() {
    local label="$1" claimed="$2" actual="$3"
    checked=$((checked + 1))
    if [[ "$claimed" != "$actual" ]]; then
        echo "check-evidence-claims: FAIL $label" >&2
        echo "    record says : $claimed" >&2
        echo "    artifact says: $actual" >&2
        status=1
    fi
}

# Pull a single capture group out of the body; empty if the phrasing moved.
# Delimiter is \x01, not /, because several of the patterns below contain a literal slash
# ("(15/15)") and a / delimiter turns those into "unknown option to `s'".
from_body() { sed -n $'s\x01.*'"$1"$'.*\x01\\1\x01p' "$BODY" | head -1; }

# 1. Parity step count.
parity_actual="$(./scripts/check-verify-parity.sh | sed -n 's/.*agree on \([0-9]\+\) verification step.*/\1/p')"
claim "parity step count in $BODY" "$(from_body 'agree at \*\*\([0-9]\+\) steps\*\*')" "$parity_actual"

# 2. Guard-suite case count. Run the suite and read its own verdict; nothing else knows it.
suite_actual="$(./tests/guards/run.sh | tail -1 | sed -n 's/^\([0-9]\+\) passed.*/\1/p')"
claim "guard-suite case count in $BODY" "$(from_body 'carries \([0-9]\+\) committed behaviour cases')" "$suite_actual"

# 3. Recipes in the verify chain.
recipes_actual="$(awk '/^verify:/{i=1;next} i&&/^[^[:space:]]/{i=0} i&&/just [a-z-]+/{n++} END{print n+0}' justfile)"
claim "verify recipe count in $BODY" "$(from_body 'runs \*\*\([0-9]\+\) recipes\*\*')" "$recipes_actual"

# 4. Dependency graph: crate count, and the RUSTSEC-2023-0071 remediation (ADR-003).
claim "Cargo.lock crate count in $BODY" \
      "$(from_body 'resolves \([0-9]\+\) crates')" \
      "$(grep -c '^\[\[package\]\]' Cargo.lock)"
claim "rsa crates in Cargo.lock (ADR-003 must keep this at 0)" "0" "$(grep -c '^name = "rsa"$' Cargo.lock || true)"

# 5. Assert-Ok coverage (ADR-009), recorded as "`Assert-Ok` (N/M)": N guarded of M steps.
# M was the literal 16 inside the pattern -- P1-00's step count frozen into the checker, so any
# later record failed it and nothing ever compared the 16 itself to anything. Both numbers are now
# read from the record, and BOTH are re-derived: N against the Assert-Ok calls in verify.ps1, M
# against the parity step count, because every verification step must be guarded. That is one
# claim more than before, and the floor below rises with it.
claim "Assert-Ok count in $BODY" \
      "$(from_body 'Assert-Ok`\{0,1\} (\([0-9]\+\)/[0-9]\+)')" \
      "$(grep -c 'Assert-Ok "' scripts/verify.ps1)"
claim "Assert-Ok denominator (verification steps) in $BODY" \
      "$(from_body 'Assert-Ok`\{0,1\} ([0-9]\+/\([0-9]\+\))')" \
      "$parity_actual"

# 6. The manifest hash the body quotes must be the hash of the manifest beside it.
claim "manifest hash quoted in $BODY" \
      "$(sed -n 's/.*sha256:\([0-9a-f]\{64\}\).*/\1/p' "$BODY" | head -1)" \
      "$(sha256sum "$MANIFEST" | cut -d' ' -f1)"

# 7. The traceability count in the manifest's own narrative. THE claim that started this file:
# it is derived from the diff against the base, so it moves with every commit and is exactly the
# kind of number prose gets wrong.
#
# A FAILING check-traceability used to kill this script silently: under `set -e` the failed
# pipeline ended the run at exit 1 with no message, so the pivot's first run looked like a crash
# rather than a verdict. Still fail-closed, now as a named claim -- and a pass that reports no
# count (nothing to trace, or a local SKIP) is a NOTE that says which, not "unresolvable".
trace_rc=0
trace_out="$(./scripts/check-traceability.sh "$BASE" 2>&1)" || trace_rc=$?
trace_actual="$(printf '%s\n' "$trace_out" | sed -n 's/.*all \([0-9]\+\) non-trivial path.*/\1/p')"
if [[ "$trace_rc" -ne 0 ]]; then
    claim "check-traceability.sh against $BASE (claim 7 is its count)" "exit 0" "exit $trace_rc"
    printf '%s\n' "$trace_out" | head -8 | sed 's/^/    /' >&2
elif [[ -n "$trace_actual" ]]; then
    claim "non-trivial path count in $MANIFEST" \
          "$(sed -n 's/.*All \([0-9]\+\) non-trivial paths traced.*/\1/p' "$MANIFEST" | head -1)" \
          "$trace_actual"
else
    echo "check-evidence-claims: NOTE traceability count not checked: $(printf '%s\n' "$trace_out" | tail -1)"
fi

# 8. The body's "Final commit" must be the commit the manifest attests to. The body asserts
# outright that it is ("the commit this record and the manifest both name"), and after a round
# of evidence commits those two drifted apart while every NUMERIC claim above still matched --
# which is exactly the boundary this script had. Numbers were never the only thing that goes
# stale. Compared on the manifest's short form so the body can keep quoting 12 characters.
#
# Read with a real JSON parser, whichever exists: jq (which generate-evidence-manifest.sh already
# requires), else python3, else python. Since P0-01 tests/guards/run.sh runs this script, and
# run.sh is a verify step on windows-authoritative too, where `python3` is not a given; before,
# this only ever ran in the Linux guards job. No reader at all is a FAIL, never a skipped claim.
manifest_commit() {
    if command -v jq >/dev/null 2>&1; then jq -er .commit "$MANIFEST"; return; fi
    local py
    for py in python3 python; do
        if "$py" -c "" >/dev/null 2>&1; then
            "$py" -c 'import json,sys;print(json.load(open(sys.argv[1]))["commit"])' "$MANIFEST"; return
        fi
    done
    echo "check-evidence-claims: FAIL no JSON reader (jq, python3 or python) to read $MANIFEST" >&2
    return 1
}
manifest_commit="$(manifest_commit)" || exit 1
manifest_commit="${manifest_commit%$'\r'}"
claim "the Final commit in $BODY vs the commit $MANIFEST attests to" \
      "$(from_body 'Final commit: *\([0-9a-f]\{7,40\}\)')" \
      "${manifest_commit:0:12}"

# A checker that compared nothing would exit 0 and look identical to a clean record.
if [[ "$checked" -lt 9 ]]; then
    echo "check-evidence-claims: FAIL only $checked claim(s) compared; expected at least 9." >&2
    echo "  A phrasing change in $BODY silently drops claims from this pass -- which is the exact" >&2
    echo "  way the traceability count escaped it for three rounds. Fix the extractor, not this bound." >&2
    exit 1
fi

if [[ "$status" -eq 0 ]]; then
    echo "check-evidence-claims: OK $checked claim(s) re-derived and matched"
else
    echo "check-evidence-claims: the completion record must be true against the artifacts it describes" >&2
    echo "  (alpha-spec.md 12.8). Regenerate with 'just evidence $WP' and correct the prose." >&2
fi
exit "$status"
