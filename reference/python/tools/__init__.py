"""Oracle-side tooling for reference/python (not upstream code).

Holds the pytest isolation guard, the MANIFEST.tsv verifier and the investigation
scripts. Kept a regular package so ``-p tools.pytest_isolation_guard`` (pytest.ini) and
``python -m tools.<module>`` resolve from this directory without a root conftest.
"""
