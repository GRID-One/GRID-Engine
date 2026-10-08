import pytest
from backend.scoring.engine import ScoringConfig, calculate_points
from backend.scoring.formats import STANDARD, HALF_PPR, FULL_PPR


def test_standard_scoring_qb():
    stats = {"passing_yards": 300, "passing_tds": 3, "interceptions": 1}
    pts = calculate_points(stats, STANDARD)
    assert pts == pytest.approx(300 * 0.04 + 3 * 4 + 1 * (-2))  # 12 + 12 - 2 = 22


def test_half_ppr_wr():
    stats = {"receptions": 6, "receiving_yards": 90, "receiving_tds": 1}
    pts = calculate_points(stats, HALF_PPR)
    assert pts == pytest.approx(6 * 0.5 + 90 * 0.1 + 1 * 6)  # 3 + 9 + 6 = 18


def test_full_ppr_wr():
    stats = {"receptions": 6, "receiving_yards": 90, "receiving_tds": 1}
    pts = calculate_points(stats, FULL_PPR)
    assert pts == pytest.approx(6 * 1.0 + 90 * 0.1 + 1 * 6)  # 6 + 9 + 6 = 21


def test_empty_stats():
    pts = calculate_points({}, STANDARD)
    assert pts == 0.0


def test_custom_config():
    config = ScoringConfig(name="4pt pass TD", rules={"passing_tds": 4, "passing_yards": 0.04})
    stats = {"passing_yards": 250, "passing_tds": 2}
    pts = calculate_points(stats, config)
    assert pts == pytest.approx(250 * 0.04 + 2 * 4)


def test_config_to_json_roundtrip():
    import json
    config = HALF_PPR
    j = config.to_json()
    restored = ScoringConfig.from_json(j)
    assert restored.name == config.name
    assert restored.rules == config.rules
