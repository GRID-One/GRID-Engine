from backend.scoring.engine import ScoringConfig

_BASE_RULES = {
    "passing_yards": 0.04,
    "passing_tds": 4,
    "interceptions": -2,
    "rushing_yards": 0.1,
    "rushing_tds": 6,
    "receiving_yards": 0.1,
    "receiving_tds": 6,
    "fumbles_lost": -2,
    "two_point_conversions": 2,
}

STANDARD = ScoringConfig(name="Standard", rules={**_BASE_RULES, "receptions": 0.0})
HALF_PPR = ScoringConfig(name="Half PPR", rules={**_BASE_RULES, "receptions": 0.5})
FULL_PPR = ScoringConfig(name="Full PPR", rules={**_BASE_RULES, "receptions": 1.0})
