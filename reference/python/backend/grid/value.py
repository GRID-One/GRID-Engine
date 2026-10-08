"""
Situational Value (SV) currency.

V(s) = expected net points to the offense before the drive resolves, given the
game state s = (down, distance, yardline). We FIT V from data (we never know
true EP in reality), then value each play as the change in expected points:

    dV(play) = V(s') - V(s)

where for a drive-ending play s' is an absorbing state with the realized
terminal value (7 for TD, 3 for FG, 0 for punt/downs), and for a continuing
play s' is the next snap's state.

This is deliberately a transparent, standard expected-points scaffold -- the
originality in GRID is in attribution (layers) and the dynamic layer, not in
re-inventing the value currency. We keep it honest about that.
"""
from __future__ import annotations
import os
import numpy as np
import pandas as pd
import joblib
from sklearn.ensemble import HistGradientBoostingRegressor

STATE_COLS = ["down", "ydstogo", "yardline_100"]

def fit_value_model(plays: pd.DataFrame, random_state: int = 0,
                    cache_path: str | None = None):
    """Fit V(s) = E[drive_points | state] on non-terminal-irrelevant rows.

    We train on every play's (state -> eventual drive points). Player effects
    average out, so the model learns the state surface, which is what V is.

    Parameters
    ----------
    plays:
        Play-by-play DataFrame with at least STATE_COLS + ``drive_points``.
    random_state:
        Seed for the gradient-boosting model.
    cache_path:
        Optional **file** path.  If given, the fitted model is persisted to
        exactly that path with joblib so it can be reloaded by
        :func:`load_value_model` without re-fitting.  Parent directories are
        created if they do not already exist.  Silently overwrites any
        previously cached model.
    """
    from pathlib import Path

    X = plays[STATE_COLS].to_numpy(float)
    y = plays["drive_points"].to_numpy(float)
    model = HistGradientBoostingRegressor(
        max_depth=4, learning_rate=0.08, max_iter=300,
        min_samples_leaf=120, random_state=random_state,
    )
    model.fit(X, y)

    if cache_path is not None:
        Path(cache_path).parent.mkdir(parents=True, exist_ok=True)
        joblib.dump(model, cache_path)

    return model


def load_value_model(path: str):
    """Load a persisted value model from *path*.

    Parameters
    ----------
    path:
        The exact file path that was passed as ``cache_path`` to
        :func:`fit_value_model` (i.e. the literal ``.joblib`` file, not a
        directory).

    Returns
    -------
    Fitted ``HistGradientBoostingRegressor`` (or whatever scikit-learn
    estimator was originally persisted).

    Raises
    ------
    FileNotFoundError
        If the path does not exist.
    """
    if not os.path.exists(path):
        raise FileNotFoundError(f"No cached value model found at: {path!r}")
    return joblib.load(path)


def compute_dv(plays: pd.DataFrame, vmodel) -> np.ndarray:
    """Return dV per play (offense perspective)."""
    s = plays[STATE_COLS].to_numpy(float)
    v_s = vmodel.predict(s)

    # next-state value: terminal -> realized terminal value; else V(next state)
    term = plays["terminal"].to_numpy(bool)
    v_sp = np.empty(len(plays), float)
    v_sp[term] = plays.loc[term, "terminal_value"].to_numpy(float)

    cont = ~term
    if cont.any():
        sp = plays.loc[cont, ["n_down", "n_ydstogo", "n_yardline_100"]].to_numpy(float)
        v_sp[cont] = vmodel.predict(sp)

    return v_sp - v_s


def attach_dv(plays: pd.DataFrame, vmodel) -> pd.DataFrame:
    out = plays.copy()
    out["dv"] = compute_dv(out, vmodel)
    return out


if __name__ == "__main__":
    from synth import simulate
    plays, players, gt = simulate()
    vm = fit_value_model(plays)
    plays = attach_dv(plays, vm)
    print("dV mean %.4f  sd %.4f" % (plays.dv.mean(), plays.dv.std()))
    # sanity: V should rise as you near the end zone on 1st & 10
    import numpy as np
    grid = pd.DataFrame({"down": 1, "ydstogo": 10, "yardline_100": [90, 60, 30, 10]})
    print("V(1st&10) by yards-to-goal [90,60,30,10]:",
          np.round(vm.predict(grid[STATE_COLS].to_numpy(float)), 3))
