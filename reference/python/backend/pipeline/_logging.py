"""Shared logging configuration for all pipeline scripts."""
from __future__ import annotations

import logging
import sys
from pathlib import Path

LOG_DIR = Path("data/logs")
LOG_FILE = LOG_DIR / "pipeline.log"


def get_logger(name: str) -> logging.Logger:
    """Return a logger that writes to both stdout and data/logs/pipeline.log."""
    LOG_DIR.mkdir(parents=True, exist_ok=True)

    logger = logging.getLogger(name)
    if logger.handlers:
        return logger  # already configured

    logger.setLevel(logging.DEBUG)

    fmt = logging.Formatter(
        "%(asctime)s %(levelname)-8s [%(name)s] %(message)s",
        datefmt="%Y-%m-%dT%H:%M:%S",
    )

    # File handler — append so runs accumulate
    fh = logging.FileHandler(LOG_FILE, encoding="utf-8")
    fh.setLevel(logging.DEBUG)
    fh.setFormatter(fmt)

    # Console handler
    ch = logging.StreamHandler(sys.stdout)
    ch.setLevel(logging.INFO)
    ch.setFormatter(fmt)

    logger.addHandler(fh)
    logger.addHandler(ch)
    return logger
