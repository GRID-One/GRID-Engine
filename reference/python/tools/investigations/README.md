# Investigation scripts (reference oracle)

These scripts are the executable evidence behind the defects and decisions recorded during
the 2026-10-01 consolidation inventory (`docs/06-sessions/2026-10-01-consolidation-inventory/`).
They were written in that session's scratchpad and are committed here so the evidence
survives the session (critic report G-2). Each one is a repro, not a test. Nothing here is
collected by pytest (`testpaths = tests`) or imported by the oracle.

Defect IDs (`KI-*`) are in `docs/00-meta/known-issues.md`. Decision IDs (`DR-*`) are in
`docs/00-meta/decision-register.md`. B and C decisions there are **proposed defaults pending
the owner**, so a script listed as "input to DR-xx" informs a decision. It does not settle one.

## How to run

From `reference/python`, in the oracle venv (see `../../README.md`):

```bash
python3 -m tools.investigations.<script> [--synth legacy|fixed]
```

`python3 tools/investigations/<script>.py` works from any directory too. Either way, every
script does three things first (`_common.bootstrap`):

- it puts `reference/python` first on `sys.path`, so `backend` is this oracle;
- it changes to `reference/python`, so the oracle's cwd-relative writes (`data/logs/`,
  `data/health.json`) land in the gitignored `data/` there;
- it sets `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS` and `MKL_NUM_THREADS` to 1 unless they
  are already set.

The scripts that touch the synthetic world print a banner that names the imported `backend`
and the generator variant.

The "Expected output" block in each script's docstring is the output recorded on
2026-10-01 with the `requirements.lock` pins on Linux x86_64 (Python 3.11.15).

## Legacy vs defender-fixed synthetic generator

The oracle's generator draws every play's "defenders" from the **offense's own** DEF
players (`backend/grid/synth.py:193`; KI-NEW-Y0; critic G-1). Scripts that use it take
`--synth`:

- `legacy` (the default) is the oracle as imported. Most inventory evidence was recorded
  on it.
- `fixed` draws defenders from `def_team`. `_common.apply_defender_fix` builds it **in
  memory, in this process only**: it reads the oracle's `synth.py`, substitutes the one
  line, compiles the patched `simulate`, and swaps its code object into the oracle's
  function. `backend/grid/synth.py` on disk is never modified. The fixed generator keeps
  the legacy planted `team_strength` (off − def). A corrected planted net strength is a
  separate decision, DR-B4 / DR-B5, still pending.

`pytest_defender_fix.py` applies the same patch inside a pytest run, so the oracle's own
gates can be re-run on the fixed generator.

## Real data

`fetch_realdata.py` downloads the five nflverse 2023 release assets the real-data scripts
used into `reference/python/data/realdata/`. Network is needed, about 27 MB. It verifies
each file's full sha256 against the pinned value and refuses a mismatch. These files are
**never committed**: `*.parquet` and `/data/` are ignored. Participation data is CC-BY-SA
4.0 ("FTN Data via nflverse" for 2023); see `docs/04-providers/nflverse/access-and-license.md`
and DR-A11. Real-data numbers are **historical, non-parity** evidence. The ingest itself is
biased (KI-NEW-I1..I5), and no real-data value is a Rust parity target.

## Index

| Script | Demonstrates | Evidence for | Generator / data | Runtime (threads=1) |
|---|---|---|---|---|
| `_common.py` | Shared helpers: path/cwd/thread bootstrap, in-memory defender fix, realdata paths | KI-NEW-Y0 (mechanism) | n/a | n/a |
| `fetch_realdata.py` | Downloads the pinned nflverse 2023 inputs and verifies sha256 | inputs of the three real-data scripts | network | about 3 s |
| `tier0_legacy_vs_fixed.py` | Every Tier-0 gate value on legacy vs fixed (critic G-1 table) | KI-NEW-Y0; DR-B3, DR-B4 | both (two subprocesses) | about 16 s |
| `pytest_defender_fix.py` | pytest plugin: the oracle's gates on the fixed generator; golden master 4 fail, calibration 1 fail | KI-NEW-Y0; DR-B1, DR-B4 | fixed | about 21 s |
| `sign_exp.py` | Which planted team convention the intercepts and realized margins track | KI-NEW-Y0, KI-NEW-A1, KI-G1; DR-B4, DR-B5 | legacy (recorded); fixed | about 4 s |
| `sign_exp2.py` | Net-strength anchor, player recovery, defender-team share | KI-NEW-Y0, KI-NEW-A1, KI-G1; DR-B4, DR-B5 | both recorded | about 4 s each |
| `def_sign.py` | Matchup-grade sign vs planted DEF quality and points allowed | KI-NEW-A2, KI-NEW-Y0; DR-B5 | both recorded | about 4 s each |
| `defsign_planted.py` | Matchup-grade sign on a planted-defence league: the ELITE defence gets the lowest grade | KI-NEW-A2; DR-B5 | own generator (valid regardless of KI-NEW-Y0) | about 3 s |
| `sign_check.py` | Market-row variants and gauge-invariant team effects (E_off / E_def) | KI-G1, KI-V1, KI-NEW-A1, KI-NEW-A2; DR-B5 | legacy (recorded); fixed | about 4 s |
| `sign_test.py` | Market-row `[+1,+1]` vs `[+1,−1]` vs none, 3 seeds | KI-G1, KI-V1, KI-NEW-A1; DR-B5 | legacy (recorded; moot per critic X-2/X-19); fixed | about 6 s |
| `sign_test2.py` | Team conventions A (off+def) and B (off−def) × market sign, 5 seeds | KI-G1, KI-V1, KI-NEW-A1; DR-B5 | legacy (recorded; moot per critic X-2/X-19); fixed | about 10 s |
| `defgrade_test.py` | Tier-2 KPI on engine-produced intercept grades, both signs | KI-NEW-V2; context KI-NEW-A2 | legacy (recorded); fixed | about 11 s |
| `defgrade_test2.py` | Tier-2 KPI on a total-defensive-effect grade | KI-NEW-V2; DR-B5 | legacy (recorded); fixed | about 13 s |
| `kparity.py` | Batch `kalman_two_component` vs incremental `kalman_step` diverge after an intervention | KI-NEW-S1; DR-C10 | none | about 2 s |
| `psd.py` | RTS smoothed covariances stay PSD across 3,000 random stress runs (not reproduced) | KI-G4; DR-B6 | none | about 6 s |
| `rollover_test.py` | `weekly_update` skips 2025 week 1 after 2024 week 18 | KI-NEW-W1; DR-C10 | none (hand-built league) | about 2 s |
| `reinit_test.py` | One added player reinitialises the RAPM accumulators | KI-NEW-W2; DR-C10 | none (hand-built league) | about 2 s |
| `cmp_stats.py` | Oracle box-score ingest vs official 2023 player stats | KI-NEW-I1..I5; DR-C12 | real 2023 | about 14 s |
| `real_contract.py` | Real 2023 plays contract: postseason rows, drive-result vocabulary, participation shape | KI-NEW-V0a, KI-NEW-V0b; DR-C1 | real 2023 | about 13 s |
| `real_rapm.py` | Real single-season RAPM scale (SD about 0.04) vs synthetic; QB vs team-offense intercept | KI-NEW-A3, KI-NEW-P4; DR-C1, DR-B4 | real 2023 + legacy synth | about 36 s |

Defender-fixed run of the oracle's own gates:

```bash
python3 -m pytest -p tools.investigations.pytest_defender_fix \
    tests/grid/test_tier0_recovery.py tests/grid/test_determinism.py \
    tests/grid/test_golden_master.py tests/grid/test_calibration_synth.py
# expected: 5 failed, 22 passed (exit 1) -- the four golden Layer B/C tests and
# test_pooled_nis_bounded (pooled NIS 1.829 vs band [0.8, 1.4])
```

## Provenance

Origin of each script in the inventory scratchpad, as named in its own docstring:

- `scratch-cnissues/`: cmp_stats, real_contract, real_rapm, rollover_test, reinit_test,
  kparity, psd, sign_test, sign_test2, defsign_planted, defgrade_test, defgrade_test2.
- `scratch-reconcile-code-first/`: sign_check.
- `scratch-specfirst/`: sign_exp, sign_exp2, def_sign.
- `scratch-critic/`: t0.py plus the patched `cnfix/` copy. This became
  `tier0_legacy_vs_fixed.py` and `pytest_defender_fix.py`.

Logic is unchanged apart from these edits:

- **Path bootstrap:** every script now sets up its paths, cwd and threads first.
- **`--synth` switch:** added to the synthetic-world scripts. It replaces the patched
  copies (`cnfix/`), which were whole duplicate trees.
- **Real-data paths:** these now point to `data/realdata/`.
- **Clean-up:** temporary directories and SQLite connections are now closed.
- **Unused locals:** a few were dropped, and each script's docstring names them.

Some inventory scratch is not carried here:

- `tier0_print.py` with `patched/`, the G1 `[+1,−1]` Tier-0 measurement. Critic X-19 rules
  those numbers moot.
- `tier0_vals.py` / `tier0_values.py`, which are superseded by `tier0_legacy_vs_fixed.py`.
- `pytest_engine.txt`, a run log.
