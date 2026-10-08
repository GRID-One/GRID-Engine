"""Verify reference/python against MANIFEST.tsv (provenance of the imported oracle).

MANIFEST.tsv has one row per file imported from cautious-nevermore, with paths relative to
reference/python:

    dest_path  source_path  source_commit  source_sha256  dest_sha256  status

``status`` is ``verbatim`` or ``patched:<ID>``; each patch ID has exactly one
``patches/<ID>-*.patch`` (a unified diff against the upstream file).

Checks, always (no network, no upstream checkout needed):

1. the header is exactly the six columns above, and every row is well formed;
2. every dest file exists and its sha256 equals ``dest_sha256``;
3. a ``verbatim`` row has ``source_sha256 == dest_sha256``; a ``patched:<ID>`` row has them
   different and a matching patch file;
4. every file under ``backend/`` and ``tests/``, plus ``run_demo.py``, is listed: nothing
   can be added to the oracle tree without a manifest row (bytecode caches are ignored).

With ``--upstream <path to a cautious-nevermore clone>`` it also re-derives provenance:

5. ``git show <source_commit>:<source_path>`` hashes to ``source_sha256``;
6. applying the row's patch to those upstream bytes reproduces the dest file exactly.

Usage, from reference/python:

    python3 -m tools.verify_manifest
    python3 -m tools.verify_manifest --upstream ../../../cautious-nevermore

Exit status: 0 all checks pass; 1 at least one mismatch; 2 usage or environment error.
Stdlib only, so it runs without the oracle's dependencies installed.

Line endings: the hashes are of the committed LF bytes. A Windows checkout with
``core.autocrlf=true`` rewrites them and every check 2 fails; the oracle is Linux-only
(README.md), and that failure is the intended signal.
"""
from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]  # reference/python
MANIFEST = ROOT / "MANIFEST.tsv"
PATCHES = ROOT / "patches"
HEADER = ["dest_path", "source_path", "source_commit", "source_sha256", "dest_sha256", "status"]
COVERED_DIRS = ("backend", "tests")
COVERED_FILES = ("run_demo.py",)
IGNORED_PARTS = {"__pycache__", ".pytest_cache"}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_rows(errors: list[str]) -> list[dict[str, str]]:
    lines = MANIFEST.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0].split("\t") != HEADER:
        errors.append(f"MANIFEST.tsv header must be: {chr(9).join(HEADER)}")
        return []
    rows = []
    for n, line in enumerate(lines[1:], start=2):
        cols = line.split("\t")
        if len(cols) != len(HEADER):
            errors.append(f"MANIFEST.tsv line {n}: expected {len(HEADER)} columns, got {len(cols)}")
            continue
        rows.append(dict(zip(HEADER, cols, strict=True)))
    return rows


def patch_file(patch_id: str) -> pathlib.Path | None:
    hits = sorted(PATCHES.glob(f"{patch_id}-*.patch"))
    return hits[0] if len(hits) == 1 else None


def check_local(rows: list[dict[str, str]], errors: list[str]) -> None:
    seen = set()
    for r in rows:
        dest = r["dest_path"]
        if dest in seen:
            errors.append(f"{dest}: listed twice")
        seen.add(dest)
        for col in ("source_sha256", "dest_sha256"):
            if len(r[col]) != 64 or any(c not in "0123456789abcdef" for c in r[col]):
                errors.append(f"{dest}: {col} is not a full lowercase sha256")
        path = ROOT / dest
        if not path.is_file():
            errors.append(f"{dest}: missing")
            continue
        actual = sha256(path.read_bytes())
        if actual != r["dest_sha256"]:
            errors.append(f"{dest}: dest sha256 {actual} != manifest {r['dest_sha256']}")
        status = r["status"]
        if status == "verbatim":
            if r["source_sha256"] != r["dest_sha256"]:
                errors.append(f"{dest}: status verbatim but source and dest hashes differ")
        elif status.startswith("patched:"):
            if r["source_sha256"] == r["dest_sha256"]:
                errors.append(f"{dest}: status {status} but source and dest hashes are equal")
            if patch_file(status.split(":", 1)[1]) is None:
                errors.append(f"{dest}: no unique patches/{status.split(':', 1)[1]}-*.patch")
        else:
            errors.append(f"{dest}: unknown status {status!r}")

    on_disk = {str(p.relative_to(ROOT)) for d in COVERED_DIRS for p in (ROOT / d).rglob("*")
               if p.is_file() and not IGNORED_PARTS.intersection(p.parts) and p.suffix != ".pyc"}
    on_disk |= {f for f in COVERED_FILES if (ROOT / f).is_file()}
    for extra in sorted(on_disk - seen):
        errors.append(f"{extra}: in the oracle tree but not in MANIFEST.tsv")


def check_upstream(rows: list[dict[str, str]], upstream: pathlib.Path, errors: list[str]) -> None:
    if not (upstream / ".git").exists():
        raise SystemExit(f"error: --upstream {upstream} is not a git checkout")
    git = shutil.which("git")
    if git is None:
        raise SystemExit("error: git not found on PATH")
    for r in rows:
        show = subprocess.run([git, "-C", str(upstream), "show", f"{r['source_commit']}:{r['source_path']}"],
                              capture_output=True)
        if show.returncode != 0:
            errors.append(f"{r['dest_path']}: upstream {r['source_commit']}:{r['source_path']} unreadable")
            continue
        if sha256(show.stdout) != r["source_sha256"]:
            errors.append(f"{r['dest_path']}: upstream bytes do not hash to source_sha256")
            continue
        if not r["status"].startswith("patched:"):
            continue
        pf = patch_file(r["status"].split(":", 1)[1])
        if pf is None:
            continue  # already reported by check_local
        with tempfile.TemporaryDirectory() as tmp:
            target = pathlib.Path(tmp) / r["source_path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(show.stdout)
            applied = subprocess.run([git, "apply", "--unidiff-zero", "-p1", str(pf.resolve())],
                                     cwd=tmp, capture_output=True, text=True)
            if applied.returncode != 0:
                errors.append(f"{r['dest_path']}: {pf.name} does not apply to upstream: {applied.stderr.strip()}")
                continue
            if sha256(target.read_bytes()) != r["dest_sha256"]:
                errors.append(f"{r['dest_path']}: upstream + {pf.name} != dest file")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--upstream", type=pathlib.Path,
                    help="cautious-nevermore git checkout: also verify source hashes and patches")
    args = ap.parse_args(argv)
    if not MANIFEST.is_file():
        print(f"error: {MANIFEST} not found", file=sys.stderr)
        return 2
    errors: list[str] = []
    rows = load_rows(errors)
    check_local(rows, errors)
    if args.upstream is not None:
        check_upstream(rows, args.upstream.resolve(), errors)
    if errors:
        for e in errors:
            print(f"MISMATCH {e}", file=sys.stderr)
        print(f"verify_manifest: FAIL ({len(errors)} problem(s), {len(rows)} rows)", file=sys.stderr)
        return 1
    n_verbatim = sum(r["status"] == "verbatim" for r in rows)
    scope = "local + upstream" if args.upstream is not None else "local"
    print(f"verify_manifest: OK ({len(rows)} rows: {n_verbatim} verbatim, "
          f"{len(rows) - n_verbatim} patched; checks: {scope})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
