---
provider: nflverse
document: access-and-license
contract-version: 1
status: Draft            # Draft | Approved | Superseded
data-licensing-owner: repository owner (holds every human role; docs/00-meta/authority-index.md)
verified-on: 2026-10-01
pending-rulings: DR-A11 (fixture licensing policy); per-dataset terms for pbp, rosters, player stats, schedules, snap counts
---

# nflverse — access, licence, attribution and redistribution

**Status: Draft.** This document records what was **verified** on 2026-10-01 and marks
everything else as requiring a ruling by the Data/Licensing owner. It is required before the
P1-03 adapter is Ready ([README.md](README.md)) and implements critic.md G-5 and owner decision
DR-A11 (adopted by the P0-01 consolidation, subject to ratification).

Nothing here is legal advice. Where a question turns on interpreting a licence, this document
names the question and leaves the answer to the Data/Licensing owner.

## 1. Access

| Item | Value |
|---|---|
| Host | `github.com`, release assets of `nflverse/nflverse-data`, over HTTPS |
| Authentication | none |
| Engine fetch budget | at most once per local calendar day per source *(proposed — DR-C14)* |
| Robots and scraping | Release assets are a published download channel. Restricted pages are never scraped to fill a gap (superseded alpha-spec §4.3) |
| Credentials in code, logs or fixtures | none exist for this provider; none may be introduced |

## 2. Licence per dataset

| Dataset | Licence | Required attribution | Verified | Source of the statement |
|---|---|---|---|---|
| Participation (`pbp_participation`), seasons **2016–2022** | **CC-BY-SA 4.0** | **"NFL NextGenStats via nflverse"** | yes, 2026-10-01 | nflreadr reference page `load_participation`: "This data is released under the CC-BY-SA 4.0 Creative Commons license and attribution must be made to FTN Data via nflverse (from 2023 onwards) or NFL NextGenStats via nflverse (for 2022 and earlier)" |
| Participation, seasons **2023 onwards** | **CC-BY-SA 4.0** | **"FTN Data via nflverse"** | yes | same page. It also says participation "from 2023 onwards is courtesy of FTN and is provided after all post-season games are completed" |
| FTN charting (`load_ftn_charting`), 2022 onwards | **CC-BY-SA 4.0** | **"FTN Data via nflverse"** | yes | nflreadr reference page `load_ftn_charting` |
| Repository `nflverse/nflverse-data` as a whole | **CC-BY-4.0** (Creative Commons Attribution 4.0 International, `LICENSE.md` at the repository root) | not stated as a string | yes | repository `LICENSE.md` |
| Play-by-play, player and team stats, rosters, players, schedules, depth charts, NGS | the repository-level CC-BY-4.0 applies on its face; **no dataset-specific statement was found** (the `load_pbp` reference page has none) | not stated | **no: requires Data/Licensing owner verification** | — |
| Snap counts and PFR advanced stats | sourced from Pro Football Reference (`load_snap_counts`: "provided by Pro Football Reference"); the originator's own terms were not checked | not stated | **no: requires Data/Licensing owner verification** | — |
| Injuries | not checked; availability itself is disputed ([README.md](README.md) "Freshness") | — | **no** | — |

Notes:

- **Attribution spelling.** The upstream text spells the pre-2023 credit **"NFL NextGenStats
  via nflverse"**, with no space. This was confirmed in the raw HTML on 2026-10-01. The inventory
  wrote "NFL NextGen Stats via nflverse" (critic.md G-5), with a space. Use the upstream spelling
  verbatim, and re-check it when this document is re-verified.
- **Originators.** The nflverse statements govern the nflverse releases. They say nothing about
  the rights of the originating data owners: the NFL (NGS, game data), FTN and Pro Football
  Reference. Before any public release or commercial use, the Data/Licensing owner reviews
  commercialization and redistribution rights (superseded alpha-spec §14).
- **Cautious-nevermore.** Its docs asserted "nflverse data is CC-BY-SA 4.0" for *all* nflverse
  data (cn-docs.md §7). That is **not** what the verified statements say. Only participation and
  FTN charting are verified as CC-BY-SA; the repository is CC-BY-4.0.

## 3. ShareAlike implications (CC-BY-SA 4.0 data: participation, FTN charting)

1. **The repository's own licence.** The repository's code is `MIT OR Apache-2.0` (ADR-001 D4).
   CC-BY-SA material, or material adapted from it, MUST NOT be committed into that tree as if it
   were covered by the code licence. Any CC-BY-SA-derived file needs **licence segregation**: a
   separate directory with its own LICENSE/NOTICE (§5).
2. **What counts as derived.** Derived from CC-BY-SA data for this purpose includes:
   - raw participation rows;
   - plays-contract frames whose `off_players`/`def_players` came from participation;
   - per-play personnel features;
   - RAPM ratings fitted on participation.

   Whether model outputs further downstream are "Adapted Material" is **open**. Examples are
   projections that use participation-based priors, and aggregate metrics. It is a
   Data/Licensing ruling under DR-A11. Until the ruling, every such output is treated as
   possibly ShareAlike-bound and is **not published or redistributed**.
3. **The reference oracle carries no nflverse data.**
   - Its tests mock every fetch, and its isolation guard blocks the network.
   - The licence ruling for `reference/python/` code (DR-A10) is therefore separate from data
     licensing.
   - The inventory's real-data investigations (`reference/python/tools/investigations/`) fetch
     data at run time. They commit only the scripts, plus URLs and sha256 values; never the
     parquet files.
4. **CC-BY-4.0 data** (repository level) carries no ShareAlike term, but attribution still
   applies (§6).

## 4. Fixture policy (DR-A11)

- **Parity fixtures are synthetic-only.** No byte derived from nflverse data, whether CC-BY-SA
  or CC-BY, enters `fixtures/parity/`
  ([parity-fixture-contract.md](../../03-contracts/parity-fixture-contract.md) §8).
- **Adapter tests use synthetic, nflverse-shaped toy frames**, as the oracle's own tests do. They
  are hand-built, with fabricated ids and values, and carry no licence constraint.
- **Real-data provider fixtures** are permitted only after a Data/Licensing ruling, and only:
  - under `fixtures/third-party/nflverse/<dataset>/`;
  - with a `LICENSE` file (the applicable licence) and a `NOTICE` file. The NOTICE records the
    attribution string, the source URL, the season, the retrieval date, the sha256 of the source
    file, and the modifications made (for example "rows sampled, columns dropped");
  - with the smallest sample that exercises the case;
  - after review and sanitization before commit (superseded alpha-spec §4.6);
  - with the directory marked `-text` in `.gitattributes`, so it stays byte-stable.
- `reference/python/backend/db/data/coaching_changes_2025.json` is **not** nflverse data. It is
  unverified and illustrative (KI-NEW-D1). It MUST NOT become a fixture or a provider input.

## 5. Raw retention

- Raw nflverse files are retained **locally and unmodified**, content-addressed by sha256 and
  registered with their provenance:
  - URL, release tag, season, `retrieved_at`, `source_timestamp`;
  - licence id, attribution string.

  This follows superseded alpha-spec §14 (retain source attribution and licence metadata).
- Raw files are **never committed** (`.gitignore` covers `/data/`). Retention duration is set in
  `source-manifest.yaml` (P1-03). Upstream re-releases can change a file's content; deleting or
  replacing a retained raw file after a licence or terms change is a Data/Licensing decision.
- Investigation inputs are referenced by sha256 only. The 2023 files used by the consolidation
  inventory:

  | File | sha256 |
  |---|---|
  | `play_by_play_2023` | `bd348473…6776` |
  | `pbp_participation_2023` | `b1577369…e5a6` |
  | `roster_2023` | `66dcb7d0…3c90` |
  | `player_stats_2023` | `94673091…c9` |
  | `stats_player_week_2023` | `ac776fbd…bc` |

## 6. Attribution on published outputs

Any output **published outside the operator's own machine** carries the attribution of every
dataset that fed it. That includes exports, reports, model cards, benchmark write-ups, and
documentation quoting real-data numbers.

| Data that fed the output | Attribution (verbatim) |
|---|---|
| Participation 2016–2022 | `NFL NextGenStats via nflverse` |
| Participation 2023 onwards | `FTN Data via nflverse` |
| FTN charting | `FTN Data via nflverse` |
| Any other nflverse release | credit to nflverse (`nflverse/nflverse-data`, CC-BY-4.0). **The exact wording requires a Data/Licensing owner ruling**; no dataset-specific string was verified |

- **Mechanics.** The export sidecar carries an attribution block
  ([engine-output-contract.md](../../03-contracts/engine-output-contract.md) §8). Every
  projection keeps a source lineage record, so the block can be computed from the lineage rather
  than written by hand.
- **Credit format.** Where a licence requires it, the credit includes the attribution string, a
  link to the licence, and an indication of whether changes were made.
- **Competitor data.** None is ever included in, or attributed in, an engine output.
- **Engine-spec §3 claim discipline** still applies to any accuracy claim that accompanies
  published numbers.

## 7. Open items for the Data/Licensing owner

1. **DR-A11.**
   - Ratify the synthetic-only parity fixture rule.
   - Decide whether any real-data provider fixture may be committed, and in what form.
   - Decide the ShareAlike status of engine outputs derived from participation (§3.2).
2. **Per-dataset terms.** Verify the terms for play-by-play, player and team stats, rosters,
   players, schedules, depth charts and NGS (repository-level CC-BY-4.0 or otherwise), and for
   the PFR-sourced snap counts and advanced stats. Fix the attribution wording for each.
3. **Injury feed.** Re-verify its availability for 2025 and later; the evidence conflicts
   ([README.md](README.md) "Freshness").
4. **Re-verification.** Record a date and owner for re-checking this document, at least once per
   season and whenever upstream licence pages change.
