"""Tests for backend.grid.situations — Task 4 TDD suite."""
import logging

import numpy as np
import pandas as pd
import pytest

from backend.grid.situations import TWO_MIN_COL, classify


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_plays(**kwargs):
    """Return a minimal plays DataFrame with required contract columns."""
    n = kwargs.pop("n", 5)
    defaults = {
        "down": [1, 2, 3, 4, 1],
        "ydstogo": [10, 7, 8, 1, 3],
        "yardline_100": [50, 30, 15, 5, 70],
    }
    defaults.update(kwargs)
    # Trim/extend lists to length n
    data = {}
    for col, vals in defaults.items():
        if isinstance(vals, list):
            data[col] = (vals * ((n // len(vals)) + 1))[:n]
        else:
            data[col] = vals
    return pd.DataFrame(data).iloc[:n].reset_index(drop=True)


# ---------------------------------------------------------------------------
# Test 1: red_zone threshold
# ---------------------------------------------------------------------------

def test_red_zone_mask_matches_yardline_threshold():
    """Plays with yardline_100 <= 20 must be True; > 20 must be False."""
    plays = pd.DataFrame({
        "down":         [1,  1,  1,  1],
        "ydstogo":      [10, 10, 10, 10],
        "yardline_100": [20, 21, 1,  50],
    })
    masks = classify(plays)
    rz = masks["red_zone"]
    assert rz[0] == True,  "yardline_100==20 should be red_zone"
    assert rz[1] == False, "yardline_100==21 should NOT be red_zone"
    assert rz[2] == True,  "yardline_100==1 should be red_zone"
    assert rz[3] == False, "yardline_100==50 should NOT be red_zone"


# ---------------------------------------------------------------------------
# Test 2: passing_downs includes 4th down regardless of distance
# ---------------------------------------------------------------------------

def test_passing_downs_includes_4th_down():
    """Every 4th down play must be flagged as a passing_down."""
    plays = pd.DataFrame({
        "down":         [4,  4,  3,  3,  1],
        "ydstogo":      [1,  10, 7,  5,  10],
        "yardline_100": [50, 50, 50, 50, 50],
    })
    masks = classify(plays)
    pd_mask = masks["passing_downs"]
    # All 4th-down rows (indices 0,1) must be True
    assert pd_mask[0] == True,  "4th & 1 is a passing_down"
    assert pd_mask[1] == True,  "4th & 10 is a passing_down"
    # 3rd & 7 should be True; 3rd & 5 should be False
    assert pd_mask[2] == True,  "3rd & 7 is a passing_down (ydstogo >= 7)"
    assert pd_mask[3] == False, "3rd & 5 is NOT a passing_down (ydstogo < 7)"
    # 1st & 10 is not a passing_down
    assert pd_mask[4] == False, "1st & 10 is NOT a passing_down"


# ---------------------------------------------------------------------------
# Test 3: two_minute omitted when clock column is absent
# ---------------------------------------------------------------------------

def test_two_minute_omitted_when_clock_col_absent(caplog):
    """classify() must NOT include 'two_minute' key when clock col is absent,
    and must log a warning mentioning the missing column."""
    plays = _make_plays()
    assert TWO_MIN_COL not in plays.columns, "Fixture should not have clock col"

    with caplog.at_level(logging.WARNING, logger="backend.grid.situations"):
        masks = classify(plays)

    assert "two_minute" not in masks, (
        "two_minute key must be absent when quarter_seconds_remaining is missing"
    )
    # A warning mentioning the column name must have been emitted
    warning_texts = [r.message for r in caplog.records]
    assert any(TWO_MIN_COL in str(w) for w in warning_texts), (
        f"Expected a warning mentioning '{TWO_MIN_COL}', got: {warning_texts}"
    )


# ---------------------------------------------------------------------------
# Test 4: masks are numpy boolean arrays with length == len(plays)
# ---------------------------------------------------------------------------

def test_masks_are_boolean_and_length_matches_plays():
    """Every mask in the returned dict must be a numpy bool array of correct length."""
    plays = _make_plays(n=8)
    masks = classify(plays)

    assert len(masks) >= 3, "At least red_zone, passing_downs, rushing_downs expected"
    for name, mask in masks.items():
        assert isinstance(mask, np.ndarray), (
            f"Mask '{name}' must be a numpy ndarray, got {type(mask)}"
        )
        assert mask.dtype == bool, (
            f"Mask '{name}' must have bool dtype, got {mask.dtype}"
        )
        assert len(mask) == len(plays), (
            f"Mask '{name}' length {len(mask)} != plays length {len(plays)}"
        )


# ---------------------------------------------------------------------------
# Test 5: no input mutation
# ---------------------------------------------------------------------------

def test_no_input_mutation():
    """classify() must not modify the passed DataFrame (no new columns, no data change)."""
    plays = _make_plays(n=6)
    original_cols = list(plays.columns)
    original_values = plays.copy(deep=True)

    classify(plays)

    assert list(plays.columns) == original_cols, (
        "classify() must not add or remove columns from the input DataFrame"
    )
    pd.testing.assert_frame_equal(plays, original_values), (
        "classify() must not change any values in the input DataFrame"
    )
