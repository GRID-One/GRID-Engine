"""pytest plugin: enforce the reference oracle's isolation property on every test run.

Loaded by ``pytest.ini`` (``addopts = -p tools.pytest_isolation_guard``). Not upstream code:
it turns the one-off proof from the consolidation inventory (446 tests green with the app
stack absent and the network blocked) into a property every run re-checks.

What it blocks, and how a violation surfaces:

* **App-only imports.** ``backend.api``, ``backend.services``, ``backend.trades``,
  ``backend.adapters``, ``backend.viz_agent`` (the fantasy-dashboard app, deliberately not
  imported), the excluded ``backend.fantasy_scoring`` and ``backend.pipeline.sync_*``, and
  the app stack ``fastapi``, ``uvicorn``, ``anthropic``, ``posthog``, ``httpx``,
  ``websockets``, ``starlette``, ``pydantic``, ``requests``. ``matplotlib`` is blocked too: it
  is a ``run_demo.py``-only dependency (requirements-demo.txt) and no test may need it.
  A meta-path finder raises :class:`IsolationViolation` (an ``ImportError``) on the attempt.
* **Network access.** ``socket.connect`` / ``connect_ex`` / ``sendto`` to any non-loopback
  INET/INET6 address, and ``getaddrinfo`` / ``gethostbyname`` for any non-loopback host,
  raise :class:`NetworkViolation` (a ``ConnectionError``). Every nflverse fetch in the suite
  is mocked, so a real attempt is always a defect.

Because oracle code sometimes catches broad exceptions (``weekly_update.run`` turns a fetch
error into a health warning), raising is not enough. Every violation is also recorded; the
test phase in which it happened is reported as failed, a violation outside any test (at
import or collection time) fails the session, and the terminal summary lists them all. A
clean run prints ``isolation guard: 0 violations``.

Disable only for a local experiment, never in CI: ``python -m pytest -p no:tools.pytest_isolation_guard``.
"""
from __future__ import annotations

import importlib.abc
import ipaddress
import socket
import sys

import pytest

#: Excluded app-only packages and modules of the cautious-nevermore tree (README.md, "Excluded").
FORBIDDEN_BACKEND = (
    "backend.api",
    "backend.services",
    "backend.trades",
    "backend.adapters",
    "backend.viz_agent",
    "backend.fantasy_scoring",
    "backend.pipeline.sync_leagues",
    "backend.pipeline.sync_trade_history",
)
#: Third-party app stack (upstream requirements.txt minus the engine dependencies).
FORBIDDEN_APP_STACK = (
    "fastapi",
    "uvicorn",
    "anthropic",
    "posthog",
    "httpx",
    "websockets",
    "starlette",
    "pydantic",
    "requests",
)
#: Demo-only dependency (requirements-demo.txt). run_demo.py is not a test.
FORBIDDEN_DEMO_ONLY = ("matplotlib",)

_LOOPBACK_NAMES = {"localhost", "localhost.localdomain", "ip6-localhost", "ip6-loopback"}

#: Every violation as (kind, target, reason). Module-level so the hooks below can read it.
VIOLATIONS: list[tuple[str, str, str]] = []
_MARK = pytest.StashKey[int]()


class IsolationViolation(ImportError):
    """An app-only or demo-only module was imported inside the oracle test run."""


class NetworkViolation(ConnectionError):
    """A test tried to reach the network."""


def _matches(name: str, prefixes: tuple[str, ...]) -> bool:
    return any(name == p or name.startswith(p + ".") for p in prefixes)


def _forbidden_reason(name: str) -> str | None:
    if _matches(name, FORBIDDEN_BACKEND):
        return "app-only cautious-nevermore module (excluded from the oracle)"
    if _matches(name, FORBIDDEN_APP_STACK):
        return "app-stack package (not an engine dependency)"
    if _matches(name, FORBIDDEN_DEMO_ONLY):
        return "demo-only package (requirements-demo.txt); no test may need it"
    return None


def _record(kind: str, target: str, reason: str) -> None:
    VIOLATIONS.append((kind, target, reason))


class _ForbiddenImportFinder(importlib.abc.MetaPathFinder):
    """First entry on ``sys.meta_path``: refuses forbidden modules before any real finder."""

    def find_spec(self, fullname, path=None, target=None):  # noqa: D401 - finder protocol
        reason = _forbidden_reason(fullname)
        if reason is None:
            return None
        _record("import", fullname, reason)
        raise IsolationViolation(f"isolation guard: import of {fullname!r} refused: {reason}")


def _is_loopback(host) -> bool:
    if host is None:
        return True
    if isinstance(host, bytes):
        host = host.decode("ascii", "replace")
    host = str(host).strip().strip("[]").split("%", 1)[0]
    if host == "" or host.lower() in _LOOPBACK_NAMES:
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def _check_address(sock, address, what: str) -> None:
    if sock.family not in (socket.AF_INET, socket.AF_INET6):
        return
    host = address[0] if isinstance(address, tuple) and address else address
    if _is_loopback(host):
        return
    target = repr(address)
    _record("network", target, f"socket.{what}")
    raise NetworkViolation(f"isolation guard: network access refused (socket.{what} {target})")


def _check_host(host, what: str) -> None:
    if _is_loopback(host):
        return
    _record("network", repr(host), f"socket.{what}")
    raise NetworkViolation(f"isolation guard: DNS lookup refused (socket.{what} {host!r})")


def _install() -> None:
    if getattr(socket, "_grid_isolation_guard_installed", False):
        return
    socket._grid_isolation_guard_installed = True  # type: ignore[attr-defined]

    # Anything forbidden that was already imported before this plugin loaded is a violation too.
    for name in list(sys.modules):
        reason = _forbidden_reason(name)
        if reason is not None:
            _record("preloaded", name, reason)
    sys.meta_path.insert(0, _ForbiddenImportFinder())

    real_connect = socket.socket.connect
    real_connect_ex = socket.socket.connect_ex
    real_sendto = socket.socket.sendto
    real_getaddrinfo = socket.getaddrinfo
    real_gethostbyname = socket.gethostbyname
    real_gethostbyname_ex = socket.gethostbyname_ex

    def connect(self, address):
        _check_address(self, address, "connect")
        return real_connect(self, address)

    def connect_ex(self, address):
        _check_address(self, address, "connect_ex")
        return real_connect_ex(self, address)

    def sendto(self, *args):
        if args:
            _check_address(self, args[-1], "sendto")
        return real_sendto(self, *args)

    def getaddrinfo(host, *args, **kwargs):
        _check_host(host, "getaddrinfo")
        return real_getaddrinfo(host, *args, **kwargs)

    def gethostbyname(host):
        _check_host(host, "gethostbyname")
        return real_gethostbyname(host)

    def gethostbyname_ex(host):
        _check_host(host, "gethostbyname_ex")
        return real_gethostbyname_ex(host)

    socket.socket.connect = connect
    socket.socket.connect_ex = connect_ex
    socket.socket.sendto = sendto
    socket.getaddrinfo = getaddrinfo
    socket.gethostbyname = gethostbyname
    socket.gethostbyname_ex = gethostbyname_ex


# Install at plugin import: `-p` plugins load during argument pre-parsing, before conftest
# files and before any test module is collected or imported.
_install()


def pytest_report_header(config):
    return ("isolation guard: network and app-only/demo-only imports blocked "
            "(tools/pytest_isolation_guard.py)")


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_protocol(item, nextitem):
    item.stash[_MARK] = len(VIOLATIONS)


@pytest.hookimpl(wrapper=True)
def pytest_runtest_makereport(item, call):
    report = yield
    start = item.stash.get(_MARK, len(VIOLATIONS))
    new = VIOLATIONS[start:]
    item.stash[_MARK] = len(VIOLATIONS)
    if new and not report.failed:
        report.outcome = "failed"
        report.longrepr = "isolation guard violation(s) during {}:\n{}".format(
            report.when, "\n".join(f"  {k}: {t} ({r})" for k, t, r in new))
    return report


def pytest_sessionfinish(session, exitstatus):
    for name in list(sys.modules):
        reason = _forbidden_reason(name)
        if reason is not None and not any(t == name for _, t, _r in VIOLATIONS):
            _record("loaded", name, reason)
    if VIOLATIONS and session.exitstatus == pytest.ExitCode.OK:
        session.exitstatus = pytest.ExitCode.TESTS_FAILED


def pytest_terminal_summary(terminalreporter, exitstatus, config):
    if not VIOLATIONS:
        terminalreporter.write_line("isolation guard: 0 violations")
        return
    terminalreporter.section("isolation guard violations", sep="=", red=True, bold=True)
    for kind, target, reason in VIOLATIONS:
        terminalreporter.write_line(f"{kind}: {target} ({reason})", red=True)
    terminalreporter.write_line(f"isolation guard: {len(VIOLATIONS)} violation(s); session failed",
                                red=True, bold=True)
