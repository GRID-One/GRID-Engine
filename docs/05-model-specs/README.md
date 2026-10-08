# Model specifications

Each production statistical component of the engine is specified here before it is implemented
(engine-spec §6.8). Model specs are **authority level 3** (`engine-spec.md` §1.5): above the work
package, the tests and the code, below `engine-spec.md` and accepted ADRs. Their equations bind the
Rust implementation once the Statistical owner approves them. The implementing agent may not
redefine them.

- **Oracle behaviour vs engine requirements.** Each spec first describes the Python reference oracle
  (`reference/python/`, cautious-nevermore `59bce1d`) exactly as implemented, with
  `reference/python/<path>:<line>` citations. It then states the engine requirements (MUST, SHOULD,
  MAY). A difference from the oracle is a deliberate divergence, also listed in
  `reference/python/PARITY.md`. The oracle is evidence, not authority (engine-spec §1.7).
- **Proposed decisions.** "(proposed — DR-xx)" marks a default awaiting the owner
  (`docs/00-meta/decision-register.md`). It is not settled, and a work package that depends on it is
  not Ready.
- **Parity classes** are defined in engine-spec §7.12.5 (proposed — DR-B3): A element-wise closed
  forms, A′ dense solves, B iterative solves, C booster stages (unattainable as written; replaced by
  the proposed C-V and C-L1), D end-to-end recovery on synthetic truth. A spec may declare a different
  class before its parity test is written.
- **New specs** start from `docs/99-templates/template-model-spec.md`.

## Index

All eight specs are version 0.1.0, written under P0-01. None is approved.

| Model spec | Component | Oracle module(s) (`reference/python/backend/`) | Target Rust crate | Work packages | Parity class | Status | Key open DRs |
|---|---|---|---|---|---|---|---|
| [value-model.md](value-model.md) | Situational value V(s), play value `dV`, situation masks | `grid/value.py`, `grid/situations.py` | `models` (`value`, `booster`); `features` (masks) | P1-06 (trait), P1-12 | C-V for V(s) (proposed — DR-D27); A for `dV` given V; masks exact | Draft | DR-C7, DR-C8, DR-C13, DR-D27, DR-B3 |
| [rapm-attribution.md](rapm-attribution.md) | Layer-2 RAPM, Layer-3 market reconciliation, team strength, defender ratings, matchup grade, GRID fixed point | `grid/layers.py`; `validation/backtest.py` (`solve_rapm`) | `models` (`ridge`, `rapm`) | P1-06, P1-12 | A (assembly); A′ or B (solve; amended Class B proposed); D (recovery) | Draft | DR-B5, DR-B6, DR-C1, DR-C6, DR-D10, DR-D11, DR-D12 |
| [layer1-credit.md](layer1-credit.md) | Layer-1 cross-fitted per-play credit (offseason) and participation-free Layer-1′ credit (live) | `grid/layers.py` (`layer1_*`); Layer-1′: none | `models` (`credit`) | P1-06 (booster), P1-12 | A (aggregation); C-L1 (context model, proposed DR-B3 amendment); spec-defined goldens (Layer-1′) | Draft | DR-C1, DR-C7, DR-D13, DR-D14, DR-D15 |
| [state-space-kalman.md](state-space-kalman.md) | Per-player Kalman over talent, form, scheme fit; RTS, fixed-lag, changepoints | `grid/statespace.py` | `models` (`statespace`) | P1-06, P1-12, P2-03 | A (A′ for near-singular smoother cases); D (recovery) | Draft | DR-C10, DR-C6, DR-D16, DR-D17, DR-D18 |
| [cross-league-priors.md](cross-league-priors.md) | Feeder → NFL equivalency and priors (engine-spec §6.5) | `grid/priors.py` | `models` (`priors`) | P1-04, P1-07, P1-12 | A′ (fit; A declared for the one-regressor closed form); A (assembly); D (recovery) | Draft | DR-C9, DR-D19, DR-C6 |
| [synthetic-world.md](synthetic-world.md) | Planted-truth generator and the recovery-gate contract | `grid/synth.py`, `grid/data_adapters.py` (`load_synthetic`) | `synth` (DR-A8 target crate) | P1-01 (loader), P1-05, P1-12; the stat-vector world before P1-07 and P1-08 | Fixtures load exactly; draw-tape replay exact (proposed — DR-D28); D over a seed ensemble (proposed — DR-D26) | Draft | DR-B4, DR-B5, DR-D26, DR-D28 |
| [projection-stack.md](projection-stack.md) | Layers A–F as a stack, volume and rates, talent features, SV→points map, preseason assembly, ensemble, distributions, scoring | `projection/*`; `scoring/{engine,columns,formats}.py` | `models`, `features`, `simulation`, `scoring` | P1-06 (scoring), P1-07, P1-08 | A or A′ on the oracle's cases (scoring presets exact); spec-defined goldens | Draft | DR-C2, DR-C3, DR-C4, DR-C5, DR-D21, DR-D25 |
| [evaluation-and-leakage.md](evaluation-and-leakage.md) | As-of information model, leakage guards, walk-forward backtest, baselines, metrics, player pool, bootstrap, diagnostics, gates | `validation/*` | `domain` (`AsOf`), `evaluation`, `governance` (thresholds) | P1-05, P1-09 | A; exact (membership); behavioural | Draft | DR-C1, DR-C5, DR-C6, DR-C11, DR-C12, DR-D24 |

The authoritative parity view per oracle module is engine-spec §7.12.7; the spec index with default
classes is engine-spec §6.8. The oracle-module-to-spec map is also in `reference/python/README.md`.
Where this table and a spec differ, the spec and the register win.

## Planned specs with no oracle counterpart

The oracle has nothing for these layers (`projection-stack.md` §2.1), so they are new work with
spec-defined goldens rather than parity targets. `projection-stack.md` fixes their place in the
stack today. Their dedicated specs are created by the work packages that build them; no file exists
yet.

| Layer | Component | Target crate | Created by |
|---|---|---|---|
| A | Availability and role eligibility (`p_active`, `p_start`, snap multiplier, limited role) | `models` | P1-07 |
| B | Team game environment: a joint team/game distribution of plays, drives, pass and rush attempts, scoring and script; GRID supplies the market-anchored team net strength | `models` | P1-07 |
| E | Matchup and context adjustment: opponent, venue, rest, weather and game script; GRID supplies the opponent defensive effect `E_def` | `models` | P1-08 |
| F | Correlated game, team and player simulation; distributions | `simulation` | P1-08 |
