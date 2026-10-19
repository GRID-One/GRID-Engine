//! The projection change log (engine-output-contract §6; engine-spec §5.6, §6.7 item 9).

use serde::{Deserialize, Serialize};

use crate::asof::ResearchFlag;
use crate::ids::{PlayerKey, PredictionVersionId, ScoringProfileRef, Season, Week};
use crate::output::projection::SettingVersion;

/// The attribution of a projection change to **exactly five categories**
/// (engine-output-contract §6). Units: expected fantasy points of the record's scoring profile.
/// How the Δ is decomposed, and how any residual or interaction is reported, is open (DR-D8,
/// P2-07): the method is identified by [`ProjectionChange::attribution_method`]. Attribution is
/// predictive, not causal. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChangeAttribution {
    /// Change attributed to the player's role and opportunity (Layer C).
    pub role: f64,
    /// Change attributed to availability (Layer A).
    pub availability: f64,
    /// Change attributed to the matchup (Layer E).
    pub matchup: f64,
    /// Change attributed to the team environment (Layer B).
    pub team_environment: f64,
    /// Change attributed to a model update (a new model version or state advance).
    pub model_update: f64,
}

/// One row of the change log: a player-week and a consecutive pair of published versions
/// (engine-output-contract §6). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionChange {
    /// The player.
    pub player_id: PlayerKey,
    /// The season.
    pub season: Season,
    /// The week (never 0).
    pub week: Week,
    /// The scoring profile the points are in.
    pub scoring_profile: ScoringProfileRef,
    /// The prior published version.
    pub prior_prediction_version_id: PredictionVersionId,
    /// The current published version.
    pub current_prediction_version_id: PredictionVersionId,
    /// The prior headline (unconditional) expected points. Units: fantasy points of
    /// `scoring_profile`.
    pub prior_points: f64,
    /// The current headline (unconditional) expected points. Units: fantasy points of
    /// `scoring_profile`.
    pub current_points: f64,
    /// `current_points − prior_points`. Units: fantasy points of `scoring_profile`.
    pub delta: f64,
    /// The five-category attribution of `delta`.
    pub attribution: ChangeAttribution,
    /// The version of the attribution method (open value: DR-D8, P2-07).
    pub attribution_method: SettingVersion,
    /// The research-only marker of the run that produced `current_prediction_version_id`
    /// (evaluation-and-leakage.md §4.1): the same flag as that version's
    /// [`ProjectionLineage`](crate::output::ProjectionLineage). Never null.
    pub research: ResearchFlag,
}
