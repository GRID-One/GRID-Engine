//! The explanation record and the GRID signal block (engine-output-contract §4; engine-spec §6.7).
//!
//! Explanations are typed fields; any text is rendered from them by the CLI. Attribution is a
//! predictive association, never a causal claim, unless the logic is rule-based (an `OUT` status
//! gives a zero projection). Δ fields are in expected fantasy points of the record's scoring
//! profile; GRID values are in `dV` (expected drive points per play) and never in fantasy points.
//! A missing contribution is `None` with the reason stated on the field, never `0` (KI-P1).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::OutputInvariantError;
use crate::ids::{GameId, PredictionVersionId, Season, TeamKey};
use crate::output::projection::{check_finite, ComponentSummary};

/// A reference to the team-game environment record of the same prediction version
/// (engine-output-contract §4.1 item 1, §5). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamGameRef {
    /// The prediction version.
    pub prediction_version_id: PredictionVersionId,
    /// The game.
    pub game_id: GameId,
    /// The player's team in that game.
    pub team: TeamKey,
}

/// The units of a driver's contribution (engine-spec §6.7 "Units"). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriverUnits {
    /// Expected fantasy points of the record's scoring profile (a Layer A–F driver).
    ExpectedFantasyPoints,
    /// Expected drive points per play (a GRID covariate driver).
    DvPerPlay,
}

/// One named model driver (engine-output-contract §4.1 items 7 and 8). The contribution is
/// computed by the attribution method each model spec declares (engine-spec §6.8 field 11).
/// Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Driver {
    /// The driver's name, as the model spec declares it (open vocabulary).
    pub name: String,
    /// The signed contribution, in `units`. For an uncertainty driver, the contribution to the
    /// predictive spread.
    pub contribution: f64,
    /// The units of `contribution`.
    pub units: DriverUnits,
}

/// The difference from the prior published projection (engine-output-contract §4.1 item 9).
/// `None` on the record when no prior version exists for the player-week. Never null when
/// present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorVersionDelta {
    /// The prior prediction version.
    pub prior_prediction_version_id: PredictionVersionId,
    /// `current − prior` headline expected points, in fantasy points of the record's profile.
    pub delta_points: f64,
}

/// One role's state-space talent track (engine-output-contract §4.2 `role_talent`; DR-C3).
/// **Filtered or one-step predictive only**; never RTS-smoothed. Units: `dV`. Never null when
/// present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoleTalent {
    /// The filtered (real-time) talent mean.
    pub filtered_mean: f64,
    /// The filtered talent variance, `≥ 0`.
    pub filtered_var: f64,
    /// The one-step predictive variance `S = H·P_pred·Hᵀ + R`, `≥ 0`: the confidence-band
    /// quantity (engine-spec §5.5 "Confidence bands are data").
    pub predictive_var: f64,
}

/// Talent per role (engine-output-contract §4.2; DR-C3). A role the player does not fill is
/// `None` (no track exists), never a zero track. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoleTalentBlock {
    /// The dropback (passing) role.
    pub dropback: Option<RoleTalent>,
    /// The carry (rushing) role.
    pub carry: Option<RoleTalent>,
    /// The target (receiving) role. Whether this credit is an individual signal is open
    /// (DR-D13); until decided, explanations never describe it as individual efficiency.
    pub target: Option<RoleTalent>,
}

/// An offseason RAPM rating with its season and exposure (engine-output-contract §4.2;
/// engine-spec §6.7 "Offseason ratings"; DR-C1). Present only for seasons whose participation
/// was published before the lock. Units: `dV`. Never null when present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OffseasonRapm {
    /// The rating.
    pub rating: f64,
    /// The season the rating was fitted on.
    pub season: Season,
    /// The number of plays behind the rating.
    pub n_plays: u32,
}

/// The season-boundary prior behind the record's GRID values (engine-output-contract §4.2
/// `prior`; engine-spec §6.5). Units: `dV` for `mean` and `var`; `q` and `n0` are the prior's
/// discount and pseudo-count parameters; `weight_now` is the prior's current weight in the
/// posterior, in `[0, 1]`. The source label (window, feeder or population; DR-D17) and the build
/// label (`prior_pre` or `prior_in`; DR-D32) are **open**: the contract version that names their
/// keys adds them as a MINOR change (engine-output-contract §12). Never null when present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriorBlock {
    /// The prior mean.
    pub mean: f64,
    /// The prior variance, `≥ 0`.
    pub var: f64,
    /// The prior's `q` parameter.
    pub q: f64,
    /// The prior's `n0` pseudo-count.
    pub n0: f64,
    /// The prior's current weight in the posterior, in `[0, 1]`.
    pub weight_now: f64,
}

/// Regime flags of the state-space layer (engine-output-contract §4.2; engine-spec §6.7 item 8).
/// Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RegimeFlags {
    /// An intervention (injury return, scheme or QB change) applied this week.
    pub intervention: bool,
    /// Observation-noise inflation ("rust") applied this week.
    pub observation_noise_inflation: bool,
    /// The scheme-fit state was reset this week.
    pub scheme_reset: bool,
    /// The changepoint z-score, when a changepoint test ran; `None` when none ran.
    pub changepoint_z: Option<f64>,
}

/// The GRID signal block (engine-output-contract §4.2; engine-spec §5.5 "GRID signal block";
/// DR-C2, DR-C3). All values are in `dV` currency and are explanation and diagnostic fields,
/// never fantasy points.
///
/// **Filtered and one-step predictive only, by design: there is no smoothed field.** RTS-smoothed
/// values use future weeks and MUST NOT appear in any live or locked projection record
/// (engine-spec §6.7 "Estimate kinds"). Never null when present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSignalBlock {
    /// Talent per role.
    pub role_talent: RoleTalentBlock,
    /// The filtered form state component; `None` when the player has no track yet.
    pub form: Option<f64>,
    /// The filtered scheme-fit state component; `None` when the player has no track yet.
    pub scheme_fit: Option<f64>,
    /// The offseason RAPM rating; `None` until that season's participation is published.
    pub offseason_rapm: Option<OffseasonRapm>,
    /// The prior behind the GRID values; `None` when the player has none.
    pub prior: Option<PriorBlock>,
    /// The regime flags.
    pub regime_flags: RegimeFlags,
}

impl GridSignalBlock {
    /// Checks that every value is finite, naming fields under `path`.
    ///
    /// **Documented but not checked here** (producer obligations): `filtered_var ≥ 0` and
    /// `predictive_var ≥ 0` on every [`RoleTalent`], and `var ≥ 0` and `weight_now` in `[0, 1]`
    /// on the [`PriorBlock`]. The closed `OutputInvariant` enum, fixed by P1-01, has no variant
    /// for a negative variance or an out-of-range prior weight, and adding one is a contract
    /// decision, not an implementer's.
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        for (role, talent) in [
            ("dropback", self.role_talent.dropback),
            ("carry", self.role_talent.carry),
            ("target", self.role_talent.target),
        ] {
            if let Some(t) = talent {
                check_finite(
                    [
                        ("filtered_mean", t.filtered_mean),
                        ("filtered_var", t.filtered_var),
                        ("predictive_var", t.predictive_var),
                    ]
                    .into_iter(),
                    &format!("{path}.role_talent.{role}"),
                )?;
            }
        }
        let mut scalars: Vec<(&str, f64)> = Vec::new();
        if let Some(v) = self.form {
            scalars.push(("form", v));
        }
        if let Some(v) = self.scheme_fit {
            scalars.push(("scheme_fit", v));
        }
        if let Some(r) = self.offseason_rapm {
            scalars.push(("offseason_rapm.rating", r.rating));
        }
        if let Some(p) = self.prior {
            scalars.push(("prior.mean", p.mean));
            scalars.push(("prior.var", p.var));
            scalars.push(("prior.q", p.q));
            scalars.push(("prior.n0", p.n0));
            scalars.push(("prior.weight_now", p.weight_now));
        }
        if let Some(z) = self.regime_flags.changepoint_z {
            scalars.push(("regime_flags.changepoint_z", z));
        }
        check_finite(scalars.into_iter(), path)
    }
}

/// The explanation record of every projection (engine-output-contract §4.1; engine-spec §6.7,
/// the nine fields). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplanationRecord {
    /// 1. Expected team play and scoring environment: a reference to the team-game record.
    pub team_environment: TeamGameRef,
    /// 2. Expected player role and opportunity: Layer-C share means.
    pub role_opportunity: Vec<ComponentSummary>,
    /// 3. Availability adjustment, Δ expected fantasy points; `None` when Layer A had no input
    ///    (`availability_source = Missing`).
    pub availability_adjustment: Option<f64>,
    /// 4. Matchup adjustment, Δ expected fantasy points, from `+E_def` (higher = tougher; DR-B5);
    ///    `None` when no opponent rating exists.
    pub matchup_adjustment: Option<f64>,
    /// 5. NCAA prior contribution: the remaining weight of the feeder base in the state, by
    ///    linearity (DR-D17). Units: dimensionless weight, the share of the filtered talent still
    ///    carried by the feeder base (state-space-kalman.md §3.5; engine-spec §6.7 row 5): `1`
    ///    is the feeder base alone, `0` is fully washed out; **not** fantasy points. Nominally in
    ///    `[0, 1]`; engine-output-contract §3.9 declares no range invariant on it, so
    ///    [`ExplanationRecord::validate`] checks it for finiteness only. `None` when the player
    ///    has no feeder prior.
    pub ncaa_prior_contribution: Option<f64>,
    /// 6. Recent NFL evidence contribution. Units: dimensionless weight, the share of the
    ///    posterior carried by NFL evidence (engine-spec §6.7 row 6: "the evidence share of the
    ///    posterior (§6.5)"); **not** fantasy points. Nominally in `[0, 1]`; as for item 5, no
    ///    range invariant is declared, so it is checked for finiteness only. `None` when the
    ///    player has no NFL evidence.
    pub recent_evidence_contribution: Option<f64>,
    /// 7a. Top positive model drivers.
    pub top_positive_drivers: Vec<Driver>,
    /// 7b. Top negative model drivers.
    pub top_negative_drivers: Vec<Driver>,
    /// 8. Uncertainty drivers.
    pub uncertainty_drivers: Vec<Driver>,
    /// 9. Difference from the prior published projection; `None` when there is no prior version.
    pub delta_from_prior: Option<PriorVersionDelta>,
    /// The GRID signal block; `None` when GRID produced no signal for the player.
    pub grid: Option<GridSignalBlock>,
}

impl ExplanationRecord {
    /// Checks that every numeric field is finite and every role-opportunity summary is valid,
    /// naming fields under `path`. A role-opportunity summary is checked exactly as a stat-vector
    /// component ([`ComponentSummary::validate`]): finiteness, `sd ≥ 0`, monotone quantiles and,
    /// since these are share and probability means (engine-output-contract §4.1 item 2, §3.9),
    /// the `[0, 1]` range with `ShareOverflow` above 1.
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        let mut scalars: Vec<(&str, f64)> = Vec::new();
        for (name, v) in [
            ("availability_adjustment", self.availability_adjustment),
            ("matchup_adjustment", self.matchup_adjustment),
            ("ncaa_prior_contribution", self.ncaa_prior_contribution),
            (
                "recent_evidence_contribution",
                self.recent_evidence_contribution,
            ),
        ] {
            if let Some(v) = v {
                scalars.push((name, v));
            }
        }
        if let Some(d) = &self.delta_from_prior {
            scalars.push(("delta_from_prior.delta_points", d.delta_points));
        }
        check_finite(scalars.into_iter(), path)?;
        for (i, c) in self.role_opportunity.iter().enumerate() {
            c.validate(&format!("{path}.role_opportunity[{i}]"))?;
        }
        for (group, drivers) in [
            ("top_positive_drivers", &self.top_positive_drivers),
            ("top_negative_drivers", &self.top_negative_drivers),
            ("uncertainty_drivers", &self.uncertainty_drivers),
        ] {
            for (i, d) in drivers.iter().enumerate() {
                check_finite(
                    [("contribution", d.contribution)].into_iter(),
                    &format!("{path}.{group}[{i}]"),
                )?;
            }
        }
        if let Some(grid) = &self.grid {
            grid.validate(&format!("{path}.grid"))?;
        }
        Ok(())
    }
}

impl fmt::Display for DriverUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DriverUnits::ExpectedFantasyPoints => "expected fantasy points",
            DriverUnits::DvPerPlay => "dV per play",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::OutputInvariant;
    use crate::output::projection::test_support::explanation;
    use crate::output::projection::{StatComponent, StatDistribution};

    fn share(mean: f64, q90: f64) -> ComponentSummary {
        ComponentSummary {
            component: StatComponent::OffensiveSnapShare,
            distribution: StatDistribution {
                mean,
                sd: 0.05,
                q10: mean.min(q90),
                q25: mean.min(q90),
                q50: mean.min(q90),
                q75: mean.min(q90),
                q90,
            },
        }
    }

    #[test]
    fn role_opportunity_shares_are_range_checked() {
        // A valid share passes.
        let mut record = explanation();
        record.role_opportunity.push(share(0.7, 0.9));
        assert_eq!(record.validate("explanation"), Ok(()));
        // A share above 1 is a share overflow, at the role-opportunity path.
        let mut record = explanation();
        record.role_opportunity.push(share(0.9, 1.05));
        let err = record.validate("explanation").unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ShareOverflow);
        assert_eq!(err.field, "explanation.role_opportunity[0].q90");
        // A negative share is a range violation.
        let mut record = explanation();
        record.role_opportunity.push(share(-0.1, 0.2));
        let err = record.validate("explanation").unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "explanation.role_opportunity[0].mean");
        // A probability component outside [0, 1] is a range violation too.
        let mut record = explanation();
        record.role_opportunity.push(ComponentSummary {
            component: StatComponent::ActiveProbability,
            distribution: StatDistribution {
                mean: -0.2,
                sd: 0.0,
                q10: -0.2,
                q25: -0.2,
                q50: -0.2,
                q75: -0.2,
                q90: -0.2,
            },
        });
        let err = record.validate("explanation").unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "explanation.role_opportunity[0].mean");
    }

    #[test]
    fn grid_block_has_no_smoothed_field_and_validates_finiteness() {
        let mut record = explanation();
        assert_eq!(record.validate("explanation"), Ok(()));
        let grid = record.grid.as_mut().unwrap();
        grid.role_talent.carry = Some(RoleTalent {
            filtered_mean: 0.02,
            filtered_var: 0.001,
            predictive_var: f64::NAN,
        });
        let err = record.validate("explanation").unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(
            err.field,
            "explanation.grid.role_talent.carry.predictive_var"
        );
    }

    #[test]
    fn driver_contributions_must_be_finite() {
        let mut record = explanation();
        record.top_negative_drivers.push(Driver {
            name: "opponent_e_def".into(),
            contribution: f64::INFINITY,
            units: DriverUnits::DvPerPlay,
        });
        let err = record.validate("explanation").unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(
            err.field,
            "explanation.top_negative_drivers[0].contribution"
        );
        assert_eq!(DriverUnits::DvPerPlay.to_string(), "dV per play");
    }
}
