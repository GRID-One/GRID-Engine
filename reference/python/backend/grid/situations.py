"""Situation classifier for GRID engine.

Slices plays into named boolean masks: red_zone, passing_downs, rushing_downs.
The two_minute mask is auto-activated only when ``quarter_seconds_remaining``
is present in the plays DataFrame; otherwise it is omitted and a warning is
logged.

Usage::

    from backend.grid.situations import classify
    masks = classify(plays)   # dict[str, np.ndarray[bool]]
"""
from __future__ import annotations

import logging
from typing import Dict

import numpy as np
import pandas as pd

logger = logging.getLogger(__name__)

TWO_MIN_COL = "quarter_seconds_remaining"

# ---------------------------------------------------------------------------
# Situation definitions — keyed by situation name, values are callables that
# accept a plays DataFrame and return a boolean array.
# Uses ONLY the plays-contract columns: down, ydstogo, yardline_100.
# ---------------------------------------------------------------------------
SITUATIONS: Dict[str, object] = {
    "red_zone":      lambda p: p["yardline_100"].to_numpy() <= 20,
    "passing_downs": lambda p: ((p["down"] == 3) & (p["ydstogo"] >= 7)) | (p["down"] == 4),
    "rushing_downs": lambda p: (p["down"] <= 2) & (p["ydstogo"] <= 4),
}


def classify(plays: pd.DataFrame) -> Dict[str, np.ndarray]:
    """Classify plays into situational boolean masks.

    Parameters
    ----------
    plays:
        DataFrame containing at minimum the contract columns ``down``,
        ``ydstogo``, and ``yardline_100``. The DataFrame is **not** mutated.

    Returns
    -------
    dict mapping situation name -> numpy boolean array of length ``len(plays)``.
    The ``two_minute`` key is included only when ``quarter_seconds_remaining``
    is a column in *plays*.
    """
    out: Dict[str, np.ndarray] = {
        name: np.asarray(fn(plays), dtype=bool)
        for name, fn in SITUATIONS.items()
    }

    if TWO_MIN_COL in plays.columns:
        out["two_minute"] = plays[TWO_MIN_COL].to_numpy() <= 120
    else:
        logger.warning("two_minute skipped: %s absent", TWO_MIN_COL)

    return out
