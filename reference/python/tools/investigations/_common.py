"""Shared helpers for tools/investigations (not upstream code; never imported by the oracle).

* :func:`bootstrap` puts ``reference/python`` first on ``sys.path`` (so ``backend`` is the
  imported oracle, not some other checkout), makes it the working directory (the oracle
  writes ``data/logs``, ``data/health.json`` ... relative to the cwd, and ``data/`` is
  gitignored there), and pins BLAS/OpenMP threads to 1 unless already set (the oracle's
  determinism and golden gates are single-threaded; multithreading also oversubscribes the
  4-vCPU runners badly). Call it before importing numpy or ``backend``.
* :func:`apply_defender_fix` turns the oracle's legacy synthetic generator into the
  **defender-fixed** generator **in this process only**, without touching
  ``backend/grid/synth.py`` (KI-NEW-Y0; critic G-1).
* :func:`realdata` resolves an nflverse 2023 input downloaded by ``fetch_realdata.py``.

The defender fix
----------------
``backend/grid/synth.py:193`` (cautious-nevermore @ 59bce1d) reads::

    off_pl, def_pl = _pick_onfield(tidx[off_team], rng)

so every play's "defenders" are the offense's *own* DEF players (100% of canonical plays;
0% from ``def_team``). The fixed generator is the one-line change measured by the
consolidation inventory (critic ``scratch-critic/cnfix``, reconcile-spec-first ``cnfix``)::

    off_pl, _unused = _pick_onfield(tidx[off_team], rng)
    _unused, def_pl = _pick_onfield(tidx[def_team], rng)

It draws twice per play, so the RNG stream (and every downstream number) changes. It does
**not** redefine the planted ``team_strength`` (still mean off - mean DEF starters, the
legacy convention), exactly like the inventory's fixed copy; the corrected planted
net-strength definition is a separate, pending decision (DR-B4 / DR-B5).

Mechanism: read the oracle's own ``synth.py`` source, require the legacy line exactly once,
substitute the two lines, compile only the patched ``simulate`` definition, and swap its
``__code__`` into the oracle's ``backend.grid.synth.simulate`` function object. Swapping the code object (rather
than rebinding the name) also reaches every module that already did
``from backend.grid.synth import simulate`` (``data_adapters.load_synthetic``, the tests).
If the oracle's ``synth.py`` ever changes so the legacy line is gone, this raises instead of
silently running the legacy generator.
"""
from __future__ import annotations

import __future__
import ast
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]  # reference/python
REALDATA_DIR = ROOT / "data" / "realdata"

LEGACY_DEFENDER_LINE = "off_pl, def_pl = _pick_onfield(tidx[off_team], rng)"
FIXED_DEFENDER_LINES = (
    "off_pl, _unused = _pick_onfield(tidx[off_team], rng)",
    "_unused, def_pl = _pick_onfield(tidx[def_team], rng)",
)
SYNTH_VARIANTS = ("legacy", "fixed")
_THREAD_VARS = ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS")


def bootstrap() -> None:
    """Make the imported oracle importable, the cwd its root, and BLAS single-threaded."""
    for var in _THREAD_VARS:
        os.environ.setdefault(var, "1")
    root = str(ROOT)
    if root in sys.path:
        sys.path.remove(root)
    sys.path.insert(0, root)
    os.chdir(ROOT)


def apply_defender_fix() -> None:
    """Patch ``backend.grid.synth.simulate`` in memory so defenders come from ``def_team``."""
    import backend.grid.synth as synth

    if getattr(synth.simulate, "_grid_defender_fixed", False):
        return
    path = pathlib.Path(synth.__file__)
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    hits = [i for i, line in enumerate(lines) if line.strip() == LEGACY_DEFENDER_LINE]
    if len(hits) != 1:
        raise RuntimeError(
            f"defender fix: expected the legacy line exactly once in {path}, found {len(hits)}; "
            "the oracle's synth.py changed, so this investigation helper must be revisited")
    i = hits[0]
    indent = lines[i][: len(lines[i]) - len(lines[i].lstrip())]
    lines[i:i + 1] = [indent + fixed + "\n" for fixed in FIXED_DEFENDER_LINES]
    # Compile only the patched ``simulate`` definition (re-executing the whole module would
    # re-run its dataclass definitions outside sys.modules). The code object reads the
    # oracle module's globals once it is swapped into the original function object.
    tree = ast.parse("".join(lines), filename=str(path))
    defs = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "simulate"]
    if len(defs) != 1:
        raise RuntimeError(f"defender fix: expected one top-level simulate() in {path}")
    code = compile(ast.Module(body=defs, type_ignores=[]), f"{path} [defender-fixed in memory]", "exec",
                   flags=__future__.annotations.compiler_flag, dont_inherit=True)
    namespace: dict = {}
    exec(code, namespace)
    synth.simulate.__code__ = namespace["simulate"].__code__
    synth.simulate._grid_defender_fixed = True


def select_synth(variant: str) -> str:
    """Apply the requested generator variant (``legacy`` = the oracle as imported)."""
    if variant not in SYNTH_VARIANTS:
        raise SystemExit(f"--synth must be one of {SYNTH_VARIANTS}, got {variant!r}")
    if variant == "fixed":
        apply_defender_fix()
    return variant


def add_synth_arg(parser) -> None:
    parser.add_argument(
        "--synth", choices=SYNTH_VARIANTS, default="legacy",
        help="synthetic generator: 'legacy' = oracle as imported (defender bug, KI-NEW-Y0); "
             "'fixed' = defenders drawn from def_team, in memory only (default: legacy)")


def defender_shares(plays, players) -> tuple[float, float]:
    """Fraction of plays whose defenders all belong to (off_team, def_team)."""
    team = dict(zip(players.player_id, players.team, strict=True))
    n = len(plays)
    from_off = sum(all(team[p] == o for p in dp) for o, dp in zip(plays.off_team, plays.def_players, strict=True))
    from_def = sum(all(team[p] == d for p in dp) for d, dp in zip(plays.def_team, plays.def_players, strict=True))
    return from_off / n, from_def / n


def banner(script: str, variant: str | None = None) -> None:
    """Print which ``backend`` was imported (``./backend`` = this oracle) and which generator."""
    import backend
    import backend.grid.synth as synth

    pkg = pathlib.Path(backend.__file__).resolve().parent
    where = f"./{pkg.relative_to(ROOT)}" if pkg.is_relative_to(ROOT) else str(pkg)
    fixed = getattr(synth.simulate, "_grid_defender_fixed", False)
    gen = "defender-fixed (in memory)" if fixed else "legacy (oracle as imported, KI-NEW-Y0)"
    shown = f" | requested --synth {variant}" if variant else ""
    print(f"[{script}] backend imported from {where} | synth generator: {gen}{shown}")


def realdata(name: str) -> pathlib.Path:
    """Path of a fetched nflverse input; tells the user how to fetch it if missing."""
    path = REALDATA_DIR / name
    if not path.is_file():
        raise SystemExit(
            f"missing {path}\n"
            "fetch the pinned nflverse 2023 inputs first (network, about 27 MB):\n"
            "  python3 -m tools.investigations.fetch_realdata")
    return path
