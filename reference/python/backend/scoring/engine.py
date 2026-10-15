from __future__ import annotations
import json
from dataclasses import dataclass, field


@dataclass
class ScoringConfig:
    name: str
    rules: dict[str, float] = field(default_factory=dict)

    def to_json(self) -> str:
        return json.dumps({"name": self.name, "rules": self.rules})

    @classmethod
    def from_json(cls, raw: str) -> ScoringConfig:
        data = json.loads(raw)
        return cls(name=data["name"], rules=data["rules"])


def calculate_points(stats: dict, config: ScoringConfig) -> float:
    total = 0.0
    for stat_key, multiplier in config.rules.items():
        total += stats.get(stat_key, 0) * multiplier
    return total
