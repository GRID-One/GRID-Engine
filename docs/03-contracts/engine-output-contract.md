---
contract: grid.output
contract-version: 1 (Draft; no producer exists yet)
status: Draft            # Draft | Approved | Superseded
authority-level: 3       # engine-spec.md §1.5
semantics-owner: Statistical owner (distributions, explanations); Product/Architecture owner (records, reports, exports)
work-packages: P1-01 (types and serialization skeleton), P1-08 (distributions, explanations), P1-10 (CLI, reports, export), P2-06/P2-07 (scorecards, change log)
---

# Contract — Engine output (projection records, explanations, reports, exports)

This is the public output contract of the GRID engine. It covers:

- what a projection run emits for each player-week;
- how it is explained;
- which operational reports exist;
- what may be exported.

It implements engine-spec §5.5 (engine output contract), §5.6 (exports and reports) and §6.7
(explainability). It is rebuilt from the superseded alpha-spec as follows:

- §5.1–§5.4 (alpha-spec §5.1–5.4, superseded): stat vectors, conditional and unconditional
  projections, distributions, the scoring transformation;
- §5.5 (alpha-spec §5.5, superseded): the contract-first Rust/Flutter boundary rules, recast as
  rules for engine output records;
- §6.5 (alpha-spec §6.5, superseded): explainability;
- §9.2 and §10.2 (alpha-spec, superseded): "Native UI" lists. They hide engine requirements: a
  per-row freshness badge, a change log with attribution categories, a data-quality console,
  and an export limited to projections and quantiles with no competitor data. Each becomes an
  output record or report here.

**State today.** No Rust producer exists. The Python oracle produces **none** of these records
in contract form. Its outputs are app SQLite sinks (`valuations`, `kalman_trajectory`,
`matchup_grades`, `situation_grades`), and they are not ported (cn-docs.md §12). Its only
reusable output semantics are:

- scoring arithmetic: `reference/python/backend/scoring/formats.py` matches the alpha-spec §2.3
  weights exactly;
- the Kalman one-step predictive mean and variance, which are GRID signal values (§4.2), not
  fantasy distributions.

The oracle has no quantiles, no P(zero), no conditional/unconditional split and no stat-vector
distribution (reconcile-spec-first.md R21–R25). Parity for this contract is therefore
spec-defined golden tests (engine-spec §12), not oracle parity.

Requirements tagged *(proposed — DR-xx)* depend on an open owner decision in
[decision-register.md](../00-meta/decision-register.md).

---

## 1. Principles (alpha-spec §5.5, superseded, recast for an engine)

1. **Rust is the source of truth** for projections and model state. Consumers receive data only
   through this versioned contract. No hand-maintained parallel domain model is kept in this
   repository. `reference/python/` types are oracle-internal and are not a contract.
2. Public request, response, record, enum and error types are defined in a versioned Rust
   module. File outputs (CSV, JSON, Parquet) have a machine-readable schema generated from those
   definitions. Generated schema artifacts are regenerated, never hand-edited.
3. **Every field states its units, nullability, enum semantics and compatibility expectations.**
   A field without all four is not part of the contract.
4. Representative records are round-tripped through serialize and deserialize in Rust tests.
   Every exported file is checked against the schema.
5. A **breaking change** requires all of the following (§9):
   - a contract-version increment;
   - an ADR or an approved work-package decision;
   - regenerated schema artifacts;
   - synchronized tests.
6. Large result sets are served through paginated, query-specific records. Database rows and
   model internals are never exposed directly.
7. Internal Rust types may be refactored freely without a contract change, unless the approved
   work package authorizes a contract change.

## 2. Keys and lineage (every projection record)

| Field | Type | Semantics |
|---|---|---|
| `season`, `week` | `u16`, `u8` | Target game week. Weeks 1–18 are produced. Week 18 carries `is_week_18 = true` because it is evaluated separately (engine-spec §2.2) |
| `player_id` | `PlayerKey` | The canonical NFL key, `gsis_id` (engine-spec §4.4). Synthetic ids appear only in runs whose `data_snapshot_id` is synthetic, and are flagged |
| `position` | enum `QB, RB, WR, TE` | The primary accuracy population. K and DST, if added in Phase 2, are separate record types and are excluded from the primary claim. IDP is out of scope |
| `team`, `opponent`, `game_id`, `is_home` | keys | From the schedule snapshot in force at the lock |
| `prediction_version_id` | id | Unique per emitted projection set. A post-lock re-projection is a **new** version and never an overwrite (superseded alpha-spec §10.3) |
| `lock` | `{ kind: Thursday \| Sunday \| Operational, lock_at: UtcInstant }` | The benchmark locks of engine-spec §7 (superseded alpha-spec §7.2). An operational projection is a separate version and cannot replace a locked snapshot |
| `data_snapshot_id`, `feature_schema_version`, `model_version_id`, `engine_version` | ids | Lineage. Every projection traces to its data, feature, model, scoring and engine versions (engine-spec §9.4) |
| `scoring_profile` | `{ id, version }` | §3.6 |
| `seed`, `n_draws` | `u64`, `u32` | The Layer-F simulation seed and draw count. Randomness is seeded and recorded (superseded alpha-spec §6.6 rule 5) |
| `computed_at` | `UtcInstant` | Wall-clock time of computation. It is metadata, not an input |

A source lineage record is kept for every published projection: which source snapshots, with
which publication timestamps, fed it (engine-spec §4.5; superseded alpha-spec §4.1.1). It is a
separate queryable record, keyed by `prediction_version_id`.

## 3. Player-week projection record

### 3.1 Horizon

Weekly is the primary contract. Rest-of-season and preseason figures are **derived products**:
sums of weekly simulated draws, labelled `horizon = RestOfSeason | Preseason`, and reported as
diagnostics *(proposed — DR-C4)*. The oracle's `week = 0` sentinel for a season total (KI-A4)
MUST NOT be reproduced. A typed `horizon` field is used instead.

### 3.2 Stat vectors (alpha-spec §5.1, superseded)

The model predicts component statistics, not only fantasy points. Counts are per game. Yards are
in yards. Probabilities and shares are in [0, 1].

| Component | QB | RB | WR/TE | Unit and support |
|---|:-:|:-:|:-:|---|
| `active_probability` | ✓ | ✓ | ✓ | probability |
| `start_probability` | ✓ | — | — | probability |
| `offensive_snap_share` | — | ✓ | ✓ | fraction of team offensive snaps |
| `pass_attempts` | ✓ | — | — | count ≥ 0; official definition, **sacks excluded** (KI-NEW-I1) |
| `completions` | ✓ | — | — | count ≥ 0, ≤ `pass_attempts` per draw |
| `passing_yards` | ✓ | — | — | yards; official definition, sack yardage not netted (KI-NEW-I1) |
| `passing_tds` | ✓ | — | — | count ≥ 0; return TDs excluded (KI-NEW-I2) |
| `interceptions` | ✓ | — | — | count ≥ 0 |
| `sacks_taken` | ✓ | — | — | count ≥ 0 |
| `rushing_attempts` / `carries` | ✓ | ✓ | ✓ | count ≥ 0; official definition (kneels per the official stat, KI-NEW-I5) |
| `rushing_yards` | ✓ | ✓ | ✓ | yards |
| `rushing_tds` | ✓ | ✓ | ✓ | count ≥ 0 |
| `targets` | — | ✓ | ✓ | count ≥ 0 |
| `receptions` | — | ✓ | ✓ | count ≥ 0, ≤ `targets` per draw |
| `receiving_yards` | — | ✓ | ✓ | yards |
| `receiving_tds` | — | ✓ | ✓ | count ≥ 0 |
| `fumbles_lost` | ✓ | ✓ | ✓ | count ≥ 0; official = sack + rushing + receiving fumbles lost (KI-NEW-I3) |
| `two_point_conversions` | ✓ | ✓ | ✓ | count ≥ 0; **modelled, not a silent 0** (KI-NEW-I5; the oracle fixes it at 0.0) |

Stat definitions follow the official nflverse weekly player statistics *(proposed — DR-C12;
engine-spec §4.7)*. The asymmetries are carried over verbatim from the superseded alpha-spec and
are open (DR-D8): QB has no snap share; RB/WR/TE have no start
probability; QB has no receiving components.

### 3.3 Availability block (Layer A)

| Field | Type | Semantics |
|---|---|---|
| `p_active`, `p_start` (QB), `snap_multiplier_if_active`, `p_limited_role` | probability / ratio | Layer A outputs (engine-spec §6.1) |
| `availability_source` | enum `Override \| Provider \| Model \| Missing` | `Missing` means no availability data. It **widens uncertainty** and MUST NOT be read as healthy (superseded alpha-spec §4.1.2) |
| `availability_observed_at` | `UtcInstant?` | Null only when the source is `Missing` |
| `manual_override` | bool | True when a timestamped operator override applied. It is included in the prediction snapshot |

Known `OUT`, `IR`, `PUP`, suspension, bye and inactive states force `p_active = 0` and the zero
stat vector.

### 3.4 Conditional and unconditional (alpha-spec §5.2, superseded)

Both are persisted and exported:

- **conditional:** production if active;
- **unconditional:** after applying `p_active`, `p_start` and workload suppression.

**The default and headline expected-points field in every output is unconditional.**

### 3.5 Stat distribution summaries

For each §3.2 component, in both the conditional and the unconditional block:

- `mean`, `sd`;
- `q10`, `q25`, `q50`, `q75`, `q90`.

The summaries are computed from the Layer-F draws (engine-spec §6.1).

### 3.6 Fantasy-point distribution per scoring profile (alpha-spec §5.3, superseded)

For each player × week × scoring profile:

- `mean`, `median`, `sd`;
- `p10`, `p25`, `p75`, `p90`;
- `floor` and `ceiling`, each tagged with its percentile definition;
- `p_zero_or_inactive`;
- `p_exceed[threshold]` for a configurable threshold set;
- `p_boom`, `p_bust`, relative to positional starter thresholds.

The floor and ceiling percentiles, the default threshold set and the definition of a positional
starter threshold are **not fixed by any spec**. Each is recorded per output version and is open
(DR-D8).

### 3.7 Scoring transformation (alpha-spec §5.4, superseded)

A scoring profile is a versioned affine transform:

```text
fantasy_points = scoring_weights · projected_stat_vector + scoring_offset
```

- **Built-in profiles.** Half-PPR (the default), Standard and PPR, with the engine-spec §2.3
  weights. The oracle's `formats.py` matches them with an implicit offset of 0.
- **Custom profiles.** Imported as versioned files. Profile identity is a content hash of the
  canonical serialization, never key-order-sensitive JSON text (KI-NEW-C1).
- **Re-scoring.** The same simulated stat draws can be re-scored for any number of profiles
  without re-running the football model. Records carry a `draws_ref { matrix_id, n_draws }`.

### 3.8 Freshness and provenance flags (every row)

| Field | Type | Semantics |
|---|---|---|
| `freshness` | enum `Fresh \| Stale \| Missing` | The worst status across the critical sources that fed this row, under the source-specific thresholds in each provider's `freshness-policy.md`. This is the superseded alpha-spec §9.2 "data freshness badge" as a field |
| `stale_sources` | list of source names | Empty when `Fresh` |
| `oldest_source_timestamp` | `UtcInstant` | Oldest publication timestamp among the critical sources |
| `experimental` | bool | True until the Phase-1 exit gate passes (claim discipline, engine-spec §3; superseded alpha-spec §3.2) |

A source that is unavailable MUST NOT silently inherit yesterday's healthy status
(superseded alpha-spec §10.3).

### 3.9 Record invariants (typed failures; superseded alpha-spec §6.6 rule 4)

A record that violates any of these is **not emitted**. The run fails with a typed error and the
prior production projection stays in force:

- every probability is in [0, 1];
- quantiles are monotone (`q10 ≤ q25 ≤ q50 ≤ q75 ≤ q90`; `p10 ≤ … ≤ p90`);
- per draw, `completions ≤ pass_attempts` and `receptions ≤ targets`;
- an inactive draw yields the zero vector;
- team-role shares sum to at most 1 (share overflow);
- no NaN or infinity anywhere.

## 4. Explanation record (alpha-spec §6.5, superseded)

### 4.1 Required fields (every projection)

1. Expected team play and scoring environment, as a reference to the team-game record (§5).
2. Expected player role and opportunity: share means.
3. Availability adjustment (Δ expected points).
4. Matchup adjustment (Δ).
5. NCAA prior contribution, null if none.
6. Recent NFL evidence contribution.
7. Top positive and top negative model drivers.
8. Uncertainty drivers.
9. Difference from the prior published projection: the Δ and the prior `prediction_version_id`.

**Language rule.** Explanations distinguish causal language from predictive association. No
explanation payload or generated text may say that a feature "caused" a projection change unless
the logic is rule-based (for example, `OUT` ⇒ zero).

### 4.2 GRID signal block (proposed — DR-C2, DR-C3)

DR-C2 proposes that GRID is a signal provider inside Layers A–F. DR-C3 proposes that GRID enters
Layer D as role-specific talent covariates. If both are ratified, the explanation record carries
the block below. All values are in GRID's dV currency (expected points per play). They are
explanation and diagnostic fields, never fantasy points.

| Field | Semantics |
|---|---|
| `role_talent { dropback \| carry \| target }` | `{ filtered_mean, filtered_var, predictive_var }` from the state-space layer. **Filtered or one-step predictive only** |
| `form`, `scheme_fit` | Filtered state components |
| `offseason_rapm` | `{ rating, season, n_plays }`, or null. Fitted only once that season's participation is published *(proposed — DR-C1)* |
| `prior` | `{ mean, var, q, n0, weight_now }`, or null. The NCAA/draft prior and its current weight (engine-spec §6.5) |
| `regime_flags` | `{ intervention, scheme_reset, changepoint_z }` |

Hard rule, not conditional on the DRs. **RTS-smoothed values MUST NOT appear in any live or
locked projection record.** Smoothing uses future weeks. Aliasing filtered and smoothed values is
the canonical "correct-looking wrong semantic" bug (cn-docs.md §8 item 6). Smoothed values may
appear only in retrospective diagnostics labelled as such.

A matchup grade, if exposed, uses the corrected sign: higher means a tougher defense, `+E_def`
*(proposed — DR-B5)*. The oracle's `−β_def` grade is inverted (KI-NEW-A2) and MUST NOT be
reproduced.

## 5. Team-game environment record

The Layer B outputs (engine-spec §6.1), one per team × game per prediction version:

- expected offensive plays and drives;
- pass and rush attempts, and sacks;
- touchdowns by type and red-zone opportunities;
- pace and neutral pass tendency;
- expected game script and implied points.

Each value carries `mean`, `sd` and `q10`, `q50`, `q90`. The two teams of a game share
`shared_latent_ids`, which records that their draws are coupled.

## 6. Projection change log

Superseded alpha-spec §10.2 item 2. For each player-week and each consecutive pair of published
versions, the record holds the prior and current expected points and the Δ. The Δ is attributed
to **exactly these five categories**:

- `role`
- `availability`
- `matchup`
- `team_environment`
- `model_update`

How the Δ is decomposed (for example sequential substitution, and how any residual or
interaction is reported) is a model-spec item and is open (DR-D8).
Attribution is predictive, not causal (§4.1).

## 7. Reports (CLI / query outputs)

Each report is a versioned record type under this contract.

| Report | Contents | Origin |
|---|---|---|
| Data-quality report | (1) stale sources; (2) unresolved identities; (3) quarantined rows, for example a drive spanning a change of possession ([plays-contract.md](plays-contract.md) D-4); (4) missing current-week context; plus source and row-count change diagnostics | superseded alpha-spec §10.2 item 5, §9.4 |
| Status report | Last successful ingestion per source, source freshness, current production model, job status and errors | superseded alpha-spec §9.2 item 3; `get_data_freshness`, `get_ingestion_status`, `get_training_status` |
| Availability report | Missing injury data; questionable/doubtful/out states; `p_active` and the workload multiplier; override provenance | superseded alpha-spec §10.2 item 1 |
| Identity review queue | Pending NCAA↔NFL and cross-source links. Read-only in Phase 1; approve/reject commands, audited, in Phase 2 | engine-spec §4.4 |
| Model scorecard | PB-MAE (primary), MAE, RMSE, rank accuracy, active/inactive Brier, interval coverage; by week and position; rookie and low-evidence slice | superseded alpha-spec §10.2 item 3; engine-spec §7.4 |
| Benchmark results | Provider comparison with provider timestamps and CIs, named or anonymized according to each provider's licence. **No raw competitor rows** | superseded alpha-spec §10.2 item 4 |

The engine emits **no VOR, tier or draft-ranking output** *(proposed — DR-C11)*.

## 8. Exports (engine-spec §5.6)

1. **Content.** Projection and quantile records only (§2–§3.8). There is **no competitor data
   in any export**, ever: no benchmark-provider rows, values or derived per-player comparisons
   (superseded alpha-spec §9.2 item 5; §14). Benchmark reports export aggregate metrics only.
2. **Formats.** CSV is required. Parquet is optional and needs a new Rust dependency, which
   needs owner approval under the dependency policy (superseded alpha-spec §14.2; root
   `CLAUDE.md`). Until that approval it is not produced.
3. **Sidecar.** Every export has a JSON sidecar recording:
   - the contract version and the column schema;
   - `prediction_version_id` and the scoring profile;
   - row count and the sha256 of the data file;
   - the source attribution block (rule 6).
4. **CSV hygiene.**
   - UTF-8 with LF line endings and a header row; the column order is fixed per contract version.
   - Numbers are written in canonical numeric form.
   - Any **text** cell that begins with `=`, `+`, `-`, `@`, tab or carriage return is escaped
     against formula injection (superseded alpha-spec §14). Numeric cells are never escaped.
5. **No secrets.** No credential, provider key, absolute path or host name appears in an export,
   sidecar or log.
6. **Attribution.** Exports carry the attribution strings of every upstream source whose data
   fed the run ([nflverse/access-and-license.md](../04-providers/nflverse/access-and-license.md)).
   Whether a projection derived from CC-BY-SA participation is an "adapted" work that triggers
   ShareAlike is a Data/Licensing ruling (DR-A11 scope). Until that ruling, exports are private
   and are not redistributed.

## 9. Versioning and compatibility

- **Contract id `grid.output`**, version `MAJOR.MINOR`. Every record, report and export sidecar
  carries it.
- **MAJOR.** Removing or renaming a field; changing a unit, enum meaning, percentile definition
  or stat definition; or changing a default (for example, the headline field moving off
  unconditional).
- **MINOR.** Adding an optional field or enum variant that consumers may ignore. Enum
  extensions are MINOR only for enums documented as open.
- A consumer-facing reader MUST reject an unknown MAJOR.
- Historical exports and locked snapshots are immutable. They are never rewritten into a newer
  version; a newer version is a new artifact.

## 10. Rust types

- `grid-domain` (`crates/domain`, P1-01) holds the record types: `PlayerWeekProjection`,
  `AvailabilityBlock`, `StatDistribution`, `FantasyDistribution`, `ExplanationRecord`,
  `GridSignalBlock`, `TeamGameEnvironment`, `ProjectionChange`, the report records and the typed
  errors. The domain crate has no external dependencies.
- **Serialization.**
  - The JSON sidecar uses `serde_json`, already declared in `[workspace.dependencies]`.
  - The CSV writer is either hand-written on `std` or uses a crate that needs approval.
  - Parquet follows §8 rule 2.
  - Serialization lives in the orchestration crate: `application` today, `pipeline` and
    `grid-cli` after DR-A8.
- Query names follow engine-spec §8 (`get_week_projection_board`,
  `get_player_projection_detail`, `get_projection_distribution`, `get_projection_change_log`,
  `get_model_scorecard`, `get_benchmark_results`, `get_data_quality_report`, …), exposed through
  the CLI and the library API.

## 11. Tests (required before a producer ships)

- Serialize/deserialize round-trip of a representative record per type.
- Schema conformance of every exported file and sidecar.
- A property test for every §3.9 invariant: a violating record is rejected with the typed error.
- Formula-injection escaping cases, including that a negative number is not escaped.
- A no-competitor-data export test: a run whose store contains benchmark rows exports none of
  them.
- Golden tests on the expanded synthetic world for distributions and scoring (engine-spec §12;
  synthetic world per DR-B4).

## 12. Open items

- **DR-D8** covers five items:
  - the floor/ceiling percentiles;
  - the default `p_exceed` thresholds;
  - the boom/bust positional starter thresholds;
  - the stat-vector asymmetries (§3.2);
  - the change-attribution decomposition method (§6).
- **DR-C2, DR-C3.** GRID signal block (§4.2).
- **DR-C4.** Derived horizons (§3.1).
- **DR-C11.** No VOR or tier outputs (§7).
- **DR-C12.** Stat definitions (§3.2).
- **DR-A11 scope.** ShareAlike status of derived exports (§8 rule 6).
