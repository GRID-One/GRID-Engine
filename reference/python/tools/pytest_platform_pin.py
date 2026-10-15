"""pytest plugin: pin the numerical platform the oracle's golden master was frozen on.

Loaded by ``pytest.ini`` (``addopts = ... -p tools.pytest_platform_pin``). Not upstream code.

The golden master (``tests/grid/golden/snapshot.npz``) is compared at rtol 1e-5, and its
Layer C outputs (player and team ratings, weekly QB credit, the focus-QB Kalman means) run
through gradient-boosted stages whose tree splits move with kernel-level floating-point
differences. Measured in PR #4 (KI-NEW-Z78): the same pinned packages give Layer C values that
differ by up to 1.4e-2 between OpenBLAS kernels (AVX-512 ``SkylakeX`` versus ``Haswell``/``Zen``)
and between CPython 3.11 and 3.12. Correction-ledger entry L0 (``PARITY.md`` (b), DR-D31,
ratified 2026-10-08) regenerated the golden once under CPython 3.11 with
``OPENBLAS_CORETYPE=Haswell``, a kernel every x86-64 AVX2 CPU (Intel or AMD) can run, so the
golden reproduces on developer machines and on CI runners alike. This plugin makes every test
run use that platform, or refuse to start:

* ``OPENBLAS_CORETYPE`` is set to ``Haswell`` before NumPy (and so OpenBLAS) is loaded. A
  different explicit value is a usage error: it would compare against a golden frozen under
  another kernel. Tolerances are never loosened to absorb a platform difference.
* The interpreter must be CPython 3.11 (``requirements.lock`` was verified on 3.11.15).
* After NumPy loads, threadpoolctl must report OpenBLAS running the ``Haswell`` kernel.

``-p no:tools.pytest_platform_pin`` disables it for a local experiment only, never in CI; the
golden-master tests are then expected to fail on most machines.
"""
from __future__ import annotations

import os
import sys

import pytest

KERNEL = "Haswell"
PYTHON = (3, 11)

_problem: str | None = None

if "numpy" in sys.modules:
    if os.environ.get("OPENBLAS_CORETYPE") != KERNEL:
        _problem = ("NumPy was imported before tools.pytest_platform_pin could set "
                    f"OPENBLAS_CORETYPE={KERNEL}; export OPENBLAS_CORETYPE={KERNEL} and rerun")
elif os.environ.get("OPENBLAS_CORETYPE", KERNEL) != KERNEL:
    _problem = (f"OPENBLAS_CORETYPE={os.environ['OPENBLAS_CORETYPE']!r}, but the golden master is "
                f"frozen under {KERNEL} kernels (DR-D31, ledger entry L0); unset it or set {KERNEL}")
else:
    # Must happen at plugin import: `-p` plugins load during argument pre-parsing, before any
    # test module (and so NumPy/OpenBLAS) is imported, and OpenBLAS reads this only at load.
    os.environ["OPENBLAS_CORETYPE"] = KERNEL

if _problem is None and sys.version_info[:2] != PYTHON:
    _problem = (f"the oracle is pinned to CPython {PYTHON[0]}.{PYTHON[1]} (requirements.lock, "
                f"KI-NEW-Z78) but this is {sys.version.split()[0]}; e.g. `uv python install 3.11`")


def pytest_configure(config):
    if _problem is not None:
        raise pytest.UsageError(f"platform pin: {_problem}")


def pytest_sessionstart(session):
    import numpy  # noqa: F401  (loads OpenBLAS so threadpoolctl can report it)
    from threadpoolctl import threadpool_info

    blas = [i for i in threadpool_info() if i.get("internal_api") == "openblas"]
    if not blas:
        raise pytest.UsageError("platform pin: no OpenBLAS found by threadpoolctl; the golden "
                                "master is frozen under OpenBLAS Haswell kernels")
    wrong = sorted({str(i.get("architecture")) for i in blas} - {KERNEL})
    if wrong:
        raise pytest.UsageError(f"platform pin: OpenBLAS is running {', '.join(wrong)} kernels, "
                                f"not {KERNEL}, despite OPENBLAS_CORETYPE={KERNEL}")


def pytest_report_header(config):
    return (f"platform pin: CPython {PYTHON[0]}.{PYTHON[1]}, OPENBLAS_CORETYPE={KERNEL} "
            "(tools/pytest_platform_pin.py; DR-D31, ledger entry L0)")
