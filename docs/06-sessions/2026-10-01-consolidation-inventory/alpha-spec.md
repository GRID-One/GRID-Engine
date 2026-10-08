# Inventory: `alpha-spec.md` for the GRID-Engine engine-only pivot

**Source:** `/home/user/GRID-Engine/alpha-spec.md` (2186 lines, branch `claude/grid-engine-consolidation-e7kmh9` @ `3823478`).
**Mirror:** `docs/00-meta/specs/alpha-spec.md` is byte-identical (`diff -q` clean). `scripts/check-authority-sync.sh` enforces this, and `tests/guards/run.sh:229-231` tests the guard.
**Review branches:** neither `origin/claude/grid-alpha-adversarial-review-ikkjuk` nor `origin/claude/adversarial-review-p1-00-l32emp` modifies `alpha-spec.md`, the mirror, or `final-build-spec.md`. The spec is unchanged since `48ee320 Add files via upload`.
**Pivot premise (owner decisions already made):** engine-only scope; Rust is the target (trimmed workspace, no Flutter/FFI/desktop); cautious-nevermore's Python engine is imported under `reference/python/` as an executable reference oracle whose synthetic-recovery gates and golden masters become parity targets.

Classification legend: **ENGINE** (modeling, features, targets, outputs, simulation, scoring), **VALIDATION** (backtests, metrics, gates, benchmarks, leakage), **DATA** (providers, freshness, identity, snapshots), **PLATFORM** (Rust core, persistence, jobs, versioning), **PROCESS** (agent governance, WPs, evidence, review; kept), **APP-ONLY** (dropped), **MIXED**.

---

## 0. Key findings (read first)

1. **About 85% of the spec survives the pivot almost verbatim.** §2, §4, §5.1–5.4, §6.1–6.4, §7, §11 and §12.1–12.5 contain no app dependency apart from a few stray "UI"/"app" words. The app-only material sits in §1.1 (3 bullets), §5.5, §8 (diagram top, §8.4 last line), §9.2 "Native UI", §10.2 "Native UI additions", §12.6, the §13 UI latency targets, §17.1 "installer" and UI lines, §18 #1/#13/#14, and the Windows-is-the-product-target rationale in §8.11.
2. **Don't delete the "Native UI" lists. Convert them.** Several of their items are engine requirements in disguise and are stated nowhere else:
   - CSV export of projections and quantiles only, with **no competitor data export** (§9.2 item 5).
   - Data-freshness indicator per projection (§9.2 item 1).
   - Projection change log with **change attribution** (role, availability, matchup, team environment, model update) (§10.2 item 2).
   - Model scorecard slices (§10.2 item 3).
   - Data Quality Console contents: stale sources, unresolved identities, quarantined rows, missing current-week context (§10.2 item 5).
   - Identity-review queue (§4.4.3).

   Each becomes a CLI/report/query output. §5.5's contract-first rules also transfer intact to the engine's public output contract.
3. **"No Python runtime in the installed application" (§1.1) must be rewritten**, not just kept or dropped. Proposed text is in §1.1 below: Python is permitted only under `reference/python/` as a dev/CI-time oracle, with no build or run dependency from any Rust crate (no pyo3), and oracle outputs enter Rust only as committed, hashed fixtures.
4. **Authority order (§1.5) puts `final-build-spec.md` at level 1.** fbs's first non-negotiable is "native Flutter desktop UI". The pivot cannot be internally consistent until the authority order is rewritten, and it also has to place the Python oracle somewhere. Recommendation: below spec, ADRs and contracts, at the tests/fixtures tier, and never able to override the spec. See §1.5.
5. **Oracle-vs-spec conflict (material).** The Python walk-forward accumulates in-season FTN participation through W-1 into RAPM at every origin (`backend/validation/backtest.py`, `asof.py::slice_pool`). Under alpha §1.2, §4.1 (Participation row), §12.3 #4 and §15, that is train/serve skew, because participation is published only after the postseason. Both sources agree on that publication lag: Appendix A and cautious-nevermore `docs/01-product-roadmap.md:217`. The Rust parity targets for RAPM-in-backtest must therefore be labeled research-only, or the as-of layer needs a **publication-lag axis**. This matters for the spec rewrite and for P1-05.
6. **Two modeling architectures must be reconciled.**
   - Alpha §6.1 specifies six layers, A–F: availability → team environment → opportunity → efficiency → context → correlated simulation.
   - The Python GRID engine is V(s)/dV → participation RAPM → 3-component Kalman → cross-league priors → volume(EB) × efficiency stat line → affine scoring.

   GRID covers parts of Layers C and D, the §6.3 priors, and ensemble components 3 and 5 (§6.4). It has **no** Layer A, Layer B, Layer F, quantile distributions, PB-MAE, player pool or locks. Recommendation: alpha §6.1 governs, and GRID components are ported as ensemble members and primitives (§5 below).
7. **The guard scripts constrain new work-package IDs.** `scripts/check-traceability.sh:70` only recognizes `P1-[0-9]{2}`, `ADR-[0-9]{3}` or literal doc paths. **P2-xx IDs fail it today**, and so would any new prefix (`E1-01`, `P1-00a`). `scripts/check-evidence-claims.sh:29` hard-codes `WP="P1-00"`. Any re-sequencing has to extend the regex in a governed package, or stay inside `P1-NN`.
8. Python scoring (`cautious-nevermore/backend/scoring/formats.py`) **matches alpha §2.3 Half-PPR exactly**: 0.04 / 4 / −2 / 0.1 / 6 / 0.5 / −2 / 2, with Standard receptions 0.0 and PPR 1.0. That makes scoring the cleanest first parity port.

---

## 1. Section classification table

| § | Title | Lines | Class | Disposition |
|---|---|---|---|---|
| hdr | Title block | 1–12 | MIXED | Rewrite title/platform/authority lines; keep data foundation, agent and mode lines |
| 0 | Executive Intent | 15–32 | MIXED | Keep claim discipline, phases, no runtime Claude; "native Windows application" becomes "engine" |
| 1 | Relationship to the Production Architecture (intro) | 35–37 | MIXED | Rewrite "temporary Python service" sentence |
| 1.1 | Non-negotiable architecture constraints | 39–54 | MIXED | Drop 2 bullets, rewrite 1, keep 11 |
| 1.2 | Alpha-specific architecture interpretation | 56–75 | ENGINE/DATA | Keep verbatim |
| 1.3 | AI-assisted development boundary | 78–102 | PROCESS | Keep; drop "Dart/Flutter", "installer" becomes "release artifact" |
| 1.4 | Human authority and required review roles | 104–114 | PROCESS | Keep; "installer publication" becomes "release artifact publication" |
| 1.5 | Order of authority | 116–128 | PROCESS | **Rewrite list**; keep conflict/stop paragraph |
| 1.6 | AI-first engineering principles | 130–139 | PROCESS | Keep verbatim |
| 2.1 | Core alpha positions | 145–154 | ENGINE | Keep |
| 2.2 | Projection horizon | 156–161 | ENGINE/VALIDATION | Keep; "in the UI" becomes "by the engine" |
| 2.3 | Default scoring profile | 163–176 | ENGINE | Keep verbatim |
| 2.4 | Exact three-season rule | 178–188 | ENGINE/DATA/VALIDATION | Keep verbatim |
| 2.5 | Newer/low-evidence player | 190–205 | ENGINE | Keep verbatim |
| 3.1 | Product success | 211–220 | VALIDATION | Keep; "Product" becomes "Engine" |
| 3.2 | Permitted wording by evidence level | 222–228 | VALIDATION/PROCESS | Keep; "product wording" becomes "published output/report wording" |
| 4.1 | nflverse inputs (table) | 234–250 | DATA | Keep verbatim |
| 4.1.1 | Data freshness policy | 252–266 | DATA | Keep verbatim |
| 4.1.2 | Injury and availability gap | 268–288 | DATA/ENGINE | Keep; "model or UI contracts" becomes "model or output contracts" |
| 4.2 / 4.2.1 / 4.2.2 | NCAA source, datasets, call budget | 290–313 | DATA | Keep verbatim |
| 4.2.3 | NCAA features by position | 315–347 | ENGINE | Keep verbatim |
| 4.3 | Other context inputs | 349–359 | DATA | Keep verbatim |
| 4.4.1–4.4.3 | Identity resolution | 361–398 | DATA | Keep; last bullet "UI identity-review queue" becomes an engine review queue |
| 4.5 | As-of snapshots and leakage prevention | 400–421 | VALIDATION/DATA | Keep; **add publication-lag axis** |
| 4.6 | Provider contracts for AI implementation | 423–447 | DATA/PROCESS | Keep; path becomes `docs/04-providers/` |
| 5.1 | Stat-vector first design | 453–499 | ENGINE | Keep verbatim |
| 5.2 | Conditional/unconditional | 501–508 | ENGINE | Keep; rewrite the UI sentence |
| 5.3 | Required distribution outputs | 510–524 | ENGINE | Keep verbatim |
| 5.4 | Scoring transformation | 526–534 | ENGINE | Keep verbatim |
| 5.5 | Contract-first Rust/Flutter boundary | 537–547 | APP-ONLY→MIXED | **Convert** to an engine public API/output contract |
| 6.1 | Structural decomposition (Layers A–F) | 553–624 | ENGINE | Keep verbatim |
| 6.2 | Statistical methods mapping | 626–638 | ENGINE | Keep verbatim |
| 6.3 | NCAA prior formulation | 640–667 | ENGINE | Keep verbatim |
| 6.4 | Ensemble design | 669–679 | ENGINE | Keep verbatim |
| 6.5 | Explainability contract | 681–695 | ENGINE | Keep; "The UI must not say" becomes "Explanation payloads must not say" |
| 6.6 | AI requirements for statistical code | 698–723 | ENGINE/PROCESS | Keep; "exposed to the UI" becomes "exposed in the output contract"; path becomes `docs/05-model-specs/` |
| 7.1 | Historical validation | 729–739 | VALIDATION | Keep verbatim |
| 7.2 | Projection locks | 741–750 | VALIDATION | Keep; "The app may publish" becomes "The engine may produce" |
| 7.3 | Player pool | 752–774 | VALIDATION | Keep verbatim |
| 7.4 | Primary metric (PB-MAE) | 776–801 | VALIDATION | Keep verbatim |
| 7.5 | Secondary metrics | 803–818 | VALIDATION | Keep verbatim |
| 7.6 | Benchmark provider registry | 820–852 | VALIDATION/DATA | Keep; "never redistributed in the app" becomes "in any engine output" |
| 7.7 | Statistical comparison | 854–860 | VALIDATION | Keep verbatim |
| 7.8 | Phase 2 competitive exit gate | 862–873 | VALIDATION | Keep verbatim |
| 7.9 | Full market-superiority claim gate | 875–889 | VALIDATION | Keep verbatim |
| 7.10 | Implementation correctness gate | 892–904 | VALIDATION | Keep verbatim (+ oracle parity as an optional evidence line) |
| 7.11 | Separation of implementer and evaluator | 906–915 | PROCESS/VALIDATION | Keep verbatim |
| 8 | System Architecture diagram | 919–952 | MIXED | Drop the Flutter UI block and FRB; keep the Rust core list |
| 8.1 | Rust module boundaries | 954–983 | PLATFORM | Keep minus `ffi/`; add `cli/` |
| 8.2 | Commands | 985–996 | MIXED→PLATFORM | Keep the 10 commands as library API + CLI subcommands |
| 8.3 | Queries | 998–1010 | MIXED→PLATFORM | Keep the 11 queries as library API + CLI/report outputs |
| 8.4 | Events | 1012–1029 | MIXED→PLATFORM | Keep the 14 events; rewrite the "notify Flutter" sentence |
| 8.5 | SQLite schema groups | 1031–1102 | PLATFORM | Keep verbatim |
| 8.6 | Daily incremental pipeline | 1104–1138 | PLATFORM/ENGINE | Keep; "Notify UI" becomes "emit completion event/report" |
| 8.7 | Claude-ready repo structure | 1141–1200 | MIXED (PROCESS/PLATFORM) | Drop Flutter/FFI paths; add `reference/python/` |
| 8.7.1 | `CLAUDE.md` policy | 1202–1216 | PROCESS | Keep; "native Flutter/Rust/SQLite constraints" becomes "Rust/SQLite + reference-oracle constraints" |
| 8.8 / 8.8.1 / 8.8.2 | Work-package contract, DoR, DoD | 1218–1269 | PROCESS | Keep; "generated bindings" becomes "generated artifacts" |
| 8.9 | Required execution workflow | 1271–1287 | PROCESS | Keep verbatim |
| 8.10 | Specialized agent roles | 1289–1301 | PROCESS | Drop "Flutter implementer"; add "Reference-parity reviewer" |
| 8.10.1 | Budgets and model routing | 1303–1309 | PROCESS | Keep verbatim |
| 8.11 | Canonical verification interface | 1311–1336 | MIXED (PROCESS/PLATFORM) | Drop Flutter/FRB lines; **re-decide Windows authority**; add oracle parity checks |
| 8.12 | AI evidence and provenance | 1338–1353 | PROCESS | Keep verbatim |
| 9.1 | Phase 1 goal | 1359–1361 | VALIDATION/ENGINE | Keep |
| 9.2 | Phase 1 functional scope | 1363–1452 | MIXED | Keep 5 of 6 sub-lists; convert "Native UI" |
| 9.3 | Phase 1 NFRs | 1454–1461 | MIXED | Keep 4; rewrite 2 |
| 9.4 | Phase 1 exit criteria | 1463–1495 | MIXED | Keep all thresholds; rewrite 3 bullets |
| 9.5 | Phase 1 workstream sequence | 1497–1532 | PROCESS | Re-sequence (§4–5 of this report) |
| 10.1 | Phase 2 goal | 1538–1540 | VALIDATION | Keep; "the app" becomes "the engine" |
| 10.2 | Phase 2 functional scope | 1542–1617 | MIXED | Keep 5 of 6 sub-lists; convert "Native UI additions" |
| 10.3 | Phase 2 operational invariants | 1619–1627 | ENGINE/VALIDATION | Keep; rewrite the UI-label bullet |
| 10.4 | Phase 2 exit criteria | 1629–1652 | MIXED | Keep all thresholds; rewrite installer and FFI bullets |
| 10.5 | Phase 2 workstream sequence | 1655–1677 | PROCESS | Re-sequence (§4–5 of this report) |
| 11.1–11.5 | Feature families | 1681–1745 | ENGINE | Keep verbatim |
| 12.1 | Unit tests | 1751–1764 | VALIDATION | Keep verbatim |
| 12.2 | Golden numerical tests | 1766–1777 | VALIDATION | Keep; add oracle-derived golden fixtures |
| 12.3 | Point-in-time and leakage tests | 1779–1785 | VALIDATION | Keep; add publication-lag test |
| 12.4 | Integration tests | 1787–1793 | VALIDATION | Keep verbatim |
| 12.5 | Failure tests | 1795–1808 | VALIDATION | Keep verbatim |
| 12.6 | FFI and UI tests | 1810–1817 | APP-ONLY→MIXED | Convert 3 of 6 to serialization/API/runtime tests; drop 3 |
| 12.7 | AI-specific regression protections | 1820–1835 | PROCESS | Keep; add oracle-tampering shortcut |
| 12.8 | PR evidence contract | 1837–1853 | PROCESS | Keep; drop the Flutter screenshot line; "FFI impact" becomes "public API/output-contract impact" |
| 12.9 | Independent review checklist | 1855–1868 | PROCESS | Keep; rewrite Q2 and Q9 |
| 13 | Performance and resource targets | 1872–1893 | MIXED | Keep engine targets; drop UI targets; re-decide the reference OS |
| 14 | Security, licensing, governance | 1897–1908 | MIXED (PLATFORM/DATA) | Keep; rewrite 2 bullets |
| 14.1 | Coding-agent security boundary | 1911–1921 | PROCESS | Keep; "sign installers" becomes "sign release artifacts" |
| 14.2 | Dependency policy | 1923–1931 | PROCESS | Keep; "Windows compatibility" becomes "supported-platform compatibility"; add Python reference-dependency rule |
| 15 | Key risks and mitigations | 1935–1957 | MIXED | Keep 18 rows (rename 1); add 3 rows |
| 16 | Explicit alpha non-goals | 1961–1980 | MIXED | Keep; fold the UI items into "any UI"; rewrite "shipped application" |
| 17.1 | Phase 1 deliverables | 1986–1999 | MIXED | Keep 9; convert installer and UI lines |
| 17.2 | Phase 2 deliverables | 2001–2011 | VALIDATION/ENGINE | Keep verbatim |
| 17.3 | AI evidence deliverables | 2014–2024 | PROCESS | Keep; "Windows" line is per platform decision |
| 18 | Final Definition of Done | 2028–2048 | MIXED | Keep 13 items; rewrite #1, #4, #11, #13, #14; add an oracle-parity item |
| App A | Source verification notes (2026-08-12) | 2052–2060 | DATA | Keep; re-verify; add participation licensing note |
| App B | Work-package template | 2064–2131 | PROCESS | Keep; "User-visible outcome" becomes "Observable outcome"; update authority lines |
| App C | PR completion record | 2133–2155 | PROCESS | Keep verbatim |
| App D | Prohibited AI shortcuts | 2157–2172 | PROCESS | Keep; edit #2 and #3; add #13 |
| App E | Claude Code workflow notes | 2174–2185 | PROCESS | Keep verbatim |

**Formatting defect to fix in the rewrite:** subsection heading levels are inconsistent. §1.1–1.6, §2.x, §3.x and §4.1.1/§4.1.2 use `###`, but §4.1, §4.2, §5.1–§17.3 use `##`, the same level as top-level sections. That breaks outline tooling and any heading-based guard.

---

## 2. Per-section detail: binding requirements and required wording changes

### Header block (lines 1–11). MIXED

Current fields:
- Title: "NFL Weekly Fantasy Projection App — Two-Phase Alpha Specification / Claude Opus 5 Implementation Edition"
- Status, Date: August 12, 2026
- **Target platform: Native Windows desktop**
- Primary data foundation: nflverse
- Rookie supplement: "CollegeFootballData (CFBD) REST API or an equivalently licensed free NCAA source"
- Primary implementation agent: Claude Opus 5 via Claude Code or equivalent harness
- Implementation mode: "AI-first, contract-driven, human-governed"
- **Production architecture authority: `final-build-spec.md`**

Changes:
- Title becomes something like "GRID Engine — NFL Weekly Projection Engine Specification (two-phase)".
- "Target platform" becomes "Deliverable: Rust engine (library crates + CLI) over SQLite; supported build/run platforms: <decision>".
- The "Production architecture authority" line goes away, or points at the consolidated engine spec / ADR that supersedes fbs.
- Add "Reference implementation: `reference/python/` (dev/CI-time oracle only)".

### §0 Executive Intent (15–32). MIXED

Binding, keep near-verbatim:
- "produces highly optimized, week-by-week NFL fantasy-football projections... generate player stat distributions first and then translate them into fantasy points for Standard, Half-PPR, PPR, and user-defined scoring systems."
- Objective: "most accurate publicly available weekly fantasy projection service for core offensive positions", treated as a **falsifiable validation target**. "must not make a public 'most accurate' or 'better than every service' claim until the market-superiority gate... has been passed using timestamped, pre-kickoff projections and a published comparison protocol."
- Dual role of the document: engineering contract for the agent. "Requirements must be decomposable into bounded work packages, grounded in versioned interfaces and fixtures, and verifiable by commands that return objective pass/fail evidence. Claude may implement the system, but it does not own product requirements, statistical claims, security decisions, data rights, model promotion, release approval, or merge authority."
- "no runtime dependency on Claude, Claude Code, Anthropic APIs, or any other coding model... remain buildable and maintainable by human engineers and by a replacement coding agent."
- Two non-throwaway phases:
  - **Phase 1, Projection Core and Historical Proof**: ingestion, identity resolution, feature generation, rookie priors, baseline models, rolling-origin backtests, ~~minimal native projection UI~~.
  - **Phase 2, Live Weekly Intelligence and Competitive Proof**: daily live operation, availability handling, probabilistic simulation, advanced ensemble, promotion/rollback, competitor snapshot evaluation, market-claim evidence.

Wording changes:
- "Build a native Windows application" becomes "Build a Rust projection engine (library + CLI)".
- "The production application must have no runtime dependency on Claude..." becomes "The engine's runtime and every released artifact must have no dependency on Claude...".
- Phase 1 "a minimal native projection UI" becomes "a CLI/report/export surface for projections, status and scorecards".
- Optionally add a sentence: "The engine has no UI; consumers (apps, dashboards) live outside this repository and integrate only through the versioned output contract."

### §1 intro (35–37). MIXED

Current: "This alpha must be implemented as a direct subset of the attached production system. It must not introduce a temporary Python service, browser UI, cloud-only inference path, or throwaway data store that would later need to be replaced."

Rewrite to: "The engine must not introduce a Python service or runtime dependency, a cloud-only inference path, or a throwaway data store. The Python implementation under `reference/python/` is a development/CI-time oracle, not a component of the engine." Drop "browser UI" (moot), or fold it into the §16 non-goal "any UI". Drop "direct subset of the attached production system" unless fbs is retained as authority.

### §1.1 Non-negotiable architecture constraints (39–54). MIXED

| Bullet (verbatim) | Disposition |
|---|---|
| Native Flutter Windows UI only. | **DROP.** Replace with: "No user interface in this repository. The engine exposes a Rust library API, a CLI, and versioned file/DB outputs." |
| Rust application and computation core. | KEEP: "Rust engine core." |
| `flutter_rust_bridge` v2 as the UI/core adapter. | **DROP** (also drop `flutter_rust_bridge = "=2.5.0"` from `Cargo.toml` `[workspace.dependencies]` and `crates/ffi`) |
| SQLite as the durable source of truth. | KEEP |
| SQLx migrations and compile-time-checked query workflow. | KEEP |
| Tokio for asynchronous I/O and job coordination. | KEEP |
| Rayon and/or `spawn_blocking` for CPU-heavy feature generation, fitting, simulation, and evaluation. | KEEP |
| Polars may be used inside Rust for analytical transformations. | KEEP (note: not yet in `[workspace.dependencies]`) |
| **No Python runtime in the installed application.** | **REWRITE** (below) |
| External data is fetched at most once per local calendar day; no continuous polling. | KEEP |
| Deterministic feature, model, data-snapshot, and prediction versioning. | KEEP |
| Candidate validation before promotion. | KEEP |
| Snapshot, rollback, crash recovery, and resumable durable jobs. | KEEP |
| The statistical engine retains first-class interfaces for ridge regression, RAPM-style sparse regularized effects, Kalman filtering, RTS/fixed-lag smoothing, empirical-Bayes shrinkage, affine transformations, and gradient boosting. | KEEP verbatim (ENGINE core) |

**Proposed replacement for the Python bullet:**
> No Python at engine build time or run time. No engine crate, binary, or release artifact may embed, spawn, link to, or require a Python interpreter or Python package (including pyo3/FFI bindings), and no engine code path may read files produced at run time by Python. Python is permitted only under `reference/python/`, as the executable reference oracle: it runs in development and CI to produce parity evidence, and its outputs enter the Rust test suite only as committed, content-hashed fixtures and golden files. Removing `reference/python/` must not break `cargo build` or `cargo test` of any engine crate, except for parity tests explicitly gated on the presence of oracle fixtures.

(Design note: parity tests should read committed fixtures, not invoke Python. A separate CI job runs the oracle's own pytest suite and a fixture-drift check that regenerates oracle fixtures and diffs their hashes.)

### §1.2 Alpha-specific architecture interpretation (56–75). ENGINE/DATA, keep verbatim

- The football domain replaces fbs's generic "possessions, lineups, and stints" with 13 entities: games, drives, plays, player-week and player-game statistics, team-week and team-game statistics, roster snapshots, depth-chart snapshots, snap counts, player participation when historically available, college player seasons and games, player identity links, weekly projection runs, player-week outcome distributions, market benchmark snapshots.
- Binding: "Full 22-player on-field participation is not reliably available in-season from the specified free sources. Therefore, a traditional basketball-style RAPM implementation is **not allowed to become a hidden dependency of the live projection path**. RAPM-style player/team effect models may be used when supported by the data, but the production ensemble must remain valid when current-season participation fields are unavailable."
- Wording change: only "The attached production document" becomes "The generic production architecture (formerly `final-build-spec.md`)", if fbs is demoted.
- **Oracle note to add:** the Python engine's RAPM (`backend/grid/layers.py`) is participation-based. Its use for prior-season ratings and preseason priors is compliant. Its in-season walk-forward use is research-only under this rule (see Finding 5).

### §1.3 AI-assisted development boundary (78–102). PROCESS, keep

- "Claude Opus 5" means the project-designated frontier coding model, referred to through a **configurable model alias** (not an embedded public API id). The actual model id, harness version, permission profile and environment are recorded per WP evidence bundle.
- **May (7):** read-only exploration and dependency tracing; implementation planning against an approved WP; ~~Rust, Dart/Flutter, SQL, build-script, test, and documentation changes~~ becomes "Rust, SQL, build-script, test, documentation, and reference-oracle fixture-export changes"; fixture generation from approved, sanitized source samples; local build/lint/test/benchmark/migration verification; atomic commits and PR descriptions; first-pass code review and remediation.
- **May not independently (9):**
  - change the non-negotiable architecture
  - redefine projection targets, benchmark metrics, lock rules, claim gates, or statistical formulas
  - select a new data source with unapproved license/terms
  - add a production dependency outside policy
  - read or expose production credentials, signing keys, private provider exports, or unrelated user files
  - modify or delete historical migrations to make a build pass
  - approve its own architectural or statistical changes
  - merge to a protected branch, ~~sign an installer~~ sign a release artifact, publish a release, or deploy external infrastructure
  - promote a candidate model solely because code compiles or a backtest improved
- **Add (recommended):** "modify `reference/python/` engine semantics or regenerate oracle golden fixtures to make a Rust parity test pass."

### §1.4 Human authority and review roles (104–114). PROCESS, keep

Five roles; one person may hold several:
1. **Product/architecture owner:** requirements and architecture conflicts, ADRs, scope.
2. **Statistical owner:** model equations, priors, evaluation design, calibration changes, promotion criteria. Add: "and approves accepted divergences from the reference oracle."
3. **Data/licensing owner:** provider access methods, retention, attribution, benchmark import rights.
4. **Security/release owner:** agent permissions, secret handling, dependencies, signing, ~~installer publication~~ release-artifact publication, releases.
5. **Merge reviewer:** final diff and evidence bundle; "the implementation agent cannot be the sole reviewer."

Risk-based review: explicit human approval for any change to architecture, destructive-potential schemas, statistical semantics, data rights, security boundaries, benchmark rules, or release packaging.

### §1.5 Order of authority (116–128). PROCESS, rewrite the list

Current list:
1. `final-build-spec.md`
2. this alpha spec
3. accepted ADRs in `docs/adr/` (the repo actually uses `docs/02-adr/`)
4. versioned contracts, model specs, schemas, provider manifests
5. the approved WP file
6. tests and fixtures implementing approved contracts
7. existing source code, comments, local conventions

Keep verbatim: "Existing code is not authoritative merely because it already exists. Tests are not authoritative if they contradict a higher-level approved requirement. When Claude detects a conflict or an absent decision that materially changes behavior, it must stop that work package at a clean boundary and produce a decision request rather than silently choosing an interpretation."

**Proposed engine-only order:**
1. the consolidated engine spec
2. accepted ADRs (`docs/02-adr/`)
3. contracts, model specs, provider manifests (`docs/03-contracts/`, `docs/05-model-specs/`, `docs/04-providers/`)
4. the approved WP
5. tests and fixtures implementing approved contracts, including **oracle-derived golden fixtures**
6. the reference implementation `reference/python/` as behavioral evidence
7. existing Rust source and conventions

Add the rule: "Where the oracle's behavior contradicts a higher authority (e.g., it violates the point-in-time rules of §4.5), the higher authority wins and the divergence is recorded in an ADR approved by the statistical owner. Oracle behavior is never a reason to weaken a spec rule."

Files that restate this list and must change in lockstep: `CLAUDE.md:6-13`, `docs/CLAUDE.md:5-9`, `docs/00-meta/authority-index.md:7-15`, the `docs/01-work-packages/p1-00-work-package.md` acceptance criterion, and `scripts/check-authority-sync.sh` (which hard-codes the filenames `alpha-spec.md` and the mirror).

### §1.6 AI-first engineering principles (130–139). PROCESS, keep verbatim

Eight principles: contract before implementation; explore→plan→implement→verify→review; bounded work packages; objective verification (commands, fixtures, tests, or visual artifacts); fresh-context review; no hidden tribal knowledge; model independence ("must not leak into production runtime behavior"); evidence over assertion.

### §2.1 Core alpha positions (145–154). ENGINE, keep

- Primary accuracy target: **QB, RB, WR, TE**.
- K and DST may be added in Phase 2, but "scored and reported separately and do not contribute to the primary market-superiority claim."
- **IDP out of scope for both phases.**

### §2.2 Projection horizon (156–161). ENGINE/VALIDATION, keep

- One NFL regular-season week at a time.
- Weeks 1–18 supported ~~in the UI~~ by the engine.
- Weeks 1–17 used for the primary market accuracy score.
- Week 18 evaluated separately ("many fantasy leagues end earlier and NFL playing-time incentives differ").
- *Consolidation gap:* the Python engine's primary horizons are **preseason (draft-prep) season stat lines** (`backend/projection/preseason.py`) and **rest-of-season** (`validation/tier1.py` H1 "ROS" kill criterion). Neither appears in alpha §2.2. **Owner decision** (see Decisions).

### §2.3 Default scoring profile (163–176). ENGINE, keep verbatim

Half-PPR default comparison profile:
- Passing yards 0.04/yd
- Passing TD 4
- INT −2
- Rushing/receiving yards 0.1/yd
- Rushing/receiving TD 6
- Reception 0.5
- Fumble lost −2
- Two-point conversion 2

"Standard and full-PPR are built-in profiles. Custom scoring is represented as a versioned affine mapping from a projected stat vector to fantasy points."

- Implied but not enumerated: Standard = reception 0, PPR = reception 1.0. **Make these explicit in the rewrite.** They match `cautious-nevermore/backend/scoring/formats.py` exactly.
- Note for the rewrite: an affine map **cannot represent threshold bonuses** (e.g., a 300-yard bonus) or per-position reception weights unless position is part of the stat vector. State this limitation explicitly, or make a decision.

### §2.4 Exact three-season rule (178–188). ENGINE/DATA/VALIDATION, keep verbatim

For season `S`, week `W`:
- If `W > 1`: season-to-date data from `S` through week `W-1`, plus seasons `S-1` and `S-2`.
- Preseason and Week 1: `S-1`, `S-2`, `S-3`.
- "No future week, postseason result, corrected statistic not yet available at the projection timestamp, or later depth-chart state may enter the feature set."
- Older NFL data retained only for walk-forward tests in which "each historical prediction still uses its own three-season window."
- Static metadata (draft position, combine, age, college identity) may predate the window.

### §2.5 Newer/low-evidence NFL player (190–205). ENGINE, keep verbatim

An NCAA-informed prior applies when any of these holds:
- `years_exp <= 2`
- fewer than the position-specific minimum NFL opportunity threshold
- position change without a stable NFL sample at the new position
- undrafted or late-added player with no usable NFL game sample

Opportunity weighting:
- QB: dropbacks, pass attempts, designed rushes, scrambles
- RB: offensive snaps, carries, targets, goal-line opportunities
- WR/TE: offensive snaps, targets, air yards, red-zone targets

"The NCAA prior decays continuously as NFL evidence accumulates; it is not switched off abruptly by season count."

Gap: the numeric thresholds are unspecified. They belong in a model spec.

### §3.1 Product success (211–220). VALIDATION, keep (rename "Engine success")

Six criteria:
1. Reproduce a historical weekly projection from exact data/feature/model versions.
2. Complete weekly stat line and fantasy-point distribution for every eligible QB/RB/WR/TE.
3. Update from the prior production model incrementally after daily ingestion.
4. Quantify uncertainty and availability risk (not only a point estimate).
5. Out-of-sample improvement over transparent baselines.
6. Fair comparison with legally obtained market projections under pre-registered rules.

### §3.2 Permitted wording by evidence level (222–228). VALIDATION/PROCESS, keep

| Evidence level | Permitted wording |
|---|---|
| Before Phase 1 exit | "Experimental projections." |
| After Phase 1 exit | "Historically validated projections." |
| After Phase 2 exit | "Live, independently timestamped projections" + factual benchmark results |
| After the full gate only | "Most accurate in the published benchmark panel", disclosing season, providers, scoring system, player pool, metric |

"more accurate than any service on the market" is prohibited unless benchmark coverage and independent audit support it.

Change "product wording" to "wording in any published engine output, report, model card, or README".

### §4.1 nflverse inputs (234–250). DATA, keep table verbatim

| Dataset | Use | Live | Notes |
|---|---|---|---|
| PBP | situation, pace, EPA, play type, air yards, red zone, game script, opponent | Yes | core; nightly snapshot; retain raw |
| Player weekly stats | weekly stat targets/outcomes | Yes | authoritative labels **after stat-correction window** |
| Team weekly stats | team volume/efficiency | Yes | game-environment models |
| Schedules/games | opponent, venue, kickoff, spread/total | Yes | version every schedule snapshot |
| Players | GSIS identity, cross-source IDs, college, draft | Yes | canonical registry |
| Rosters/weekly rosters | membership/status | Yes | timestamped snapshots |
| Depth charts | role priors, starter hierarchy | Yes, with timestamp semantics | "From 2025 onward, use source timestamp rather than assuming a week field" |
| Snap counts | playing time, role change | Yes | central to opportunity models |
| NGS | rush/rec/pass efficiency | Yes | missing below qualification thresholds modeled explicitly |
| PFR advanced | supplemental efficiency | Usually | never assume every field/week |
| FTN charting | play-level charting | Delayed | only fields available within the prediction timetable; preserve attribution |
| Participation | historical player-on-play/personnel | **No for current in-season use** | research/backtests OK "but cannot create train/serve skew" |
| Injuries | availability, workload suppression | **No current feed after 2024** | mandatory fallback (§4.1.2) |

### §4.1.1 Data freshness policy (252–266). DATA, keep verbatim

Every ingested source records: `source_name`, `source_version`, `source_timestamp`, `retrieved_at`, `ingestion_run_id`, content hash, row count, schema version, validation status.

"A feature is eligible only if its source timestamp is earlier than the projection lock for the relevant player/game."

### §4.1.2 Injury and availability gap (268–288). DATA/ENGINE, keep

- "must not interpret missing injury rows as 'healthy.'"
- Phase 1 uses a versioned operator import with schema `availability_overrides.csv`: `player_id,season,week,status,practice_status,expected_active_probability,expected_snap_multiplier,source_note,observed_at`.
- Phase 2 adds a provider-neutral `AvailabilityProvider` adapter; "can replace the manual import without changing model or ~~UI~~ output contracts."
- Five requirements:
  1. Missing availability data increases uncertainty.
  2. `OUT`, `IR`, `PUP`, suspension, bye and inactive force active probability to **zero**.
  3. Questionable/doubtful/limited affect both active probability and conditional workload.
  4. Every override is timestamped, attributable, and included in the prediction snapshot.
  5. An override entered after a game lock cannot modify the benchmarked pre-lock prediction.

### §4.2 / §4.2.1 / §4.2.2 NCAA source (290–313). DATA, keep verbatim

- Default: CFBD REST API free tier, subject to terms/limits; **adapter must be replaceable**.
- Datasets: rosters/identity; season and game player stats; usage; PPA/advanced (tier-dependent); recruiting; team context and opponent strength; NFL draft picks for identity reconciliation. "Use only fields that can be reproduced and legally retained."
- Call budget:
  - cache all raw responses
  - batched backfill
  - store `X-CallLimit-Remaining` or equivalent
  - hard configurable monthly budget below the provider limit
  - daily NFL update does not re-download unchanged college history
  - in-season NCAA refresh is event-driven (new roster entrant, unresolved identity, explicit operator refresh)

### §4.2.3 NCAA features by position (315–347). ENGINE, keep verbatim

- QB (8): attempts and volume share; completion rate (+ adjusted context); YPA; pass TD and INT rates; rushing attempts/yards/TD share; PPA/success; opponent/conference strength; age, starts, experience.
- RB (7): carries and team carry share; receptions and receiving-yard share; scrimmage yds/opportunity; TD and goal-line proxies; explosive-play rate; PPA/success; opponent/conference strength.
- WR/TE (6): rec, rec yds, TDs; team receiving-yard and TD share; YPR; usage and PPA; age, breakout timing, recruiting, draft capital; opponent/conference strength.

"NCAA features are translated to NFL latent priors; college fantasy points are never inserted directly into an NFL weekly projection."

*Oracle overlap:* `backend/grid/priors.py` handles NCAA/USFL/XFL/UFL→NFL equivalency from shared players. Its feeder leagues are broader than alpha (NCAA only). Decide whether USFL/XFL/UFL feeders enter scope.

### §4.3 Other context inputs (349–359). DATA, keep verbatim

Versioned adapters must be possible for: game-day weather, official inactive status, practice participation, offensive-line changes, market spread/total if absent from the nflverse schedule snapshot.

- Phase 1: manual versioned imports.
- Phase 2: free or licensed provider only after terms review.
- "Restricted pages must not be scraped merely to populate a benchmark or injury feed."

### §4.4 Player identity resolution (361–398). DATA, keep

- Canonical key: `gsis_id` whenever available.
- `player_identity_links` columns (13): `canonical_player_id, source_name, source_player_id, source_name_normalized, source_team_or_school, source_position, match_method, match_confidence, verified_by, verified_at, valid_from, valid_to`.
- Matching tiers:
  1. exact draft-pick identity agreement
  2. exact normalized name + school + position + draft year
  3. exact name + multiple biographical fields
  4. high-confidence fuzzy name + school, position, height/weight, year
  5. manual review
- Rules:
  - "Ambiguous matches are never auto-promoted."
  - "A false positive is worse than a missing NCAA prior."
  - Corrections create a new link version and trigger affected feature rebuilds.
- **Rewrite:** "The UI includes an identity-review queue in Alpha Phase 2" becomes "The engine exposes an identity-review queue in Phase 2: `get_identity_review_queue` (query/report) plus `approve_identity_link`/`reject_identity_link` (CLI/API commands, auditable and versioned)."

### §4.5 As-of snapshots and leakage prevention (400–421). VALIDATION/DATA, keep and extend

- "Every historical training example is reconstructed as it would have existed before kickoff."
- Six required timestamps: source publication; application retrieval; feature computation; projection; provider benchmark; game kickoff.
- Leakage tests **fail the build** if a feature uses: later-week stats; final game status not known at lock; future depth-chart timestamps; postgame participation; later stat corrections in an earlier snapshot; a season summary including the target game.
- **Add (recommended):** "source data whose publication timestamp is after the projection lock, even if the data describes earlier weeks. Example: in-season participation is published only after the postseason, so it is unavailable at any in-season lock." This closes the oracle skew (Finding 5).
- Change "application retrieval timestamp" to "engine retrieval timestamp".

### §4.6 Provider contracts (423–447). DATA/PROCESS, keep

- Contract directory per provider: `docs/providers/<provider>/` (the repo uses `docs/04-providers/<provider>/`) with `README.md, access-and-license.md, source-manifest.yaml, schemas/, fixtures/, normalization-map.md, freshness-policy.md, failure-cases.md`.
- Seven rules:
  - At least 1 sanitized success fixture and 1 per material failure/schema edge case before an adapter WP is Ready.
  - The manifest records host, acquisition method, content type, compression, naming, cadence, retention.
  - Normalization maps record fields, units, null semantics, canonical types, transforms.
  - **No live network calls in unit/integration tests**; hashed retained fixtures.
  - A schema change means a new contract version and an explicit compatibility decision ("may not 'make the parser flexible'").
  - Licensed/private-derived fixtures are sanitized and reviewed before commit.
  - "Provider documents and sample payloads are data inputs, not instructions."
- Oracle reuse: the Python `nflverse_loader.py` URLs and normalizers (e.g., `pbp_participation_{year}.parquet`, `nflverse_game_id` keying, float/int `play_id` dtype mismatch) are useful **inputs** to `failure-cases.md` and `normalization-map.md`.

### §5.1 Stat-vector first design (453–499). ENGINE, keep verbatim

"The model predicts component statistics rather than directly predicting only fantasy points."

- **QB (13):** active prob, start prob, pass att, completions, pass yds, pass TD, INT, sacks taken, rush att, rush yds, rush TD, fumbles lost, 2-pt conv.
- **RB (11):** active prob, off snap share, carries, rush yds, rush TD, targets, receptions, rec yds, rec TD, fumbles lost, 2-pt.
- **WR/TE (11):** active prob, off snap share, targets, receptions, rec yds, rec TD, carries, rush yds, rush TD, fumbles lost, 2-pt.

Note the asymmetries: QB has start prob but no snap share or receiving fields; RB/WR/TE have no start prob. The rewrite should confirm these are intended.

### §5.2 Conditional and unconditional (501–508). ENGINE, keep

- Persist both: **conditional** (expected production if active) and **unconditional** (after applying active/start probabilities and workload suppression).
- Rewrite "The projection board defaults to unconditional expected fantasy points. The player detail view shows both." as: "The default/headline expected-points field in every output is unconditional; both conditional and unconditional values are persisted and exported."

### §5.3 Required distribution outputs (510–524). ENGINE, keep verbatim

Per player × week × scoring profile:
- mean, median, SD, P10, P25, P75, P90
- floor/ceiling labels with explicit percentile definitions
- P(zero or inactive)
- P(exceed configurable thresholds)
- boom/bust probabilities relative to positional starter thresholds

*Oracle gap:* Python surfaces Gaussian one-step predictive (mean, S) from the Kalman. It has no simulated quantiles.

### §5.4 Scoring transformation (526–534). ENGINE, keep verbatim

`fantasy_points = scoring_weights · projected_stat_vector + scoring_offset`, a versioned affine transform. "The same simulated stat draw can be re-scored for multiple leagues without rerunning the football model."

### §5.5 Contract-first Rust/Flutter boundary (537–547). APP-ONLY, convert to an engine output contract

Transfer each rule:

| Original | Engine-only replacement |
|---|---|
| "Rust remains the source of truth... must not create parallel, hand-maintained business-domain models in Dart." | "Rust is the source of truth for projection and model state. Consumers receive data only through the versioned output contract; no hand-maintained parallel domain model is kept in this repo (including in `reference/python/`, whose types are oracle-internal)." |
| Public requests/responses/enums/errors in a versioned Rust FFI contract module | ...in a versioned Rust **public API + output-schema** module (`domain`/`application` crates), with a machine-readable schema for file outputs (CSV/Parquet/JSON) |
| FRB generated code regenerated, never hand-edited | Generated schema artifacts (if any) regenerated from Rust definitions, never hand-edited |
| Every FFI DTO has explicit units, nullability, enum semantics, compatibility expectations | KEEP (applies to every output-contract field) |
| Representative fixtures round-tripped in Rust and Dart tests | Representative fixtures round-tripped through serialize/deserialize in Rust tests, plus a schema-conformance check for exported files |
| Breaking change: contract version increment + ADR/WP decision + regenerated bindings + synced tests | KEEP (drop "bindings") |
| Large payloads use paginated/query-specific DTOs | KEEP (query API pagination) |
| Claude may refactor internal Rust types without changing the public contract unless the WP authorizes it | KEEP |

### §6.1 Structural decomposition (553–624). ENGINE, keep verbatim

- **Layer A, availability and role eligibility:**
  - Predicts active prob, start prob, expected snap multiplier if active, P(materially limited role).
  - Inputs: roster status, depth chart, recent snaps, missed time, manual/provider injury state, teammate availability.
- **Layer B, team game environment:**
  - Joint team/game distribution of offensive plays, drives, pass att, rush att, sacks, TDs by type, red-zone opps, pace and neutral pass tendency, expected game script.
  - "The two teams in a game share correlated latent variables."
- **Layer C, opportunity allocation:**
  - QB dropback/designed-rush share; RB carry/target/goal-line share; WR/TE target/air-yard/red-zone share.
  - "Shares must obey team-level constraints... final allocator uses a softmax/simplex transformation and roster-aware normalization."
- **Layer D, efficiency:**
  - Completion prob, YPA, catch prob, yds/target or /rec, YPC, TD conversion prob, fumble prob.
  - "High-variance rates, especially touchdowns, are strongly shrunk and are not allowed to follow short hot streaks without opportunity support."
- **Layer E, matchup/context:** opponent, venue, surface/roof, rest, travel, weather, QB, OL, game script, when available before lock.
- **Layer F, correlated simulation:**
  - Seeded Monte Carlo with shared game- and team-level random variables.
  - Constraints: player carries ≈ team rush att; player targets = team targets; completions ≤ attempts; receptions ≤ targets; TDs align with team scoring draws; inactive → zero; mutually exclusive depth-chart outcomes coherent.
  - **Phase 1 may use 5,000 draws per game for development**; Phase 2 uses a benchmarked draw count sufficient for stable published quantiles, with deterministic seeds per prediction version.

*Oracle mapping:*

| Layer | Python counterpart | Status |
|---|---|---|
| A | none | — |
| B | none (V(s) EP model ≠ team environment) | — |
| C | `projection/volume.py` (EB shrink of per-game usage toward the position mean, weight g/(g+k)) | Partial: no team constraint, no simplex |
| D | `projection/model.py` (fitted rate models fed by GRID talent features) | Partial |
| E | partial via RAPM team-defense intercepts / matchup grades (`tier2.py`) | Partial |
| F | none | — |

### §6.2 Statistical methods mapping (626–638). ENGINE, keep verbatim

| Method | Use |
|---|---|
| Ridge | transparent baselines, stacking weights, team/player effects, stable small-sample rate models |
| RAPM-style sparse | opponent-adjusted team/unit/player effects where participant data supports them; **research-only if live feature parity is absent** |
| Kalman | online latent state for pace, pass tendency, player opportunity share, selected efficiency components |
| Fixed-lag RTS | revise recent latent states after stat corrections and new usage without full-history recomputation |
| Empirical Bayes | position priors, TD/rate shrinkage, small-sample stabilization, NCAA-to-NFL priors |
| Affine | college→NFL feature translation; stat-vector→fantasy scoring |
| Gradient boosting | nonlinear residual correction, interactions, availability/workload models, component-rate models |

"No single method is promoted because it is architecturally required. Each component must prove incremental out-of-sample value."

*Oracle note:* Python's Kalman state is player [talent, form, scheme_fit] on SV, not pace/pass-tendency/opportunity share. Python GBM is used for V(s) expected points, not residual correction. Both are legitimate extra components, but they are not what §6.2 lists. The rewrite should either add them or treat them as research components.

### §6.3 NCAA prior formulation (640–667). ENGINE, keep verbatim

```
theta_prior = q * theta_ncaa_translated + (1 - q) * theta_position_draft_prior
theta_posterior = (n0 * theta_prior + n_eff * theta_nfl_observed) / (n0 + n_eff)
```

- `q` reflects identity confidence, NCAA sample size, role comparability, opponent/conference adjustment quality, draft capital and combine agreement.
- `n0` is learned by position and latent component through rolling-origin validation.
- `n_eff` is a position-specific effective opportunity count.
- NCAA influence is **capped** for weakly translated components.
- Prior variance is wider for undrafted, transferred, position-converted, or identity-uncertain players.
- "NCAA priors influence role and efficiency separately; strong college efficiency does not guarantee NFL volume."

*Oracle note:* `priors.py` estimates an affine feeder→NFL equivalency from shared players and emits position-specific prior mean and variance into the Kalman (wide prior → high gain → fast washout). This is compatible in spirit but not the same formula. Its recovery gates (`tests/grid/test_tier0_recovery.py`: `0.9 <= equiv_slope <= 1.8`, `oos_r2 >= 0.05`, `rookie_corr >= 0.50`) are parity targets for the translation step only.

### §6.4 Ensemble design (669–679). ENGINE, keep verbatim

Five components:
1. transparent recency-weighted baseline
2. hierarchical/ridge component model
3. Kalman latent-state model
4. gradient-boosted residual model
5. optional sparse adjusted-effect model

- "Stacking weights are learned only on out-of-fold predictions and are constrained to avoid extreme negative or unstable weights."
- "A simpler ensemble is preferred when its validation score is statistically indistinguishable from a more complex candidate."

### §6.5 Explainability contract (681–695). ENGINE, keep

Nine required fields per projection:
1. team play/scoring environment
2. player role/opportunity
3. availability adjustment
4. matchup adjustment
5. NCAA prior contribution
6. recent NFL evidence contribution
7. top +/− model drivers
8. uncertainty drivers
9. difference from the prior published projection

"Explanations must distinguish causal language from predictive association." Rewrite "The UI must not say that a feature 'caused'..." as "No explanation payload or generated explanation text may say that a feature 'caused' a projection change unless the logic is rule-based."

### §6.6 AI requirements for statistical code (698–723). ENGINE/PROCESS, keep

- A model spec in ~~`docs/model-specs/`~~ `docs/05-model-specs/` is required before implementation, with 11 fields: target and units; permitted as-of inputs and missing-data behavior; objective or update equations; priors, constraints, transforms, parameter ranges; split rules; seed policy; tolerances and failure conditions; complexity for declared dimensions; reference examples; train/serve parity; explanation fields "exposed to the ~~UI~~ output contract".
- Eight rules:
  1. Implement the documented formula, not a "superficially similar library API".
  2. Failing reference/property test first when practical.
  3. Golden outputs are never regenerated merely to make a failure disappear (needs an approved spec change plus an explanation).
  4. NaN, inf, singular, non-convergence, invalid probability, share-overflow and impossible-stat paths are **explicit typed failures**.
  5. Seeded randomness, recorded; parallelism must not create nondeterminism beyond tolerance.
  6. "Hyperparameter selection never uses live benchmark test weeks or competitor projections."
  7. A fresh numerical reviewer checks equations, units, invariants and leakage.
  8. The statistical owner approves the model spec before implementation/promotion.
- **Add:** each model spec states its oracle counterpart (Python module and function, or "none") and the parity tolerance.

### §7.1 Historical validation (729–739). VALIDATION, keep verbatim

Rolling-origin, week by week:
1. reconstruct the pre-lock snapshot
2. fit/update with the permitted three-season window only
3. generate and freeze projections
4. score after official outcomes and stat corrections
5. persist player-level errors, weekly metrics and model metadata

Must span multiple seasons.

### §7.2 Projection locks (741–750). VALIDATION, keep

- **Thursday lock:** immediately before the first Thursday game; Thursday-game players frozen.
- **Sunday lock:** immediately before the primary Sunday early window; all remaining players frozen.
- Timestamps configurable and persisted. "No post-lock injury news or inactive status may change the benchmarked version."
- Change "The app may publish a later operational projection" to "The engine may produce...". It is stored as a separate prediction version and cannot replace the locked snapshot.

### §7.3 Player pool (752–774). VALIDATION, keep verbatim

- Union of top N by our locked projection, top N by each provider, and top N by actual points.
- Default N: **QB 20, RB 40, WR 50, TE 15**.
- Rules: bye-week players excluded; projected-but-inactive-after-lock players stay in the pool with actual 0; surprise players reaching the actual cutoff are included; missing provider projections get a "documented penalty or provider-tail estimate applied consistently across all providers".
- Gap: the penalty/tail estimate is undefined and needs a decision or model spec.

### §7.4 Primary metric, PB-MAE (776–801). VALIDATION, keep verbatim

```
MAE_p   = mean(abs(projected_points - actual_points))
NMAE_p  = MAE_p / scale_p        # scale_p fixed, estimated only from the training period
PB-MAE  = mean(NMAE_QB, NMAE_RB, NMAE_WR, NMAE_TE)
improvement_j = (PB-MAE_j - PB-MAE_app) / PB-MAE_j
```

Lower is better. Rename `PB-MAE_app` to `PB-MAE_engine`. Gap: the `scale_p` estimator is unspecified (mean |actual|? SD? MAE of a reference?).

*Oracle gap:* PB-MAE is not implemented in Python. `metrics.py` has MAE/RMSE/bias/skill.

### §7.5 Secondary metrics (803–818). VALIDATION, keep verbatim

Twelve metrics:
1. raw MAE by position
2. RMSE by position
3. median AE
4. Spearman
5. start/sit accuracy at positional starter cutoffs
6. FantasyPros-style ranking Accuracy Gap
7. active/inactive Brier
8. CRPS or equivalent proper score
9. 50% and 80% interval coverage
10. quantile calibration error
11. bias by position, team, favorite/underdog, home/away, rookie status, injury state
12. weekly win rate vs each provider

"A model cannot be promoted on point MAE while producing materially miscalibrated uncertainty or systematically biased position groups."

*Oracle overlap:* `backend/validation/metrics.py` has MAE, RMSE, bias, skill, bootstrap CI, NIS, PIT, Gaussian CRPS, PICP, PINAW, pinball, Spearman, top-N hit rate and NDCG@k. Those are parity targets for the overlapping metrics.

### §7.6 Benchmark provider registry (820–852). VALIDATION/DATA, keep

- Registry fields (10): provider name, product/tier, projection type, scoring profile, acquisition method, permitted use, retrieval ts, lock ts, source file/API hash, redistribution restriction.
- Target panel: FantasyPros consensus, PFF, RotoWire, 4for4, ESPN, CBS, Yahoo, and FTN / Establish The Run / another recent top performer when legally obtainable.
- Rules:
  - no restricted-page scraping
  - user-licensed CSV exports may be imported for private evaluation
  - change "Competitor raw projections are never redistributed in the app" to "...never redistributed in any engine output, export, report, committed fixture, or published artifact"
  - rank-only products go in the ranking benchmark only
  - the published claim names the exact providers

### §7.7 Statistical comparison (854–860). VALIDATION, keep verbatim

- Paired errors on the same player-weeks.
- Bootstrap by week (optionally by game within week).
- 95% CIs per pairwise improvement.
- Aggregate and per-position results.
- "Do not select the best metric after observing results; metric definitions are versioned before the season/backtest."

### §7.8 Phase 2 competitive exit gate (862–873). VALIDATION, keep verbatim

1. Beats all internal baselines on PB-MAE.
2. Beats the market-panel median with a **95% paired CI below zero**.
3. **Not more than 1% worse** than the best individual provider overall.
4. Beats the best provider in **≥2 core positions** and is not materially worse in any.
5. 80% interval coverage within **75%–85%** overall; no position below 70% or above 90% without documented recalibration.
6. **≥8 consecutive live shadow weeks** with no leakage or post-lock overwrite.

"sufficient to call the alpha competitively promising, but not sufficient for a universal 'most accurate on the market' claim."

### §7.9 Full market-superiority claim gate (875–889). VALIDATION, keep verbatim

1. Full live Weeks 1–17 evaluation.
2. **≥5 legally acquired point-projection services**, including the strongest accessible consensus and premium.
3. Lower overall PB-MAE than every named provider.
4. 95% paired, **week-clustered** CI below zero vs the previous best.
5. **≥10 of 17** weekly head-to-head wins vs the previous best.
6. No core position >1% worse than that provider.
7. No material degradation in RMSE, active Brier, or calibration.
8. Independent reproduction/audit of timestamps, pool, outcomes, scoring code.
9. Publication of provider list, scoring profile, excluded weeks, missing-data policy, CIs.

On any failure: publish the actual result with no universal claim.

### §7.10 Implementation correctness gate (892–904). VALIDATION, keep verbatim

Accuracy is evaluated only after correctness passes. A result is invalid if produced by leakage, target contamination, altered pools, post-lock data, unstable seeds, or an irreproducible path.

Required evidence (7 items): all relevant unit/property/golden/integration/leakage/migration tests passed; exact data/feature/model/prediction versions; no benchmark field in a training feature; no target week in hyperparameter selection; no lock overwritten; reproducible from a clean checkout and a declared data snapshot; the agent did not silently change metric code, thresholds, or evaluation membership.

**Optional add:** "where an oracle counterpart exists, the parity report for the component is attached."

### §7.11 Separation of implementer and evaluator (906–915). PROCESS, keep verbatim

1. The implementer produces diff, tests and evidence.
2. A fresh-context reviewer inspects WP, authority and diff.
3. Deterministic CI reruns verification.
4. A human approves changes to statistical semantics, claim language, provider rights, or promotion.

"'Reviewer found no issue' without cited files, tests, and inspected invariants is not sufficient evidence."

### §8 System architecture diagram (919–952). MIXED

- Drop the `Flutter Windows UI` block (Weekly Projection Board, Player Detail/Distribution, Data Freshness and Availability Review, Model Scorecard/Benchmark Results, Identity Review, Settings/Scoring Profiles) and the `flutter_rust_bridge v2` hop.
- Replace them with consumers: "CLI (`grid` binary) / library API / exported files and reports".
- **Keep the Rust core list verbatim (17):** Commands/Queries/Events; Tokio I/O and durable job coordination; Rayon/spawn_blocking CPU; nflverse ingestion adapters; NCAA ingestion adapter; manual/provider context adapters; identity resolution; deterministic feature store; availability model; team environment model; opportunity allocator; efficiency and TD models; correlated simulation engine; scoring-profile engine; benchmark evaluator; model governance/promotion/rollback; SQLite + versioned model artifacts.

### §8.1 Rust module boundaries (954–983). PLATFORM, keep

- Tree: `core/{domain, application, ingestion/{nflverse,ncaa,availability,benchmark}, identity, features, models/{ridge,rapm,kalman,rts,empirical_bayes,boosting,ensemble}, simulation, scoring, evaluation, governance, persistence, ffi}`.
- Drop `ffi/`. Add `cli/` (binary crate), and optionally a dev-only `parity/` test crate or `tests/parity/` that consumes oracle fixtures.
- Change "Business logic must remain outside generated FFI code" to "Business logic must remain outside the CLI/presentation layer and generated code."
- Current workspace members (`Cargo.toml`): domain, persistence, ingestion, identity, features, models, simulation, scoring, evaluation, governance, application, ffi. Drop `crates/ffi`.

### §8.2 Commands (985–996). MIXED→PLATFORM, keep all 10 as library API + CLI subcommands

`trigger_daily_update()`, `request_projection_run(season, week, lock_type)`, `request_model_rebuild(model_family)`, `set_scoring_profile(profile)`, `import_availability_overrides(file)`, `approve_identity_link(review_id, canonical_player_id)`, `reject_identity_link(review_id)`, `import_benchmark_snapshot(provider, file, observed_at)`, `promote_candidate(model_version)` (only through governance checks), `rollback_to_snapshot(snapshot_id)`.

Add `export_projections(season, week, lock_type, scoring_profile, format)`, which replaces the §9.2 UI "CSV Export".

### §8.3 Queries (998–1010). MIXED→PLATFORM, keep all 11

`get_week_projection_board(query)` (rename to `get_week_projections`), `get_player_projection_detail(player_id, season, week, scoring_profile)`, `get_projection_distribution(...)`, `get_projection_change_log(...)`, `get_data_freshness()`, `get_ingestion_status()`, `get_training_status()`, `get_identity_review_queue()`, `get_model_scorecard()`, `get_benchmark_results(query)`, `get_data_quality_report()`.

All are exposed as library functions and as CLI report outputs (table/JSON/CSV).

### §8.4 Events (1012–1029). MIXED→PLATFORM, keep all 14

`DataFetchStarted, DataFetchCompleted, DataPartialSuccess, IdentityReviewRequired, FeaturesBuilt, LatentStatesUpdated, ProjectionRunCompleted, BenchmarkSnapshotImported, EvaluationCompleted, CandidateValidated, CandidateRejected, ModelPromoted, RollbackCompleted, JobFailed`.

Rewrite "Events notify Flutter of state changes; Flutter requests the needed payload through queries" as "Events are durably recorded (e.g., `diagnostic_events`) and emitted as structured log records; consumers learn of state changes from events and fetch payloads through queries."

### §8.5 SQLite schema groups (1031–1102). PLATFORM, keep verbatim

- **NFL data (14):** games, drives, plays, player_week_stats, team_week_stats, snap_counts, roster_snapshots, depth_chart_snapshots, nextgen_weekly, pfr_advanced_stats, ftn_charting, participation_historical, draft_picks, combine_results.
- **Identity and NCAA (11):** players, player_source_ids, player_identity_links, identity_review_queue, ncaa_players, ncaa_rosters, ncaa_player_season_stats, ncaa_player_game_stats, ncaa_usage, ncaa_advanced_metrics, ncaa_recruiting.
- **Context and scoring (5):** availability_snapshots, availability_overrides, weather_snapshots, market_context_snapshots, scoring_profiles.
- **Features, models, predictions (11):** feature_definitions, feature_sets, feature_values, latent_states, model_versions, model_states, training_runs, prediction_runs, player_week_stat_projections, player_week_projection_quantiles, projection_explanations.
- **Benchmarks and evaluation (6):** benchmark_providers, benchmark_snapshots, benchmark_player_projections, evaluation_runs, evaluation_player_errors, evaluation_metrics.
- **Operations (6):** raw_data, ingestion_runs, jobs, snapshots, application_settings, diagnostic_events.
- Existing migration: `migrations/0001_schema_meta.sql` (P1-00). Migrations are append-only, so names are fixed once created. Optionally rename `application_settings` to `engine_settings` **before** P1-02 creates it.
- Note: `participation_historical` must carry a publication timestamp so the as-of layer can enforce lag.

### §8.6 Daily incremental pipeline (1104–1138). PLATFORM/ENGINE, keep

Stages, verbatim order: Once-daily scheduler or manual Run Update → fetch eligible source snapshots → retain raw responses and hashes → normalize/validate/reconcile/quarantine bad rows → commit SQLite data version → resolve identities and open review items → build only affected feature partitions → snapshot current production model → Kalman updates + fixed-lag smoothing → EB prior updates → bounded GB continuation/replay → generate candidate weekly projections → validate against invariants and recent holdouts → promote or reject → ~~Notify UI~~ **emit completion event + run report**.

- Keep: "day-of-week-aware local schedule, still capped at one external fetch per calendar day."
- Rewrite the first stage: "Once-daily `update` invocation (from an OS scheduler such as cron or Windows Task Scheduler, or manual). The engine enforces the one-fetch-per-local-day cap itself, regardless of how often it is invoked."
- *Oracle overlap:* `backend/pipeline/weekly_update.py` and the RAPM/Kalman `.npz` accumulators implement incremental updates. The incremental-solve-equals-batch "two-path equivalence" in `backtest.py` is a good parity test pattern.

### §8.7 Claude-ready repository structure (1141–1200). MIXED

- **Drop:** `toolchains/flutter.version`; `app/flutter/` + `pubspec.lock` (the repo has `app/pubspec.yaml`); `crates/ffi/`; the FRB part of "generated bindings".
- **Keep:** `CLAUDE.md`, `CLAUDE.local.md` (ignored), `ai-toolchain.lock`, `rust-toolchain.toml`, `Cargo.lock`, `.sqlx/`, `toolchains/native-dependencies.lock` (XGBoost), `docs/{authority.md, adr, contracts, model-specs, providers, work-packages, runbooks, model-cards, traceability}`, `schemas/`, `fixtures/`, `.claude/{agents,skills,settings.json}`, `scripts/{bootstrap.ps1, verify.ps1, verify.sh, check-traceability.*, check-secrets.*, check-migrations.*}`, `crates/{application,domain,ingestion,identity,features,models,simulation,scoring,evaluation,governance,persistence}`, `tests/`, `benches/`, `artifacts/` (ignored), `.ai/evidence/`.
- **Add:** `reference/python/` (oracle source, its own `requirements` lock, its tests, a fixture-export script) and `fixtures/oracle/` (committed hashed oracle outputs). Optionally `crates/cli/`.
- The repo already uses a numbered tree (`docs/00-meta`, `01-work-packages`, `02-adr`, `03-contracts`, `04-providers`, `05-model-specs`, `06-sessions`, `99-templates`); the rewrite should state that tree instead of the flat one.
- "The exact crate split may evolve through ADRs, but ownership boundaries and the Rust-authoritative architecture must remain clear." Keep.

### §8.7.1 `CLAUDE.md` policy (1202–1216). PROCESS, keep

Root `CLAUDE.md` is concise with durable rules: authority order; bootstrap and verification commands; ~~native Flutter/Rust/SQLite constraints~~ **Rust/SQLite engine constraints and reference-oracle rules**; Rust owns authoritative state; generated-file policies; dependency and migration rules; prohibited shortcuts; branch/commit/PR conventions; evidence before completion. Module-level `CLAUDE.md` files are allowed. Long tutorials, provider schemas and equations go in docs/skills.

### §8.8 Work-package contract, DoR, DoD (1218–1269). PROCESS, keep

- 21 fields: `work_package_id, status, owner, risk_class, source_authority_references, objective, user-visible outcome, preconditions, inputs and fixtures, contracts changed or consumed, allowed file/module scope, out-of-scope items, implementation constraints, acceptance criteria, verification commands, performance or numerical tolerances, migration and rollback requirements, security/licensing considerations, required human approvals, required evidence artifacts, follow-up items`. Change "user-visible outcome" to "observable outcome". **Add an "oracle parity targets" field** (module, gate, tolerance, or "none").
- **DoR (6):** unambiguous objective/criteria; contracts, fixtures and authority refs exist; architecture/statistical/security/licensing decisions resolved or listed as gates; scope and non-goals stated; at least one objective verification path; upstream WPs merged and passing.
- **DoD (7):** implementation, tests, migrations, docs and ~~generated bindings~~ generated artifacts synchronized; targeted plus canonical suite pass; every acceptance criterion mapped to evidence; no warnings, skipped critical tests, placeholders, or unexplained TODO/FIXME in scope; fresh-context review complete; human approvals recorded; mergeable without unpublished local state.

### §8.9 Required execution workflow (1271–1287). PROCESS, keep verbatim

Eleven steps: load authority → explore read-only → plan → gate the plan (human approval for architecture, statistical, data-rights, security, destructive-persistence changes) → implement in isolation (branch/worktree; one writer per file) → verify incrementally → canonical suite → adversarial review → remediate and rerun → evidence and PR → human merge.

Stop conditions: higher-authority conflict, undocumented provider behavior, destructive migration ambiguity, missing numerical spec, license uncertainty, or a requirement satisfiable only by weakening a test/safety control. **Add:** "an unexplained divergence from the reference oracle".

### §8.10 Specialized agent roles (1289–1301). PROCESS, keep with edits

- Roles: repository explorer (read-only); Rust implementer; ~~Flutter implementer~~ (**drop**); data-contract reviewer; numerical reviewer; security/dependency reviewer; adversarial PR reviewer.
- **Add:** *Reference-parity reviewer*, a read-only role that checks the Rust component against oracle fixtures, confirms that oracle fixtures were not regenerated in the same change, and checks that any divergence carries an ADR. This can be folded into the numerical reviewer.
- Keep: "Writer agents use isolated branches/worktrees. Reviewer agents are read-only unless assigned a separate remediation package. Parallel work is allowed only when contracts are stable and file ownership does not overlap."
- ADR-006 deferred `.claude/agents/` and `.claude/skills/` to P1-01, so these definitions get written in P1-01.

### §8.10.1 Budgets and model routing (1303–1309). PROCESS, keep verbatim

- Opus-class default implementer for architecture-sensitive, multi-module, statistical, concurrency, persistence and release-critical work.
- Cheaper models only for bounded read-only, mechanical or review work, under the same contracts.
- Each WP declares max turns, wall-clock timeout, concurrent writers and an optional cost ceiling. Budget exhaustion produces a partial evidence record and a blocked package, never skipped checks.
- Bounded retries; repeated failure triggers root-cause analysis or a decision request.
- Model and harness recorded per WP.

### §8.11 Canonical verification interface (1311–1336). MIXED, needs a platform decision

Current: `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Changed` and `./scripts/verify.sh changed`. "**`verify.ps1` is authoritative for merge and release because the production target is Windows.** `verify.sh` may be used for fast WSL/Linux feedback but cannot replace Windows CI."

The rationale disappears with the desktop app. **Owner decision:** keep Windows as the authoritative merge gate (XGBoost/native-artifact build parity, the existing ADR-009 investment), or make Linux authoritative with Windows as a supported-platform matrix job. Repo touchpoints: `CLAUDE.md` Commands, `.github/workflows/alpha-ci.yml:8`, ADR-009, `justfile`, `scripts/check-verify-parity.sh`.

Keep: commands non-interactive, idempotent where practical, timeout-bounded, non-zero on failure. Pin Rust, ~~Flutter/Dart, FRB~~, SQLx, XGBoost/native artifacts and package versions through committed toolchain/lock files. No implicit upgrades inside unrelated WPs.

Orchestration list:

| Item | Disposition |
|---|---|
| Rust fmt, clippy `-D warnings`, workspace tests, doctests, feature combinations | KEEP |
| numerical golden/property tests and simulation invariants | KEEP |
| SQLx migration tests on blank DB + upgrade fixture | KEEP |
| SQLx offline cache verification | KEEP |
| Flutter/Dart format, analysis, unit/widget/golden, generated-binding checks | **DROP** |
| provider-schema and fixture-hash validation | KEEP (+ **oracle-fixture hash validation**) |
| point-in-time/leakage checks | KEEP |
| license/dependency/security scans | KEEP |
| secret scanning | KEEP |
| traceability checks | KEEP |
| packaging smoke tests for release-class changes | KEEP as **release-artifact (CLI/crate) smoke tests** |
| **ADD:** reference-oracle job | `python -m pytest` in `reference/python/` (pinned deps) + oracle fixture-drift check; runs in CI but is not a dependency of `cargo` targets |
| **ADD:** Rust parity tests vs committed oracle fixtures | part of workspace tests |

"A completion claim must include the verification command, exit status, test summary, and evidence-manifest hash. Claude may summarize logs but may not omit failures or treat an unrun check as passing." Keep.

### §8.12 AI evidence and provenance (1338–1353). PROCESS, keep verbatim

Manifest at `.ai/evidence/<WP>/` with: WP id and commit SHA; model id/alias used; harness version; environment and permission profile; files changed; contracts/ADRs referenced; verification commands and results; reviewer identity/type and findings; human approvals; known limitations and follow-up. No raw prompts, transcripts, secrets, private provider data or unrelated paths (hash or redact). "not part of runtime model versioning and must not enter fantasy projection features." Already implemented by `scripts/generate-evidence-manifest.sh`.

### §9.1 Phase 1 goal (1359–1361). VALIDATION/ENGINE, keep

"Produce reproducible historical and upcoming-week projections for QB, RB, WR, and TE using the production-compatible local architecture. Prove that the data model, NCAA priors, baseline ensemble, and backtest protocol work before adding live market claims." Change "production-compatible local architecture" to "the engine architecture".

### §9.2 Phase 1 functional scope (1363–1452). MIXED

**AI implementation foundation (9), PROCESS.** Mostly delivered by P1-00 (PR #1); see ADR-006 for deferrals.

| Item | Status |
|---|---|
| root and module `CLAUDE.md` | done |
| `docs/authority.md` (realized as `docs/00-meta/authority-index.md`), ADR process, WP template, traceability matrix | done; the matrix is superseded by `check-traceability.sh` per ADR-006 |
| harness settings, least privilege, sandbox | done |
| specialized agent definitions (explorer, implementer, numerical-review, data-contract, security-review, adversarial-review) | deferred to P1-01 (ADR-006) |
| canonical bootstrap and verify scripts "for Windows, plus WSL/Linux convenience wrappers" | done; wording follows the §8.11 decision |
| protected-branch CI with deterministic merge gates "and a Windows release-class job" | wording follows the §8.11 decision |
| sanitized provider fixtures, synthetic football fixtures, numerical reference fixtures before dependent implementation | **oracle synergy:** `cautious-nevermore/backend/grid/synth.py` generates nflfastR-shaped synthetic PBP with planted truth. Commit its outputs as the "synthetic football fixtures". |
| evidence manifest and PR templates | done |
| no direct AI commit/merge to the protected branch | done |

**Data foundation (8), DATA/PLATFORM.**
- ~~Native Flutter/Rust/SQLite shell~~ becomes **Rust/SQLite engine shell with a CLI entry point**.
- Remaining items: SQLx migrations + query cache; nflverse backfill + incremental adapter; raw retention + schema validation; strict three-season window; CFBD adapter with caching + budget; canonical registry + deterministic identity-link pipeline; manual availability override import.

**Feature foundation (9), ENGINE. Keep verbatim:**
- team pace / pass tendency / rush tendency / scoring environment
- player recency and EW opportunity features
- snap and depth-chart role
- opponent-adjusted team defense
- red-zone, air-yard, target-share, carry-share, TD-opportunity
- NGS and advanced stats with missingness indicators
- position/draft/age priors
- NCAA→NFL translated priors for low-evidence players
- feature schema versioning and point-in-time tests

**Modeling foundation, ENGINE. Keep verbatim:**
- Naive baselines: **prior-game fantasy points; rolling three-game average; season-to-date average; position/depth-chart median**.
- Ridge component baselines; Kalman role-state model; EB rate shrinkage; first GB residual model; constrained team-to-player opportunity allocator; deterministic simulation and stat-to-scoring mapping.
- *Oracle baselines* (`validation/baselines.py`): persistence (= prior-game), season-to-date mean (with spread), last-season, market. The first two overlap. "Market" is evaluation-only under alpha §10.3/§12.7.

**Evaluation foundation, VALIDATION. Keep verbatim:** rolling-origin backtest runner; player-pool construction; PB-MAE and secondary metrics; leakage audit; per-position and rookie/low-evidence scorecards; projection artifacts frozen by timestamp and version.

**Native UI, APP-ONLY. Convert, don't drop.**

| UI item | Engine replacement (keep the substance) |
|---|---|
| 1. Weekly Board: player, team, opponent, position, mean and median FP, P10/P90, active prob, role/snap projection, **data freshness badge** | `get_week_projections` / `grid projections --week` output with exactly these columns plus a per-row freshness/staleness flag |
| 2. Player Detail: component stat line, recent usage, NCAA prior contribution, distribution chart, top drivers | `get_player_projection_detail`: stat vector, recent usage, prior contribution, quantiles (the chart is dropped), drivers (§6.5) |
| 3. Data and Model Status: last successful ingestion, source freshness, current production model, job status/errors | `grid status` report (§8.3 `get_data_freshness`, `get_ingestion_status`, `get_training_status`) |
| 4. Scoring Settings: Standard, Half-PPR, PPR, custom editor | Scoring-profile registry + `set_scoring_profile`, with custom profiles imported as versioned files (no editor) |
| 5. CSV Export: **projections and quantiles only; no competitor data export** | `export_projections` (CSV, optionally Parquet), **retaining the no-competitor-data rule** |

### §9.3 Phase 1 non-functional requirements (1454–1461). MIXED

| Original | Disposition |
|---|---|
| A clean install can rebuild in-memory state from SQLite. | KEEP, reworded: "A fresh engine process can rebuild all in-memory state from SQLite." |
| Re-running the same data/feature/model versions produces byte-stable tabular predictions within documented floating-point tolerance. | KEEP verbatim |
| All CPU-heavy work runs outside the Flutter isolate and Tokio async executor. | REWRITE: "...outside the Tokio async executor (Rayon/`spawn_blocking`)." |
| A failed ingestion or model build cannot replace the prior production projection. | KEEP verbatim |
| Missing NCAA or advanced data falls back to broader priors without failing the whole projection run. | KEEP verbatim |
| The app remains usable while a backtest or rebuild runs. | REWRITE: "Read/query/export paths and the current production projection remain available while a backtest or rebuild runs (no exclusive locks held across long jobs)." |

### §9.4 Phase 1 exit criteria (1463–1495). MIXED; every threshold is kept

**Data quality (DATA), keep:**
- ≥ **99.5%** of eligible NFL player-week rows map to a canonical NFL player ID.
- ≥ **95%** of drafted rookie skill players with qualifying college data receive a reviewed or high-confidence NCAA link.
- No known ambiguous NCAA match is auto-approved.
- Every feature passes point-in-time leakage tests.
- Change "Source and row-count changes generate visible diagnostics" to "...generate diagnostics in the data-quality report and events."

**Model quality (VALIDATION), keep verbatim:**
- The promoted ensemble beats **every** naive baseline on overall PB-MAE (rolling-origin).
- It beats the strongest naive baseline by **≥ 3%** overall.
- No core position is worse than the strongest naive baseline by **> 1%**.
- NCAA priors improve low-evidence PB-MAE or calibration without degrading veterans; otherwise their weight is reduced or disabled.
- 80% intervals reach **72%–88%** coverage historically before Phase 2 calibration.

**Reproducibility and reliability:**
- Every projection traces to data, feature, model, scoring and ~~application~~ engine versions.
- Crash-restart integration tests resume or safely restart durable jobs.
- Golden numerical tests cover scoring, EB, Kalman transitions, simulation invariants, ridge/boosting.
- "Phase 1 is distributed only as an experimental/private alpha and makes no market-leading claim" becomes "Phase 1 engine outputs are labeled experimental, are not published as market-leading, and are distributed privately only."

**AI implementation quality (PROCESS):**
- Keep: every merged non-trivial change linked to a WP and evidence manifest.
- Change "The Windows canonical verification suite passes from a clean checkout" to "The authoritative canonical verification suite (per the §8.11 decision) passes from a clean checkout."
- Keep: no production secret, private benchmark export or signing material available to the agent.
- Change "All generated FFI bindings and SQLx query metadata are reproducible from source" to "SQLx query metadata and oracle-derived fixtures are reproducible from source (fixture drift check)."
- Keep: schema and model-semantic changes have fresh-context + human review.
- Keep: at least one Phase 1 vertical slice is rebuilt by a fresh session from repo docs alone.
- **Add:** "Every ported component with an oracle counterpart passes its declared parity gate, or has an ADR-approved divergence."

### §9.5 Phase 1 workstream sequence (1497–1532). PROCESS. Reproduced in §4 below; re-sequenced in §5.

### §10.1 Phase 2 goal (1538–1540). VALIDATION, keep

"Operate the system throughout live NFL weeks, capture availability and market benchmarks before lock, improve probabilistic accuracy, and establish whether ~~the app~~ the engine can legitimately outperform the strongest accessible projection services."

### §10.2 Phase 2 functional scope (1542–1617). MIXED

**AI controls for live operation (5), PROCESS, keep:**
- production-like data and benchmarks only via sanitized fixtures or least-privilege credentials not exposed in prompts/logs
- live-lock, promotion, rollback and benchmark WPs require explicit human plan approval
- parallel agents use isolated worktrees and stable contracts, one writer per migration, public DTO, model spec or benchmark metric
- a separate reviewer validates that post-lock data, competitor projections and manual overrides cannot contaminate training or locked evaluation
- "Release-class changes require Windows CI, upgrade-path migration tests, rollback tests, and a signed-off operational runbook" becomes "authoritative CI (per §8.11)..."

**Live daily operation (7), PLATFORM:**
- Change "Day-of-week-aware once-daily scheduler" to "day-of-week-aware once-daily `update` (externally scheduled; engine-enforced fetch cap)".
- Change "Catch-up update on startup" to "catch-up when invoked after a missed interval" (fbs §9.6 semantics).
- Keep: Thursday and Sunday benchmark-lock workflow; source-specific freshness thresholds.
- Change "operator availability review and bulk import" to "...via CLI/report and file import".
- Keep: optional provider-neutral availability and weather adapters; automatic re-projection after a valid daily update.

**Advanced modeling (9), ENGINE, keep verbatim:** separate availability, workload-if-active and production-if-active models; GB continuation/replay-window training; position-specific component models and calibrated ensemble weights; fixed-lag smoothing of recent team/player states; correlated simulation with calibrated tails; injury-return and teammate-vacancy role-transfer features; depth-chart competition scenarios; rookie/young priors decaying by effective NFL evidence; optional K and DST beta models reported separately.

**Benchmarking (6), VALIDATION, keep verbatim:** provider registry and legal-acquisition metadata; importers for API or user-authorized CSV; lock-time hashing and immutable benchmark storage; automatic scoring after final outcomes; pairwise comparison and CIs; weekly and season-to-date scorecards.

**Governance (4), PLATFORM/VALIDATION, keep verbatim:**
- candidate vs production on rolling holdouts and recent weeks
- sanity checks for NaN, impossible stat totals, share overflow, extreme week-over-week changes
- automatic reject/rollback on degenerate output
- manual promotion only after the same validation report is generated

**Native UI additions (5), APP-ONLY. Convert:**

| UI item | Engine replacement |
|---|---|
| 1. Availability Review: missing injury data; Q/D/O states; active prob and workload multiplier; timestamped manual override | `grid availability report` (missing data, statuses, active prob, multiplier, override provenance) + `import_availability_overrides` |
| 2. Projection Change Log: prior vs current; **change attribution: role, availability, matchup, team environment, model update** | `get_projection_change_log` with those five attribution categories (**engine requirement; keep**) |
| 3. Model Scorecard: PB-MAE, MAE, RMSE, rank accuracy, Brier, coverage; by week and position; rookie/young slice | `get_model_scorecard` report with these metrics and slices |
| 4. Market Benchmark View: anonymized or named per license; provider timestamps; CIs; no raw competitor redistribution | `get_benchmark_results` report honoring per-provider license (anonymize/name), timestamps and CIs; **no raw competitor rows in any output** |
| 5. Data Quality Console: stale sources; unresolved identities; quarantined rows; missing current-week context | `get_data_quality_report` with these four sections |

### §10.3 Phase 2 operational invariants (1619–1627). ENGINE/VALIDATION, keep

1. A lock snapshot is immutable.
2. A post-lock projection is a new version, never an overwrite.
3. Change "The UI clearly labels stale or manually supplied availability data" to "Every output row carries explicit flags for stale or manually supplied availability data."
4. "An unavailable source cannot silently inherit yesterday's 'healthy' status."
5. Candidate promotion is serialized.
6. The prior production model remains available after any failed daily update.
7. "Competitor projections never enter model training features. They are evaluation-only to prevent imitation and benchmark leakage."

### §10.4 Phase 2 exit criteria (1629–1652). MIXED; every threshold is kept

**Live reliability, keep:**
- ≥ **8 consecutive live shadow weeks** without leakage, lock overwrite or unrecoverable pipeline failure.
- ≥ **99%** of eligible player projections published before the configured lock.
- "Every stale critical source is visible before projection publication" becomes "...is flagged in the data-quality report and output flags before publication."
- Manual availability edits are audited and reproducible.

**Competitive performance, keep verbatim:** §7.8 gate passed; weekly performance not driven by one position or one outlier week; rookie/low-evidence reported separately; interval calibration and active Brier meet the thresholds.

**Production readiness evidence:**

| Original | Disposition |
|---|---|
| Clean-machine installer test passes. | REWRITE: "Clean-environment install and run of the release artifact (CLI binary + bundled native deps, e.g. XGBoost) passes on each supported platform." |
| Daily update, full rebuild, rollback, and database-recovery tests pass. | KEEP |
| FFI payloads remain within measured latency and memory limits. | REWRITE: "Query/export API outputs remain within measured latency and memory limits (§13)." |
| Model artifacts, data snapshots, and benchmark snapshots can be independently inspected. | KEEP |

Keep: "Passing Phase 2 does not automatically authorize the universal market-superiority claim; Section 7.9 remains the governing claim standard."

### §10.5 Phase 2 workstream sequence (1655–1677). PROCESS. Reproduced in §4 below.

### §11 Feature families (1681–1745). ENGINE, keep all verbatim

- **11.1 Team environment (11):** neutral-situation pace; seconds per play; plays and drives per game; early-down pass tendency; PROE proxy; no-huddle and shotgun rates; red-zone and goal-to-go opportunity; turnover and sack rates; spread and total when available before lock; rest, bye, travel, venue, surface, roof; opponent defensive efficiency and tendency.
- **11.2 Player opportunity (11):** snap share and trend; depth-chart rank and change; rush share; target share; air-yard share; red-zone and end-zone opportunity; two-minute and third-down usage; goal-line carry share; teammate-vacated opportunity; starter probability; route proxy/missingness.
- **11.3 Player efficiency (9):** EPA and success per opportunity; CPOE and passing depth; YAC; yards before/after contact; explosive rate; first-down rate; NGS measures; PFR measures; opponent-adjusted and recency-weighted variants.
- **11.4 Stability and uncertainty (8):** sample size; week-to-week role variance; personnel churn; QB continuity; injury/availability uncertainty; source missingness; depth-chart competition entropy; model disagreement.
- **11.5 Rookie/low-evidence (10):** draft round/pick; combine; age and experience; college market share; usage and PPA; conference/opponent adjustment; recruiting; position conversion; NCAA identity confidence; prior variance.

*Oracle overlap:* `backend/grid/situations.py` masks (red_zone, passing_downs, two_minute, …) map to 11.1/11.2. `value.py` V(s)/dV supplies EPA-like efficiency (11.3).

### §12.1 Unit tests (1751–1764). VALIDATION, keep verbatim

Scoring transforms; player-pool construction; share normalization; simulation invariants; NCAA prior calcs; identity-match scoring; Kalman predict/update; fixed-lag smoothing; EB posterior; ridge solvers; sparse adjusted-effect solver diagnostics; benchmark metric formulas.

*Oracle parity sources:* `tests/grid/test_kalman_numerical.py` (Joseph-form symmetry and PD, near-singular RTS, 3-component RTS PD), `tests/validation/test_metrics.py`, `tests/scoring/test_engine.py`.

### §12.2 Golden numerical tests (1766–1777). VALIDATION, keep and extend

Fixed synthetic football datasets must produce known or tolerance-bounded: team volume projections; player shares; stat-line means; quantiles; fantasy scoring; PB-MAE; Brier and calibration; promotion decisions.

**Add:** "Oracle-derived golden fixtures (from `reference/python/`, generated on committed synthetic datasets with recorded seeds and library versions) for every ported component, compared at declared tolerances." Python golden master: `tests/grid/golden/snapshot.npz` + `tests/grid/test_golden_master.py`. Synthetic-recovery gates (`tests/grid/test_tier0_recovery.py`): `overall >= 0.77`, `team_corr >= 0.60`, `total_smooth_corr >= 0.92`, `tau_smooth_corr >= 0.60`, `nis <= 10.0`, `0.9 <= equiv_slope <= 1.8`, `oos_r2 >= 0.05`, `rookie_corr >= 0.50`.

### §12.3 Point-in-time and leakage tests (1779–1785). VALIDATION, keep and extend

- Future-week row injection must fail.
- Season-summary target leakage must fail.
- Post-lock availability injection must not alter the locked prediction.
- "current-season participation unavailable at serve time must not appear in promoted live features."
- Stat corrections must create a new data version.

**Add:** "data published after the lock (publication-lag), even if describing earlier weeks, must fail." Python parity: `backend/validation/asof.py` `TripwireFrame` / `LeakageError`, `tests/validation/test_leakage_guards.py`.

### §12.4 Integration tests (1787–1793). VALIDATION, keep verbatim

- nflverse raw → normalization → SQLite → features → model update → projection
- NCAA raw → identity link → prior → projection
- manual availability import → workload update → new prediction version
- benchmark import → lock → outcome scoring → scorecard
- crash during each durable job stage → restart/recovery

### §12.5 Failure tests (1795–1808). VALIDATION, keep verbatim

Duplicate source files; provider schema change; missing player IDs; ambiguous NCAA identity; partial game data; network timeout; call-limit exhaustion; DB write failure; NaN model output; impossible team/player totals; interrupted promotion; corrupt model file.

### §12.6 FFI and UI tests (1810–1817). APP-ONLY, convert

| Original | Disposition |
|---|---|
| precision-preserving numeric-vector round trips | CONVERT: precision-preserving serialization round trips (f64 through CSV/Parquet/JSON/SQLite) |
| large projection-board pagination | CONVERT: large-result query pagination |
| event/query synchronization | CONVERT: event-log / query consistency (an event is never emitted before its payload is committed) |
| stale-data warning presentation | CONVERT: stale flags present in outputs |
| chart rendering with confidence bands | DROP |
| no UI freeze during training or simulation | CONVERT: no Tokio executor blocking during training/simulation (a test or lint) |

### §12.7 AI-specific regression protections (1820–1835). PROCESS, keep

Ten prohibited shortcuts:
1. deleting or weakening a failing test without an approved requirement change
2. regenerating golden files without a reviewed semantic explanation
3. broad exception handling converting failures to defaults
4. typed errors replaced with log-and-continue on a critical path
5. disabling lints, warnings, compiler, migration or leakage checks
6. hard-coding fixture-specific outputs
7. using competitor projections or target-week outcomes in features
8. changing thresholds after seeing benchmark results without versioning the protocol
9. silent fallback for unknown provider fields
10. marking tests ignored/skipped without risk acceptance

**Add:** "regenerating oracle fixtures, or editing `reference/python/`, in the same change as the Rust component they gate."

"CI includes checks for newly ignored tests, changed golden artifacts, migration rewrites, generated-file drift, and unexplained dependency additions." Keep, and add oracle-fixture drift.

### §12.8 PR evidence contract (1837–1853). PROCESS, keep with edits

Items: WP link and authority refs; outcome and non-goals; architecture, contract, schema, model and ~~FFI~~ **public API/output-contract** impact; migration and rollback; tests added/changed and why; exact verification commands and results; performance measurements when targets are affected; ~~screenshots/golden diffs for visible Flutter changes~~ (**drop**; replace with "golden/oracle-fixture diffs when numerical outputs change"); data/licensing/security implications; reviewer findings and resolutions; known limitations and follow-up.

"A PR may not state 'all tests pass' unless the listed command was run against the final commit. CI remains authoritative if local and CI results differ." Keep.

### §12.9 Independent review checklist (1855–1868). PROCESS, keep with edits

1. Does the diff satisfy the WP without expanding scope? Keep.
2. "Does it preserve the Flutter/Rust/SQLite ownership boundary?" becomes "Does it preserve the Rust-authoritative engine / SQLite / reference-oracle boundary (no runtime Python, no business logic in CLI/presentation)?"
3. Provider timestamps, units, null semantics, as-of rules. Keep.
4. Could any target, post-lock fact or competitor value leak? Keep.
5. Equations, constraints, seeds, tolerances as specified? Keep, and add "and parity with the oracle where declared?"
6. Can a failure corrupt or partially promote production state? Keep.
7. Migrations append-only, reversible where required, tested from blank and prior DBs? Keep.
8. Secrets, unsafe commands, new deps, licensing risks? Keep.
9. "Are UI states, errors, loading states, accessibility, and stale-data warnings represented?" becomes "Are error states, typed failures, and stale/manual-data flags represented in outputs?"
10. Is the verification evidence sufficient to reproduce? Keep.

### §13 Performance and resource targets (1872–1893). MIXED

"All targets are provisional and must be benchmarked on a declared reference ~~Windows~~ machine." The OS follows the §8.11 decision. Reference class: **≥8 logical cores, 16 GB RAM, NVMe SSD**.

| Target | Disposition |
|---|---|
| projection-board query p95 **< 200 ms** after materialization | KEEP as an engine query-API target (`get_week_projections`), or drop (owner) |
| player-detail query p95 **< 250 ms** | KEEP as API target, or drop |
| visible UI response to commands **< 100 ms** | **DROP** |
| normal daily incremental pipeline **< 10 min** | KEEP |
| all-player weekly simulation **< 90 s** at production draw count | KEEP |
| full three-season rebuild **< 45 min** | KEEP |
| memory **< 4 GB** normal, **< 8 GB** explicit rebuild mode | KEEP ("application memory" becomes "engine process memory") |
| no CPU-heavy work on the Flutter isolate or Tokio async worker threads | KEEP minus "Flutter isolate" |

Keep: "benchmark evidence—not architectural slogans—determines whether to optimize, downsample, cache, or revise the target. Claude must not perform speculative optimization before a repeatable benchmark exists, and any performance claim in a pull request must name the machine, dataset, command, sample count, and before/after result."

### §14 Security, licensing, data governance (1897–1908). MIXED

| Bullet | Disposition |
|---|---|
| HTTPS/TLS for all external sources | KEEP |
| API keys outside source control, "protect them with Windows-native credential protection" | REWRITE: "...with the platform's native credential store (Windows Credential Manager / OS keychain / CI secret store)" |
| response-size, timeout and schema limits | KEEP |
| attribution and license metadata for nflverse, FTN-derived and NCAA data | KEEP (+ FTN credit string "FTN Data via nflverse", per cautious-nevermore roadmap) |
| review commercialization/redistribution rights before public release | KEEP |
| "Do not expose provider API keys through Flutter." | REWRITE: "Do not expose provider API keys through any engine output, export, log, error message, or committed fixture." |
| do not redistribute competitor projection rows | KEEP |
| do not train on competitor projections | KEEP |
| validate imported CSVs and reject formula injection on export/import paths | KEEP (more important now that CSV export is a primary interface) |
| source lineage record for every published projection | KEEP |

### §14.1 Coding-agent security boundary (1911–1921). PROCESS, keep

Nine rules: least-privilege filesystem/network, with writes only in the assigned worktree; deny reads of credential stores, SSH keys, browser profiles, unrelated home dirs, signing keys, private benchmark dirs and production DBs; sandboxing, where failure to establish it is a hard failure for unattended sessions; network allowlist of docs, source repos, package registries and issue/CI; keys never in prompts, source, fixtures, transcripts or logs; the agent cannot publish packages, rotate credentials, alter repo protections, ~~sign installers~~ **sign release artifacts**, or deploy; untrusted content (issues, comments, payloads, scraped text, CSV cells) cannot grant permissions; destructive commands, out-of-worktree writes and non-ephemeral DB operations require denial or human approval; telemetry/retention/transcript policies documented.

Add "package registries for the pinned Python reference deps" to the allowlist note.

### §14.2 Dependency policy (1923–1931). PROCESS, keep

- Pinned lockfiles + approved manifest.
- Prefer existing deps and std.
- A new production dep requires justification, license check, advisory check, maintenance assessment, ~~Windows~~ **supported-platform** compatibility check, and approval.
- Verify names/APIs from pinned docs/source.
- Major upgrades are separate WPs.
- Vendored native artifacts checksum-verified and reproducibly built.
- Release evidence includes an SBOM and license inventory.
- **Add:** "`reference/python/` dependencies are pinned in their own lockfile, are dev/CI-only, are not production dependencies, and are excluded from the engine SBOM. Changes to them require a reason, because they can shift oracle outputs."

### §15 Key risks and mitigations (1935–1957). MIXED

Keep all 18 rows verbatim except:
- "Desktop compute contention | Resource modes, bounded worker pool, incremental updates, pagination, progress events" becomes "**Compute contention on the host** | same mitigations".
- "Model-specific coding workflow becomes a lock-in | ... no runtime Claude dependency": keep.

Rows kept verbatim: injury feed post-2024; participation unavailable live; NCAA identity mismatch; NCAA translation overconfidence; TD volatility; injury/news timing under once-daily fetch; competitor licensing; historical benchmark scarcity ("Maintain live shadow archive from first alpha week"); overfitting; claim overreach; Claude hallucinates crate/API/field/command; long sessions lose constraints; implementer changes tests to fit code; AI self-review misses errors; parallel agents conflict; agent exposes secrets / prompt injection; AI-authored migration corrupts data.

**Add three rows:**

| Risk | Mitigation |
|---|---|
| Rust port silently diverges from the reference oracle | Committed oracle fixtures with hashes; per-component parity gates; fixture-drift CI; ADR for any accepted divergence |
| Oracle itself encodes leakage or train/serve skew (e.g., in-season participation in walk-forward) | Spec outranks oracle; publication-lag as-of axis; oracle paths that violate §4.5 are labeled research-only and excluded from parity gates |
| Python reference rots or blocks the Rust build | Oracle isolated under `reference/python/` with its own pinned env and CI job; no cargo dependency on it; explicit retirement criterion |

### §16 Explicit alpha non-goals (1961–1980). MIXED, keep with consolidation

Current (18):
- Mobile application
- Web application or browser dashboard
- Cloud model inference
- Always-running server
- Real-time in-game projections
- Continuous news/injury polling
- Automated league sync
- Draft-room assistant
- DFS lineup optimizer
- Sports betting recommendations
- IDP projections
- Social features
- NL news generation
- Paid data dependency as a requirement for Phase 1
- Public redistribution of nflverse/NCAA/competitor data beyond terms
- Embedding Claude/Anthropic/coding agent "in the shipped application"
- Allowing an AI agent to merge/sign/publish/promote without human gates
- Treating generated code or agent confidence as evidence

Engine-only edits:
- Replace "Mobile application" and "Web application or browser dashboard" with "**Any user interface** (desktop, mobile, web, dashboard). UIs are external consumers of the output contract."
- Add "FFI/language bindings for UI frameworks."
- Add "Python (or any interpreter) in engine runtime/release artifacts."
- Change "in the shipped application" to "in the engine or any released artifact."
- Keep "Automated league sync", "Draft-room assistant", "DFS lineup optimizer", "Social", "NL news generation". These explicitly exclude the cautious-nevermore app layers that are **not** imported: `backend/adapters` (ESPN/Sleeper), `backend/trades`, `backend/services` (mock draft, draft grade, live draft), `backend/viz_agent`, `backend/api`, `frontend/`.
- Note: cautious-nevermore's VOR/tiers (`backend/scoring/vor.py`) and lineup simulation (`validation/lineup_sim.py`, H2) sit at the boundary. VOR is a ranking transform and arguably app-level; H2 lineup sim is a validation harness. Owner decision on whether either is in engine scope.
- Also from fbs §24 (not already listed): Pandas, Cloud database, Full model retraining after every new game, Literal one-file `.exe` (the last is moot).

### §17.1 Phase 1 deliverables (1986–1999). MIXED

- Keep: AI-ready repository controls (`CLAUDE.md`, authority index, WP template, ADR workflow, agent/skill definitions, permission policy, verification scripts, PR/evidence templates, traceability checks); SQLite schema and migrations; nflverse and NCAA adapters; raw retention and DQ reports; identity resolver and review artifacts; versioned feature catalog; baseline and first ensemble models; rolling-origin backtest runner; exported historical validation report; model card (data window, targets, features, limitations).
- **Drop/convert:** "Native Windows alpha installer" becomes "versioned engine release artifact (CLI binary + library crates + bundled native deps)". "projection board, player detail, status, and scoring-profile UI" becomes "CLI/report/export surface for projections, player detail, status and scoring profiles".
- **Add:** "reference-oracle parity report for each ported component."

### §17.2 Phase 2 deliverables (2001–2011). VALIDATION/ENGINE, keep

Live once-daily ~~scheduler~~ update and lock workflow; availability review (CLI/report) and adapter interface; advanced ensemble and correlated simulation; promotion/rejection/rollback workflow; benchmark registry and importers; live weekly scorecard and CIs; market-claim evidence package; production-hardening test report; updated model card and data-source/license register.

### §17.3 AI implementation evidence deliverables (2014–2024). PROCESS, keep

`ai-toolchain.lock`; sanitized per-WP manifests; approved WP backlog and dependency map; architecture/model/provider contract catalog; ~~canonical Windows~~ authoritative-platform bootstrap and verify logs; fresh-context review reports for high-risk work; dependency/SBOM and license reports; clean-checkout reconstruction report; "runbook for replacing the coding model or harness without changing production code".

### §18 Final Definition of Done (2028–2048). MIXED

| # | Original (abridged) | Disposition |
|---|---|---|
| 1 | "native Windows Flutter/Rust application using SQLite and the production concurrency/governance pattern" | REWRITE: "The engine is a Rust library + CLI using SQLite and the concurrency/governance pattern of §1.1 and §8." |
| 2 | every weekly projection reproducible from immutable source/feature/model/scoring versions | KEEP |
| 3 | strict three-season NFL window; NCAA priors only for low-evidence players | KEEP |
| 4 | missing injury data explicitly handled "and visible" | REWRITE: "...and flagged in outputs" |
| 5 | coherent stat distributions, not only point estimates | KEEP |
| 6 | historical and live evaluation point-in-time correct | KEEP |
| 7 | candidates cannot replace production without validation | KEEP |
| 8 | competitor comparisons timestamped, legal, non-redistributive, same player-week outcomes | KEEP |
| 9 | Phase 1 and Phase 2 exit criteria met | KEEP |
| 10 | accuracy claims limited to evidence | KEEP |
| 11 | "shipped application contains no Claude/Anthropic runtime dependency, model key, transcript, or agent-only state" | REWRITE: "No engine artifact contains a Claude/Anthropic runtime dependency, model key, transcript, agent-only state, or Python runtime." |
| 12 | every non-trivial merged change traceable to WP, final commit, verification result, reviewer | KEEP |
| 13 | "A clean Windows checkout can bootstrap, build, test, migrate, and package using repository scripts" | REWRITE per the §8.11 decision: "A clean checkout on each supported platform..." |
| 14 | "Generated FFI bindings, SQLx metadata, model artifacts, and fixtures are reproducible and checked for drift" | REWRITE: drop FFI; add "oracle-derived fixtures" |
| 15 | no critical-path placeholder, unexplained ignored test, silent fallback, weakened warnings, unreviewed golden change | KEEP |
| 16 | statistical, architecture, security, data-rights, promotion and release decisions have human approvals | KEEP |
| 17 | continuable by a fresh Claude session, human, or replacement model from repo contracts alone | KEEP |
| **18 (new)** | — | "Every ported component with an oracle counterpart meets its declared parity gate, or carries a statistical-owner-approved ADR documenting the divergence." |

### Appendix A: Source verification notes as of 2026-08-12 (2052–2060). DATA, keep and re-verify

- nflverse provides PBP, player/team stats, rosters, players, snaps, advanced stats, NGS, depth charts and related datasets via automated repos.
- PBP and stats update after game days; other datasets have their own cadences.
- **Participation from 2023 onward is not in-season; it is provided after the postseason.**
- **The injury source ended after 2024; 2025 data unavailable.**
- CFBD offers a free REST tier (historical, team, player, recruiting, betting-line, advanced) subject to limits and terms.
- Market services with weekly projections: FantasyPros, PFF, RotoWire, 4for4, ESPN, CBS; rights differ.
- FantasyPros methodology uses Thursday/Sunday pregame snapshots, a pool from consensus + actual leaders, and Weeks 1–17. "This alpha uses a compatible lock concept while retaining point-projection metrics as its primary standard."

Additions from cautious-nevermore `docs/01-product-roadmap.md:198,213,217`:
- nflverse is **CC-BY-SA 4.0**.
- `pbp_participation_{year}.parquet` is free for 2016–2025 with 11 players per side.
- From 2023 it is FTN Data (credit "FTN Data via nflverse"), published once per year after the postseason.

The notes are dated; the rewrite should carry a fresh "as of" date and a re-verification owner.

### Appendix B: Work-package template (2064–2131). PROCESS, keep

Sections: Status (Draft | Ready | In Progress | Review | Done | Blocked); Ownership and risk (Owner, Implementer, Reviewer, Risk class Low | Medium | High | Release-critical, Required approvals); Authority; Objective ("One measurable outcome"); User-visible outcome; Preconditions; Scope (modules/files, contracts consumed/changed); Non-goals; Inputs and fixtures; Implementation constraints; Acceptance criteria (checkboxes with objective evidence); Verification (targeted + canonical command); Numerical/performance tolerances; Migration and rollback; Evidence required; Stop/decision conditions; Follow-up.

Edits:
- Authority lines "`final-build-spec.md`: <sections>" and "Alpha spec: <sections>" become "Engine spec: <sections>" (one line).
- "User-visible outcome" becomes "Observable outcome (CLI/API/report/file), or 'none' for infrastructure".
- Evidence: "Test output, screenshots, benchmark results, hashes..." loses "screenshots" and gains "oracle parity report".
- **Add a section** "Oracle parity targets: reference module(s), gate(s), tolerance, or 'none'."
- The repo copy is `docs/99-templates/template-work-package.md`; P1-00's acceptance criterion requires it to "follow the exact Appendix B structure", so the template and the spec appendix must change together.

### Appendix C: PR completion record (2133–2155). PROCESS, keep verbatim

Work package; final commit; model/harness identifier; environment; authority documents read; contracts changed; migrations changed; dependencies changed; targeted tests; canonical verification command; exit status; golden files changed and approval; performance evidence; security/licensing review; fresh-context reviewer; reviewer findings resolved; human approvals; known limitations; evidence manifest hash.

Optionally add "Oracle parity result".

### Appendix D: Prohibited AI shortcuts (2157–2172). PROCESS, keep with edits

Claude must never:
1. claim a command passed when not run against the final commit;
2. invent a provider field, crate, package, ~~Flutter API~~, SQLx behavior, or model formula;
3. edit ~~generated FFI code~~ **generated code or artifacts (SQLx cache, generated schemas, oracle-derived fixtures)** by hand instead of changing the source and regenerating;
4. rewrite/squash/delete historical migrations;
5. weaken a test, metric, leakage rule, lock rule, warning policy, or security control to finish a package;
6. auto-accept golden changes without a reviewed reason;
7. use post-lock info, competitor projections, or target outcomes in training features;
8. convert malformed/unknown critical data to healthy/zero/default without explicit semantics;
9. add an unapproved production dependency or enable a new external host;
10. expose credentials, private benchmark data, transcripts, signing material, or unrelated files;
11. merge, sign, publish, promote, or deploy on its own authority;
12. leave a critical path stubbed while marking the package done.

**Add #13:** "change `reference/python/` behavior or regenerate oracle fixtures to make a Rust parity test pass."

`CLAUDE.md` "Prohibited" section and `scripts/check-migrations.sh:4,59` cite Appendix D (#4).

### Appendix E: Workflow reference notes (2174–2185). PROCESS, keep verbatim

Six practices: executable verification; separate exploration/planning from implementation; concise persistent instructions with detail in scoped docs/skills; isolated specialized agents; enforce critical actions with CI, permissions, sandboxing and hooks; protected PRs with human merge/release. Record the actual model id in `ai-toolchain.lock`; production code stays model-independent.

---

## 3. Cross-references to `final-build-spec.md`

### 3.1 Explicit (by filename) in alpha-spec.md, 3 occurrences

| Line | Text | Engine-only action |
|---|---|---|
| 11 | `**Production architecture authority:** \`final-build-spec.md\`` | Remove, or repoint to the consolidated spec / superseding ADR |
| 120 | §1.5 item 1: `` `final-build-spec.md` `` (top of authority order) | Rewrite the authority order (§1.5 above) |
| 2080 | Appendix B template: ``- `final-build-spec.md`: <sections>`` | Replace with one "Engine spec: <sections>" line |

### 3.2 Implicit (by phrase) in alpha-spec.md

| Alpha line(s) | Phrase | fbs section depended on |
|---|---|---|
| 23 (§0) | "The production application must have no runtime dependency on Claude" | fbs as "production" system (generic) |
| 35 (§1 heading), 37 | "Relationship to the Production Architecture"; "direct subset of the attached production system" | fbs whole |
| 39–54 (§1.1) | non-negotiable list | fbs §1 (3 non-negotiables: Flutter UI; 7 methods; once-daily + incremental), §2 (architecture), §3.1 (no WebView etc.), §7 (Tokio/Rayon, resource modes, serialized publication), §8 (SQLite source of truth), §8.2 (SQLx, `.sqlx` cache, migrations), §10 (Polars, feature schema versions), §12.3/§16 (snapshot, rollback, crash recovery), §13 (validation before promotion), §14 (versioning), §15 (durable jobs), §24 ("Python runtime" non-goal) |
| 58 (§1.2) | "The attached production document contains generic sports entities such as possessions, lineups, and stints" | fbs §8.1 (`lineups`, `stints`, `possessions` tables) and §11.3 (RAPM "stint boundaries, players on court, home-court") |
| 94 (§1.3) | "change the non-negotiable production architecture" | fbs §1 |
| 626–638 (§6.2) | "Mapping the required statistical methods" | fbs §1 item 2, §11.1–§11.8 (nalgebra/sprs, hand-rolled ridge, sparse-CG RAPM with diagnostics `converged, iterations, residual_norm, tolerance, regularization_lambda`, Kalman persisted x_k/P_k, full and fixed-lag RTS, EB persisted fields, affine `y = Ax + b` with inverse/serialization, `IncrementalBooster` trait with xgb primary and linfa-trees fallback) |
| 919–952 (§8) | Flutter UI → FRB → Rust core diagram | fbs §2 |
| 954–983 (§8.1) | Rust module tree incl. `ffi/` | fbs §2 ("FFI layer is an adapter..."), §11 |
| 985–1029 (§8.2–8.4) | Commands / Queries / Events; "Events notify Flutter" | fbs §5.1–§5.4 (incl. `TrainingStatus` streams) |
| 1031–1102 (§8.5) | schema groups | replaces fbs §8.1 domains |
| 1104–1138 (§8.6) | daily pipeline | fbs §9.1–§9.2, §12.1 |
| 1320 (§8.11) | "XGBoost/native artifacts ... pinned" | fbs §3.2 (pin XGBoost, vendor artifacts, bundle MSVC redist, code signing), §21 |
| 1456 (§9.3) | "clean install can rebuild in-memory state from SQLite" | fbs §8 intro, §16 rule 4 |
| 1583–1588 (§10.2 Governance) | NaN/sanity checks, auto reject/rollback | fbs §12.3, §13 (model states TRAINING…SUPERSEDED), §16 |
| 1749–1817 (§12.1–12.6) | testing levels | fbs §19.1–§19.5 (FFI `Vec<f64>`↔`Float64List` round trips) |
| 1872–1893 (§13) | performance | fbs §20 |
| 1897–1908 (§14) | security | fbs §18 |
| 1949 (§15) | "Desktop compute contention ... Resource modes" | fbs §7 (`normal`, `background`, `manual_rebuild`) |
| 1961–1980 (§16) | non-goals | fbs §24 |
| 1497–1532 (§9.5) | workstream sequence | parallels fbs §23 Phases 1–6 (the fbs order is shell → data → stat engine → incremental → visualization → hardening) |
| 2032 (§18 #1) | "production concurrency/governance pattern" | fbs §7, §13–§16 |

**fbs content that alpha does NOT restate but an engine-only spec must absorb** (otherwise lost when fbs is demoted). The fbs inventory agent should confirm; the engine-relevant items are:
- **fbs §9.5 failure-state enum (14):** `NOT_RUN, RUNNING, SUCCESS, NO_NEW_DATA, PARTIAL_SUCCESS, NETWORK_FAILURE, TIMEOUT, RATE_LIMITED, AUTHENTICATION_FAILURE, INVALID_RESPONSE, SCHEMA_FAILURE, DATA_VALIDATION_FAILURE, DATABASE_FAILURE, LEARNING_FAILURE`.
- §9.3 idempotency (`ingestion_run_id`; safe to run twice).
- §9.4 partial acceptance with quarantine.
- §9.6 catch-up.
- §12.2 three GB training modes (daily continuation, replay-window, periodic rebuild).
- §13 model states `TRAINING, VALIDATING, CANDIDATE, PRODUCTION, REJECTED, SUPERSEDED`.
- §14 model metadata fields (12) and the `Data Version → Feature Version → Model Version → Prediction Version` chain.
- §15 job types (10) and job fields (11).
- §16 recovery rules (6).
- §17 observability fields.
- §11 solver diagnostics and the `IncrementalBooster` trait.
- §21 core dependency list.

### 3.3 Other repo files citing fbs sections (will need repointing if fbs is demoted)

- `docs/01-work-packages/p1-00-work-package.md:24`: fbs §2, §3, §7, §8, §19, §21, §22.
- `docs/02-adr/001-repo-bootstrap-decisions.md:254` (§3.2).
- `docs/02-adr/002-sqlx-offline-cache.md:20,139` (§8.2).
- `docs/02-adr/003-sqlx-0-9-upgrade.md:28,36,54,119` (§8, §8.2, §8.3, §3.2).
- `docs/02-adr/009-verify-ps1-exit-codes.md:152` (§3.2, "Windows-native gate by design"; directly affected by the platform decision).
- `scripts/check-env-contract.sh:38` (fbs 8).
- `scripts/check-migrations.sh:4` (fbs 8.2).
- `CLAUDE.md:8`, `docs/CLAUDE.md:8`, `docs/00-meta/authority-index.md:7,14` (authority order).

---

## 4. Workstream sequences as written

### 4.1 §9.5 Phase 1 (lines 1497–1532), verbatim

> The two-phase product plan is unchanged, but Phase 1 implementation is decomposed into dependency-ordered workstreams suitable for bounded agent sessions. Each workstream contains multiple work packages; it is not automatically a single pull request.

```text
P1-00  Agent-ready repository, authority index, CI, fixtures, verify scripts
   ↓
P1-01  Domain IDs, time/as-of types, errors, FFI contract skeleton
   ↓
P1-02  SQLite schema, migrations, durable jobs, raw-data/version primitives
   ↓
P1-03  nflverse provider contracts and raw/normalized ingestion
   ↓
P1-04  Canonical player registry, NFL identity, NCAA adapter and linking
   ↓
P1-05  Point-in-time feature store and leakage test harness
   ↓
P1-06  Numerical primitives: scoring, ridge, Kalman, EB, affine, boosting adapter
   ↓
P1-07  Team environment, opportunity allocation, efficiency, rookie priors
   ↓
P1-08  Correlated simulation, distributions, explanation payloads
   ↓
P1-09  Rolling-origin backtest, player pool, PB-MAE and calibration evaluation
   ↓
P1-10  Flutter projection board, detail views, status, settings, CSV export
   ↓
P1-11  Recovery, installer, clean-machine and end-to-end acceptance evidence
```

Permitted parallelism (verbatim):
- "P1-02 and the pure numerical portions of P1-06 may proceed after P1-01 contracts stabilize."
- "Flutter shell work may proceed after FFI request/response contracts are frozen, but UI business logic may not be invented to compensate for missing Rust services."
- "Provider adapters may be implemented in parallel only when their normalized destination contracts are stable and they do not edit the same migration or identity files."
- "Integration occurs through contract fixtures and protected CI, not through informal agreement between concurrent agent sessions."

Observations:
- P1-06 omits RAPM and RTS even though §1.1 and §6.2 require them and §12.1 unit-tests them. Fixed-lag smoothing appears only in P2-03.
- Current status: P1-00 is on PR #1 (`wp/P1-00-repo-bootstrap` @ `3823478`), approved after round-4 adversarial review with 0 blockers. ADR-006 assigns P1-00 deferrals: agent definitions → P1-01; `schemas/`, `fixtures/` → P1-03/P1-04; `benches/` → P1-07; runbooks and model-cards → P1-11; `app/lib/main.dart`, `app/pubspec.lock`, `toolchains/native-dependencies.lock` → P1-10/P1-11.

### 4.2 §10.5 Phase 2 (lines 1655–1677), verbatim

```text
P2-00  Live-operation threat model, permission profile, runbooks, shadow environment
   ↓
P2-01  Once-daily scheduler, catch-up behavior, Thursday/Sunday immutable locks
   ↓
P2-02  Availability snapshots, manual review, provider-neutral adapter contract
   ↓
P2-03  Advanced state updates, fixed-lag smoothing, bounded boosting continuation
   ↓
P2-04  Scenario-aware opportunity transfer and calibrated correlated simulation
   ↓
P2-05  Benchmark registry/import, legal-use metadata, immutable provider snapshots
   ↓
P2-06  Pairwise evaluation, confidence intervals, scorecards, claim-evidence package
   ↓
P2-07  Candidate governance, promotion serialization, rollback and audit UI
   ↓
P2-08  Installer, recovery, performance, security and independent audit hardening
```

Human gates (verbatim): "enabling a new live provider, approving a destructive migration, changing a benchmark rule, changing a model equation, promoting a production model, authorizing public claim language, and signing or publishing a release."

---

## 5. Proposed engine-only re-sequencing

### 5.1 Constraints that shape the proposal

- **Guard ID format.** `scripts/check-traceability.sh:70` matches only `P1-[0-9]{2}`, `ADR-[0-9]{3}` and doc paths, and resolves IDs by globbing `docs/01-work-packages/*<lowercased id>*`. P2-xx IDs are **not recognized today**. Suffixes like `P1-00a` would also glob-match `p1-00-work-package.md`. **Recommendation:** extend the regex to `P[0-9]-[0-9]{2}` inside the pivot change (it needs an ADR because it changes a guard), and keep IDs in `P<phase>-<NN>` form. `scripts/check-evidence-claims.sh:29` hard-codes `WP="P1-00"`; generalize it in the same change.
- **Number stability.** ADR-006 and `docs/00-meta/authority-index.md:68` reference P1-01, P1-03/04, P1-07, P1-10 and P1-11 by number. Keeping P1-01…P1-11 meanings as close as possible (re-scoping P1-10 and P1-11 rather than deleting them) avoids rewriting those records. Add new numbers (P1-12+) for genuinely new work.
- **The pivot itself** (trim Flutter/FFI, import `reference/python/`, rewrite authority docs and spec, CI oracle job, regex fix) is the current consolidation branch. Govern it with **ADR-011 "Engine-only pivot"**, which supersedes the relevant parts of ADR-001, ADR-006 and ADR-009 and cites the owner decisions. The traceability guard already accepts `ADR-011` references, so no new WP ID is needed for it.

### 5.2 Phase 1 (engine-only): DAG

```text
P1-00  Agent-ready repo, authority, CI, verify scripts                  [DONE — PR #1 @ 3823478]
   ↓
ADR-011 / consolidation PR  Engine-only pivot:
       - drop crates/ffi, app/, toolchains/flutter.version, FRB pin in [workspace.dependencies]
       - import cautious-nevermore engine → reference/python/ (grid, projection, validation,
         scoring; engine parts of pipeline) + its tests + pinned requirements; oracle CI job
       - rewrite spec + authority order (oracle placement) + CLAUDE.md + authority-index
       - extend traceability regex to P[0-9]-[0-9]{2}; generalize check-evidence-claims
       - decide authoritative CI platform (§8.11)
   ↓
P1-01  Domain IDs, time/as-of types (incl. publication timestamps), typed errors,
       ENGINE PUBLIC API + OUTPUT-CONTRACT skeleton (replaces "FFI contract skeleton");
       oracle-fixture format + Rust parity-harness skeleton (reads committed fixtures);
       agent definitions per ADR-006 item 1 (+ reference-parity reviewer role)
   ↓────────────────────────────────┬─────────────────────────────────────────────┐
P1-02  SQLite schema, migrations,   P1-06  Numerical primitives — scoring (affine), │
       durable jobs, raw-data/              ridge (dense Cholesky + sparse CG w/      │
       version primitives                   diagnostics = RAPM solver), Kalman        │
       (fbs §9.5 states, §13 model          predict/update (Joseph form), RTS full + │
       states, §15 jobs absorbed)           fixed-lag, EB, affine, IncrementalBooster │
   ↓                                        adapter.  PARITY: scoring/engine+formats, │
P1-03  nflverse provider contracts +        statespace.kalman_step / RTS, layers      │
       raw/normalized ingestion             ridge-with-prior-mean, sv_to_points,      │
       (oracle loaders as failure-case      test_kalman_numerical.py                  │
       inputs)                       ───────┘ (starts right after P1-01)              │
   ↓                                                                                  │
P1-04  Registry, identity, NCAA adapter + linking                                     │
   ↓                                                                                  │
P1-05  Point-in-time feature store + leakage harness, incl. PUBLICATION-LAG axis;     │
       PARITY: oracle AsOf/TripwireFrame semantics (temporal/scope/state axes);       │
       commit synth.py-generated synthetic football fixtures here (or in P1-01)       │
   ↓────────────────────────────────┬─────────────────────────────────────────────────┘
P1-07  §6.1 Layers A–D + §6.3       P1-12  GRID component port (ensemble members 3/5):
       NCAA/rookie priors;                 V(s) expected-points + dV, situations masks,
       naive baselines (§9.2).             RAPM w/ team intercepts + market reconciliation
       PARITY (partial): volume.py,        (incremental accumulators; two-path equivalence),
       model.py, priors.py                 3-component Kalman talent/form/scheme + changepoint
                                           regime drops, cross-league priors.
                                           PARITY: test_tier0_recovery gates, golden master
                                           snapshot.npz.  In-season participation use =
                                           research-only (§1.2/§4.5).
   ↓────────────────────────────────┘
P1-08  Layer E context, Layer F correlated simulation (5,000 draws/game dev), §5.3
       distributions, §5.2 conditional/unconditional, §6.4 stacking, §6.5 explanations.
       (No oracle counterpart — spec-defined golden tests §12.2.)
   ↓
P1-09  Rolling-origin backtest (three-season window), player pool, PB-MAE + §7.5 metrics,
       calibration, rookie/low-evidence scorecards.
       PARITY: metrics.py (MAE/RMSE/bias/CRPS/PICP/Spearman/bootstrap CI), backtest two-path
       equivalence; PB-MAE + pool are new.  Produces the §9.4 model-quality evidence.
   ↓
P1-10  (re-scoped, was Flutter UI) Engine CLI, reports, export: projections table
       (board columns + freshness flag), player detail, status/freshness, scoring-profile
       registry, CSV/Parquet export (projections + quantiles only; no competitor data),
       identity review queue (read-only in P1).
   ↓
P1-11  (re-scoped) Recovery/crash-restart, release artifact (CLI binary + vendored native deps,
       native-dependencies.lock), clean-checkout/clean-environment reproduction, end-to-end
       acceptance evidence, model card, exported historical validation report, runbooks.
```

Permitted parallelism (engine-only rewrite of the §9.5 rules):
- P1-06 runs in parallel with P1-02…P1-05 after P1-01. This is a strengthening of the original "pure numerical portions of P1-06" rule: P1-06 is the highest-value parity work and needs no persistence.
- P1-07 and P1-12 run in parallel after P1-05 and P1-06, with no shared files. Both consume the P1-05 feature store and the P1-06 primitives.
- Provider adapters (P1-03, P1-04) run in parallel only with stable destination contracts and no shared migration/identity files (verbatim rule kept).
- The Flutter-shell rule is replaced by: "CLI/report work (P1-10) may start after the P1-01 output contract is frozen, but must not compute business logic the engine services lack."
- Integration happens through contract fixtures (including oracle fixtures) and protected CI (verbatim rule kept).

### 5.3 Phase 2 (engine-only)

```text
P2-00  Live-operation threat model, permission profile, runbooks, shadow environment   [unchanged]
   ↓
P2-01  Once-daily `update` (externally scheduled; engine-enforced one-fetch-per-local-day cap,
       day-of-week aware), catch-up on invocation, Thursday/Sunday immutable locks
   ↓
P2-02  Availability snapshots, review via CLI/report + file import, provider-neutral adapter
   ↓
P2-03  Advanced state updates, fixed-lag smoothing, bounded boosting continuation/replay   [unchanged]
   ↓
P2-04  Scenario-aware opportunity transfer, calibrated correlated simulation (calibrated tails;
       benchmarked production draw count)                                                 [unchanged]
   ↓
P2-05  Benchmark registry/import, legal-use metadata, immutable provider snapshots         [unchanged]
   ↓
P2-06  Pairwise evaluation, CIs (week-clustered bootstrap), scorecards/benchmark reports,
       claim-evidence package                                                             [UI → reports]
   ↓
P2-07  Candidate governance, promotion serialization, rollback and audit CLI/report        [UI → CLI]
   ↓
P2-08  Release packaging (CLI/library), recovery, performance (§13), security and
       independent audit hardening                                                        [installer dropped]
```

- Human gates: keep verbatim, with "signing or publishing a release" referring to engine release artifacts. Add "accepting a divergence from the reference oracle."
- Prerequisite: the traceability regex must recognize `P2-NN` before P2-00 starts.
- Optional, at owner discretion: **P2-09 Oracle retirement review.** Once every ported component has parity evidence and Phase 2 live evidence exists, decide whether `reference/python/` is frozen, archived or kept as a CI oracle.

---

## 6. Inbound references that break when alpha-spec.md is rewritten or renamed

Files citing alpha-spec section numbers (`git grep` counts of §/Appendix refs, excluding the specs themselves):

| File | Refs |
|---|---|
| `docs/02-adr/001-repo-bootstrap-decisions.md` | 25 |
| `.ai/evidence/P1-00/PR-BODY.md` | 21 |
| `docs/01-work-packages/p1-00-work-package.md` | 16 |
| `docs/02-adr/006-deferred-deliverables.md` | 16 |
| `docs/02-adr/003-sqlx-0-9-upgrade.md` | 15 |
| `docs/00-meta/authority-index.md` | 7 |
| `docs/02-adr/009-verify-ps1-exit-codes.md` | 6 |
| `docs/99-templates/template-work-package.md` | 6 |
| `docs/02-adr/002-sqlx-offline-cache.md` | 4 |
| `docs/CLAUDE.md` | 4 |
| `CLAUDE.md` | 3 |
| ADR-004/007/008 | 3 each |
| provider READMEs, model-spec and provider templates | 3 each |
| scripts: `check-traceability.sh` (9.4, 12.8), `check-verify-parity.sh` (8.11), `generate-evidence-manifest.sh` (8.12, 8.11), `check-secrets.sh` (14.1), `check-evidence-claims.sh` (12.8), `check-migrations.sh` (App D #4), `bootstrap-repo.sh` (8.7) | 1–3 each |
| `justfile` (8.11), `.github/workflows/alpha-ci.yml` (8.11, 14.1, 14.2), `.claude/settings.json` (14.1), `Cargo.toml` (8.11, on the FRB pin), `deny.toml`, `.gitignore`, `.gitattributes`, `_typos.toml`, `migrations/0001_schema_meta.sql`, `crates/persistence/src/lib.rs`, `toolchains/*` | 1–3 each |

Hard dependencies on the **filename**:
- `scripts/check-authority-sync.sh:15-16` (`CANONICAL="alpha-spec.md"`, `MIRROR="docs/00-meta/specs/alpha-spec.md"`) and `tests/guards/run.sh:229-231`.
- If the spec is renamed (e.g., `engine-spec.md`), update both in the same change, or keep the filename.

**Recommendation:**
- Either keep the §-numbering of retained sections stable where possible (e.g., leave §1.5, §8.7, §8.11, §8.12, §12.7, §12.8, §14.1, §14.2 and the appendices at the same numbers), or
- ship an "old § → new §" map in the rewritten spec and update citations in scripts, CLAUDE files and templates.

Immutable historical records (`.ai/evidence/P1-00/*`, ADR bodies) should **not** be rewritten. Add a superseded-by note pointing to ADR-011 and the § map instead.

---

## 7. Spec defects and ambiguities to fix in the rewrite (independent of the pivot)

1. Heading levels are inconsistent (`##` used for subsections from §4.1 onward).
2. §1.5 cites `docs/adr/`, §4.6 `docs/providers/`, §6.6 `docs/model-specs/`, §8.7 `docs/authority.md`. The repo uses `docs/02-adr/`, `docs/04-providers/`, `docs/05-model-specs/` and `docs/00-meta/authority-index.md`.
3. §2.3: Standard and PPR weights are implied, not enumerated. The affine-only custom scoring cannot express threshold bonuses.
4. §2.5: the position-specific "minimum NFL opportunity threshold" values are unspecified.
5. §5.1: QB has no snap share; RB/WR/TE have no start probability. Confirm this is intended.
6. §7.3: the "documented penalty or provider-tail estimate" for missing provider projections is undefined.
7. §7.4: the `scale_p` estimator is undefined ("fixed position scale estimated only from the training period").
8. §9.5 P1-06 omits RAPM and RTS although §1.1, §6.2 and §12.1 require them.
9. §6.2 Kalman scope (pace, pass tendency, opportunity share, efficiency) differs from the oracle's player talent/form/scheme Kalman. Decide whether both are in scope.
10. §4.3/§11.1 allow spread/total "when available before lock". The oracle's Layer-3 market reconciliation uses "closing line" strength. For games after the Sunday early-window lock, a closing line is post-lock, a latent leakage path. The model spec must define which line snapshot is permitted.
11. §8.6 runs "Kalman state updates + fixed-lag smoothing" daily in Phase 1, but P2-03 is where fixed-lag smoothing is scheduled. Make these consistent.
12. §13's reference machine is "Windows", and §8.11's authority rationale is "production target is Windows". Both are orphaned by the pivot.

---

## 8. Decisions needed (owner-level, not obvious defaults)

See the structured summary. They cover:
- Authority order and oracle placement.
- Authoritative CI platform.
- Which modeling architecture governs.
- In-season-participation oracle paths.
- Projection horizons beyond weekly.
- Feeder leagues beyond NCAA.
- VOR and lineup-sim scope.
- Numbering and filename stability.
- The oracle retirement criterion.
