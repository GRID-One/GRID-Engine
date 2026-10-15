"""Validation utilities (sniff tests, and — later — the walk-forward backtest /
leakage guards per the retrospective-validation plan §11)."""
from __future__ import annotations

from backend.validation.sniff import SniffResult, check_rank_bands

__all__ = ["SniffResult", "check_rank_bands"]
