//! The planted truth of a synthetic world (plays-contract §6.1 `gt`, D-13;
//! `docs/05-model-specs/synthetic-world.md` §4.10 G-2 to G-4).
//!
//! The truth bundle is **not** part of the input contract: it exists only for validation and
//! recovery gates. It is a separate type so that estimator code cannot reach `ability` or
//! `is_starter` (plays-contract D-13; the v0 world carried them inside the `players` frame,
//! KI-NEW-Z40). `grid-models` MUST NOT depend on this crate (engine-spec §8.1 rule 3); whether its
//! parity tests may take it as a dev-dependency is an open owner item recorded by P1-01.

use std::collections::BTreeMap;

use grid_domain::{PlayerKey, TeamKey};

/// The corrected planted **net** team strength of synthetic-world.md §4.10 G-2 (DR-B5):
/// `N*(t) = Q_off(t) + Q_def(t)`, exposure-weighted over the team's offensive and defensive plays.
/// Units: ability units (the generating sums of §4.3). Present only on a world whose exporter
/// emits the G-2 keys (ledger entry 1 onwards); never computed here.
#[derive(Debug, Clone, PartialEq)]
pub struct NetStrengthTruth {
    /// `Q_off(t)`: mean over `t`'s offensive plays of the on-field offensive ability sum.
    pub q_off: BTreeMap<TeamKey, f64>,
    /// `Q_def(t)`: mean over `t`'s defensive plays of the on-field defensive ability sum.
    pub q_def: BTreeMap<TeamKey, f64>,
    /// `N*(t) = Q_off(t) + Q_def(t)`.
    pub n_star: BTreeMap<TeamKey, f64>,
}

/// The planted truth of one synthetic world (plays-contract §6.1; synthetic-world.md §4.10).
///
/// Every map is keyed by the domain key and sorted by it, so a listing follows the oracle's
/// column order (`PlayerKey` by string form, `TeamKey` natively). "Missing" is `None`, never a
/// sentinel (KI-P1): a missed game in `focus_tau` is `None` where the v0 bundle wrote NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticTruth {
    /// The static planted ability of every player (v0 `players.ability`). Units: ability units.
    /// For the focus QB this is the static draw, which §4.10 G-4 labels unused as his gate truth
    /// once the effective truth `a_eff` is exported; it is kept here as the world wrote it.
    pub ability: BTreeMap<PlayerKey, f64>,
    /// Whether the player is a starter (v0 `players.is_starter`).
    pub is_starter: BTreeMap<PlayerKey, bool>,
    /// The **legacy** planted team strength, `mean(offense starters) − mean(DEF starters)`
    /// (v0 `gt.team_strength`, `synth.py:285-294`; KI-NEW-Y1). **Semantically void after
    /// correction-ledger entry 1** (synthetic-world.md §4.10 G-2: kept so the unchanged team gate
    /// still runs, labelled void; DR-B5 replaces it by [`NetStrengthTruth`]). Never a gate target
    /// for the Rust engine (§10.2). Units: ability units.
    pub team_strength_legacy: BTreeMap<TeamKey, f64>,
    /// The focus quarterback whose ability varies by week (v0 `gt.focus_qb`).
    pub focus_qb: PlayerKey,
    /// The focus QB's weekly ability `τ_w`, indexed by week − 1 (v0 `gt.focus_tau`, 14 entries on
    /// the canonical world). `None` = a missed game (NaN in v0). Units: ability units.
    pub focus_tau: Vec<Option<f64>>,
    /// The feeder→NFL equivalency factor used to generate `college.feeder_sv` (v0
    /// `gt.league_factor`, 0.62 on the canonical world). Dimensionless.
    pub league_factor: f64,
    /// The corrected net strength of §4.10 G-2. `None` on the legacy world, whose bundle carries
    /// no such keys; present once the exporter emits them (ledger entry 1).
    pub net_strength: Option<NetStrengthTruth>,
}

impl SyntheticTruth {
    /// The focus QB's `τ` over played weeks, in week order (`None` entries skipped).
    pub fn focus_tau_played(&self) -> impl Iterator<Item = f64> + '_ {
        self.focus_tau.iter().filter_map(|t| *t)
    }

    /// The plain mean of `τ` over played weeks (synthetic-world.md §10.4 W-1: 0.15375 on the
    /// canonical trajectory), or `None` when no week was played.
    pub fn focus_tau_mean(&self) -> Option<f64> {
        let mut n = 0usize;
        let mut sum = 0.0f64;
        for t in self.focus_tau_played() {
            n += 1;
            sum += t;
        }
        if n == 0 {
            None
        } else {
            Some(sum / n as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_tau_mean_matches_w1() {
        // synthetic-world.md §10.4 W-1: the canonical focus-QB trajectory, weeks 1–14.
        let tau = [
            0.075,
            0.100,
            0.125,
            0.150,
            0.175,
            0.200,
            0.225,
            f64::NAN,
            f64::NAN,
            0.060,
            0.115,
            0.170,
            0.225,
            0.225,
        ];
        let truth = SyntheticTruth {
            ability: BTreeMap::new(),
            is_starter: BTreeMap::new(),
            team_strength_legacy: BTreeMap::new(),
            focus_qb: PlayerKey::Synthetic(0),
            focus_tau: tau
                .iter()
                .map(|t| if t.is_nan() { None } else { Some(*t) })
                .collect(),
            league_factor: 0.62,
            net_strength: None,
        };
        assert_eq!(truth.focus_tau_played().count(), 12);
        let mean = truth.focus_tau_mean().expect("played weeks");
        assert!((mean - 0.15375).abs() < 1e-15, "{mean}");
        let empty = SyntheticTruth {
            focus_tau: vec![None, None],
            ..truth
        };
        assert_eq!(empty.focus_tau_mean(), None);
    }
}
