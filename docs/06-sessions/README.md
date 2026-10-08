# Session logs and reviews

Canonical location for per-session traces and review reports.
Template: `docs/99-templates/template-session-log.md`.

Records here are evidence of what happened, not authority. They are not in the order of
authority in `docs/00-meta/authority-index.md`. A record is never edited after the fact.
Records imported from elsewhere are **byte-identical** to their source, and their provenance
is recorded in this index (or in the README of a session directory), never written into the
record itself. A finding that still matters is carried into the document that owns it:
`docs/00-meta/known-issues.md`, `docs/00-meta/decision-register.md`, an ADR, or a work package.

## Naming

- **Review reports:** `review-<work-package-id>-<kind>-round<N>.md`. `<kind>` is `adversarial`
  for fresh-context adversarial reviews. `<N>` counts successive reviews of the same work
  package, in the order of the commit each one reviewed, starting at 1. Each round is its own
  file; a later round never overwrites an earlier one.
- **Session directories:** `<YYYY-MM-DD>-<topic>/` for a session that produced several files.
  Its `README.md` carries the provenance and caveats; the other files are the verbatim records.
- **Single session logs** (from the template): `<YYYY-MM-DD>-<work-package-id>.md`, with a
  `-<topic>` suffix when one day holds more than one session for the same package.

## `docs/05-sessions/` is not a valid path

`05` is `docs/05-model-specs/` in the authority index's alias table and in
`scripts/bootstrap-repo.sh`. Rounds 1 and 2 of the P1-00 adversarial review were nonetheless
written to `docs/05-sessions/review-P1-00-adversarial.md`, each on its own review branch,
because the review brief named that path. Round 1 flagged the collision (minor 8), and round 2
flagged it again (minor 1). The two rounds had the same path on two different branches, so
merging both PRs would have created a directory the authority index calls invalid.

**Resolved by P0-01 (2026-10-01).** Rounds 1 and 2 were imported into this directory verbatim
as `review-P1-00-adversarial-round1.md` and `review-P1-00-adversarial-round2.md`. Rounds 3 and 4
were already at the right path and keep their names. Their round numbers were determined from
content:
- round 3's front matter says `round: 3`;
- round 3's path note says rounds 1 and 2 went to `docs/05-sessions/`;
- round 2 cites PR #2's review of `7787921` as the earlier review;
- the reviewed heads follow commit order (`7787921` → `de3b36c` → `8e07b04` → `3823478`).

Two actions remain, and neither can be done from inside this repository:

1. **Close PRs #2 and #3 unmerged** (owner action). Their content is here now. Merging either
   would recreate `docs/05-sessions/`.
2. **Fix the reviewer brief** that names `docs/05-sessions/review-P1-00-adversarial.md`. It lives
   in the review-workflow prompt, not in this repository. A brief that contradicts the authority
   index is a defect in the brief.

## Index

Date is the record's own `date:` front matter where it has one.

| Date | Session | Verdict | Record | Branch |
|------|---------|---------|--------|--------|
| 2026-08-20 | P1-00 implementation | — | no log in this directory; evidence in `.ai/evidence/P1-00/` | `wp/P1-00-repo-bootstrap` (PR #1) |
| 2026-08-20 | P1-00 adversarial review, round 1, at `7787921` | CONDITIONAL: 6 blockers, 10 major, 9 minor | [`review-P1-00-adversarial-round1.md`](review-P1-00-adversarial-round1.md) | `claude/grid-alpha-adversarial-review-ikkjuk` (PR #2) |
| 2026-08-20 | P1-00 adversarial review, round 2, at `de3b36c` | CONDITIONAL: 3 blockers, 9 major, 11 minor | [`review-P1-00-adversarial-round2.md`](review-P1-00-adversarial-round2.md) | `claude/adversarial-review-p1-00-l32emp` (PR #3) |
| 2026-08-20¹ | P1-00 adversarial review, round 3, at `8e07b04` | CONDITIONAL: 1 blocker, 3 major, 4 minor | [`review-P1-00-adversarial-round3.md`](review-P1-00-adversarial-round3.md) | `claude/adversarial-review-p1-00-l32emp` (PR #3) |
| 2026-08-30 | P1-00 adversarial review, round 4 (verification pass), at `3823478` | APPROVED with findings: 0 blockers, 1 major, 1 minor | [`review-P1-00-adversarial-round4.md`](review-P1-00-adversarial-round4.md) | `claude/adversarial-review-p1-00-l32emp` (PR #3) |
| 2026-10-08 | P0-01 adversarial review, round 1 (three fresh-context lenses), at `f3fb390` | 26 findings (0 blockers, 9 major, 17 minor), all dispositioned | [`review-P0-01-adversarial-round1.md`](review-P0-01-adversarial-round1.md) | written in this repository by P0-01 (not imported) |
| 2026-10-01 | P0-01 consolidation inventory: 8 inventory reports plus a completeness critic | — | [`2026-10-01-consolidation-inventory/`](2026-10-01-consolidation-inventory/README.md) | `claude/grid-engine-consolidation-e7kmh9` |

¹ Round 3's front matter says 2026-08-20. The commit that wrote it, `c220a69`, is dated
2026-08-29.

## Provenance

Every imported review record is byte-identical to its source blob. To check one, run
`git rev-parse <source commit>:<original path>` and compare it with
`git hash-object <record>`: the two must be equal.

| Record | PR | Source branch (head) | Written by commit | Original path | Git blob |
|--------|----|----------------------|-------------------|---------------|----------|
| `review-P1-00-adversarial-round1.md` | #2 | `origin/claude/grid-alpha-adversarial-review-ikkjuk` (`f958ed8`) | `25ac6a5`, amended in `f958ed8` | `docs/05-sessions/review-P1-00-adversarial.md` | `f6f4c6a686bd4ce8fa05f1533cc7735138f2fe01` |
| `review-P1-00-adversarial-round2.md` | #3 | `origin/claude/adversarial-review-p1-00-l32emp` (`33e84c5`) | `b88e2d5` | `docs/05-sessions/review-P1-00-adversarial.md` | `86c8206e72294cd369771221ded5e8b4585df2f3` |
| `review-P1-00-adversarial-round3.md` | #3 | `origin/claude/adversarial-review-p1-00-l32emp` (`33e84c5`) | `c220a69` | `docs/06-sessions/review-P1-00-adversarial-round3.md` | `a1c531a509248bbe6c09712b105fea3fad060c98` |
| `review-P1-00-adversarial-round4.md` | #3 | `origin/claude/adversarial-review-p1-00-l32emp` (`33e84c5`) | `33e84c5` | `docs/06-sessions/review-P1-00-adversarial-round4.md` | `1f0a64e29b0d6438229a630910446dbb8fd1b826` |
| `2026-10-01-consolidation-inventory/*.md` (9 reports) | — | not from git: the 2026-10-01 consolidation session's scratchpad | — | `scratchpad/inventory/<report>.md` | sha256 per report in the directory's [`README.md`](2026-10-01-consolidation-inventory/README.md) |

The two `docs/05-sessions/` originals are different files that share a path: round 1 (624
lines) on PR #2's branch and round 2 (629 lines) on PR #3's. Both PR branches are based on
`48ee320`, and neither touches anything outside its review files.
