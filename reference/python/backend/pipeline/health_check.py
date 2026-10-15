"""Pipeline health check utilities.

Write and read a JSON health file so external monitors can check pipeline status.

Usage:
    from backend.pipeline.health_check import write_health, read_health

    write_health(status="ok", message="Pipeline completed successfully")
    result = read_health()  # {"status": "ok", "message": ..., "timestamp": ...}
"""
from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_HEALTH_PATH = "data/health.json"


def write_health(
    path: str = DEFAULT_HEALTH_PATH,
    *,
    status: str,
    message: str,
) -> None:
    """Write pipeline health status to a JSON file.

    Args:
        path: File path to write health status to.
        status: Pipeline status string (e.g. "ok", "error").
        message: Human-readable status message.
    """
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    data = {
        "status": status,
        "message": message,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    p.write_text(json.dumps(data, indent=2))


def read_health(path: str = DEFAULT_HEALTH_PATH) -> dict:
    """Read pipeline health status from a JSON file.

    Args:
        path: File path to read health status from.

    Returns:
        Dict with keys ``status``, ``message``, and ``timestamp``.
        If the file does not exist, returns ``{"status": "unknown", ...}``.
    """
    p = Path(path)
    if not p.exists():
        return {"status": "unknown", "message": "No health file found", "timestamp": None}
    return json.loads(p.read_text())


if __name__ == "__main__":
    print(json.dumps(read_health(), indent=2))
