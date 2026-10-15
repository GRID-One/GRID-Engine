"""
The dynamic (state-space) layer.

Latent state per player x_w = [tau_w, f_w, s_w]:
  * tau  (talent):     near-random-walk, slow, persistent.
  * f    (form):       AR(1), mean-reverting transient deviation.
  * s    (scheme_fit): very-slow random walk; changes only when scheme changes.
Observed weekly value y_w = tau_w + f_w + s_w + noise.  Components are separated
by timescale + persistence (form is autocorrelated, scheme_fit extremely persistent).

Process noise via a DISCOUNT FACTOR d (West-Harrison): each step the talent
variance is inflated by 1/d, so d is the single interpretable knob -- d~1 means
very stable (QB talent), lower d means responsive. The "spike" is a temporary
drop in d at known regime-change weeks (return from injury, scheme/QB change),
plus an inflated observation variance for the first game(s) back (rust).

Filtered estimate (uses data through w)  -> "current form" / real-time grade.
RTS-smoothed estimate (uses all weeks)    -> best retrospective talent.

Missing weeks (player did not play) are handled by predicting without updating,
so uncertainty grows during an absence exactly as it should.
"""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
import numpy as np

_KALMAN_PATH = Path("data/cache/kalman_state.npz")

# Number of latent state components: [talent, form, scheme_fit]
N_STATE = 3


@dataclass
class KalmanState:
    """Per-player Kalman filter state for incremental weekly updates.

    mu:    shape (n_players, N_STATE) — [talent, form, scheme_fit] estimates
    sigma: shape (n_players, N_STATE, N_STATE) — per-player covariance matrices
    player_ids: list of player IDs in row order
    """
    mu: np.ndarray
    sigma: np.ndarray
    player_ids: list[str]

    @classmethod
    def init(cls, player_ids: list[str]) -> "KalmanState":
        n = len(player_ids)
        mu = np.zeros((n, N_STATE))
        sigma = np.stack([np.diag([0.05, 0.02, 0.01])] * n)
        return cls(mu=mu, sigma=sigma, player_ids=list(player_ids))

    def save(self, path: "Path | str | None" = None) -> None:
        """Persist state to ``path`` (default: module ``_KALMAN_PATH``).

        ``path`` is injectable so backtest/fold code can confine all writes to a
        scratch namespace and never touch the production ``data/cache/`` state.
        """
        path = Path(path) if path is not None else _KALMAN_PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        np.savez(
            str(path),
            mu=self.mu,
            sigma=self.sigma,
            player_ids=np.array(self.player_ids),
            state_version=np.array(N_STATE),
        )

    @classmethod
    def migrate(cls, mu2: np.ndarray, sigma2: np.ndarray, pids,
                scheme_var: float = 0.01) -> "KalmanState":
        """Pad a legacy 2-component state to 3 components losslessly.

        mu2:    shape (n, 2) — [talent, form]
        sigma2: shape (n, 2, 2) — per-player 2×2 covariance matrices
        pids:   player ID sequence
        """
        n = mu2.shape[0]
        mu = np.hstack([mu2, np.zeros((n, 1))])
        sigma = np.zeros((n, 3, 3))
        sigma[:, :2, :2] = sigma2
        sigma[:, 2, 2] = scheme_var
        return cls(mu=mu, sigma=sigma, player_ids=list(pids))

    @classmethod
    def load(cls, path: "Path | str | None" = None) -> "KalmanState | None":
        """Load state from ``path`` (default: module ``_KALMAN_PATH``).

        ``path`` is injectable so backtest/fold code reads an isolated namespace
        rather than the production ``data/cache/`` state (and cannot contaminate
        a fold with a developer's real cache).
        """
        path = Path(path) if path is not None else _KALMAN_PATH
        if not path.exists():
            return None
        # allow_pickle stays False: save() writes only plain float ndarrays and a
        # unicode string array for player_ids, so no object deserialization is
        # needed — important now that the load path is injectable.
        data = np.load(str(path))
        mu = data["mu"]
        sigma = data["sigma"]
        pids = data["player_ids"]
        width = mu.shape[1]

        # Width takes precedence over state_version: a 3-wide file without
        # state_version is a Task-8-era file that must be loaded as-is rather
        # than discarded.  state_version is a fast-path diagnostic only.
        if width == N_STATE:
            # Current or Task-8 3-wide format — load as-is regardless of whether
            # state_version is present
            return cls(mu=mu, sigma=sigma, player_ids=list(pids))

        if width == 2:
            # Legacy 2-component file: pad to 3 components
            return cls.migrate(mu, sigma, pids)

        # Any other width (corrupt / future format) — cannot use safely
        return None


_POSITION_PARAMS: dict[str, dict[str, float]] = {
    # QB r_scale calibrated on synth: pooling the week-held-out weekly Layer-1
    # predictive innovations across all QBs, r_scale=0.55 lands pooled NIS≈1.24
    # and median per-QB NIS≈0.93 (both ~ the ideal [0.8,1.25]) with PICP@80≈0.78;
    # the prior 0.35 was overconfident (NIS≈1.6, PICP@80≈0.71).  See tests/grid/
    # test_calibration_synth.py.  Other positions await their weekly Layer-1
    # signal (Phase 2b) before they can be calibrated the same way.
    "QB":  {"d_steady": 0.95, "r_scale": 0.55},
    "RB":  {"d_steady": 0.85, "r_scale": 0.45},
    "WR":  {"d_steady": 0.90, "r_scale": 0.40},
    "TE":  {"d_steady": 0.90, "r_scale": 0.40},
    "DEF": {"d_steady": 0.95, "r_scale": 0.55},
}


@dataclass
class SSParams:
    phi: float = 0.50            # form persistence (AR1)
    phi_scheme: float = 0.985    # scheme_fit persistence (very slow random walk)
    d_steady: float = 0.90       # talent discount (per position)
    d_spike: float = 0.70        # discount during an intervention week
    q_form: float = 0.0008       # form innovation variance
    q_scheme: float = 0.0002     # scheme_fit innovation variance (very small)
    scheme_reset_var: float = 0.04  # extra scheme_fit variance at intervention
    r_scale: float = 0.40        # obs variance = r_scale / snaps
    post_event_r_mult: float = 2.0   # inflate R for games right after a spike
    post_event_games: int = 2

    @classmethod
    def from_position(cls, pos: str) -> "SSParams":
        overrides = _POSITION_PARAMS.get(pos, {})
        return cls(**overrides)


def _F(p: SSParams) -> np.ndarray:
    """State transition matrix (N_STATE × N_STATE)."""
    return np.array([
        [1.0, 0.0, 0.0],
        [0.0, p.phi, 0.0],
        [0.0, 0.0, p.phi_scheme],
    ])


# Observation matrix: y = talent + form + scheme_fit
_H = np.array([1.0, 1.0, 1.0])


def kalman_two_component(y, snaps, played, weeks=None, interventions=None,
                         params: SSParams | None = None, x0=None, P0=None):
    """Run filter + RTS smoother.

    y, snaps, played : length-W arrays (played=False -> missing observation).
    interventions    : set of week indices (0-based) to spike the discount.
    Returns dict with filtered/smoothed tau, total (tau+form+scheme), and their variances.
    """
    p = params or SSParams()
    W = len(y)
    interventions = set(interventions or [])
    F = _F(p)
    H = _H  # shape (N_STATE,) — observation row vector

    # init: x is a 3-vector [talent, form, scheme_fit]
    if x0 is not None:
        x = np.array(x0, dtype=float)
    else:
        talent_init = np.nanmean(y[:3]) if np.any(np.isfinite(y[:3])) else 0.0
        x = np.array([talent_init, 0.0, 0.0])
    P = np.array(P0, dtype=float) if P0 is not None else np.diag([0.05, 0.02, 0.01])

    xp_s, Pp_s, xf_s, Pf_s = [], [], [], []
    pred_mean_s, pred_var_s = [], []
    games_since_event = 99
    for w in range(W):
        # ---- predict ----
        xp = F @ x
        Pp = F @ P @ F.T
        d = p.d_spike if w in interventions else p.d_steady
        Pp[0, 0] /= d                       # discount inflation on talent
        Pp[1, 1] += p.q_form                # form innovation
        Pp[2, 2] += p.q_scheme              # scheme_fit slow random walk
        if w in interventions:
            Pp[1, 1] += 5 * p.q_form        # extra form uncertainty at the jump
            Pp[2, 2] += p.scheme_reset_var  # scheme_fit can shift at regime change
            games_since_event = 0

        # ---- one-step-ahead predictive of the observation y_w (BEFORE update) ----
        # This is the honest forecast distribution N(mean, S) calibration must be
        # scored against: mean = H·xp, S = H·Pp·Hᵀ + R.  S is the innovation
        # variance; unlike var_total_filt (the post-update FILTERED variance,
        # which omits R and is mildly circular) it is overconfident by neither.
        # R needs snaps, which are unknown on missing weeks -> use the snaps floor
        # so the predictive is defined every week; for played weeks this R/S is
        # exactly the one used in the gain below (no change to the filter).
        r_mult = p.post_event_r_mult if games_since_event < p.post_event_games else 1.0
        R = r_mult * p.r_scale / max(snaps[w], 1.0)
        pred_mean = float(H @ xp)
        S = float(H @ Pp @ H) + R           # scalar innovation / predictive variance
        pred_mean_s.append(pred_mean)
        pred_var_s.append(S)

        # ---- update (skip if missing) ----
        if played[w] and np.isfinite(y[w]):
            K = (Pp @ H) / S                # N_STATE-vector
            innov = y[w] - pred_mean
            xf = xp + K * innov
            IKH = np.eye(N_STATE) - np.outer(K, H)
            Pf = IKH @ Pp @ IKH.T + np.outer(K, K) * R
            games_since_event += 1
        else:
            xf, Pf = xp.copy(), Pp.copy()   # predict-only; variance grows

        xp_s.append(xp); Pp_s.append(Pp.copy()); xf_s.append(xf); Pf_s.append(Pf.copy())
        x, P = xf, Pf

    xf_s = np.array(xf_s); xp_s = np.array(xp_s)

    # ---- RTS smoother ----
    xs = [None] * W; Ps = [None] * W
    xs[-1] = xf_s[-1]; Ps[-1] = Pf_s[-1]
    for w in range(W - 2, -1, -1):
        Pp_next = Pp_s[w + 1] + 1e-10 * np.eye(N_STATE)
        C = np.linalg.solve(Pp_next.T, (Pf_s[w] @ F.T).T).T
        xs[w] = xf_s[w] + C @ (xs[w + 1] - xp_s[w + 1])
        Ps[w] = Pf_s[w] + C @ (Ps[w + 1] - Pp_next) @ C.T
    xs = np.array(xs)

    return dict(
        tau_filt=xf_s[:, 0], form_filt=xf_s[:, 1], scheme_filt=xf_s[:, 2],
        total_filt=xf_s[:, 0] + xf_s[:, 1] + xf_s[:, 2],
        var_total_filt=np.array([ float(H @ Pf_s[w] @ H) for w in range(W) ]),
        # One-step-ahead predictive of the observation: the honest forecast
        # (mean, S) for weekly calibration (NIS/PIT/coverage).  S includes R.
        total_pred=np.array(pred_mean_s),
        var_total_pred=np.array(pred_var_s),
        tau_smooth=xs[:, 0], total_smooth=xs[:, 0] + xs[:, 1] + xs[:, 2],
        var_tau_smooth=np.array([Ps[w][0, 0] for w in range(W)]),
        sigma_smooth=np.array(Ps),  # shape (T, N_STATE, N_STATE) — RTS smoothed covariances
    )


def detect_changepoints(
    state: KalmanState,
    obs: np.ndarray,
    snaps: np.ndarray | None = None,
    params: SSParams | None = None,
    z_thresh: float = 3.0,
    use_cusum: bool = False,
) -> set[str]:
    """Detect players whose observation deviates sharply from the Kalman prediction.

    For each player with a non-NaN observation, compute the standardised innovation:

        z_i = (obs_i - H @ mu_pred_i) / sqrt(S_i)

    where:
        mu_pred_i = (F @ state.mu[i])
        S_i       = H @ sigma_pred_i @ H.T + R_i
        sigma_pred_i applies the same steady-state noise inflation as kalman_step
                     (d_steady, q_form, q_scheme) but NO per-player intervention
                     adjustments (those are unknown before detection).

    Flags players where |z_i| > z_thresh and returns their player_ids as a set.

    Missing observations (NaN) are never flagged.

    Parameters
    ----------
    state     : Current KalmanState before this week's update.
    obs       : shape (n_players,) RAPM observations; NaN = did not play.
    snaps     : shape (n_players,) snap counts; defaults to 1.0 per player.
    params    : SSParams for this position group (use SSParams.from_position(pos)
                inside the per-position loop — NOT a global SSParams() before the loop).
    z_thresh  : Standardised-innovation threshold (default 3.0).
    use_cusum : Reserved for future CUSUM detection (default False — off).
                Do not gate on this flag; the z-score path always runs.

    Returns
    -------
    set[str] of player_ids whose |z| > z_thresh.
    """
    p = params or SSParams()
    n = len(state.player_ids)
    F = _F(p)
    H = _H  # shape (N_STATE,)

    if snaps is None:
        snaps = np.ones(n)

    # Predict: apply steady-state noise inflation (same as kalman_step, no intv)
    mu_pred = state.mu @ F.T                             # (n, N_STATE)
    sigma_pred = np.stack([F @ state.sigma[i] @ F.T for i in range(n)])
    sigma_pred[:, 0, 0] /= p.d_steady   # talent discount inflation
    sigma_pred[:, 1, 1] += p.q_form     # form innovation
    sigma_pred[:, 2, 2] += p.q_scheme   # scheme_fit slow random walk

    flagged: set[str] = set()
    for i, pid in enumerate(state.player_ids):
        if np.isnan(obs[i]):
            continue
        R = p.r_scale / max(float(snaps[i]), 1.0)
        S = float(H @ sigma_pred[i] @ H) + R
        z = (obs[i] - float(H @ mu_pred[i])) / np.sqrt(S)
        if abs(z) > z_thresh:
            flagged.add(pid)

    return flagged


def kalman_step(
    state: KalmanState,
    obs: np.ndarray,
    snaps: np.ndarray | None = None,
    params: SSParams | None = None,
    interventions: set[str] | None = None,
    scheme_resets: set[str] | None = None,
    return_pred: bool = False,
):
    """Advance KalmanState by one week's RAPM observations.

    obs:           shape (n_players,) — this week's per-player RAPM estimates; NaN = did not play.
    snaps:         shape (n_players,) — snap counts for scaling observation noise; defaults to 1.
    interventions: set of player IDs with regime changes this week (injury return, scheme change).
    scheme_resets: set of player IDs whose scheme_fit component is reset this week (coaching change).
                   A scheme reset zeroes mu[i,2], sets sigma[i,2,2]=scheme_reset_var, and zeroes
                   the scheme cross-covariances sigma[i,2,:2] and sigma[i,:2,2].
                   DISTINCT from interventions — does not touch talent/form.
    return_pred:   when True, also return a per-player one-step predictive variance array
                   ``S_i = H·Σ_pred_i·Hᵀ + R_i`` (the honest forecast variance, R included)
                   as ``(KalmanState, pred_var)``.  Default False -> returns KalmanState only
                   (backward compatible).  Used by the trajectory write path so the shipped
                   uncertainty band is the predictive S, not the R-omitted filtered variance.
    """
    p = params or SSParams()
    n = len(state.player_ids)
    F = _F(p)
    H = _H  # shape (N_STATE,)
    intv_set = interventions or set()

    if snaps is None:
        snaps = np.ones(n)

    pid_to_idx = {pid: i for i, pid in enumerate(state.player_ids)}
    intv_indices = {pid_to_idx[pid] for pid in intv_set if pid in pid_to_idx}

    # Predict (vectorized)
    mu_pred = state.mu @ F.T
    sigma_pred = np.stack([F @ state.sigma[i] @ F.T for i in range(n)])
    sigma_pred[:, 0, 0] /= p.d_steady   # talent discount inflation
    sigma_pred[:, 1, 1] += p.q_form     # form innovation
    sigma_pred[:, 2, 2] += p.q_scheme   # scheme_fit slow random walk

    # Intervention players: spike discount + extra form/scheme uncertainty
    for i in intv_indices:
        sigma_pred[i, 0, 0] *= p.d_steady / p.d_spike
        sigma_pred[i, 1, 1] += 5 * p.q_form
        sigma_pred[i, 2, 2] += p.scheme_reset_var

    new_mu = mu_pred.copy()
    new_sigma = sigma_pred.copy()

    # Scheme-reset players: zero scheme_fit mean, set scheme_reset_var, zero cross-covariances.
    # Applied AFTER the predict step and BEFORE the update, so the zeroed prior propagates
    # through the Kalman gain calculation.  Talent (index 0) and form (index 1) are untouched.
    reset_indices = {pid_to_idx[pid] for pid in (scheme_resets or set()) if pid in pid_to_idx}
    for i in reset_indices:
        new_mu[i, 2] = 0.0
        new_sigma[i, 2, 2] = p.scheme_reset_var
        new_sigma[i, 2, :2] = 0.0
        new_sigma[i, :2, 2] = 0.0

    # Per-player one-step predictive variance S_i = H·Σ_pred_i·Hᵀ + R_i, captured
    # from the SAME prior covariance used in the gain (post scheme-reset).  Defined
    # for every player, including those who did not play (NaN obs) — there the snaps
    # floor sets R.  This is the honest forecast variance (R included), distinct from
    # the post-update filtered variance the trajectory previously persisted.
    pred_var = np.empty(n)
    for i in range(n):
        r_mult = p.post_event_r_mult if i in intv_indices else 1.0
        pred_var[i] = float(H @ new_sigma[i] @ H) + r_mult * p.r_scale / max(float(snaps[i]), 1.0)

    # Update (only players with observations)
    for i in range(n):
        if np.isnan(obs[i]):
            continue
        r_mult = p.post_event_r_mult if i in intv_indices else 1.0
        R = r_mult * p.r_scale / max(float(snaps[i]), 1.0)
        S = float(H @ new_sigma[i] @ H) + R
        K = new_sigma[i] @ H / S
        new_mu[i] += K * (obs[i] - float(H @ new_mu[i]))
        IKH = np.eye(N_STATE) - np.outer(K, H)
        new_sigma[i] = IKH @ new_sigma[i] @ IKH.T + np.outer(K, K) * R

    updated = KalmanState(mu=new_mu, sigma=new_sigma, player_ids=state.player_ids)
    if return_pred:
        return updated, pred_var
    return updated


if __name__ == "__main__":
    # quick self-test on a planted trajectory with an injury gap
    from synth import simulate
    from value import fit_value_model, attach_dv
    from layers import fit
    import numpy as np

    plays, players, gt = simulate()
    plays = attach_dv(plays, fit_value_model(plays))
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=2, verbose=False)
    wk = res["qb_weekly"].set_index("week")

    W = gt["focus_tau"].shape[0]
    y = np.full(W, np.nan); snaps = np.zeros(W); played = np.zeros(W, bool)
    for w in range(1, W + 1):
        if w in wk.index:
            y[w - 1] = wk.loc[w, "qb_credit"]; snaps[w - 1] = wk.loc[w, "snaps"]; played[w - 1] = True
    out = kalman_two_component(y, snaps, played, interventions={9})  # week 10 return

    truth = gt["focus_tau"]
    m = np.isfinite(truth) & played
    c_total = np.corrcoef(out["total_smooth"][m], truth[m])[0, 1]
    c_tau = np.corrcoef(out["tau_smooth"][m], truth[m])[0, 1]
    print("weeks played:", int(played.sum()))
    print("corr(current-ability tau+form+scheme, planted) = %.3f" % c_total)
    print("corr(talent tau only, planted)                 = %.3f  (smooth arc, ignores transient)" % c_tau)
    print("current-ability variance: healthy wk3=%.4f  return wk10=%.4f (intervention widens it)"
          % (out["var_total_filt"][2], out["var_total_filt"][9]))
