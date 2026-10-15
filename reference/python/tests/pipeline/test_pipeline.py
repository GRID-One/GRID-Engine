import pytest
import json
from pathlib import Path
from backend.pipeline.health_check import write_health, read_health


def test_health_write_and_read(tmp_path):
    health_file = tmp_path / "health.json"
    write_health(str(health_file), status="ok", message="Pipeline complete")
    result = read_health(str(health_file))
    assert result["status"] == "ok"
    assert result["message"] == "Pipeline complete"
    assert "timestamp" in result


def test_health_missing_file(tmp_path):
    result = read_health(str(tmp_path / "nope.json"))
    assert result["status"] == "unknown"
