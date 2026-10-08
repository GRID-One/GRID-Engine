from __future__ import annotations
import time
from pathlib import Path
import pandas as pd


class ParquetCache:
    def __init__(self, cache_dir: str):
        self.dir = Path(cache_dir)
        self.dir.mkdir(parents=True, exist_ok=True)

    def _path(self, key: str) -> Path:
        safe_key = key.replace("/", "_").replace(":", "_")
        return self.dir / f"{safe_key}.parquet"

    def get(self, key: str, ttl_hours: float) -> Path | None:
        p = self._path(key)
        if not p.exists():
            return None
        age_hours = (time.time() - p.stat().st_mtime) / 3600
        if age_hours > ttl_hours:
            return None
        return p

    def put(self, key: str, df: pd.DataFrame) -> Path:
        p = self._path(key)
        df.to_parquet(p, index=False)
        return p
