---
contract: grid.output
contract-version: 1.0 (Approved 2026-10-19 with P1-01; skeleton types in grid-domain, no producer yet)
status: Approved         # Draft | Approved | Superseded
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
| `lock` | `{ kind: Thursday \| Sunday \| Operational, at: UtcInstant }` | The benchmark locks of engine-spec §7 (superseded alpha-spec §7.2). An operational projection is a separate version and cannot replace a locked snapshot |
| `data_snapshot_id`, `feature_schema_version`, `model_version_id`, `engine_version` | ids | Lineage. Every projection traces to its data, feature, model, scoring and engine versions (engine-spec §9.4) |
| `scoring_profile` | `{ id, version }` | §3.6 |
| `seed`, `n_draws` | `u64`, `u32` | The Layer-F simulation seed and draw count. Randomness is seeded and recorded (superseded alpha-spec §6.6 rule 5) |
| `computed_at` | `UtcInstant` | Wall-clock time of computation. It is metadata, not an input |
| `research` | `{ research_only: bool, relaxed_rule: string? }` | Set on every record a `Research`-mode run produces: `research_only = true` and the relaxed rule named (`docs/05-model-specs/evaluation-and-leakage.md` §4.1). Such a record MUST NOT feed a gate, a promotion or a published claim. `research_only = false` and `relaxed_rule` null on every other record. Carried by the lineage group, the team-game record (§5), the change-log record (§6) and the run-level scorecard and benchmark reports (§7). Added by P1-01 |

A source lineage record is kept for every published projection: which source snapshots, with
which publication timestamps, fed it (engine-spec §4.5; superseded alpha-spec §4.1.1). It is a
separate queryable record, keyed by `prediction_version_id`.

## 3. Player-week projection record

### 3.1 Horizon

Weekly is the primary contract. Rest-of-season and preseason figures are **derived products**:
sums of weekly simulated draws, labelled `horizon = RestOfSeason | Preseason`, and reported as
diagnostics *(DR-C4)*. The oracle's `week = 0` sentinel for a season total (KI-A4)
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

Stat definitions follow the official nflverse weekly player statistics *(DR-C12; engine-spec
§4.7)*. The asymmetries are carried over verbatim from the superseded alpha-spec and are
intended (DR-D7): QB has no snap share; RB/WR/TE have no start probability; QB has no receiving
components.

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
starter threshold are **not fixed by any spec**. Each is a versioned setting, recorded per output
version and set with Statistical-owner approval before P1-08 is Ready (DR-D8). The values are
still open.

### 3.7 Scoring transformation (alpha-spec §5.4, superseded)

A scoring profile is a versioned affine transform:

```text
fantasy_points = scoring_weights · projected_stat_vector + scoring_offset
```

- **Built-in profiles.** Half-PPR (the default), Standard and PPR, with the engine-spec §2.3
  weights. The oracle's `formats.py` matches them with an implicit offset of 0.
- **Custom profiles.** Imported as versioned files. Profile identity is a content hash of the
  canonical serialization, never key-order-sensitive JSON text (KI-NEW-C1).
- **Non-affine rules.** A profile containing a threshold bonus or another non-affine rule is
  rejected with a typed error and never approximated (DR-D6).
- **Re-scoring.** The same simulated stat draws can be re-scored for any number of profiles
  without re-running the football model. Records carry a `draws_ref { matrix_id, n_draws }`.

### 3.8 Freshness and provenance flags (every row)

| Field | Type | Semantics |
|---|---|---|
| `freshness` | enum `Fresh \| Stale \| Missing` | The worst status across the critical sources that fed this row, under the source-specific thresholds in each provider's `freshness-policy.md`. This is the superseded alpha-spec §9.2 "data freshness badge" as a field |
| `stale_sources` | list of source names | Empty when `Fresh` |
| `oldest_source_timestamp` | `UtcInstant` | Oldest publication timestamp among the critical sources |
| `experimental` | bool | True until the Phase-1 exit gate passes (claim discipline, engine-spec §3; superseded alpha-spec §3.2) |
| `manual_data` | bool | True when a timestamped manual availability override applied to this row: the manual-data flag of engine-spec §5.5 ("Freshness and flags") and §10.3. Added by P1-01 |
| `missing_current_week_context` | bool | True when current-week context the row needed was missing: the third flag of engine-spec §5.5 ("Freshness and flags"). Added by P1-01 |

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
- the standard deviation is non-negative (engine-spec §5, "Invariants"; added by P1-01 so that this
  section states every invariant `OutputInvariant` checks);
- every stat-component summary is a component of the record position's §3.2 vector, and each
  component appears at most once per block (DR-D7; added by P1-01, work-package Amendments row 18);
- no NaN or infinity anywhere.

## 4. Explanation record (alpha-spec §6.5, superseded)

### 4.1 Required fields (every projection)

1. Expected team play and scoring environment, as a reference to the team-game record (§5).
2. Expected player role and opportunity: share means.
3. Availability adjustment (Δ expected points).
4. Matchup adjustment (Δ).
5. NCAA prior contribution: the remaining weight of the player's feeder (NCAA) base in the state,
   computed by linearity (`state-space-kalman.md` §3.5; DR-D17). Null if he has no feeder prior.
6. Recent NFL evidence contribution.
7. Top positive and top negative model drivers.
8. Uncertainty drivers.
9. Difference from the prior published projection: the Δ and the prior `prediction_version_id`.

**Language rule.** Explanations distinguish causal language from predictive association. No
explanation payload or generated text may say that a feature "caused" a projection change unless
the logic is rule-based (for example, `OUT` ⇒ zero).

### 4.2 GRID signal block (DR-C2, DR-C3)

GRID is a signal provider inside Layers A–F (DR-C2). It enters Layer D as role-specific talent
covariates, and the state-space layer keeps one track per player and role (DR-C3). Whether
RB/WR/TE role credit is an individual signal is still open (DR-D13); until it is decided,
explanations never describe that credit as individual efficiency (the DR-D13 interim rule,
confirmed 2026-10-09). The explanation record
carries the block below. All values are in GRID's dV currency (expected points per play). They are
explanation and diagnostic fields, never fantasy points.

| Field | Semantics |
|---|---|
| `role_talent { dropback \| carry \| target }` | `{ filtered_mean, filtered_var, predictive_var }` from the state-space layer. **Filtered or one-step predictive only** |
| `form`, `scheme_fit` | Filtered state components |
| `offseason_rapm` | `{ rating, season, n_plays }`, or null. Fitted only once that season's participation is published *(DR-C1)* |
| `prior` | `{ mean, var, q, n0, weight_now }`, or null. The season-boundary prior behind this record's GRID values, and its current weight (engine-spec §6.5). It is labelled with its source (window, feeder or population; DR-D17) and its build (DR-D32): `prior_pre`, built on S−3..S−1 for the preseason and week-1 forecast, or `prior_in`, built on S−2..S−1, from which the season-S filter restarts (`state-space-kalman.md` §6.5). The keys of the two labels are not yet named (§12): the P1-01 type `PriorBlock` carries the five numeric fields and no label field; the contract version that names the keys adds them as a MINOR change (§9) |
| `regime_flags` | `{ intervention, observation_noise_inflation, scheme_reset, changepoint_z }`. The four flags of engine-spec §5.5 ("regime flags (intervention, observation-noise inflation, scheme reset, changepoint z-score)"), which outranks this contract (§1.5): an earlier draft of this row omitted `observation_noise_inflation`, and P1-01 fixed the type `RegimeFlags` to the spec's four. `changepoint_z` is null when no changepoint statistic was computed for the slot |

Hard rule, not conditional on the DRs. **RTS-smoothed values MUST NOT appear in any live or
locked projection record.** Smoothing uses future weeks. Aliasing filtered and smoothed values is
the canonical "correct-looking wrong semantic" bug (cn-docs.md §8 item 6). Smoothed values may
appear only in retrospective diagnostics labelled as such.

A matchup grade, if exposed, uses the corrected sign: higher means a tougher defense, `+E_def`
*(DR-B5)*. The oracle's `−β_def` grade is inverted (KI-NEW-A2) and MUST NOT be
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
interaction is reported) is a model-spec item. It is still open and is set with Statistical-owner
approval before P2-07 is Ready (DR-D8).
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

The engine emits **no VOR, tier or draft-ranking output** *(DR-C11)*.

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
   ShareAlike is a Data/Licensing ruling (DR-D3, open). Until that ruling, exports are private
   and are not redistributed (the DR-D3 interim rule, confirmed 2026-10-09).

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

## 10. Rust types (fixed by P1-01)

The names below are the ones `grid-domain` (`crates/domain`, package `grid-domain`) defines since
P1-01. Every record and enum derives `Serialize` and `Deserialize` (ADR-014, accepted 2026-10-19);
the serialized field names are the contract's field names, so a rename is a contract change (§9).
No record carries `#[serde(deny_unknown_fields)]`: a reader tolerates a MINOR addition. A missing
value is `Option::None` with a documented reason, never 0 or NaN (KI-P1). The crate root re-exports
every item, so `grid_domain::PlayerWeekProjection` and `grid_domain::output::PlayerWeekProjection`
both resolve.

**Contract identity** (`grid_domain::output`): `GRID_OUTPUT_CONTRACT_ID = "grid.output"`,
`GRID_OUTPUT_CONTRACT_VERSION = ContractVersion { major: 1, minor: 0 }`, `grid_output_contract_id()`.
`Horizon { Week, RestOfSeason, Preseason }` (closed; §3.1) and `Position { Qb, Rb, Wr, Te }` (closed;
serialized `QB`, `RB`, `WR`, `TE`; §2) live at the module root.

**Keys, versions and time** (`grid_domain::ids`, `grid_domain::time`, `grid_domain::asof`):
`Season`, `Week` (never 0, KI-A4; `Week::new` is fallible), `Slot`, `PlayerKey { Gsis, Synthetic }`,
`TeamKey { Nfl, Synthetic }`, `GameId`, `DataVersion`, `FeatureSchemaVersion`, `ModelVersionId`,
`PredictionVersionId`, `ScoringProfileId`, `ScoringProfileVersion`, `ScoringProfileRef { id, version }`
(§3.6), `IngestionRunId`, `JobId`, `EvaluationRunId`, `ReviewId`, `ModelFamily` (open),
`EngineVersion { crate_version, git_sha }`, `ContractId`, `ContractVersion { major, minor }`;
`UtcInstant` (whole seconds since the Unix epoch, UTC); `LockKind { Thursday, Sunday, Operational }`
and `Lock { kind, at }` (the `lock` object of §2: the serialized key of the lock timestamp is `at`).

**Player-week projection** (`grid_domain::output::projection`): `ProjectionKey` (§2 keys, with
`horizon`, `is_week_18` and `synthetic_ids`), `ProjectionLineage` (§2 lineage, with `lock`, `seed`,
`n_draws`, `computed_at`), `SourceLineageRecord { prediction_version_id, sources }`,
`AvailabilitySource { Override, Provider, Model, Missing }` (closed) and `AvailabilityBlock` (§3.3),
`StatUnit { Probability, Share, Count, Yards }`, `StatComponent` (closed; the 18 rows of §3.2, with
`applies_to(Position)` encoding the DR-D7 asymmetries, `unit()` and `contract_name()`),
`StatDistribution { mean, sd, q10, q25, q50, q75, q90 }` (§3.5), `ComponentSummary { component,
distribution }`, `StatVectorSummary { components }`, `SettingVersion` (a DR-D8 versioned-setting
identifier), `LabelledQuantile { value, percentile_definition }`, `ExceedanceProbability
{ threshold_points, probability }`, `DrawsRef { matrix_id, n_draws }` (§3.7), `FantasyDistribution`
(§3.6; the DR-D8 items are `SettingVersion` fields with no value fixed), `Freshness { Fresh, Stale,
Missing }` (closed) and `FreshnessFlags` (§3.8), `ClaimLabel` (open; engine-spec §3.2 wording) and
`PlayerWeekProjection { contract, key, lineage, availability, conditional, unconditional, fantasy,
explanation, flags, claim_label, attribution }` with `validate()`.

**Explanation** (`grid_domain::output::explanation`): `TeamGameRef`, `DriverUnits
{ ExpectedFantasyPoints, DvPerPlay }`, `Driver { name, contribution, units }`, `PriorVersionDelta`,
`RoleTalent { filtered_mean, filtered_var, predictive_var }`, `RoleTalentBlock { dropback, carry,
target }`, `OffseasonRapm { rating, season, n_plays }`, `PriorBlock { mean, var, q, n0, weight_now }`,
`RegimeFlags { intervention, observation_noise_inflation, scheme_reset, changepoint_z }`,
`GridSignalBlock` (§4.2; filtered and one-step predictive only, **no smoothed field**, by design) and
`ExplanationRecord` (the nine §4.1 items as ten fields, item 7 being two driver lists, plus
`grid: Option<GridSignalBlock>`).

**Environment, change log and reports** (`grid_domain::output::{environment, change, reports}`):
`EnvironmentSummary { mean, sd, q10, q50, q90 }`, `TouchdownType` (open) and `TeamGameEnvironment`
(§5); `ChangeAttribution` (exactly the five §6 categories) and `ProjectionChange` (§6);
`DataQualityReport` with `QuarantinedRowRef` and `SourceChange`, `SourceFreshness`, `StatusReport`
with `JobSummary`, `AvailabilityReportRow`, `IdentityReviewItem`, `ScorecardSlice { All,
RookieLowEvidence }` (closed), `ScorecardRow`, `ModelScorecard`, `ConfidenceInterval`,
`BenchmarkResultRow` and `BenchmarkResults` (§7).

**Typed errors and status** (`grid_domain::error`, `grid_domain::status`): `OutputInvariant` (ten
variants, one per §3.9 rule, with the two per-draw rules as two variants; `NegativeSd` is the
sd ≥ 0 rule that §3.9 carries from engine-spec §5; `ComponentNotForPosition` and
`DuplicateComponent` are the component-set rule) and `OutputInvariantError { invariant, field, detail }`, returned by the
`validate()` methods; `EngineErrorCode` (closed, 17 codes) and `EngineError { code, message }`;
`TerminalState` (14 values) with `exit_code()` (§13), `JobStatus`, `StageId`, `RunOutcome` and
`TrainingStatus` (engine-spec §8.4). The `validate()` methods check the §3.9 invariants that a
summary record can carry: probabilities and shares in [0, 1], quantiles monotone in every
`StatDistribution` and `FantasyDistribution`, `sd ≥ 0`, no NaN or infinity, and the summary-level
consequences of the per-draw rules (completions ≤ attempts and receptions ≤ targets in mean and every
quantile; a `p_active = 0` record with non-zero unconditional summaries). The per-draw forms are
checked on draw matrices by `grid-simulation` (P1-08).

**Serialization and formats** (`grid-pipeline`, `crates/pipeline`; ADR-014 decision 3).
- `grid-domain` depends on `serde` with `derive` and on nothing else outside `std`
  (ADR-014, accepted 2026-10-19; the structural test `crates/pipeline/tests/crate_boundaries.rs`
  fails if a second external dependency appears). `Display` and `std::error::Error` are hand-written.
- JSON lives in `grid-pipeline::serialization`: `RecordEnvelope<T> { contract, version, record }`
  stamps the contract id and version on every record (§9), `to_json` refuses a non-finite number,
  and `from_json` rejects a foreign contract id and an unknown MAJOR with a typed
  `EngineError` (§9 "A consumer-facing reader MUST reject an unknown MAJOR"). `serde_json` is an
  already-declared workspace dependency; its `float_roundtrip` feature is enabled so that a parsed
  `f64` returns the exact bits of its shortest decimal form (engine-spec §12.6).
- The sidecar of §8 rule 3 is `ExportSidecar { contract, columns: Vec<ColumnSpec { name, unit,
  nullable }>, prediction_version_id, scoring_profile, row_count, data_sha256, attribution }`.
- The CSV writer (§8 rules 2 and 4) is P1-10's and is either hand-written on `std` or uses a crate
  that needs approval; Parquet follows §8 rule 2.
- Query and command names follow engine-spec §8.2 and §8.3 (`grid_pipeline::commands::COMMAND_NAMES`,
  `grid_pipeline::queries::QUERY_NAMES`): `get_week_projections`, `get_player_projection_detail`,
  `get_projection_distribution`, `get_projection_change_log`, `get_model_scorecard`,
  `get_benchmark_results`, `get_data_quality_report`, …, exposed through the `grid-pipeline`
  library façade (`EngineQueries`, `EngineCommands`; traits with no implementor in P1-01) and,
  from P1-10, the CLI.

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

- **DR-D8** (decided in part 2026-10-08: each item is a versioned setting, recorded per output
  version and set with Statistical-owner approval before the package that uses it is Ready).
  Still open:
  - the floor/ceiling percentiles (P1-08);
  - the default `p_exceed` thresholds (P1-08);
  - the boom/bust positional starter thresholds (P1-08);
  - the change-attribution decomposition method (§6; P2-07).
- **DR-C12.** The per-dataset nflverse terms and the canonical weekly-stats release behind the
  §3.2 stat definitions (P1-03). The statistical policy is decided.
- **DR-D3.** ShareAlike status of derived exports (§8 rule 6).
- **DR-D13.** Whether RB/WR/TE role credit in the GRID signal block (§4.2) is an individual signal.
- **DR-D14.** The name of the per-record substitution count. When this contract adds the
  `layer1_credit` record, the record carries, as an audit field, the number of its plays whose
  opponent feature used the counted substitute (`layer1-credit.md` §3.2, §4.3).
- **`oldest_source_timestamp` with no source** (§3.8). The field is non-null, but a row whose `freshness` is `Missing` may have no critical source with a timestamp. Whether the field becomes nullable (a MAJOR change once a producer exists) or `Missing` rows always name the source whose timestamp is missing is open, raised by P1-01; no producer exists until P1-08.
- **The key of a derived-horizon record** (§2, §3.1). For `horizon = RestOfSeason | Preseason`, the meaning of `week`, `opponent`, `game_id`, `is_home` and `is_week_18` is not stated; P1-08, which produces the derived sums, fixes the whole key.
- **Serialized spellings not stated by any document** (raised by P1-01). The status enums serialize as their Rust variant names (`"NoNewData"`, `"Succeeded"`), while engine-spec §8.6.4 and §8.10 spell the states `NO_NEW_DATA` and `SUCCEEDED`; `PlayerKey` and `TeamKey` serialize externally tagged (`{"Gsis": "00-…"}`), while §2 describes `player_id` as the `gsis_id`. `crates/pipeline/tests/serialized_spelling.rs` pins the current forms, so a change shows as a reviewed contract change; the owner fixes the forms before the first producer (P1-08) or export (P1-10).
- **Research runs and the benchmark query** (raised by P1-01). `BenchmarkResults` carries `research` (§2), but query 10 (`get_benchmark_results`) returns pages of `BenchmarkResultRow`, so a page can mix research and non-research runs. Whether the row gains `research`, the query returns one `BenchmarkResults` per evaluation run, or research rows are excluded by default is a decision request (evaluation-and-leakage §4.1), before P2-06 implements the query.
- **DR-D17, DR-D32.** The keys of the `prior` field's source label (window, feeder or population)
  and build label (`prior_pre` or `prior_in`) (§4.2), fixed by the contract version that adds them
  (§9).

Settled in the decision interview of 2026-10-08/09: the stat-vector asymmetries (DR-D7), the GRID
signal block (DR-C2, DR-C3), the derived horizons (DR-C4), the rejection of non-affine scoring
rules (DR-D6) and the absence of VOR and tier outputs (DR-C11). Specified on 2026-10-14 to follow
DR-D17 and DR-D32 (not separately put to the owner): the NCAA prior contribution (§4.1 item 5) and
what the `prior` field's source and build labels mean, though not their keys (§4.2).

## 13. Exit codes (engine-spec §8.6.4)

Engine-spec §8.6.4 requires every terminal state of `grid update` to map to a distinct, documented
process exit code, and fixes the numeric mapping in this contract. P1-01 fixes the table below as the
skeleton (engine-spec §8.2, §8.6.4: "skeleton in P1-01, completed in P1-10"). It is implemented by
`grid_domain::status::TerminalState::exit_code()` (`Option<i32>`; `None` for the two non-terminal
states), the constants `grid_domain::status::EXIT_USAGE` and `EXIT_INTERNAL`, and printed by
`grid exit-codes`. A `grid-domain` test asserts that every terminal code is distinct and equal to
this table.

| Terminal state (engine-spec §8.6.4 spelling) | `TerminalState` variant | Exit code |
|---|---|---|
| `SUCCESS` | `Success` | 0 |
| `NO_NEW_DATA` | `NoNewData` | 10 |
| `PARTIAL_SUCCESS` | `PartialSuccess` | 11 |
| `NETWORK_FAILURE` | `NetworkFailure` | 20 |
| `TIMEOUT` | `Timeout` | 21 |
| `RATE_LIMITED` | `RateLimited` | 22 |
| `AUTHENTICATION_FAILURE` | `AuthenticationFailure` | 23 |
| `INVALID_RESPONSE` | `InvalidResponse` | 24 |
| `SCHEMA_FAILURE` | `SchemaFailure` | 25 |
| `DATA_VALIDATION_FAILURE` | `DataValidationFailure` | 26 |
| `DATABASE_FAILURE` | `DatabaseFailure` | 30 |
| `LEARNING_FAILURE` | `LearningFailure` | 40 |
| `NOT_RUN`, `RUNNING` | `NotRun`, `Running` | none: not terminal, never a process exit |

Two process-level codes are not terminal states:

| Condition | Constant | Exit code |
|---|---|---|
| Usage error: an unknown, malformed or not-yet-available subcommand (message on stderr) | `EXIT_USAGE` | 2 |
| Internal error: a failure the engine cannot classify, including a closed output stream | `EXIT_INTERNAL` | 70 |

The terminal-state enumeration is closed (engine-spec §8.6.4, "a binding enum"): a new state or a
moved code is a change to the spec and a MAJOR change to this table. P1-10 completes the CLI spelling
and arguments of every subcommand that returns these codes; until then the first argument selects
the invocation: `--version` or `-V`, `--help`, `-h` or `help` (and no argument at all) and
`exit-codes` exit 0 and ignore any further arguments until P1-10 fixes the argument grammar; every
other first argument exits 2 with a message on stderr. `crates/grid-cli/tests/cli.rs` asserts
`--version`, `--help`, `help`, no arguments, `exit-codes`, 13 recognised-but-unavailable argument
lists and 5 unknown ones; `-V` and `-h` are accepted by the parser and asserted by no test.
