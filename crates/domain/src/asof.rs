//! The `AsOf` information model (engine-spec §4.5; `docs/05-model-specs/evaluation-and-leakage.md`
//! §4.1; DR-C1).
//!
//! Every historical training example is reconstructed as it would have existed before the lock
//! that applies to the player and game. Two axes govern usability:
//!
//! - **temporal**: a record's event slot `(season, week)` against the as-of slot, compared
//!   lexicographically (a week-only cutoff admits future-season low weeks: KI-NEW-Z61);
//! - **publication lag**: a record's publication time against the lock instant. Having
//!   *occurred* before the lock is not enough; it must have been *published* before the lock.
//!
//! The engine rule is stricter than the oracle's (`asof.py`): lock-timestamp granularity instead
//! of week granularity, plus the publication-lag axis. Where they differ the engine rule governs
//! and the oracle case is a recorded divergence (engine-spec §4.5). The oracle's week-only
//! fallback for a frame without a season is **not** implemented: a `Season` is required by every
//! record type here, so a season-less frame cannot be built, and the frame reader that meets a
//! source without a season column raises `ContractError::MissingSeasonKey`
//! (`evaluation-and-leakage.md` §5.4; the variant is added by P1-05). It is never a week
//! comparison.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{LeakageError, LeakageKind};
use crate::ids::{Season, Slot, Week};
use crate::time::UtcInstant;

/// Which lock a projection is made at (engine-spec §7.2; engine-output-contract §2 `lock.kind`).
///
/// `Thursday` and `Sunday` are the benchmark locks; `Operational` is any other projection time.
/// An operational projection is a separate prediction version and cannot replace a locked
/// snapshot. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockKind {
    /// The Thursday benchmark lock.
    Thursday,
    /// The Sunday benchmark lock.
    Sunday,
    /// An operational (non-benchmark) projection time.
    Operational,
}

impl fmt::Display for LockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LockKind::Thursday => "Thursday",
            LockKind::Sunday => "Sunday",
            LockKind::Operational => "Operational",
        })
    }
}

/// A lock: its kind and the instant it applies (engine-spec §7.2).
///
/// The field is named `at` (P1-01 fixes the name; engine-output-contract §2 wrote `lock_at`).
/// Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lock {
    /// Which lock this is.
    pub kind: LockKind,
    /// The lock instant. A datum is usable only if published strictly before it.
    pub at: UtcInstant,
}

/// The mode an as-of view runs in (`evaluation-and-leakage.md` §4.1).
///
/// `Backtest` and `Production` apply the full rule set. `Research` MAY relax the publication-lag
/// rule (for example to reproduce the oracle's in-season RAPM); every output of such a run carries
/// a [`ResearchFlag`] naming the relaxed rule and MUST NOT feed a gate, a promotion or a published
/// claim. The relaxation is the caller's explicit choice: the methods on [`AsOf`] apply the strict
/// rule in every mode. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AsOfMode {
    /// A rolling-origin backtest origin.
    Backtest,
    /// A live production projection.
    Production,
    /// A research run whose outputs are labelled research-only.
    Research,
}

impl fmt::Display for AsOfMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AsOfMode::Backtest => "Backtest",
            AsOfMode::Production => "Production",
            AsOfMode::Research => "Research",
        })
    }
}

/// The as-of point of a projection or a backtest origin.
///
/// Semantics (`evaluation-and-leakage.md` §4.1, with `S = season`, `W = week`):
///
/// ```text
/// outcome_usable(r)  ⇔ (season_r, week_r) ≤_lex (S, W − 1)  ∧  published_at_r < lock.at
/// pregame_usable(r)  ⇔ (season_r, week_r) ≤_lex (S, W)      ∧  published_at_r < lock.at
/// static metadata    ⇔ exempt from the slot rule;  published_at_r < lock.at still applies
/// ```
///
/// `(season_r, week_r) ≤_lex (S, W − 1)` is written here as `slot < (S, W)`, which is the same
/// predicate without a `W − 1` that could underflow at week 1.
///
/// ```
/// use grid_domain::{AsOf, AsOfMode, Lock, LockKind, Season, Slot, UtcInstant, Week};
///
/// let lock_at = UtcInstant::from_unix_seconds(1_000);
/// let asof = AsOf::new(
///     Season(2023),
///     Week::new(5).unwrap(),
///     Lock { kind: LockKind::Sunday, at: lock_at },
///     AsOfMode::Production,
/// );
/// let before_lock = UtcInstant::from_unix_seconds(999);
/// let same_week = Slot::new(Season(2023), Week::new(5).unwrap());
/// let last_week = Slot::new(Season(2023), Week::new(4).unwrap());
///
/// // A completed week's outcomes are usable; the current week's are not.
/// assert!(asof.outcome_usable(last_week, before_lock));
/// assert!(!asof.outcome_usable(same_week, before_lock));
/// // Pre-game information about the current week is usable.
/// assert!(asof.pregame_usable(same_week, before_lock));
/// // Nothing published at or after the lock is usable, whatever its slot.
/// assert!(!asof.pregame_usable(last_week, lock_at));
/// // A watermark that has absorbed the current week is leakage.
/// assert!(asof.check_watermark(same_week).is_err());
/// assert!(asof.check_watermark(last_week).is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AsOf {
    /// The target season `S`.
    pub season: Season,
    /// The target week `W` (never 0).
    pub week: Week,
    /// The lock that applies: its kind and instant.
    pub lock: Lock,
    /// The mode of the run.
    pub mode: AsOfMode,
}

impl AsOf {
    /// Builds an as-of point.
    pub fn new(season: Season, week: Week, lock: Lock, mode: AsOfMode) -> Self {
        Self {
            season,
            week,
            lock,
            mode,
        }
    }

    /// The target slot `(S, W)`.
    pub fn slot(&self) -> Slot {
        Slot::new(self.season, self.week)
    }

    /// Whether a record was published strictly before the lock. This is the publication-lag
    /// axis alone; every other predicate here includes it.
    pub fn published_before_lock(&self, published_at: UtcInstant) -> bool {
        published_at < self.lock.at
    }

    /// Whether an **outcome** record (a completed play, a stat line, a game result) with event
    /// slot `slot` and publication time `published_at` is usable at this as-of point:
    /// `slot < (S, W)` lexicographically **and** `published_at < lock.at`.
    pub fn outcome_usable(&self, slot: Slot, published_at: UtcInstant) -> bool {
        slot < self.slot() && self.published_before_lock(published_at)
    }

    /// Whether a **pre-game** record (a market quote, an availability report, a depth chart, an
    /// intervention) with event slot `slot` and publication time `published_at` is usable:
    /// `slot <= (S, W)` lexicographically **and** `published_at < lock.at`.
    pub fn pregame_usable(&self, slot: Slot, published_at: UtcInstant) -> bool {
        slot <= self.slot() && self.published_before_lock(published_at)
    }

    /// Whether **static metadata** (a roster biography, a schedule row, a provider manifest) is
    /// usable. Static metadata is exempt from the slot rule because it has no event slot, but it
    /// is **not** exempt from the publication rule: `published_at < lock.at` still applies
    /// (`evaluation-and-leakage.md` §4.1; superseded alpha-spec §2.4).
    pub fn static_metadata_usable(&self, published_at: UtcInstant) -> bool {
        self.published_before_lock(published_at)
    }

    /// Checks an accumulator watermark against this as-of point (engine-spec §4.5 "state whose
    /// watermark exceeds the as-of point"; guard G2 of `evaluation-and-leakage.md` §4.2).
    ///
    /// `last_absorbed` is the last slot whose outcomes the state has absorbed. It must be strictly
    /// before `(S, W)`: a state that has absorbed the current week's outcomes, or a later slot,
    /// has leaked. Watermarks are `(season, week)` tuples and are checked across season boundaries;
    /// the oracle checks within a season only (KI-V2) and keys by week only (KI-NEW-W1).
    pub fn check_watermark(&self, last_absorbed: Slot) -> Result<(), LeakageError> {
        if last_absorbed >= self.slot() {
            return Err(LeakageError {
                kind: LeakageKind::Watermark,
                detail: format!(
                    "state watermark {last_absorbed} is not before the as-of slot {}",
                    self.slot()
                ),
            });
        }
        Ok(())
    }

    /// Whether this is a research-mode view, whose outputs must carry a [`ResearchFlag`].
    pub fn is_research(&self) -> bool {
        self.mode == AsOfMode::Research
    }
}

impl fmt::Display for AsOf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "as of {} at {} lock {} [{}]",
            self.slot(),
            self.lock.kind,
            self.lock.at,
            self.mode
        )
    }
}

/// The research-only marker every output of a `Research`-mode run carries
/// (`evaluation-and-leakage.md` §4.1).
///
/// `research_only = true` names, in `relaxed_rule`, the rule the run relaxed (for example
/// `"publication-lag: in-season participation admitted"`). Such an output MUST NOT feed a gate, a
/// promotion or a published claim. A `Backtest` or `Production` output carries
/// `research_only = false` and `relaxed_rule = None`. `relaxed_rule` is `None` exactly when
/// `research_only` is `false`; [`ResearchFlag::is_consistent`] checks that.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResearchFlag {
    /// True when the producing run was in `Research` mode.
    pub research_only: bool,
    /// The relaxed rule, named. `None` exactly when `research_only` is false.
    pub relaxed_rule: Option<String>,
}

impl ResearchFlag {
    /// The flag of a `Backtest` or `Production` output: nothing relaxed.
    pub fn none() -> Self {
        Self {
            research_only: false,
            relaxed_rule: None,
        }
    }

    /// The flag of a `Research` output that relaxed the named rule.
    pub fn relaxed(rule: impl Into<String>) -> Self {
        Self {
            research_only: true,
            relaxed_rule: Some(rule.into()),
        }
    }

    /// Whether `relaxed_rule` is present exactly when `research_only` is set.
    pub fn is_consistent(&self) -> bool {
        self.research_only == self.relaxed_rule.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn week(w: u8) -> Week {
        Week::new(w).expect("non-zero week")
    }

    fn asof_2023_w5(mode: AsOfMode) -> AsOf {
        AsOf::new(
            Season(2023),
            week(5),
            Lock {
                kind: LockKind::Sunday,
                at: UtcInstant::from_unix_seconds(1_000),
            },
            mode,
        )
    }

    #[test]
    fn record_published_after_the_lock_is_rejected() {
        let asof = asof_2023_w5(AsOfMode::Production);
        let old_slot = Slot::new(Season(2022), week(3));
        let at_lock = UtcInstant::from_unix_seconds(1_000);
        let after_lock = UtcInstant::from_unix_seconds(1_001);
        // Occurring long before the lock is not enough; publication must precede the lock.
        assert!(!asof.outcome_usable(old_slot, after_lock));
        assert!(!asof.pregame_usable(old_slot, after_lock));
        assert!(!asof.outcome_usable(old_slot, at_lock));
        assert!(!asof.pregame_usable(old_slot, at_lock));
        assert!(!asof.static_metadata_usable(at_lock));
        assert!(!asof.static_metadata_usable(after_lock));
    }

    #[test]
    fn same_week_pregame_record_is_pregame_usable_but_not_outcome_usable() {
        let asof = asof_2023_w5(AsOfMode::Production);
        let same_week = Slot::new(Season(2023), week(5));
        let before_lock = UtcInstant::from_unix_seconds(999);
        assert!(asof.pregame_usable(same_week, before_lock));
        assert!(!asof.outcome_usable(same_week, before_lock));
    }

    #[test]
    fn previous_season_high_week_record_is_usable_on_both_axes() {
        let asof = asof_2023_w5(AsOfMode::Backtest);
        // Week 18 > week 5 numerically; a week-only cutoff (KI-NEW-Z61) would reject it.
        let prior_season_late = Slot::new(Season(2022), week(18));
        let before_lock = UtcInstant::from_unix_seconds(0);
        assert!(asof.outcome_usable(prior_season_late, before_lock));
        assert!(asof.pregame_usable(prior_season_late, before_lock));
    }

    #[test]
    fn future_season_low_week_record_is_rejected() {
        let asof = asof_2023_w5(AsOfMode::Backtest);
        let next_season_early = Slot::new(Season(2024), week(1));
        let before_lock = UtcInstant::from_unix_seconds(0);
        assert!(!asof.outcome_usable(next_season_early, before_lock));
        assert!(!asof.pregame_usable(next_season_early, before_lock));
    }

    #[test]
    fn static_metadata_is_exempt_from_the_slot_rule_only() {
        let asof = asof_2023_w5(AsOfMode::Production);
        assert!(asof.static_metadata_usable(UtcInstant::from_unix_seconds(999)));
        assert!(!asof.static_metadata_usable(UtcInstant::from_unix_seconds(1_000)));
    }

    #[test]
    fn watermark_at_the_asof_slot_is_a_typed_leakage_error() {
        let asof = asof_2023_w5(AsOfMode::Production);
        let err = asof
            .check_watermark(Slot::new(Season(2023), week(5)))
            .expect_err("a watermark at (S, W) has absorbed the target week");
        assert_eq!(err.kind, LeakageKind::Watermark);
        assert!(err.detail.contains("(2023, 5)"));
        let later = asof
            .check_watermark(Slot::new(Season(2024), week(1)))
            .expect_err("a later season is leakage too");
        assert_eq!(later.kind, LeakageKind::Watermark);
        assert!(asof
            .check_watermark(Slot::new(Season(2023), week(4)))
            .is_ok());
        assert!(asof
            .check_watermark(Slot::new(Season(2022), week(18)))
            .is_ok());
    }

    #[test]
    fn research_mode_is_reported_and_flagged() {
        assert!(asof_2023_w5(AsOfMode::Research).is_research());
        assert!(!asof_2023_w5(AsOfMode::Production).is_research());
        assert!(!asof_2023_w5(AsOfMode::Backtest).is_research());
        let flag = ResearchFlag::relaxed("publication-lag: in-season participation admitted");
        assert!(flag.research_only && flag.is_consistent());
        assert!(ResearchFlag::none().is_consistent());
        let inconsistent = ResearchFlag {
            research_only: true,
            relaxed_rule: None,
        };
        assert!(!inconsistent.is_consistent());
    }

    #[test]
    fn display_names_the_slot_lock_and_mode() {
        let shown = asof_2023_w5(AsOfMode::Backtest).to_string();
        assert_eq!(shown, "as of (2023, 5) at Sunday lock 1000s [Backtest]");
    }
}
