"""Download the exact nflverse 2023 inputs the real-data investigations used, sha256-pinned.

What it does
    Fetches five public nflverse release assets into ``reference/python/data/realdata/``
    (gitignored), verifies each file's full sha256 against the value recorded when the
    consolidation inventory ran (2026-10-01), and only then moves it into place. A file
    already present with the right hash is skipped. On a hash mismatch the download is
    discarded and the script exits 1: nflverse re-publishes release assets, and the outputs
    recorded in ``cmp_stats.py``, ``real_contract.py`` and ``real_rapm.py`` belong to these
    exact bytes only.
Evidence for
    The inputs of KI-NEW-I1..KI-NEW-I5, KI-NEW-V0a, KI-NEW-V0b, KI-NEW-A3 and KI-NEW-P4.
Real data, never committed
    Third-party data stays out of the repository (``*.parquet`` and ``/data/`` are ignored in
    reference/python/.gitignore). Participation is CC-BY-SA 4.0 ("FTN Data via nflverse" from
    2023 on; "NFL NextGen Stats via nflverse" for 2022 and earlier); see
    docs/04-providers/nflverse/access-and-license.md and DR-A11 (fixture licensing policy,
    pending the owner). Real-data results are historical and non-parity (README.md).
Run (from reference/python; network; about 27 MB)
    python3 -m tools.investigations.fetch_realdata             # fetch missing, verify all
    python3 -m tools.investigations.fetch_realdata --verify-only
Expected output (recorded 2026-10-01)
    one "ok" line per file (fetched or already present), then
    fetch_realdata: OK (5 files verified in .../reference/python/data/realdata)
Origin
    The inventory's ``scratch-cnissues/realdata/`` downloads (critic report G-2 lists the
    hashes in abbreviated form; the full values below were recomputed from those files and
    re-checked against fresh downloads of the same URLs on 2026-10-01).
"""
from __future__ import annotations

import argparse
import hashlib
import os
import pathlib
import sys
import tempfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]  # reference/python
DEFAULT_DEST = ROOT / "data" / "realdata"
RELEASES = "https://github.com/nflverse/nflverse-data/releases/download"

#: local name -> (url, sha256, size in bytes, what reads it)
PINNED = {
    "pbp_2023.parquet": (
        f"{RELEASES}/pbp/play_by_play_2023.parquet",
        "bd3484731408def6b0ec93225bba2bd7b2c65769ca707a2b9444d891abdc6776", 20534088,
        "cmp_stats.py, real_contract.py, real_rapm.py (same URL template as nflverse_loader.NFLVERSE_PBP_URL)"),
    "part_2023.parquet": (
        f"{RELEASES}/pbp_participation/pbp_participation_2023.parquet",
        "b157736972248e6f71e0c5ad6c1010b71375a500a63944dc4956c65de263e5a6", 4700238,
        "real_contract.py, real_rapm.py (nflverse_loader.NFLVERSE_PARTICIPATION_URL)"),
    "roster_2023.parquet": (
        f"{RELEASES}/rosters/roster_2023.parquet",
        "66dcb7d0e203c41b49e997b93eb55ed77bcc960e5dd0774b7e2426a87cef3c90", 554634,
        "real_contract.py, real_rapm.py (nflverse_loader.NFLVERSE_ROSTER_URL)"),
    "player_stats_2023.parquet": (
        f"{RELEASES}/player_stats/player_stats_2023.parquet",
        "94673091ea041bc4bddd7848f813abcf0f9be085540e3f187fb6dd1b1dacf2c9", 332923,
        "cmp_stats.py (official weekly player stats, the comparison baseline)"),
    "stats_player_week_2023.parquet": (
        f"{RELEASES}/stats_player/stats_player_week_2023.parquet",
        "ac776fbd7c9fefdb4069b72304f4c5fa05ae417acd49f6e4e27be14b9d7a4bbc", 846050,
        "no committed script; downloaded by the inventory alongside the others, kept for completeness"),
}


def sha256_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def fetch(url: str, dest: pathlib.Path, timeout: float) -> pathlib.Path:
    """Download ``url`` next to ``dest`` as a ``.part`` file and return its path."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=dest.name + ".", suffix=".part", dir=dest.parent)
    try:
        with os.fdopen(fd, "wb") as out, urllib.request.urlopen(url, timeout=timeout) as resp:
            while True:
                chunk = resp.read(1 << 20)
                if not chunk:
                    break
                out.write(chunk)
    except BaseException:
        pathlib.Path(tmp).unlink(missing_ok=True)
        raise
    return pathlib.Path(tmp)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--dest", type=pathlib.Path, default=DEFAULT_DEST,
                    help=f"target directory (default: {DEFAULT_DEST})")
    ap.add_argument("--verify-only", action="store_true", help="check files already present; no network")
    ap.add_argument("--timeout", type=float, default=120.0, help="per-request timeout, seconds")
    args = ap.parse_args(argv)
    dest_dir = args.dest.resolve()
    failures = 0
    for name, (url, digest, size, used_by) in PINNED.items():
        target = dest_dir / name
        if target.is_file() and sha256_file(target) == digest:
            print(f"ok       {name} (already present; {size} bytes; used by {used_by})")
            continue
        if args.verify_only:
            state = "missing" if not target.exists() else "sha256 mismatch"
            print(f"FAIL     {name}: {state}", file=sys.stderr)
            failures += 1
            continue
        try:
            staged = fetch(url, target, args.timeout)
        except OSError as exc:
            print(f"FAIL     {name}: download error from {url}: {exc}", file=sys.stderr)
            failures += 1
            continue
        got = sha256_file(staged)
        if got != digest:
            staged.unlink(missing_ok=True)
            print(f"FAIL     {name}: sha256 {got} != pinned {digest} ({url}); nflverse re-published "
                  "the asset, so the recorded investigation outputs no longer apply to it",
                  file=sys.stderr)
            failures += 1
            continue
        os.replace(staged, target)
        print(f"ok       {name} (fetched; {size} bytes; used by {used_by})")
    if failures:
        print(f"fetch_realdata: FAIL ({failures} of {len(PINNED)} files)", file=sys.stderr)
        return 1
    print(f"fetch_realdata: OK ({len(PINNED)} files verified in {dest_dir})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
