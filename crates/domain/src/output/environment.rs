//! The team-game environment record (engine-output-contract §5; engine-spec §6.1 Layer B).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::asof::ResearchFlag;
use crate::error::{OutputInvariant, OutputInvariantError};
use crate::ids::{GameId, PredictionVersionId, Season, TeamKey, Week};
use crate::output::projection::{check_finite, check_monotone, check_probability};

/// The summary of one environment quantity: `mean`, `sd` and `q10`, `q50`, `q90`
/// (engine-output-contract §5). Units: those of the quantity. Invariants: `sd ≥ 0`,
/// `q10 ≤ q50 ≤ q90`, every value finite. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSummary {
    /// The mean.
    pub mean: f64,
    /// The standard deviation, `≥ 0`.
    pub sd: f64,
    /// The 10th percentile.
    pub q10: f64,
    /// The median.
    pub q50: f64,
    /// The 90th percentile.
    pub q90: f64,
}

impl EnvironmentSummary {
    /// Checks finiteness, `sd ≥ 0` and quantile monotonicity, naming fields under `path`.
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        check_finite(
            [
                ("mean", self.mean),
                ("sd", self.sd),
                ("q10", self.q10),
                ("q50", self.q50),
                ("q90", self.q90),
            ]
            .into_iter(),
            path,
        )?;
        if self.sd < 0.0 {
            return Err(OutputInvariantError::new(
                OutputInvariant::NegativeSd,
                format!("{path}.sd"),
                format!("sd = {}", self.sd),
            ));
        }
        check_monotone(
            &[("q10", self.q10), ("q50", self.q50), ("q90", self.q90)],
            path,
        )
    }
}

/// The kind of a touchdown in the environment record (engine-output-contract §5 "touchdowns by
/// type"). **Open** enum (engine-spec §5.5 "Compatibility": consumers MUST tolerate unknown
/// values; engine-output-contract §9: a new kind is a MINOR change). Serialized as the raw label
/// (`Passing`, `Rushing`, `Receiving`, or the label of any other kind) and parsed back
/// case-sensitively, so a label this version does not name, such as a defensive or special-teams
/// kind added later, is kept verbatim as `Other(label)` instead of being rejected.
///
/// A reader never produces `Other` with a known label: deserialization (`From<String>`) maps a
/// known label to its variant. `Other("Passing")` can still be built by hand; it serializes to the
/// same text `"Passing"` as `Passing`, and therefore deserializes to `Passing`. Equality after a
/// serialization round trip holds only for values a reader produced. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum TouchdownType {
    /// A passing touchdown.
    Passing,
    /// A rushing touchdown.
    Rushing,
    /// A receiving touchdown.
    Receiving,
    /// Any other kind (defensive, special teams, or one named by a later contract version), with
    /// its label kept verbatim.
    Other(String),
}

impl TouchdownType {
    /// The raw label.
    pub fn as_str(&self) -> &str {
        match self {
            TouchdownType::Passing => "Passing",
            TouchdownType::Rushing => "Rushing",
            TouchdownType::Receiving => "Receiving",
            TouchdownType::Other(raw) => raw,
        }
    }
}

impl From<String> for TouchdownType {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "Passing" => TouchdownType::Passing,
            "Rushing" => TouchdownType::Rushing,
            "Receiving" => TouchdownType::Receiving,
            _ => TouchdownType::Other(raw),
        }
    }
}

impl From<&str> for TouchdownType {
    fn from(raw: &str) -> Self {
        Self::from(raw.to_owned())
    }
}

impl From<TouchdownType> for String {
    fn from(kind: TouchdownType) -> Self {
        kind.as_str().to_owned()
    }
}

impl fmt::Display for TouchdownType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The Layer-B team-game environment, one per team × game per prediction version
/// (engine-output-contract §5). Units: counts per game unless stated. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGameEnvironment {
    /// The prediction version.
    pub prediction_version_id: PredictionVersionId,
    /// The season.
    pub season: Season,
    /// The week (never 0).
    pub week: Week,
    /// The game.
    pub game_id: GameId,
    /// The team.
    pub team: TeamKey,
    /// The opponent.
    pub opponent: TeamKey,
    /// Whether `team` is at home.
    pub is_home: bool,
    /// Expected offensive plays.
    pub offensive_plays: EnvironmentSummary,
    /// Expected drives.
    pub drives: EnvironmentSummary,
    /// Expected pass attempts.
    pub pass_attempts: EnvironmentSummary,
    /// Expected rush attempts.
    pub rush_attempts: EnvironmentSummary,
    /// Expected sacks taken.
    pub sacks: EnvironmentSummary,
    /// Expected touchdowns by type.
    pub touchdowns_by_type: Vec<(TouchdownType, EnvironmentSummary)>,
    /// Expected red-zone opportunities.
    pub red_zone_opportunities: EnvironmentSummary,
    /// Pace. Units: seconds per play.
    pub pace: EnvironmentSummary,
    /// Neutral pass tendency. Units: fraction of neutral-script plays that are passes, in
    /// `[0, 1]`.
    pub neutral_pass_tendency: EnvironmentSummary,
    /// Expected game script. Units: points, `team − opponent`.
    pub game_script: EnvironmentSummary,
    /// Implied points for `team`. Units: points.
    pub implied_points: EnvironmentSummary,
    /// The shared latent ids coupling this record's draws with the opponent's.
    pub shared_latent_ids: Vec<String>,
    /// The research-only marker of the producing run (evaluation-and-leakage.md §4.1): the same
    /// flag as the [`ProjectionLineage`](crate::output::ProjectionLineage) of
    /// `prediction_version_id`. Never null.
    pub research: ResearchFlag,
}

impl TeamGameEnvironment {
    /// Checks every summary's invariants ([`EnvironmentSummary::validate`]: finiteness, `sd ≥ 0`,
    /// monotone quantiles), naming fields under `environment`, and then the range
    /// `neutral_pass_tendency` carries as a share (engine-spec §5.5 "Field rules": probabilities
    /// and shares lie in `[0, 1]`): its `mean`, `q10`, `q50` and `q90` must lie in `[0, 1]`
    /// (`ProbabilityOutOfRange`). No other summary has a range rule here: counts, pace, game
    /// script and implied points are unbounded quantities on this record.
    pub fn validate(&self) -> Result<(), OutputInvariantError> {
        for (name, s) in [
            ("offensive_plays", &self.offensive_plays),
            ("drives", &self.drives),
            ("pass_attempts", &self.pass_attempts),
            ("rush_attempts", &self.rush_attempts),
            ("sacks", &self.sacks),
            ("red_zone_opportunities", &self.red_zone_opportunities),
            ("pace", &self.pace),
            ("neutral_pass_tendency", &self.neutral_pass_tendency),
            ("game_script", &self.game_script),
            ("implied_points", &self.implied_points),
        ] {
            s.validate(&format!("environment.{name}"))?;
        }
        for (kind, s) in &self.touchdowns_by_type {
            s.validate(&format!("environment.touchdowns_by_type[{kind}]"))?;
        }
        let t = &self.neutral_pass_tendency;
        for (field, v) in [
            ("mean", t.mean),
            ("q10", t.q10),
            ("q50", t.q50),
            ("q90", t.q90),
        ] {
            check_probability(v, &format!("environment.neutral_pass_tendency.{field}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(mean: f64) -> EnvironmentSummary {
        EnvironmentSummary {
            mean,
            sd: mean.abs() * 0.1,
            q10: mean * 0.8,
            q50: mean,
            q90: mean * 1.2,
        }
    }

    fn env() -> TeamGameEnvironment {
        TeamGameEnvironment {
            prediction_version_id: PredictionVersionId(7),
            season: Season(2023),
            week: Week::new(5).unwrap(),
            game_id: GameId("2099_05_KC_MIN".into()),
            team: TeamKey::Nfl("KC".into()),
            opponent: TeamKey::Nfl("MIN".into()),
            is_home: false,
            offensive_plays: summary(64.0),
            drives: summary(11.0),
            pass_attempts: summary(36.0),
            rush_attempts: summary(26.0),
            sacks: summary(2.0),
            touchdowns_by_type: vec![
                (TouchdownType::Passing, summary(1.9)),
                (TouchdownType::Rushing, summary(1.1)),
            ],
            red_zone_opportunities: summary(3.5),
            pace: summary(27.0),
            neutral_pass_tendency: summary(0.58),
            game_script: summary(3.5),
            implied_points: summary(26.5),
            shared_latent_ids: vec!["game-2099_05_KC_MIN".into()],
            research: ResearchFlag::none(),
        }
    }

    #[test]
    fn touchdown_type_is_open_and_keeps_unknown_labels() {
        // The serde form goes through `From<String>` / `Into<String>`, so these conversions are
        // what a reader and a writer see.
        assert_eq!(TouchdownType::from("Passing"), TouchdownType::Passing);
        assert_eq!(TouchdownType::from("Receiving"), TouchdownType::Receiving);
        assert_eq!(
            TouchdownType::from("Defensive"),
            TouchdownType::Other("Defensive".into())
        );
        assert_eq!(String::from(TouchdownType::Rushing), "Rushing");
        assert_eq!(
            String::from(TouchdownType::Other("SpecialTeams".into())),
            "SpecialTeams"
        );
        assert_eq!(
            TouchdownType::Other("Defensive".into()).to_string(),
            "Defensive"
        );
        // Case-sensitive: a differently cased label is not silently normalized.
        assert_eq!(
            TouchdownType::from("passing"),
            TouchdownType::Other("passing".into())
        );
    }

    #[test]
    fn a_hand_built_other_with_a_known_label_reads_back_as_the_known_variant() {
        // The serde form goes through `Into<String>` then `From<String>`, so this is the round
        // trip a writer and a reader perform.
        let hand_built = TouchdownType::Other("Passing".into());
        let text = String::from(hand_built.clone());
        assert_eq!(text, String::from(TouchdownType::Passing));
        let read_back = TouchdownType::from(text);
        assert_eq!(read_back, TouchdownType::Passing);
        assert_ne!(read_back, hand_built);
        // A value a reader produced survives the round trip unchanged.
        for label in ["Passing", "Rushing", "Receiving", "Defensive"] {
            let read = TouchdownType::from(label);
            assert_eq!(TouchdownType::from(String::from(read.clone())), read);
        }
    }

    #[test]
    fn environment_validates_each_summary() {
        assert_eq!(env().validate(), Ok(()));
        let mut e = env();
        e.touchdowns_by_type[1].1.q90 = e.touchdowns_by_type[1].1.q50 - 1.0;
        let err = e.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::QuantilesNotMonotone);
        assert_eq!(err.field, "environment.touchdowns_by_type[Rushing].q90");
        let mut e = env();
        e.pace.sd = -1.0;
        let err = e.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NegativeSd);
        assert_eq!(err.field, "environment.pace.sd");
        let mut e = env();
        e.implied_points.mean = f64::NAN;
        let err = e.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
    }

    #[test]
    fn neutral_pass_tendency_is_a_share_in_unit_interval() {
        // A share above 1 is rejected (engine-spec §5.5: shares lie in [0, 1]) …
        let mut e = env();
        e.neutral_pass_tendency = summary(1.3);
        let err = e.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "environment.neutral_pass_tendency.mean");
        // … so is a negative quantile …
        let mut e = env();
        e.neutral_pass_tendency.q10 = -0.05;
        let err = e.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "environment.neutral_pass_tendency.q10");
        // … while the same values on an unbounded quantity are fine.
        let mut e = env();
        e.implied_points = summary(1.3);
        assert_eq!(e.validate(), Ok(()));
    }
}
