# GRID-Engine bootstrap infrastructure: inventory for the engine-only pivot and `reference/python/`

**Repo state inventoried:** `/home/user/GRID-Engine`, branch `claude/grid-engine-consolidation-e7kmh9` at
`3823478` (identical to PR #1 head `wp/P1-00-repo-bootstrap`). `origin/main` is `48ee320`, the initial upload.
PR #1 is **not merged**. Its base is `main`, and PRs #1, #2 and #3 are all still open against `main`.
The remote copy of the consolidation branch still points at `48ee320`, so only the local branch has
been fast-forwarded.

**Method:** every tracked file was read. All work ran in a scratch clone at
`/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-infra/ge`, and nothing was
modified in either source repo (`git status` is clean in both, re-checked at the end). The scratch clone has a branch
`sim/engine-pivot` that carries a **simulated minimal pivot** (section 4). Downstream agents can inspect it or reuse it.

---

## 0. Key findings

1. **The minimal pivot passes everything except one CI step.** I removed `app/`, `crates/ffi/`,
   `toolchains/flutter.version`, the FRB workspace dependency and the `test-ffi` step from all three verify
   implementations. I added `reference/python/` (the engine subset of cautious-nevermore), extended `_typos.toml`,
   added Python ignores, and wrote an ADR-011 that names every touched path. In the scratch copy, `just verify`
   exited 0, `pwsh scripts/verify.ps1 -Scope Full` exited 0, `tests/guards/run.sh` reported **54/54**, and parity
   reported **15 steps**. Both base scenarios were tested: `origin/main`, and the P1-00 tip `3823478`.
2. **`scripts/check-evidence-claims.sh` will fail the CI `guards` job on the pivot PR, and on any later
   non-trivial PR.** It hard-codes `WP="P1-00"` (line 29) and the literal `/16` (line 79). On the simulated pivot it
   fails 5 of 9 claims: parity 16→15, recipes 9→8, Cargo.lock crates 172→171, Assert-Ok 16→15, and traced paths
   68→176 against `origin/main` or 120 against `3823478`. The defect is latent and predates the pivot, which is just
   the first PR to hit it. A minimal, backward-compatible fix is given in §3.6 and prototyped. It still passes 9/9 on
   the P1-00 record.
3. **`typos` fails on the imported Python** unless `_typos.toml` is extended. There are about 150 hits:
   `vor`/`VOR` (Value Over Replacement, 133), `ot_*`, `fo_*`, `pn`, `yhat`, and one prose `mis-slice`. The
   cautious-nevermore docs add another 23 `VOR`/`vor` hits and one `mis`. The exact allowlist is in §3.10.
   **`check-secrets` is clean** on the full cautious-nevermore tree: 0 deny-tier and 0 warn-tier matches.
4. **Python caches break `verify` locally unless they are gitignored.** I hit this during testing. Untracked
   `__pycache__/*.pyc` under `reference/python/` failed `check-traceability` (103 of 171 paths untraced) on a branch
   without Python ignores, because the guard counts untracked files. `.gitignore` must add `__pycache__/`,
   `*.py[cod]`, `.pytest_cache/` and `/reference/python/data/`.
5. **Traceability for the pivot PR is satisfiable without changing the guard.** At least one **commit message**
   in the range must cite `ADR-0NN` (or `P1-NN`, or a full `docs/01-work-packages/<file>.md` path), and the cited
   document must name every non-trivial path, **deleted paths included**. `PR_BODY` alone is not enough, because
   `linux-smoke` and `windows-authoritative` run the guard with no `PR_BODY`. The exact matching rules are in §3.3.
6. **Round-4 review: two findings still open.** **R4-1 (major)** and **R4-2 (minor)**. I reproduced both at
   `3823478`, and §5 gives a prototyped R4-1 fix that keeps all 54 cases green. The four review documents should
   be imported verbatim into `docs/06-sessions/`, which also resolves the open `05-sessions` collision.
7. **Removing `test-ffi` reduces the frozen verify chain.** ADR-007, ADR-008 and ADR-009 say a coverage-reducing
   D5 amendment "should be refused outright". The removal is legitimate only as an **owner-approved de-scoping
   ADR**. That ADR must show the recipe guards no code that will still exist (it runs **0 tests** today, verified)
   and that the other 15 steps stay byte-identical.
8. **Every tool except Flutter can run in this container.** Pinned prebuilt binaries download in seconds;
   `sqlx-cli` 0.9.0 compiles in 3m47s. Portable pwsh 7.4.6 runs `verify.ps1` on Linux. They are installed under
   the scratch `tools/` directory (§2).

---

## 1. What was run (baseline at `3823478`, scratch copy)

| Command | Result |
|---|---|
| `bash tests/guards/run.sh` | **54 passed, 0 failed**, exit 0, about 9 s |
| `scripts/check-migrations.sh` | OK, 1 migration, base origin/main |
| `scripts/check-secrets.sh` | OK, diff vs origin/main, 7558 added lines from 82 files |
| `scripts/check-traceability.sh` | OK, **68** non-trivial paths traced to ADR-001…010 and P1-00 |
| `scripts/check-authority-sync.sh` | OK, `alpha-spec.md == docs/00-meta/specs/alpha-spec.md` (94813 bytes) |
| `scripts/check-verify-parity.sh` | OK, **16** steps, three-way |
| `scripts/check-env-contract.sh` | OK, with a note that `target/` was not created yet |
| `scripts/check-evidence-claims.sh origin/main` | OK, **9** claims re-derived and matched |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo build --workspace --locked` | exit 0, 44 s cold |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test --workspace` | exit 0; 3 tests (all in `grid-persistence`), the other crates have 0 |
| `cargo test --workspace --doc` | exit 0, 0 doctests |
| `cargo test -p grid-ffi --features flutter-bridge-tests` | exit 0, **0 tests**. The FFI gate verifies nothing today |
| `env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace` | exit 0 (offline `.sqlx` gate) |
| `just verify` (all pinned tools on PATH, after `sqlx database create && sqlx migrate run`) | **exit 0**, 33 s warm |
| `pwsh -NoProfile -File scripts/verify.ps1 -Scope Full` (pwsh 7.4.6 on Linux) | **exit 0** |

`cargo deny check` printed `advisories ok, bans ok, licenses ok, sources ok`, plus `multiple-versions` warnings
(hashbrown, thiserror 1/2), which `deny.toml` sets to `warn`. `cargo audit` fetched the RustSec DB (1277
advisories) through the proxy and passed.

Side effect: the first `rustup`/`cargo` call inside the repo triggers rustup to install the components and targets
that `rust-toolchain.toml` pins (`llvm-tools-preview`, `x86_64-pc-windows-gnu`). This is about 1 minute and is not a
repo change.

---

## 2. Tool availability in this container

| Tool (pinned version) | Preinstalled? | How I got it to run | Time |
|---|---|---|---|
| cargo/rustc 1.98.1 stable, rustfmt, clippy | yes | — | — |
| just 1.58.0 | **no** | `https://github.com/casey/just/releases/download/1.58.0/just-1.58.0-x86_64-unknown-linux-musl.tar.gz` | seconds |
| cargo-nextest 0.9.143 | **no** | `https://get.nexte.st/0.9.143/linux` (tar.gz) | seconds |
| sqlx-cli 0.9.0 (`--no-default-features --features sqlite,rustls`) | **no** | `cargo install … --locked --root <scratch>`. No prebuilt binary exists | **3m47s** |
| cargo-deny 0.20.2 | **no** | `https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` | seconds |
| cargo-audit 0.22.2 | **no** | `https://github.com/rustsec/rustsec/releases/download/cargo-audit%2Fv0.22.2/cargo-audit-x86_64-unknown-linux-musl-v0.22.2.tgz` | seconds |
| typos-cli 1.49.0 | **no** | `https://github.com/crate-ci/typos/releases/download/v1.49.0/typos-v1.49.0-x86_64-unknown-linux-musl.tar.gz` | seconds |
| pwsh | **no** | portable `https://github.com/PowerShell/PowerShell/releases/download/v7.4.6/powershell-7.4.6-linux-x64.tar.gz` | seconds |
| jq | yes (`/usr/bin/jq`) | — | — |
| python3 3.11.15 + numpy 2.4.6, pandas 3.0.6, scipy 1.17.1, scikit-learn 1.9.1, pyarrow 25.0.1, pytest 9.1.1 | yes | — | — |
| flutter / dart | no | not needed after the pivot | — |
| shellcheck | no | not used by any recipe | — |

**Reuse:** these are installed (not globally) under
`/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-infra/tools/`.
Use:
```bash
T=/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-infra/tools
export PATH=$T/bin:$T/sqlx/bin:$T/pwsh:$PATH
mkdir -p target && sqlx database create && sqlx migrate run   # the DB half of `just bootstrap`
just verify
```
`just bootstrap` itself compiles every tool from source with `cargo install`. That is much slower, roughly
10–20 minutes total. Use the prebuilt path in sessions, and keep `just bootstrap` as is because it is the
documented contract.

**Limits:** pwsh on Linux proves `verify.ps1`'s logic, but it is **not** the `windows-latest` merge gate
(CRLF, `.cmd` stubs, Git Bash path handling). It cannot stand in for that gate under §8.11 or ADR-009.

**Python suite runtime:** the engine tests are slow. Under 4-CPU contention with another inventory agent, the full
cautious-nevermore suite was about 13% done after 13 minutes. I stopped my own run to free CPU, so these are partial
numbers, not a measurement. The conclusion still holds: gating `just verify` on the full Python suite would add tens
of minutes. A fast oracle-smoke subset belongs in the verify chain, and the full suite belongs in a separate job (§6).

---

## 3. Guard-by-guard analysis

Rule applied throughout: changing a guard to make a pivot failure disappear counts as weakening. A change is
legitimate when the thing the guard protected no longer exists by owner decision **and** an ADR records the
de-scoping. Changes to scripts **outside** the frozen trio (justfile verify chain, `verify.sh`, `verify.ps1`) need
no D5 amendment, which is ADR-010's precedent.

### 3.1 `justfile` (CRLF; frozen verify chain per ADR-001 D5)
- **Enforces:** the `verify` chain of **9 recipes / 16 commands**: `check-fmt`, `check-lint`, `check-sqlx`
  (`cargo sqlx prepare --check --workspace -- --lib`), `test-rust` (`cargo nextest run --workspace`), `test-doc`,
  **`test-ffi`**, `audit` (deny + audit), `check-guards` (`run.sh` plus 6 guards), `check-typos`.
- **Breaks on pivot:** line 33 `just test-ffi`, lines 55–56 recipe `test-ffi` (`cargo test -p grid-ffi --features
  flutter-bridge-tests`, which fails once the crate is gone), and lines 84–86 `serve-ui` (`cd app && flutter run -d
  linux`).
- **Minimal change:** delete line 33 and the `test-ffi:` recipe plus its trailing blank line. Delete the
  `# Flutter …`/`serve-ui:` block. Keep CRLF (ADR-004), and **edit the bytes without renormalizing**.
  The result is 8 recipes / 15 commands.
- **Leave alone:** `bootstrap` (pins match `toolchains/dev-tools.lock` and CI), `db-*`, `evidence`,
  `bootstrap-bench`, `mutants` (targets `grid-models`, which is kept), and `bench` (its comment says "after P1-07",
  so refresh only the WP reference).
- `.claude/settings.json` has `Write(./justfile)` in `ask`, so expect a permission prompt.

### 3.2 `scripts/verify.sh` (LF) and `scripts/verify.ps1` (CRLF, `-text`)
- **Enforces:** the same 16 commands. `verify.sh` validates scope at **column 0** (ADR-008). `verify.ps1` follows
  every native command with `Assert-Ok "<label>"` (ADR-009).
- **Breaks:** `verify.sh` lines 36–37 (plus blank 38) and `verify.ps1` lines 51–53 (plus blank 54): the FFI step.
- **Minimal change:** delete exactly those lines. Write `verify.ps1` back with CRLF, which I did in the simulation
  with Python `newline=''`; `file` still reports CRLF. Keep `cargo fmt` as the first command and `typos` as the last,
  and keep `Assert-Ok "cargo audit"` and `Assert-Ok "cargo fmt --all -- --check"` byte-identical. `run.sh` and the CI
  self-test anchor on all of these (§3.9, §3.11).
- **Verified after the change:** parity 15, `run.sh` "15 guarded, 0 unguarded", self-test emulation messages
  `[verify] FAILED: cargo fmt --all -- --check (exit 7)` and `[verify] FAILED: typos (exit 3)`.

### 3.3 `scripts/check-traceability.sh` (must be satisfied by the consolidation PR)
- **Enforces (alpha-spec §9.4, §12.8):** every non-trivial changed path is named, or covered by a named parent
  directory, in a **referenced** WP or ADR.
- **Change set** (lines 47–51): `git diff --name-only BASE...HEAD`, plus unstaged, plus staged, plus
  **untracked-not-ignored**. Deleted paths are included.
- **Trivial (exempt)** (lines 39–45): `docs/99-templates/*`, `.github/*`, `*.lock` (which also covers
  `ai-toolchain.lock` and `toolchains/dev-tools.lock`), `Cargo.lock`, `.sqlx/*`, `.gitignore`, `.gitattributes`,
  `*/.gitkeep`, `.ai/evidence/*`.
- **Where references come from** (lines 65–67): `git log --format='%s%n%b' BASE..HEAD`, plus `$PR_BODY`, plus the
  `$PR_BODY_FILE` contents. CI passes `PR_BODY` **only in the `guards` job**. `just verify` in `linux-smoke` and
  `windows-authoritative` runs the guard **without** it. **Therefore at least one commit message in the range must
  carry the reference.**
- **Accepted reference regex** (line 70), exactly:
  `docs/01-work-packages/[A-Za-z0-9._-]+\.md|docs/02-adr/[0-9]{3}[A-Za-z0-9._-]*\.md|P1-[0-9]{2}|ADR-[0-9]{3}`.
  - `P1-NN` resolves to `docs/01-work-packages/*p1-nn*`. The glob is lower-cased, so the file name must contain
    lowercase `p1-nn`. A name like `p1-00a-…` would also be picked up by every `P1-00` mention, so do not use it.
  - `ADR-NNN` resolves to `docs/02-adr/NNN*.md`.
  - Any other ID scheme (`ENG-01`, `E1-01`, `P2-…`) is **not** recognized as a bare ID. Cite it by full path, or
    extend line 70 and the `case` at lines 80–85. Extending accepts a new reference syntax but keeps per-path
    coverage, so it is not weakening. It should be recorded in the ADR and given a `run.sh` case with a control.
  - If no referenced document exists on disk, the guard dies.
- **`covered()`** (lines 96–105) has two forms:
  1. **Exact form:** `grep -qF -- "$path"`. The full path must appear as a substring anywhere in the corpus.
  2. **Directory form:** for each ancestor `d`, it matches
     `(^|[^A-Za-z0-9._/-])${d}/([[:space:]*\`,\")]|$)`.
     - The directory must end in `/`.
     - The character before it must not be a letter, digit, `.`, `_`, `/` or `-`.
     - The character after the slash must be whitespace, `*`, a backtick, `,`, `"`, `)`, or end of line.
     - **These do not count:** `reference/python/.` (period), `reference/python/:`, `[reference/python/](…)`
       (`]`), `./reference/python/` (preceded by `/`), `'reference/python/'`.
     - **Safe form:** backticks, e.g. `` `reference/python/` ``.
     - `${d}` is not regex-escaped, so `.` matches any character. This only makes the guard more permissive, and
       is harmless.
- **Breaks on the pivot:** nothing in the script. What breaks is the *coverage* of new and deleted paths.
- **Minimal legitimate way to satisfy it (no script change):** the new pivot ADR, and/or the new WP, must contain a
  scope list naming:
  - each deleted path: `` `app/` `` (covers `app/pubspec.yaml`), `` `crates/ffi/` ``, `` `toolchains/flutter.version` ``;
  - each modified non-trivial path: `Cargo.toml`, `justfile`, `scripts/verify.sh`, `scripts/verify.ps1`,
    `scripts/bootstrap-repo.sh`, `scripts/check-evidence-claims.sh`, `_typos.toml`, `CLAUDE.md`, `docs/CLAUDE.md`,
    `docs/00-meta/` files, the spec files, ADR edits (`` `docs/02-adr/` ``), `docs/06-sessions/`, any crate
    `lib.rs`/`Cargo.toml` edits, and any guard or `tests/guards/run.sh` edits;
  - each new path: `` `reference/python/` ``, the ADR file itself (e.g. `docs/02-adr/011-…md`), the new WP file,
    and any new spec or doc files.

  Every commit subject should then carry `ADR-011` (or the WP ID or path). In the simulation, an ADR-011 naming
  exactly these paths gave **OK 176 paths** against `origin/main` and **OK 120 paths** against `3823478`, traced
  "to: ADR-011" alone.
- **Two pitfalls:**
  - **Do not delete `docs/01-work-packages/p1-00-work-package.md`.** If PR #1 is not merged separately, the P1-00
    commits are in the range, and their paths are covered only because `P1-00` resolves to that file.
  - **Untracked files count.** Running the Python oracle without the `.gitignore` additions makes every
    `__pycache__/*.pyc` an untracked non-trivial path. I reproduced this: FAIL with 103 of 171 paths untraced.

### 3.4 `scripts/check-authority-sync.sh`
- **Enforces:** `alpha-spec.md` (root, authority level 2) is byte-identical to `docs/00-meta/specs/alpha-spec.md`
  (lines 15–16 hard-code both). `final-build-spec.md` has no mirror and is not guarded.
- **Breaks if** the spec rewrite renames, deletes or splits `alpha-spec.md`: the guard fails with "missing
  authority document".
- **Minimal change, option A (zero script churn):** keep the file names `alpha-spec.md` and `final-build-spec.md`
  for the rewritten engine-only specs. Rewrite the root file and copy it over the mirror in the same commit.
  Nothing else changes.
- **Option B (rename):** change `CANONICAL`/`MIRROR` (lines 15–16), better as a list of pairs. In the same commit,
  update the `run.sh` fixture at **lines 228–231**, which writes `alpha-spec.md` into a throwaway repo. Otherwise
  "identical spec copies pass" fails because the guard looks for the new names.
- **Option C (single copy, no mirror):** removes the hazard instead of guarding it. This is legitimate de-scoping,
  but it deletes a guard and 2 `run.sh` cases (54→52), so it needs an ADR.
- **Recommendation:** A or B. Either way, `CLAUDE.md`, `docs/CLAUDE.md` and `docs/00-meta/authority-index.md` must
  name the same files, because they state the authority order in three places (ADR-001 compliance table).
- If specs are renumbered, provide an **old→new section crosswalk** in the new spec. Guard comments and ADRs cite
  `alpha-spec.md 8.11` (8 times), `14.1` (3), `final-build-spec.md 8.2` (3), `9.4`, `8.7`, `12.7` (2 each), and
  `1.4`, `3.2`, `7.4`, `8.5`, `8.12`, `12.8` (1 each). Editing every comment would churn frozen files.

### 3.5 `scripts/check-verify-parity.sh`
- **Enforces:** the justfile `verify` chain commands equal the `verify.sh` command list and the `verify.ps1` command
  list. The check is order- and duplicate-sensitive. A `bash ` prefix is stripped on the ps1 side only. It fails if
  any extraction is empty.
- **Hard-coded assumptions:**
  - the three file paths;
  - the `^verify:` recipe;
  - verify.sh's command filter `^[[:space:]]+[a-zA-Z._/]` (hence the column-0 scope guard);
  - the ps1 extractor's construct list;
  - the `/Write-Host/` and `/Assert-Ok/` **substring** skips (R4-2 applies here too).
- **On pivot:** passes unchanged once all three files drop the FFI step (verified: **15**). **No change needed.**
- If an oracle gate is added later, spell it identically in all three (§6).

### 3.6 `scripts/check-evidence-claims.sh` — breaks the pivot PR (CI `guards` job only, not `just verify`)
- **Enforces:** re-derives 9 claims in `.ai/evidence/P1-00/PR-BODY.md` and `manifest.json`, with a floor of 8
  (line 110):
  1. parity step count, hook "agree at **N steps**";
  2. guard-suite case count, "carries N committed behaviour cases";
  3. recipe count, "runs **N recipes**";
  4. Cargo.lock package count, "resolves N crates", plus `rsa` = 0;
  5. Assert-Ok count, "(N/16)";
  6. manifest sha256, "sha256:<64hex>";
  7. traceability count in the manifest, "All N non-trivial paths traced";
  8. "Final commit: <sha>" vs the manifest's `commit`.
- **Hard-coded:** `WP="P1-00"` (line 29) and the literal `/16` inside the claim-5 pattern (line 79).
- **Breaks (reproduced):** on the simulated pivot, 5 claims FAIL: 16→15, 9→8, 172→171, 16→15, and 68→176 (or
  →120). Every later non-trivial PR also fails claim 7, because the count is re-derived from *that* PR's diff
  against P1-00's prose. Push-to-main runs after the pivot also fail claims 1–5.
- **Do not "fix" it by rewriting the P1-00 record.** That falsifies a historical evidence record, and §8.12 makes
  each WP's manifest its own record.
- **Minimal legitimate change** (prototyped in scratch; still **OK 9/9** on the P1-00 record):
  ```diff
  -WP="P1-00"
  +WP="${2:-${EVIDENCE_WP:-P1-00}}"
  ...
  -      "$(from_body '(\([0-9]\+\)/16)')" \
  +      "$(from_body '(\([0-9]\+\)/\1)')" \
  ```
  (`\1` is a BRE backreference, so it matches "(16/16)" or "(15/15)".)
- **CI must also pass the PR's WP** to the `guards` step, which is currently
  `./scripts/check-evidence-claims.sh "origin/${{ github.base_ref || 'main' }}"`:
  1. If exactly one `.ai/evidence/<WP>/` directory is added or modified in `BASE...HEAD`, use that WP. If more than
     one is, fail.
  2. Otherwise use the most recently added `.ai/evidence/*/PR-BODY.md` in history. That makes a non-trivial PR with
     no new evidence fail when it changes a counted fact, which matches §9.4.
- **The pivot's own evidence:** its `PR-BODY.md` must reuse the 8 phrase hooks above verbatim, so the extractor
  finds them. Otherwise the floor of 8 trips.
- This needs no D5 amendment, because the script is not in the verify chain (ADR-010). Record it in the pivot ADR.
- Also update the comment at line 56 ("(16/16)").

### 3.7 `scripts/check-secrets.sh` and `scripts/secret-patterns.txt`
- **Enforces:**
  - diff-mode added lines vs base (or the full tracked tree when there is no base), plus staged, unstaged and
    untracked files, plus the two detector files in full;
  - the deny tier fails (ghp_/github_pat_, sk-ant-, `CFBD_API_KEY=`, `AWS_SECRET_ACCESS_KEY|ANTHROPIC_API_KEY=`
    literal, credentialed DB URLs, private keys);
  - the warn tier reports only.
- **On pivot:** **passes**. The simulated diff had 24,751 added lines and 0 matches. The full cautious-nevermore
  tree (`git grep`, all patterns) has **0** deny-tier and 0 warn-tier matches.
- `.env.example` (ESPN_SWID/ESPN_S2/ANTHROPIC_API_KEY/POSTHOG token, empty values) is app-only. **Do not import
  it.**
- Optional hardening (Security/Release owner): ESPN cookie patterns, if any loader that reads them is ever ported.
- **Open review finding R4-1** (partial unresolved paths) applies here. See §5.
- **`run.sh` coupling — preserve these exact lines when editing:**
  - the CR-strip line `        f="${f%$'\r'}"` (8-space indent);
  - the `plus() …` definition line;
  - the `# UNRESOLVABLE` comment marker.

  The fixtures `sed`-patch them. My R4-1 prototype kept them, and the suite stayed at 54/54.

### 3.8 `scripts/check-migrations.sh` and `scripts/check-env-contract.sh`
- **check-migrations:**
  - **Enforces:** migrations present in BASE may not be modified, deleted or renamed. It FAILs under `$CI` with no
    base.
  - **Pivot:** keep `migrations/0001_schema_meta.sql`. **Do not delete it.** Once PR #1 is in the base, deletion is
    a hard FAIL, and `check-sqlx` and `test-rust` depend on it (ADR-002: nextest exits 4 with zero tests).
  - No change.
- **check-env-contract:**
  - **Enforces:** `DATABASE_URL` is `sqlite:` relative (expected `sqlite:target/grid-dev.db`, line 14), and
    `SQLX_OFFLINE=true` requires `.sqlx/`.
  - **Pivot:** no change, as long as SQLx/persistence stays. "GRID-Alpha uses SQLite exclusively" is cosmetic.

### 3.9 `tests/guards/run.sh` (54 cases) — what it asserts about **real** repo files
Most cases use throwaway fixture repos. These couple to the real tree:
1. `new_repo` copies `$ROOT/scripts/check-*.sh` and `scripts/secret-patterns.txt`. Cases call
   `check-secrets.sh`, `check-migrations.sh`, `check-traceability.sh`, `check-authority-sync.sh`,
   `check-verify-parity.sh` and `check-env-contract.sh` by name, so renaming any of them breaks cases.
2. The check-secrets fixtures `sed`-patch exact source lines (§3.7).
3. The traceability fixture uses `P1-00` commit messages and `docs/01-work-packages/p1-00-work-package.md`. If the
   reference regex drops `P1-NN`, it breaks.
4. The authority-sync fixture (lines 228–231) uses the `alpha-spec.md` names (§3.4).
5. The scope cases copy the **real** `scripts/verify.sh`, stub `cargo` to exit 99, and expect 99 for valid scopes.
   **The first check command in `verify.sh` must stay a `cargo …` invocation.**
6. The ADR-009 analysis reads the **real** `scripts/verify.ps1`:
   - every native command must be followed by `Assert-Ok`;
   - the mutation control removes `Assert-Ok "cargo audit"`, so **that line must exist**;
   - the M1 form table inserts after `Assert-Ok "cargo fmt --all -- --check"`, so **that line must exist verbatim**;
   - the first label must start with `cargo fmt` and the last must equal `typos` (line 468).
7. The scope guard in the **real** `verify.sh` must stay at column 0.

**Pivot impact:** none of these break with the minimal FFI removal (verified 54/54). The case count (54) is a
counted claim in evidence records.

### 3.10 `_typos.toml`
- **Enforces:** extend-words and exclusions for the frozen `typos` recipe. The header says additions need real
  justification and must never silence a genuine misspelling.
- **Breaks:** about 150 hits in the Python subset (14 files), plus 24 in cautious-nevermore docs if they are
  imported.
- **Minimal change used in the simulation** (typos exit 0 afterwards). The allowlist covers domain terms and
  identifiers, not misspellings:
  ```toml
  [default.extend-words]
  NMAE = "NMAE"
  vor = "vor"     # Value Over Replacement (reference/python/backend/scoring/vor.py)
  yhat = "yhat"   # conventional fitted-value name
  mis = "mis"     # "mis-slice", reference/python/backend/validation/asof.py:220 (prefix, not a typo of "miss")
  [default.extend-identifiers]
  ot_yl = "ot_yl"
  ot_dn = "ot_dn"
  ot_yds = "ot_yds"
  fo_s = "fo_s"
  fo_w = "fo_w"
  pn = "pn"
  ```
  The `ot_*`, `fo_*` and `pn` identifiers come from `tests/pipeline/test_weekly_situations.py`,
  `backend/validation/verdict.py:166` and `tests/scoring/test_vor.py:238`.
- `typos` also flags the **file names** `vor.py`/`test_vor.py`. `vor = "vor"` covers those.
- **Not recommended:** `extend-exclude = ["reference/python/"]`. It removes coverage of about 17k lines and reads
  as weakening.
- The exact list depends on which Python subset is finally imported. Re-run `typos` after the import.

### 3.11 `.github/workflows/alpha-ci.yml`
- **Jobs:**
  - `guards`: `bash -n scripts/*.sh`, `run.sh`, the 6 guards with `origin/<base_ref>`, and `PR_BODY` passed to
    traceability **only here**;
  - `check-evidence-claims`;
  - `linux-smoke`: pinned `cargo install` of 6 tools, offline `cargo check`, `just bootstrap`, `just verify`;
  - `windows-authoritative`: the same tools, the **ADR-009 self-test**, offline check, bootstrap, `verify.ps1`.
- **There are no Flutter steps.** Nothing in CI breaks from removing Flutter or FFI.
- **Hard-coded:**
  - tool versions duplicated from `toolchains/dev-tools.lock`, and the justfile `bootstrap`;
  - `DATABASE_URL: sqlite:target/grid-dev.db`;
  - the self-test stubs `cargo.cmd`/`bash.cmd`/`typos.cmd` and expects `'[verify] FAILED: cargo fmt'` and
    `'[verify] FAILED: typos'`.
- **Required change:** the `check-evidence-claims` step must pass the WP (§3.6).
- **Comment staleness:**
  - line 195 "#1 of 16" becomes 15;
  - the header (lines 7–8) says verify.ps1 is authoritative "because the production target is Windows". Whether
    that remains true for an engine-only library is an owner decision (§10).
- `.github/*` is *trivial* for traceability, but the header calls it a **security boundary**:
  - No third-party actions beyond `actions/checkout`. If a Python oracle gate is added, use the runner's
    preinstalled Python with `python3 -m pip install -r <pinned lock>`. Adding `actions/setup-python` is a
    supply-chain decision for the Security/Release owner.
  - **If any new verify step invokes a binary other than `cargo`/`bash`/`typos`** (for example `python`), the
    Windows self-test's "last-assert" run will really execute it, and the self-test will fail with the wrong
    message. Route new steps through `bash ./scripts/<x>.sh` to avoid this (§6).

### 3.12 `scripts/generate-evidence-manifest.sh` and `ai-toolchain.lock`
- **Generator:** works unchanged. It needs `jq`, finds the WP file by the lower-case glob (line 72), lists all ADRs
  (line 73), and emits `schema: "grid-alpha/evidence-manifest/1"` (line 111; the rename is cosmetic, leave it).
  `contracts`/`model_specs` are always `[]`, which is worth a follow-up once `docs/05-model-specs/` exists.
- **`ai-toolchain.lock`** (CRLF, trivial path):
  - `repository: Seismic-Fate/GRID-Alpha` should become `GRID-One/GRID-Engine`.
  - `project_alias`, `model_public_id`, `harness` and `harness_version` are read **into the manifest as the model
    "actually used"** (§8.12). The consolidation is run by a different model and harness, so update them to the
    real values, or the pivot's evidence will misstate provenance.

### 3.13 `scripts/bootstrap-repo.sh` (scaffolding helper, not a verify step)
- **Hard-coded:** `CRATES` includes `ffi` (lines 20–21), `mk crates/ffi/src/generated` (25), `mk app/lib`,
  `mk app/test` (48–49), and docs dirs commented "P1-01"/"P1-06".
- **Change:** drop ffi and app, optionally add `mk reference/python`, and update the WP comments. Simulated; it
  needs no guard change.

### 3.14 Other config
- **`Cargo.toml`:**
  - remove `"crates/ffi",` (line 15) and the FRB block (lines 36–41);
  - set `authors` (line 21) and `repository` (line 23, still `Seismic-Fate/GRID-Alpha`) for GRID-One/GRID-Engine.
  - The file has mixed CRLF/LF already, so preserve the bytes.
  - `flutter_rust_bridge` is declared but never used, so it was never in `Cargo.lock`.
  - Removing `grid-ffi` makes cargo rewrite `Cargo.lock` (172→171 packages). **Commit the regenerated lock.**
- **`deny.toml`:** keep. The license comment cites "redistributing the Windows installer (final-build-spec 3.2)" as
  rationale, which is stale but harmless. The policy is unchanged.
- **`.claude/settings.json`:** keep. It is a security boundary, and `just test-*` and `just check-*` prefixes
  already allow future recipes. The permission rule `Bash(python3 -m pytest:*)` is absent, so agents get prompted.
  Adding it needs Security/Release owner approval.
- **`.gitattributes`:** keep. Optionally add `*.py text eol=lf`; imported files are already LF, verified.
- **`.env`, `.sqlx/*`, `rust-toolchain.toml`, `toolchains/dev-tools.lock`, `LICENSE-*`:** keep.
  `rust-toolchain.toml` must not be touched in a feature PR (CLAUDE.md).

---

## 4. Simulated minimal pivot (proof)

This is branch `sim/engine-pivot` in
`/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-infra/ge`.

**Steps:**
1. `git rm -r app crates/ffi toolchains/flutter.version`.
2. `Cargo.toml`: remove the ffi member and the FRB block.
3. `justfile`: remove the `test-ffi` invocation and recipe, and `serve-ui`.
4. `verify.sh`/`verify.ps1`: remove the FFI step, preserving CRLF in `verify.ps1`.
5. `bootstrap-repo.sh`: remove ffi and app.
6. `.gitignore`: replace the Flutter block with Python ignores.
7. `_typos.toml`: the allowlist from §3.10.
8. Copy `reference/python/` = cautious-nevermore's
   `backend/{__init__.py,grid,projection,validation,scoring,pipeline,db}`, `tests/{__init__.py,grid,projection,
   validation,scoring,pipeline,test_fantasy_scoring.py}`, `run_demo.py` and `requirements.txt`. This subset is
   **illustrative only**: it is not import-closed (`backend.services`, `backend.adapters` are missing).
   The engine-subset decision belongs to the Python inventory.
9. Add `docs/02-adr/011-engine-only-pivot.md` naming every path, in backticks.
10. Commit with subject `ADR-011: …`, then commit the regenerated `Cargo.lock`.

**Results:**

| Check | Result |
|---|---|
| `tests/guards/run.sh` | 54/54 ("15 guarded, 0 unguarded") |
| `just verify` | exit 0 |
| `pwsh verify.ps1 -Scope Full` | exit 0 |
| parity | 15 |
| traceability | OK 176 (origin/main) and OK 120 (3823478) |
| secrets | OK |
| migrations | OK |
| ADR-009 self-test emulation | both expected messages |
| `check-evidence-claims` | **FAIL, 5 claims** (§3.6) |

---

## 5. Review records (PRs #2 and #3) and open findings

**Documents:**

| Round | Source | Head reviewed | Verdict |
|---|---|---|---|
| 1 | `origin/claude/grid-alpha-adversarial-review-ikkjuk:docs/05-sessions/review-P1-00-adversarial.md` (624 lines) | `7787921` | CONDITIONAL, 6C/10M/9m |
| 2 | `origin/claude/adversarial-review-p1-00-l32emp:docs/05-sessions/review-P1-00-adversarial.md` (629 lines) | `de3b36c` | CONDITIONAL, 3C/9M/11m |
| 3 | `…l32emp:docs/06-sessions/review-P1-00-adversarial-round3.md` (309 lines) | `8e07b04` | CONDITIONAL, 1C/3M/4m |
| 4 | `…l32emp:docs/06-sessions/review-P1-00-adversarial-round4.md` (113 lines) | `3823478` | **APPROVED with findings, 0 blockers** |

Round 4 states all round-3 findings (C1, M1–M3, min-1..4) are closed and verified by re-execution. Round 3 verified
all round-2 blockers and majors as closed or owner-recorded.

**Still OPEN at round 4:**

**R4-1 (major), `scripts/check-secrets.sh` `scan_files()`.**
- The guard fires only when *nothing* was opened (`listed > 0 && opened == 0 && accounted < listed`), so a
  **partial** unresolved set passes silently.
- **Reproduced at `3823478`:** two clean untracked files with one made unresolvable gives
  `OK … 2 whole file(s) read` and exit 0, with one path never read.
- **Prototyped fix** (all 54 cases still pass; the same reproduction now exits 1 with
  `untracked files: 1 of 2 path(s) could not be opened and are not deliberate skips.`):
  ```bash
  local unresolved=$((listed - excluded - skipped - opened))
  if [[ "$unresolved" -gt 0 ]]; then
      die "$label: $unresolved of $listed path(s) could not be opened and are not deliberate skips."
  fi
  ```
- Add a no-secret partial-failure case to `run.sh`, plus a control (55+ cases).
- **Edge case to decide:** in full-tree mode, a tracked file deleted in the worktree (unstaged) becomes
  "unresolvable" and would now die. Either account it as deliberate, or scan its index blob.

**R4-2 (minor), `tests/guards/run.sh` `ps1_unguarded()`; the same rules sit in `check-verify-parity.sh`'s ps1
extractor.**
- **Reproduced:** all three evasions are certified clean.
  - `cargo sbom generate; Write-Host "done"`: 15 guarded, 0 unguarded.
  - `typos --config Assert-Ok.toml`: 15/0.
  - `cargo a; cargo b` followed by one `Assert-Ok`: **16 guarded**.
- **Fix:** anchor the skips as `/^[[:space:]]*Write-Host/` and `/^[[:space:]]*Assert-Ok/`, and reject
  `;`-joined native commands. Apply the fix to **both** files (ADR-010: shape, not instance).

**Owner and human items from round 4 (not code):**
- Architecture owner sign-off (authority order, crate boundaries, the P1-02→P1-00 rework).
- Security/Release owner sign-off on `.claude/settings.json`.
- Merge reviewer sign-off on the final diff.
- Move the two round-1/2 files out of the invalid `docs/05-sessions/`.
- Fix the **reviewer brief** that names `docs/05-sessions/`. It lives in the review-workflow prompt, not in this
  repo.

**Carried deferrals and known limitations** (from the P1-00 manifest; not review findings):
- `-Scope Changed` unimplemented (P1-11).
- CI caching (a ~32-minute uncached Windows job).
- `deny.toml` finalization and `toolchains/native-dependencies.lock`.
- Line-ending normalization (ADR-004 threshold).
- `SQLX_OFFLINE` → `.cargo/config.toml` (round-2 minor 2, declined with reasoning).
- `.claude/agents/` definitions (ADR-006 → "P1-01 (blocking)").
- An instruction-file vs ADR cross-check guard.
- Branch protection on `main`.
- Correcting the cloud environment's stale `DATABASE_URL`.
- Evidence prose staleness: the claim checker checks numbers, not sentences.

Two `follow_up` entries in the P1-00 manifest are **already done** and stale: "extend check-verify-parity to
verify.ps1" and "a guard asserting Assert-Ok coverage". Note this in the pivot WP rather than editing P1-00's
record. Round-1 **M10** (Flutter/Dart analysis absent) and the `app/pubspec.lock` blocker become **moot** once
Flutter is de-scoped. Record them as closed by ADR-011.

**Recommendation: where to carry the review records**
- Import all four **verbatim** into `docs/06-sessions/`, the canonical location; the index says `05-sessions` is
  invalid:
  - `docs/06-sessions/review-P1-00-adversarial-round1.md` ← round 1 (PR #2)
  - `docs/06-sessions/review-P1-00-adversarial-round2.md` ← round 2 (PR #3, `05-sessions` path)
  - `docs/06-sessions/review-P1-00-adversarial-round3.md` and `…-round4.md` ← unchanged names
- Prepend a single provenance line (source branch, commit, original path) **above** the frontmatter, or record
  provenance in the WP, so the bodies stay byte-identical.
- They pass `typos` with the current config and contain 0 secret-pattern matches (checked). Traceability needs
  `` `docs/06-sessions/` `` named in the pivot ADR or WP.
- Update `docs/06-sessions/README.md`: mark the collision resolved and add index rows.
- After the import, PRs #2 and #3 can be **closed unmerged** (owner action). Merging them would create the invalid
  directory. Rounds 1–2 should **not** keep the `docs/05-sessions/` path.

---

## 6. If the Python oracle is gated in verify (recommended design; not needed for the minimal pivot)

- **This is a D5 amendment that *adds* coverage**, so ADR-005's three conditions apply and it needs explicit owner
  approval and its own ADR. Best done in the first port WP or in the pivot ADR as a separate decision.
- **Implement it as a bash script,** e.g. `scripts/test-reference.sh`, invoked as:
  - `./scripts/test-reference.sh` in the `justfile` recipe `test-reference` and in `verify.sh`;
  - `bash ./scripts/test-reference.sh` followed by `Assert-Ok "test-reference.sh"` in `verify.ps1`.

  Benefits:
  - the parity guard's `bash ` normalization already handles it;
  - CI's `guards` job `bash -n` check covers it;
  - the Windows self-test already stubs `bash.cmd`, so "last-assert" passes through;
  - `.claude/settings.json` already allows `just test-*`, so `just test-reference` needs no permission change. A
    direct `./scripts/test-reference.sh` call is not allowed; only the `./scripts/check-*.sh` prefix is.
- **Placement:** after `test-doc` and before `audit`. **Never last** (the last label must stay `typos`) and **never
  first** (scope cases need `cargo` first).
- **Scope of the step:** a fast smoke subset — the synthetic-recovery gates and golden masters. The full suite is
  too slow for `verify` (§2). Put the full suite in a separate CI job.
- **Pin Python deps.** cautious-nevermore's `requirements.txt` is all `>=` and includes app deps (fastapi, uvicorn,
  anthropic, posthog, websockets, httpx, pydantic, python-dotenv). The reference needs an engine-only **exact-pinned**
  lock, e.g. `reference/python/requirements.lock` with numpy==2.4.6, pandas==3.0.6, scipy==1.17.1,
  scikit-learn==1.9.1, pyarrow==25.0.1, joblib, pytest==9.1.1 — the versions present here. Pin to whatever the
  golden masters were generated with. §8.11 requires pinned dependencies, and GBM and float outputs are
  version-sensitive.
- **Install:** `bootstrap` and both CI jobs would add `python3 -m pip install -r reference/python/requirements.lock`.
  Adding it to `bootstrap` is not a verify recipe, so it needs no D5 amendment.

---

## 7. ADRs: validity for an engine-only repo

ADR README rule: an Accepted ADR is immutable. Supersede it by writing a new ADR and setting `superseded-by` on the
old one. Editing only the frontmatter is the sanctioned change. For partial supersession, use
`superseded-by: 011 (in part — <decision>)` plus a one-line status note.

| ADR | Topic | Engine-only verdict | Action |
|---|---|---|---|
| 001 | Bootstrap decisions | **Mostly valid.** D1 (wp/ branches), D3 (numbered vault), D4 (MIT OR Apache-2.0), D5 (frozen recipes), D6 (env) all stand. Superseded **in part:** the twelve-crate list (drop `ffi`, and possibly `application`/`governance` scope), and the Flutter/FRB architecture premise. The alternatives section mentions "relax check-sqlx/test-ffi" | `superseded-by: 011 (in part: crate list, FRB/Flutter)`. **D4 must be extended** by a Data/Licensing owner ruling for `reference/python/`. cautious-nevermore has **no LICENSE file**; its authors are Ethan Nelson and Seismic-Fate (same email, 84 commits), 3 Claude commits, and 1 `posthog[bot]` commit that touched only `backend/api/*`, `.env.example` and `requirements.txt` |
| 002 | SQLx offline cache | **Valid** while `grid-persistence` and SQLite stay. The compliance bullet names `crates/ffi/src/generated/` | No supersession. ADR-011 notes the stale FFI mention. If the engine later drops SQLite, that is its own coverage-reducing ADR (check-sqlx and test-rust depend on it) |
| 003 | sqlx 0.8→0.9 | **Valid.** Its Windows-compatibility section rests on a Windows production target, which is contextual | None |
| 004 | Line endings | **Valid.** Mentions "Dart source" in the threshold text | None. Optionally add `*.py` LF |
| 005 | `check-sqlx --workspace` | **Valid** | None |
| 006 | Deferred §8.7/§9.2 deliverables | **Partially obsolete.** Item 6 (`app/lib/main.dart`, `app/pubspec.lock`, `toolchains/flutter.version`) no longer applies. Agent definitions, schemas/fixtures, benches, runbooks, model-cards and traceability stay valid but need re-targeting to the new WP sequence. Status says "ratification pending" | `superseded-by: 011 (in part: item 6; re-target WP IDs)` |
| 007 | verify covers guards and doctests | **Valid.** The chain listing includes `test-ffi` and counts 16 | `superseded-by: 011 (in part: chain listing)`. ADR-011 restates the chain as 8 recipes / 15 steps |
| 008 | verify.sh scope guard | **Valid.** Step counts (16) are stale prose | None (ADR-011 records the new count) |
| 009 | verify.ps1 exit codes | **Valid.** Context assumes Windows is the production target. The fix stands either way | None. If the owner de-designates Windows as authoritative, that is a later ADR |
| 010 | Guard the shape, not the instance | **Fully valid** | None |

**New ADR-011 (the pivot) must contain:**
1. The owner scope decision: engine only, no app, UI, FFI or installer. Rust is the target and Python is a
   reference oracle.
2. **D5 amendment #5 framed as de-scoping:**
   - `test-ffi` guarded a crate that this decision removes;
   - it executed **0 tests** (measured);
   - the other 15 steps are byte-identical, and parity, Assert-Ok and `run.sh` are verified;
   - an explicit answer to "a coverage-reducing amendment should be refused outright": no remaining code loses
     coverage.
3. The `check-evidence-claims` generalization and why it is not weakening.
4. Changes to the Architecture non-negotiables. "No Python runtime in the installed application" becomes "Python is
   reference-only: never a dependency of any crate, never shipped".
5. **The authority level of `reference/python/`.** Recommended: golden masters and recovery gates sit at **level 6**
   (tests and fixtures implementing approved contracts). The Python source is **level 7** and subordinate to
   `docs/05-model-specs/` (level 4). A Python/spec disagreement is a decision request, not a silent port.
6. A **paths list in backticks** for traceability (§3.3).
7. `supersedes: 001 (in part), 006 (in part), 007 (in part)`.

Also update `docs/02-adr/README.md` (the index) in the same PR.

---

## 8. Per-file classification (all 89 tracked files)

| # | Path | Verdict | Exact action / reason |
|---|---|---|---|
| 1 | `.ai/evidence/P1-00/PR-BODY.md` | KEEP | Immutable historical P1-00 record. Do not regenerate. `check-evidence-claims` must stop being hard-wired to it (§3.6) |
| 2 | `.ai/evidence/P1-00/manifest.json` | KEEP | Same |
| 3 | `.ai/evidence/P1-00/verification-input.json` | KEEP | Same |
| 4 | `.claude/settings.json` | KEEP | Security boundary. Optional `Bash(python3 -m pytest:*)` needs Security/Release owner |
| 5 | `.env` | KEEP | ADR-002 contract |
| 6 | `.gitattributes` | KEEP | Optional `*.py text eol=lf` |
| 7 | `.github/workflows/alpha-ci.yml` | MODIFY | Pass the WP to `check-evidence-claims` (§3.6). Refresh the "#1 of 16" comment. Header rationale per the Windows decision. No Flutter steps to remove |
| 8 | `.gitignore` | MODIFY | Delete lines 20–26 (Flutter/Dart). Add `__pycache__/`, `*.py[cod]`, `.pytest_cache/`, `/reference/python/data/` |
| 9 | `.sqlx/query-4965…json` | KEEP | Generated cache for `applied_schema_version` |
| 10 | `.sqlx/query-dc67…json` | KEEP | Generated cache for `schema_meta_value` |
| 11 | `CLAUDE.md` | MODIFY | Title GRID-Alpha→GRID-Engine. Non-negotiables: remove the Flutter/FRB lines (42, 44), reframe "installed application"/Python. Generated-files table: drop the `crates/ffi/src/generated/` row (56). Authority file names if renamed |
| 12 | `Cargo.lock` | MODIFY | Regenerate (`grid-ffi` removed, 172→171) and commit |
| 13 | `Cargo.toml` | MODIFY | Remove line 15 `"crates/ffi",` and lines 36–41 (FRB). `authors`/`repository` (21, 23) → GRID-One/GRID-Engine |
| 14 | `LICENSE-APACHE` | KEEP | — |
| 15 | `LICENSE-MIT` | KEEP | Extend coverage to `reference/python/` by owner ruling |
| 16 | `_typos.toml` | MODIFY | §3.10 allowlist |
| 17 | `ai-toolchain.lock` | MODIFY | `repository:` → GRID-One/GRID-Engine. Model and harness fields set to what is actually used (§3.12). CRLF |
| 18 | `alpha-spec.md` | MODIFY | Engine-only rewrite (spec agent). Keep the name, or change the authority-sync guard (§3.4) |
| 19 | `app/pubspec.yaml` | **DROP** | Flutter app is out of scope. Name `app/` in ADR-011 (traceability covers deletions) |
| 20 | `crates/application/Cargo.toml` | KEEP | — |
| 21 | `crates/application/src/lib.rs` | MODIFY | Boundary doc comment: "Commands, queries, events, app services" → engine façade/pipeline orchestration, **or DROP the crate (owner decision §10)** |
| 22–23 | `crates/domain/{Cargo.toml,src/lib.rs}` | KEEP | Doc comment fine |
| 24–25 | `crates/evaluation/*` | KEEP | Maps to `backend/validation` |
| 26–27 | `crates/features/*` | KEEP | — |
| 28 | `crates/ffi/Cargo.toml` | **DROP** | FRB boundary is out of scope. Its only feature exists for the `test-ffi` recipe |
| 29 | `crates/ffi/src/generated/.gitkeep` | **DROP** | Trivial path |
| 30 | `crates/ffi/src/lib.rs` | **DROP** | Doc comment only |
| 31–32 | `crates/governance/*` | KEEP (MODIFY doc) | Promotion, rollback, snapshot and versioning are engine concerns. "job queue" may leave scope (owner) |
| 33–34 | `crates/identity/*` | KEEP | Needed for cross-league priors and registry |
| 35–36 | `crates/ingestion/*` | KEEP | nflverse/CFBD. Maps to the data-adapter contract |
| 37–38 | `crates/models/*` | KEEP | Ridge, RAPM, Kalman, EB, affine, boosting. Target of `just mutants` |
| 39–40 | `crates/persistence/*` | KEEP | **Only crate with code and tests.** `check-sqlx` and `test-rust` (nextest exit 4 on zero tests) depend on it |
| 41–42 | `crates/scoring/*` | KEEP | Maps to `backend/scoring` |
| 43–44 | `crates/simulation/*` | KEEP | Maps to projection/simulation |
| 45 | `deny.toml` | KEEP | The "Windows installer" rationale comment is stale. The policy is unchanged |
| 46 | `docs/00-meta/authority-index.md` | MODIFY | Title. Row 7 `crates/`, `app/` → `crates/`, `reference/python/`. Place the oracle in the order (§7). Active phase and WP. Windows rationale. Alias table |
| 47 | `docs/00-meta/daily-log.md` | MODIFY | Append a pivot entry, leaving the history intact |
| 48 | `docs/00-meta/dashboard.md` | MODIFY | Re-sequence the WP table. Today P1-01 includes an "FFI contract skeleton", P1-10 is Flutter and P1-11 is the installer |
| 49 | `docs/00-meta/specs/alpha-spec.md` | MODIFY | Byte-identical copy of the root after the rewrite (authority-sync) |
| 50 | `docs/01-work-packages/p1-00-work-package.md` | KEEP | **Do not delete** (traceability resolution of P1-00). Set status Done or Superseded when merged |
| 51 | `docs/02-adr/001-repo-bootstrap-decisions.md` | MODIFY (frontmatter) | `superseded-by: 011 (in part)` |
| 52 | `docs/02-adr/002-sqlx-offline-cache.md` | KEEP | — |
| 53 | `docs/02-adr/003-sqlx-0-9-upgrade.md` | KEEP | — |
| 54 | `docs/02-adr/004-line-ending-policy.md` | KEEP | — |
| 55 | `docs/02-adr/005-check-sqlx-workspace-flag.md` | KEEP | — |
| 56 | `docs/02-adr/006-deferred-deliverables.md` | MODIFY (frontmatter) | `superseded-by: 011 (in part: item 6)` |
| 57 | `docs/02-adr/007-verify-covers-guards-and-doctests.md` | MODIFY (frontmatter) | `superseded-by: 011 (in part: chain listing)` |
| 58 | `docs/02-adr/008-verify-scope-guard.md` | KEEP | — |
| 59 | `docs/02-adr/009-verify-ps1-exit-codes.md` | KEEP | — |
| 60 | `docs/02-adr/010-guard-the-shape-not-the-instance.md` | KEEP | — |
| 61 | `docs/02-adr/README.md` | MODIFY | Add rows for ADR-011+ to the index |
| 62 | `docs/04-providers/cfbd/README.md` | KEEP (minor) | "UI badges" → "staleness flags". WP refs P1-04 → new IDs |
| 63 | `docs/04-providers/nflverse/README.md` | KEEP (minor) | Same, P1-03 |
| 64 | `docs/06-sessions/README.md` | MODIFY | Collision resolved. Index rows for the 4 imported reviews and this session |
| 65 | `docs/99-templates/template-adr.md` | KEEP | — |
| 66 | `docs/99-templates/template-model-spec.md` | MODIFY (recommended) | Add "Reference oracle / parity" (Python module, golden-master path, recovery threshold, tolerance) |
| 67 | `docs/99-templates/template-provider-contract.md` | KEEP | — |
| 68 | `docs/99-templates/template-session-log.md` | KEEP | — |
| 69 | `docs/99-templates/template-work-package.md` | KEEP (optional MODIFY) | Optionally add a parity-target field |
| 70 | `docs/CLAUDE.md` | MODIFY | Drop the Flutter, FRB and "Flutter isolate" lines (23, 25, 37), the crate line `ffi` (74), and "Flutter APIs" (98). Re-state the crate boundaries and the oracle rule. CRLF file |
| 71 | `final-build-spec.md` | MODIFY | Engine-only rewrite (spec agent). Most of §3–6, §22 and §23 phase 1/5 are app-only |
| 72 | `justfile` | MODIFY | Delete line 33, lines 55–56 (plus blank) and lines 84–86 (plus blank). CRLF |
| 73 | `migrations/0001_schema_meta.sql` | KEEP | Append-only. Required by `check-sqlx` and `test-rust` |
| 74 | `rust-toolchain.toml` | KEEP | Not touched in a feature PR |
| 75 | `scripts/bootstrap-repo.sh` | MODIFY | §3.13 |
| 76 | `scripts/check-authority-sync.sh` | KEEP / MODIFY | Only if the spec files are renamed (§3.4) |
| 77 | `scripts/check-env-contract.sh` | KEEP | — |
| 78 | `scripts/check-evidence-claims.sh` | **MODIFY (required)** | Lines 29 and 79 (§3.6) |
| 79 | `scripts/check-migrations.sh` | KEEP | — |
| 80 | `scripts/check-secrets.sh` | KEEP (recommended MODIFY) | R4-1 fix (§5) |
| 81 | `scripts/check-traceability.sh` | KEEP | Extend the ID regex only if the new WP scheme is not `P1-NN` (§3.3) |
| 82 | `scripts/check-verify-parity.sh` | KEEP (recommended MODIFY) | R4-2 anchor fix in the ps1 extractor (§5) |
| 83 | `scripts/generate-evidence-manifest.sh` | KEEP | — |
| 84 | `scripts/secret-patterns.txt` | KEEP | — |
| 85 | `scripts/verify.ps1` | MODIFY | Delete lines 51–54, preserving CRLF |
| 86 | `scripts/verify.sh` | MODIFY | Delete lines 36–38 |
| 87 | `tests/guards/run.sh` | KEEP (MODIFY if R4 fixes or authority-sync rename) | §3.9 |
| 88 | `toolchains/dev-tools.lock` | KEEP | Add Python pins here only if the oracle gate is added; otherwise in `reference/python/` |
| 89 | `toolchains/flutter.version` | **DROP** | Flutter/FRB pin is out of scope |

**Not present but needed:**
- `reference/python/**`, with its own `README.md` stating its oracle role and provenance (source repo `@ 59bce1d`);
- the pinned Python requirements lock;
- ADR-011;
- the pivot WP;
- `.ai/evidence/<pivot-WP>/{PR-BODY.md,manifest.json,verification-input.json}`;
- the 4 imported review documents.

**Do not import from cautious-nevermore:** `.env.example`, `frontend/`, `backend/api/`, `backend/viz_agent/`,
`backend/adapters/` (ESPN cookie auth), `scripts/setup_scheduler.ps1`, `scripts/run_pipeline.bat`, PNGs
(unless kept as recovery-demo evidence), and the posthog-touched app files.

---

## 9. Recommended sequencing for the consolidation PR

1. **Decide PR #1 first.** Recommended: merge PR #1 as is with a **merge commit or rebase, not a squash**. The P1-00
   manifest attests to `8d43203`, which must stay reachable. Then base the pivot on the new `main`.
   - If the consolidation instead goes straight to `main` including the P1-00 commits, the guards also pass
     (simulated). Either way, never squash.
2. Commit 1: `ADR-011: engine-only pivot …`, containing the ADR plus the WP with the full path list (§3.3).
3. Commit 2: de-scope FFI and Flutter (§8 rows 7–8, 12–13, 19, 28–30, 72, 75, 85–86, 89) and regenerate
   `Cargo.lock`.
4. Commit 3: generalize `check-evidence-claims.sh` and the CI step (§3.6). Optionally the R4-1 and R4-2 fixes, each
   with `run.sh` cases and controls.
5. Commit 4: `.gitignore` Python ignores, `_typos.toml`, and import `reference/python/` plus its pinned
   requirements and README.
6. Commit 5: authority docs (`CLAUDE.md`, `docs/CLAUDE.md`, `authority-index`, `dashboard`, ADR frontmatter and
   README, spec rewrite with the mirror copied).
7. Commit 6: import the 4 reviews into `docs/06-sessions/` and update its README.
8. Run `just verify`, `pwsh verify.ps1 -Scope Full` and `check-evidence-claims.sh <base> <WP>`, then
   `just evidence <WP>`. Write `PR-BODY.md` with the 8 phrase hooks.
9. Every commit subject cites `ADR-011` or the WP.

---

## 10. Owner decisions surfaced (no obvious default)

1. **The Windows merge gate.** With no desktop app, is Windows still the "production target" that makes
   `verify.ps1` and `windows-authoritative` merge-authoritative (§8.11, ADR-009)? Recommendation: keep both
   unchanged in the pivot PR, since removing them would reduce coverage. Let the rewritten spec state the platform
   targets, and record any re-designation in its own ADR.
2. **Which Rust crates survive.** `ffi` is dropped (decided). `application` ("commands, queries, events, app
   services") can be repurposed as the engine façade or pipeline orchestrator, or dropped. `governance` can keep or
   drop "job queue". Recommendation: keep both stubs and re-word their boundaries now. Rename or drop in the first
   engine WP, to avoid churn in the bootstrap PR. **`persistence` and SQLite must stay** unless a separate
   coverage-reducing ADR is accepted, because `check-sqlx` and `test-rust` depend on them.
3. **License for `reference/python/`.** cautious-nevermore has no LICENSE. A Data/Licensing owner ruling is needed to
   bring it under `MIT OR Apache-2.0` (it extends ADR-001 D4). This is the one irreversible-in-the-world item
   (ADR-001 D4).
4. **The Python oracle gate.** Is it in the frozen verify chain as a smoke subset (a D5 amendment that adds coverage,
   §6), or a separate CI job only? Which exact pinned Python dependency set defines the golden masters?
5. **The WP ID scheme after the pivot.** Continue `P1-NN` (no guard change), or start a new scheme. A new scheme
   needs the `check-traceability.sh` regex at line 70, the resolution `case`, and a `run.sh` case extended
   (§3.3).
6. **Spec file names.** Keep `alpha-spec.md`/`final-build-spec.md` with rewritten content (no guard change), or
   rename (update `check-authority-sync.sh` lines 15–16 and the `run.sh` fixture at lines 228–231).
7. **PR #1 merge order and method.** Merge first without squash (recommended), or fold it into the consolidation PR.
   Also: close PRs #2 and #3 unmerged after importing their review documents into `docs/06-sessions/`.
8. **Whether to fix R4-1 and R4-2 in the consolidation PR or a follow-up.** Recommended: fix them in the pivot PR.
   Both are guard-only (no D5 amendment), small and prototyped. Leaving R4-1 open leaves a known security-control
   gap.
