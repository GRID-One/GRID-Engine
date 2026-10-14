---
adr-id: 013
status: Proposed
date: 2026-10-14
deciders: Statistical owner (DR-D19)
supersedes:
superseded-by:
---

# ADR-013 — Feeder-equivalency target: the season level, with no seed

## Status

**Proposed** 2026-10-14 under work package P0-01. It is not accepted. The deciding role is the
Statistical owner (DR-D19).

- **What it records.** The Statistical owner's answer 10 in `docs/06-sessions/2026-10-14-P0-01.md`,
  "Season level, no seed (Recommended)", on DR-D19; answer 11 directs its transcription. The option
  was the agent's recommendation; the register had no recommended default for this part. Its
  description, as offered, said how it is recorded: "This changes DR-D19's estimand from talent to
  season level, so it's recorded through a Proposed ADR citing DR-D19 that binds once you accept it
  (e.g. in the merge comment)."
- **When it binds.** Only once the Statistical owner accepts it, for example in the P0-01 merge
  comment. An implementing agent writes ADRs as Proposed and never marks one Accepted
  (`docs/02-adr/README.md`, "Status").
- **Until then.** DR-D19's 2026-10-09 text stands and governs (`docs/00-meta/decision-register.md`,
  DR-D19). No seed fits that text (Context), so if the owner rejects this ADR, the target is a decision
  request to the Statistical owner before P1-12 implements the feeder equivalency.
- **Reverses** DR-D19's 2026-10-09 decision in part (register rule 6; "Reverses" below).
- **Rank.** Once accepted it is authority level 2 (`engine-spec.md` §1.5) and cannot override
  `engine-spec.md`. The documents that implement DR-D19 carry the same rule, marked as binding on this
  ADR's acceptance (`docs/00-meta/decision-register.md`, DR-D19, "Where implemented").
- **What it does not decide.** The items under "Still open" below.

## Context

- **DR-D19 as decided on 2026-10-09.** The owner chose Option 1, "Make the equivalency target the
  same quantity as the Kalman talent state", answering "Fit to tracker talent (Recommended)"
  (`docs/06-sessions/2026-10-09-P0-01-decision-interview.md`, question 42). The register records it
  as: "the feeder translation is fitted directly against the tracker's own talent scale -
  end-of-season (smoothed) talent computed without the college prior, so the prior never trains on
  itself; no separate scale map". Its reason stands: the oracle fits the equivalency against the RAPM
  rating, while the Kalman state is on the weekly Layer-1 credit scale (KI-NEW-Z24;
  `cross-league-priors.md` §3.1).
- **DR-D17 and DR-D32, 2026-10-14.** At each season boundary S every season-boundary prior is built
  twice from the same data version, `prior_pre` on S−3..S−1 and `prior_in` on S−2..S−1. Apart from
  season S's frozen V(s), fitted on S−3..S−1 and shared by both builds (the gap DR-C6(b) accepts),
  each build uses only its own seasons' NFL evidence, including its own feeder equivalency and DR-D19
  targets (`state-space-kalman.md` §6.5, "Two builds per boundary"). Two canaries prove the window
  (`engine-spec.md` §12.3; `evaluation-and-leakage.md` §4.2, G13). S−3 data reaches no season-S
  state after week 1's forecast through a season-boundary prior, except through that V(s) and the
  model version's learned hyperparameters (κ_c, λ_c and the `SSParams`). Apart from those
  hyperparameters, pre-window data reaches neither season-boundary prior. Both canaries hold the
  model version fixed: its learned constants are calibrated by rolling origin, generally on seasons
  before the window, and are not window evidence (DR-D32; answer 11). The first also holds fixed the
  three inputs that carry S−3 by design, that V(s) among them.
  The windowed prior combines its base with the season means ȳ_s of the build's participation-free
  prior-builder residuals r̃ (`layer1-credit.md` §4.8.2), each with the variance
  `v_s = r_c/N_s + ω_c·Σ_w (n_{s,w}/N_s)²` (DR-D17, answer 9).
- **Each build needs window-local targets.** Every option offered computed each build's targets from
  that build's own window, and "Keep open" said so explicitly (`docs/06-sessions/2026-10-14-P0-01.md`,
  answer 10). With DR-D32's second canary (answer 6) and DR-D19's own words, "the prior never trains
  on itself", that window-local part does not wait on this ADR.
- **A stored production track cannot be the target.** Either reason is enough:
  - a feeder-linked player's production track restarts from a windowed prior whose base is his own
    college prior (`state-space-kalman.md` §6.5, "Base"), so the prior would train on itself;
  - that track also used its own season's V(s), live context model g′ and team effect, all fitted on
    seasons before the build's window (`layer1-credit.md` §4.8.2, §4.8.3), so neither canary could
    pass.
- **The seed problem.** A filter run that produces end-of-season talent has to start from a prior.
  DR-D19 rules out the college prior. `engine-spec.md` §6.2 Component 8 ("Causal initialization")
  allows only "the prior (§6.5) or a position prior", never computed from observations of the season it
  starts or a later one, and DR-C10 (ratified) requires "prior-based x0/P0 with no look-ahead".
  - The build's own population prior N(m, σ²_c) is estimated on the whole window, including the
    target season and the track's own events, so seeding from it looks ahead.
  - A prior from before the target season is pre-window data whenever that season is the window's
    first, and the second canary forbids that.
  - A declared vague seed that uses no data is not a prior that Component 8 allows today.

  So no seed fits the current text without a change.
- **What a seed does to talent.** A one-season run keeps part of its seed in end-of-season talent.
  The share falls as exposure rises, and exposure tends to rise with talent, so the pull falls
  hardest on low-exposure players. In the scratch runs (not evidence) a population seed attenuated the
  fitted slope and distorted the prior variance in both directions. Removing the seed's share exactly
  turns talent into a weighted season level that gives early weeks negative weight, so any correction
  changes the estimand. End-of-season talent also depends on `P0_form` and `P0_scheme`, which stay
  open until P1-12's washout and calibration measurements, and those measurements need the
  equivalency.
- **The windowed prior already measures the season level.** A season mean ȳ_s measures talent,
  scheme fit and average form as one constant per track, and `x0 = [μ, 0, 0]` makes μ a prior for the
  starting level (`state-space-kalman.md` §6.5). End-of-season talent is a different quantity: it
  leaves out scheme fit, weights the late weeks most and is pulled toward the seed.
- **Scratch figures.** The figures shown to the owner came from toy runs in the session scratchpad,
  in synthetic units. They are not evidence (`engine-spec.md` Appendix D item 16; KI-NEW-Y0;
  KI-NEW-P4), and none is carried here. The toy's planted truth was this ADR's own measurement model,
  so it could not show which estimand is right.

## Decision

In each DR-D32 build b (`prior_pre` on S−3..S−1; `prior_in` on S−2..S−1), the DR-D19
feeder-equivalency target of a track is the season-mean level of b's own window evidence in the
track's first window season with evidence, with its known variance. No prior enters the target.

1. **Sources.** The targets come only from b's own prior-builder records (`layer1-credit.md` §4.8.2)
   and b's own r_c and ω_c, read from b's hyperparameter record (`state-space-kalman.md` §6.5); b's
   population prior N(m, σ²_c) does not enter the target. None of these enters: a stored production
   track or state, live Layer-1′ residuals, RAPM ratings, offseason Layer-1 credit, an earlier
   boundary's prior, or NFL data from before b's window other than through season S's frozen V(s)
   behind each record's dV, which both builds share (DR-C6; DR-D32).
2. **Tracks.** There is one target per feeder-linked, non-ambiguous (player, role) track with at
   least one regular-season role event in b's window, from the base's placement season on
   (`state-space-kalman.md` §6.5, "Base"). Its target season s* is the earliest such season.
3. **Target.** ȳ_i is the mean of the track's prior-builder residuals r̃ over its role events in s*.
   Its known variance is the DR-D17 per-season variance with b's r_c and ω_c:

   ```text
   v_i = r_c/N + ω_c·Σ_w (n_w/N)²
   ```

   Here N is the track's role-event count in s* and n_w its count in week w of s*.
4. **No seed.** No filter is run, and no prior of any kind (college, population or vague) enters the
   target.
5. **Model.** Per equivalency class, a Fay–Herriot-type measurement-error regression:

   ```text
   ȳ_i = α + β·feeder_sv_i + u_i + e_i,    u_i ~ N(0, σ²_u),    e_i ~ N(0, v_i), v_i known
   θ_ncaa_translated = α̂ + β̂·feeder_sv
   ```

   σ²_u is the spread of the true season level around the line, net of the known target noise.
6. **Estimator.** The estimator, including whether the line uses precision weights 1/(σ²_u + v_i) or
   equal weights with a variance estimator net of the known noise, is named in
   `cross-league-priors.md` before P1-12 implements the fit (open — DR-C5).
7. **Prior variance.** In DR-C9's proposed engine form (`engine-spec.md` §6.5;
   `cross-league-priors.md` §6.2), "the residual variance of the equivalency" is σ̂²_u, the
   between-player variance net of the known target noise. The §6.5 widening for undrafted,
   transferred, position-converted or identity-uncertain players applies to it. Whether σ²_prior also
   carries the line's parameter uncertainty is not decided here (open — DR-C9).
8. **Kept from 2026-10-09.** The prior is on the Kalman credit scale, there is no separate scale map,
   and the prior never trains on itself: no prior of any kind enters the target.

With these targets both DR-D32 canaries can hold for feeder-linked tracks.

### Still open

This ADR does not decide the following.

- **Estimator.** The estimator for the line and σ²_u, including the weighting (precision weights, or
  equal or design weights with a variance estimator net of the known noise) and variance clustered by
  player and by team-season, since teammates share team-offence context in r̃. A zero estimate is
  already a typed failure (`engine-spec.md` §6.4.7); DR-D17's degenerate-pool thresholds stay open
  (open — DR-C5).
- **Equivalency class.** The position class (DR-C9's proposed text, `cross-league-priors.md` §6.2) or
  (position class, role) (DR-C3). For RB, WR and TE role tracks it also depends on DR-D13. The same
  decision covers how a player's several role tracks enter one fit: one row per track in a
  position-class fit would repeat a player id (`cross-league-priors.md` §8.2, P-7,
  `DuplicatePlayerId`) and mix role scales (open — DR-D19, DR-C9).
- **Minimum shared players and failure handling.** The minimum number of shared players per class and
  build (open — DR-D19, DR-C9; KI-G8, `InsufficientSharedPlayers`). What happens when one build's
  equivalency fails and the other's passes is also open: no fallback is specified, and substituting
  the population base would be a silent fallback (open — DR-D19).
- **Own evidence used twice.** Whether a shared player's own feeder base is fitted on folds that
  exclude him (KI-#48 k-fold; fold assignments are an input, `cross-league-priors.md` §5.2), so that
  his window evidence is not used twice (open — DR-D19).
- **Parameter uncertainty and the blend.** Whether σ²_prior also carries the fitted line's parameter
  uncertainty; and the q blend and its variance, which must be defined together with this target, the
  influence cap and exposure weighting (open — DR-C9).
- **Career stage.** For the rookie base, an age-at-s* term, which goes back to the Statistical owner
  and enters only through `theta_position_draft_prior` (`engine-spec.md` §6.5). It is never inferred
  from pre-window NFL data (open — DR-D19).
- **P0.** `P0_form` and `P0_scheme`, and P0 for window-restarted tracks. The base variance is now a
  level variance, so P0 must not count the starting level's variance twice (open — DR-D19).
- **`feeder_sv` from an NFL-fitted model.** If the college rating is built with an NFL-fitted model,
  whether that model must be season S's V(s), so that the static-metadata exemption carries no
  pre-window NFL data (open — DR-C9).
- **Measurements.** The P1-12 measurements, pre-registered per class and build as "Compliance"
  states.

## Reverses

**Reverses, in part: DR-D19's decision of 2026-10-09** (`docs/00-meta/decision-register.md`, DR-D19;
register rule 6).

- **Replaced.** The estimand of the chosen Option 1, "Make the equivalency target the same quantity
  as the Kalman talent state", and the words "fitted directly against the tracker's own talent
  scale - end-of-season (smoothed) talent computed without the college prior". In their place: the
  season-mean level of the build's own window evidence in the track's first window season with
  evidence, with its known variance; no prior enters the target.
- **Kept.** "So the prior never trains on itself", now because no prior of any kind enters the
  target; "no separate scale map"; the Kalman credit scale.
- **Not touched.** DR-D19's open part: `P0_form`, `P0_scheme`, and P0 for window-restarted tracks.
- **No ADR is superseded.** DR-D19 was recorded in the register and in the documents that implement
  it, not in an ADR, so `supersedes` is empty.

## Consequences

**Easier.**

- **The window canaries can hold.** Every input to a target is the build's own window evidence and
  its r_c and ω_c, apart from season S's frozen V(s) behind the records' dV, which is fitted on
  S−3..S−1 and which the first canary holds fixed. So for feeder-linked tracks both DR-D32 canaries
  can hold by construction.
- **No seed and no exemption.** No look-ahead exemption from DR-C10 or Component 8 is needed, and no
  seed constants are declared.
- **One quantity.** The target is the season level that the windowed prior already combines with its
  base, measured from the same records with the same variance (DR-D17).
- **No loop inside P1-12.** The fit does not depend on `P0_form` and `P0_scheme`, so their washout and
  calibration measurements can use the equivalency.
- **A defined prior variance.** The equivalency's part of σ²_prior is the between-player variance net
  of measurement noise.

**Harder.**

- **It reverses the owner's 2026-10-09 choice in part.** The estimand of Option 1 changes. It binds
  only once the owner accepts this ADR, and until then the 2026-10-09 text, for which no seed fits,
  governs. P1-12 does not implement the feeder equivalency before the owner accepts or rejects it.
- **The weighting matters.** Precision weights are informative when exposure follows talent: a
  precision-weighted line overstates the base mean. In the scratch runs (not evidence) that bias did
  not shrink as the pool grew, and an equal-weight line showed little of it. The DR-C5 estimator
  decides the weighting.
- **Small pools.** With few shared players the slope is noisy. In the scratch runs (not evidence),
  seeded targets scored better on log score with a small pool, because the seed acted as accidental
  shrinkage. Deliberate pooling (KI-#44) is the controlled fix.
- **Zero variance estimates.** A plain likelihood estimate of σ²_u can come out at zero with small
  pools. A zero estimate is a typed failure (`engine-spec.md` §6.4.7; `cross-league-priors.md` §4.6),
  so such a class fails the build instead of producing a prior. An adjusted estimator avoids this but
  is conservative.
- **The known variance is slightly understated** if weekly form is autocorrelated. This is the
  overdispersion question DR-D17 already leaves open.
- **Career stage.** For a veteran, s* is his first window season, not his entry season, so the shared
  pool is mostly players past their entry season, while a rookie takes his base unchanged
  (`state-space-kalman.md` §6.5). If players improve after NFL entry, the rookie base sits at the
  pool's later-career level and is biased upward. Every option offered shares this limitation, and no
  provider document yet specifies a field-level NFL entry-season source. A multi-season or
  entry-season-only target, or a restriction of the pool to entry seasons, would change the decided
  target (Decision, items 2 and 3) and needs a new decision request and an ADR amending ADR-013;
  either entry-season form also needs a provider document that specifies a field-level draft- or
  entry-year source.
- **Several role tracks per player.** They cannot each enter one position-class fit as a separate
  row. How they enter is part of the open class question.
- **The blend still pulls.** The ratified blend
  `θ_prior = q·θ_ncaa_translated + (1 − q)·θ_position_draft_prior` pulls toward the population mean
  under every option, and σ²_prior as written does not reflect the blend, so q and the blend's
  variance must be defined together (DR-C9).
- **What the scratch runs could not test.** Within-season talent drift, which the filter's discount
  assumes; the team-offence context that r̃ keeps; NFL career development; and a difference between
  rookies and the shared pool. The P1-12 measurements plant them.

**Inherited.**

- The open items above stay open under their tags. As DR-D19's Blocks field says, the estimator
  (DR-C5), the equivalency class and the minimum shared players per class and build (DR-D19, DR-C9)
  are set before P1-12 implements the fit. `P0_form`, `P0_scheme` and P0 for window-restarted tracks
  are set afterwards, by P1-12's washout and calibration measurements, which use the fit ("No loop
  inside P1-12"). Whether σ²_prior also carries the line's parameter uncertainty, and the variance of
  the q blend, are settled under DR-C9 before P1-07 assembles σ²_prior. P1-12 inherits the
  measurements under "Compliance", pre-registered per class and build before it runs them.
- The required report of the population the map was fitted on (`cross-league-priors.md` §1) now
  describes feeder-linked tracks with at least one window event.
- The prior-builder records, read today only by the windowed prior builder, are also read by the same
  build's equivalency fit (`layer1-credit.md` §4.8.2; `state-space-kalman.md` §6.5).

## Compliance

| Rule | Enforced or detected by |
|---|---|
| Window-local targets | The two DR-D32 window canaries (`engine-spec.md` §12.3; `evaluation-and-leakage.md` §4.2, G13), run on feeder-linked tracks with the model version held fixed: its learned hyperparameters (κ_c, λ_c and the `SSParams`) are calibrated by rolling origin and are not window evidence (DR-D32). Changing any S−3 play or Layer-1′ input, with the three inputs that carry S−3 by design held fixed, leaves `prior_in` and every season-S state from week 1's update on bit-identical. Apart from those learned hyperparameters, pre-window data reaches neither prior: changing any NFL data from before S−3, static metadata excepted, leaves both priors bit-identical. The leakage harness (P1-05) carries the canaries; they cover feeder-linked tracks once P1-12 implements the fit |
| Window-local target audit | Review in P1-12: the equivalency's targets read only b's own prior-builder records and the r_c and ω_c of b's hyperparameter record. The fit never reads a stored production track or state, live Layer-1′ residuals, RAPM ratings, offseason Layer-1 credit, an earlier boundary's prior, or NFL data from before b's window other than through season S's frozen V(s) behind the records' dV, and it runs no filter and reads no prior to build a target |
| Prior variance | σ̂²_u is persisted in the equivalency's versioned record (`cross-league-priors.md` §6.2, "Lifecycle requirements"). A zero estimate is a typed failure, never clamped and never replaced by a default (`engine-spec.md` §6.4.7) |
| Pre-registered measurements | Pre-registered per class and build before P1-12 runs them. Primary, and neutral between estimands: weekly one-step predictive log score, NIS, PIT and coverage for rookies and feeder-linked veterans run through the filter from `x0 = [μ_f, 0, 0]`. Secondary: slope and base-mean bias against planted truth, by exposure tier and career stage; prior variance against planted truth; the learned κ_c; the share of zero variance estimates. They run on a multi-season synthetic world (DR-B4) that plants within-season talent drift, team-offence context, career development, and exposure and selection driven by talent. A rolling-origin real-data check on the same yardstick follows once Layer-1′ and CFBD `feeder_sv` exist (KI-NEW-Z50, DR-C9). The `engine-spec.md` §6.5 and §9.4 rule that NCAA priors improve low-evidence-player PB-MAE or calibration is checked in P1-09 on the §2.5 low-evidence slice (`evaluation-and-leakage.md` §4.12), where DR-D9 gates it; it is not one of these measurements |
| Binding | Until the owner accepts this ADR, nothing enforces it, and DR-D19's 2026-10-09 text governs. The window-local part does not wait on this ADR (Context) |

## Alternatives considered

- **Talent, vague seed.** Fit against end-of-season talent from a one-season filter run in the target
  season, seeded with a declared vague prior that uses no data. It keeps DR-D19's estimand. It lost
  because:
  - a declared vague seed is not a prior that Component 8 allows today, so it needs a spec change;
  - it adds declared seed constants and the target run's `SSParams` and P0, which would join the
    second canary's held-fixed list;
  - the target depends on `P0_form` and `P0_scheme`, whose measurements need the equivalency, a loop
    inside P1-12;
  - it weights the late weeks most, leaves out scheme fit and is pulled toward its seed, so it
    measures a different quantity from the season means the windowed prior uses.
- **Talent, population seed.** The same run, seeded with b's own population prior N(m, σ²_c). It lost
  because that prior is estimated on the whole window, including s* and the track's own events, so
  the seed looks ahead and needs a scoped exemption from DR-C10 (ratified) and Component 8. It keeps
  the vague seed's other costs: the run's `SSParams` and P0 on the held-fixed list, the P0 loop and
  the late-week weighting. The seed's share, which falls hardest on low-exposure players, attenuated
  the slope in the scratch runs (not evidence), and its pull toward the population mean adds to the
  blend's.
- **Keep open.** It would leave the seed and the estimand open and DR-D19's text unchanged. Every
  option offered computed each build's targets from that build's own window, and "Keep open" said so
  explicitly: "each build computes its targets only from its own window records, never from a stored
  production track" (`docs/06-sessions/2026-10-14-P0-01.md`, answer 10). It lost because no seed fits
  the 2026-10-09 text, so P1-12 would have no implementable target for the feeder equivalency.
- **Correct the seed's share.** Not a separate option: removing the share exactly gives early weeks
  negative weight, so it changes the estimand and needs an amendment just as this ADR does.
- **Season-end level (not offered).** Talent plus scheme fit at season end, with a variance
  correction. It was not put to the owner, because it amends DR-D19's estimand just as this ADR does,
  and it keeps what the two talent options carry: the seed, the look-ahead exemption, the P0 loop and
  the late-week weighting.
