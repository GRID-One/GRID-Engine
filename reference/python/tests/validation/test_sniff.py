"""Tests for the known-player rank-band sniff gate (Phase 1 / #9)."""
import pandas as pd
import pytest

from backend.validation.sniff import check_rank_bands


def _ranking(order):
    """list[dict] ranking (rank 1 = best) from an ordered list of player ids."""
    return [{"player_id": pid, "rank": i + 1} for i, pid in enumerate(order)]


def test_passes_when_elites_lead():
    """Elites occupying the top spots pass both band checks."""
    order = ["A", "B", "C", "D", "E"] + [f"x{i}" for i in range(30)]
    res = check_rank_bands(_ranking(order), elites=["A", "B", "C"],
                           top_k=5, min_in_top_k=3, max_rank=24)
    assert res.passed and not res.failures
    assert res.in_top_k == 3


def test_fails_when_too_few_in_top_k():
    """Too few elites inside the top band trips the top-k failure."""
    order = ["x0", "x1", "A", "x2", "x3", "B", "C"]   # only A in top-5... actually none
    res = check_rank_bands(_ranking(order), elites=["A", "B", "C"],
                           top_k=2, min_in_top_k=2)
    assert not res.passed
    assert any("in top 2" in f for f in res.failures)


def test_fails_when_elite_outside_ceiling():
    """An elite ranked past the ceiling trips the max-rank failure."""
    order = ["A", "B", "C"] + [f"x{i}" for i in range(30)] + ["D"]   # D at rank 34
    res = check_rank_bands(_ranking(order), elites=["A", "B", "C", "D"],
                           top_k=5, min_in_top_k=3, max_rank=24)
    assert not res.passed
    assert any("worse than 24" in f for f in res.failures)


def test_reports_missing_elites():
    """An elite absent from the ranking is reported and fails the gate."""
    res = check_rank_bands(_ranking(["A", "B"]), elites=["A", "B", "GHOST"],
                           top_k=5, min_in_top_k=2, max_rank=24)
    assert "GHOST" in res.missing
    assert any("absent" in f for f in res.failures)
    assert not res.passed


def test_accepts_dataframe_and_coerces_ids():
    """A DataFrame ranking with int ids is accepted; ids coerce to str."""
    df = pd.DataFrame({"player_id": [10, 20, 30], "rank": [1, 2, 3]})
    # elite ids given as ints; checker coerces both sides to str
    res = check_rank_bands(df, elites=[10, 20], top_k=3, min_in_top_k=2, max_rank=24)
    assert res.passed


def test_accepts_literal_id_rank_rows():
    """A list of 2-item [id, rank] sequences (not dicts) is normalized and accepted."""
    rows = [("A", 1), ("B", 2), ("C", 3), ("x0", 4), ("x1", 5)]
    res = check_rank_bands(rows, elites=["A", "B", "C"],
                           top_k=5, min_in_top_k=3, max_rank=24)
    assert res.passed and res.in_top_k == 3


def test_max_rank_none_skips_ceiling_check():
    """max_rank=None disables the ceiling check so a far-down elite still passes."""
    order = ["A", "B", "C"] + [f"x{i}" for i in range(50)] + ["D"]
    res = check_rank_bands(_ranking(order), elites=["A", "B", "C", "D"],
                           top_k=5, min_in_top_k=3, max_rank=None)
    assert res.passed   # D far down, but ceiling check disabled
