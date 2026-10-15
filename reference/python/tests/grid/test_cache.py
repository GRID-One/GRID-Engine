import pandas as pd
import pytest
from backend.grid.cache import ParquetCache


@pytest.fixture
def cache(tmp_path):
    return ParquetCache(str(tmp_path))


def test_put_and_get(cache):
    df = pd.DataFrame({"a": [1, 2, 3], "b": [4, 5, 6]})
    cache.put("test_key", df)
    result = cache.get("test_key", ttl_hours=1)
    assert result is not None
    loaded = pd.read_parquet(result)
    assert list(loaded.columns) == ["a", "b"]
    assert len(loaded) == 3


def test_get_missing(cache):
    assert cache.get("nonexistent", ttl_hours=1) is None


def test_ttl_expired(cache):
    import time
    df = pd.DataFrame({"x": [1]})
    cache.put("old", df)
    assert cache.get("old", ttl_hours=0) is None
