#!/usr/bin/env python3
"""Generate the PUBLIC agent skill for `cargo refine`.

The file this renders is served by the network worker at
https://riir.ai/skills/cargo-refine/SKILL.md and installed by agents via
curl (the refine.riir.ai Agents section). Until 2026-11-18 it was a
hand-maintained copy in the worker repo and rotted (it still taught the
removed --mine/--unmine/--purge surface, the kat_service release set, and
--fix as the primary spelling). Now the command facts are EXTRACTED FROM
CODE in the riir-refine repo — the one home that moves with the CLI:

  - verbs        src/bin/heal.rs            (`cargo refine <verb>` lines in
                                            user_help() + usage_text())
  - version      Cargo.toml                 [package] version
  - release set  src/build_stamp.rs         RELEASE_FEATURES rows + the
                                            CRATESIO_RELEASE_LINE copy
  - domains      src/draft/domain_router.rs DomainId::label() arms

THE ENFORCEMENT IS THE CLASSIFIER: every verb the source help surface
declares must be classified in CATEGORIES below (or META). An unclassified
verb — a new CLI mode nobody taught the skill — FAILS THIS RUN LOUDLY, so
a stale skill cannot ship. The reverse holds too: a classified verb the
source no longer declares is a stale row and also fails. The committed
bytes are gated by tests/agent_skill_md_fresh.rs (generator --check +
binary --help coverage), and the release workflow runs the check before
any artifact ships.

Usage:
  python3 scripts/gen_skill_md.py             # regenerate the committed file
  python3 scripts/gen_skill_md.py --check     # exit 1 if it would differ
  python3 scripts/gen_skill_md.py --stdout    # print, write nothing
  python3 scripts/gen_skill_md.py --dump-verbs # print the extracted verbs (debug)
"""

from __future__ import annotations

import re
import sys
import difflib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Bumped by hand at each content revision — never derived from the clock,
# so --check is deterministic across days.
SKILL_VERSION = 2
SKILL_DATE = "2026-11-18"

SKILL_PATH = REPO / "assets" / "agent_skill" / "SKILL.md"

HEAL_RS = REPO / "src" / "bin" / "heal.rs"
CARGO_TOML = REPO / "Cargo.toml"
BUILD_STAMP_RS = REPO / "src" / "build_stamp.rs"
DOMAIN_ROUTER_RS = REPO / "src" / "draft" / "domain_router.rs"

SERVED_URL = "https://riir.ai/skills/cargo-refine/SKILL.md"

# ---------------------------------------------------------------------------
# Verb classification. (verb -> (category_key, gloss))
# A verb found in the source but missing here fails the run; a verb here
# that the source no longer declares fails the run. META verbs are real
# surface that the prose already names; they are exempt from classification
# but still asserted present in the output by the gate test.
# ---------------------------------------------------------------------------

CATEGORIES = {
    # The heal loop.
    "fix": ("heal", "REAL FIX — writes the edits, compile-gated (the gate reverts any edit that breaks the build)"),
    "suggest": ("heal", "review per span, no edits"),
    "scan": ("heal", "secret-blind scan of the tree — reports secret-shaped content, sends nothing"),
    "fix-compile": ("heal", "compile-error repair loop (review by default, --write applies)"),
    # The contribution network (the points surface — free, opt-in).
    "join": ("points", "join the contribution network (policy + GitHub login, public credit)"),
    "leave": ("points", "leave — log off, zero data sent (`left` is the same verb)"),
    "rank": ("points", "the contribution leaderboard (joined identities only)"),
    "info": ("points", "what the network fixed, by domain — counts only, never code"),
    "corpus": ("points", "corpus state + the freshness offer (always free)"),
    "sync": ("points", "push redacted batches + pull the corpus lease"),
    "update": ("points", "corpus lease refresh (auto-runs at most once per 12 h before a run)"),
    "upgrade": ("points", "verified binary self-update (checksums verified, dev builds refused)"),
    # Local posture + service URL.
    "offline": ("posture", "firewall: every network lane refuses until `online` (marker-based; env cannot override)"),
    "online": ("posture", "clear the firewall + remind what settings.toml chose"),
    "off": ("posture", "persist an auto-check family OFF in settings.toml (--auto-update|--auto-upgrade|--auto-corpus|--app-check)"),
    "on": ("posture", "persist an auto-check family ON (bare off/on prints the state)"),
    "config": ("posture", "service URL — `config get` / `config set --url <devnet|mainnet|https://…>` / `config adopt-state`"),
    # Read-only reports (never write, never gate).
    "misses": ("reports", "the authoring queues in frontier order"),
    "frontier-report": ("reports", "learnability-frontier band report"),
    "coverage-report": ("reports", "corpus coverage debt vs the installed clippy"),
    "horizon-report": ("reports", "fixseq lead-k decodability report"),
    "blueprint-report": ("reports", "campaign posture↔verdict audit (read-only)"),
    "gate-calibration": ("reports", "admission-gate evidence report"),
    "active-set": ("reports", "print demotions + caps (`--active-set-demote <domain:rule>` demotes; nothing demotes automatically)"),
    "oracle-census": ("reports", "the auto-oracle's blind-spot report (no compile)"),
    "corpus-gap": ("reports", "partition `[no corpus rule]` anchors: fresh / gap / unknown"),
    "config-audit": ("reports", "config knobs nobody can turn"),
    "orphan-report": ("reports", "knobs orphaned by optimization (git-history evidence)"),
    "platform-audit": ("reports", "platform-dead declarations (decl-vs-use cfg lattice)"),
    "frame-time-report": ("reports", "frame-time budget tiers over .frames.json sidecars"),
    "stats": ("reports", "the standing self-evolve health panel (read-only)"),
    "score-bench": ("reports", "the frozen score bench (needs --features score_bench)"),
    "necessity-bench": ("reports", "component-necessity flip-rate harness (score_bench)"),
    "fixer": ("reports", "the fixer-rate contribution view"),
    "trainer": ("reports", "the trainer (self-evolve) lane's health view (read-only, offline)"),
    # Ops / money / devnet-ops — they exist, but the PUBLIC agent skill omits
    # this category entirely: the zero-token law (2026-11-18 owner decree)
    # keeps token vocabulary off the refine product surfaces, and money verbs
    # are operator lanes, not healing lanes. The riir-refine adjudication
    # still covers them two-sided (a verb here that the CLI drops fails the
    # generation, and a CLI verb that appears lands here or elsewhere).
    "login": ("ops", "account sign-in (operator builds)"),
    "account": ("ops", "local account view (operator builds)"),
    "refill": ("ops", "DEVNET-OPS ONLY faucet credit; refuses on mainnet"),
    "claim": ("ops", "queue/submit a payment claim"),
    "stake": ("ops", "staking: transfer QR / position"),
    "unstake": ("ops", "drain unlocked tranches"),
    "vote": ("ops", "governance voting (weight + evidence render)"),
    "pay": ("ops", "pay request: signed create + link"),
    "topup": ("ops", "checkout (catalog + signed intent + QR)"),
    "buy": ("ops", "resolve the catalog pack from an exact amount"),
    "kat": ("ops", "the one-line ledger balance (+ `kat totp enroll|disarm`)"),
    "tuna": ("ops", "the one-line trial-credit balance + expiry"),
    "get": ("ops", "airdrop ADMIN hand (target own account)"),
    "send": ("ops", "airdrop ADMIN hand (target a 32-hex account id)"),
    "faucet": ("ops", "faucet preset (the amount is the deployment's cap)"),
    "wetrun": ("ops", "wet-run + G-WET; refuses mainnet with no override"),
    "replay": ("ops", "lease → local clippy-oracle replay → signed verdict"),
    "corpus-pr": ("ops", "devnet→mainnet corpus PR (attested-intent operator lane)"),
    "totp": ("ops", "TOTP second factor — `kat totp enroll|disarm` / `account totp`"),
    "migrate": ("ops", "account-key rotation filing (`account migrate`)"),
}

# Real surface the prose names directly; exempt from CATEGORIES but still
# part of the extracted set (the gate test asserts the output mentions them).
META = {"help", "--version", "--help-all"}

CATEGORY_ORDER = ["heal", "points", "posture", "reports", "ops"]

# Categories the PUBLIC skill omits (the zero-token law — token/money
# vocabulary stays off the refine product surfaces; the ops verbs still
# exist and are still adjudicated against the CLI source two-sided).
PUBLIC_OMITTED_CATEGORIES = ["ops"]

CATEGORY_TITLES = {
    "heal": "Fix your code",
    "points": "Contribution network (free, opt-in)",
    "posture": "Local posture + service URL",
    "reports": "Read-only reports (never write, never gate)",
    "ops": "Ops / money / devnet-ops (operator lanes — omitted from the public skill)",
}

# ---------------------------------------------------------------------------
# Extraction
# ---------------------------------------------------------------------------


# Prose false positives — English words the help text happens to spell right
# after "cargo refine" inside a sentence ("… cargo refine is never named").
# A real CLI verb named one of these would need a CATEGORIES row instead.
PROSE_NOISE = {"is"}

# Verb-shaped token: lowercase start, letters/digits/hyphens/pipes, and NOT
# followed by more word chars — so a trailing backtick/quote/paren/period in
# prose ("`cargo refine join`.") ends the match cleanly.
VERB_RE = re.compile(r"cargo refine ([a-z][a-z0-9|-]*)(?![\w|-])")


def extract_verbs(heal_src: str) -> set[str]:
    """Every verb-shaped token the help surface spells after `cargo refine`.

    Scans the SOURCE text, so feature-gated fragments (points, kat_service)
    are covered regardless of which features this box compiles. Flag
    spellings (tokens starting with `-`) and non-verb tokens (`[<path>…]`)
    are not verbs and are dropped here.
    """
    verbs: set[str] = set()
    # The verb-first law line wraps its pipe-separated verb list across
    # lines — join `|\n` continuations so the whole list is one token.
    joined = re.sub(r"\|\s*\n\s*", "|", heal_src)
    for tok in VERB_RE.findall(joined):
        for part in tok.split("|"):  # `get|send` declares two verbs
            if part in PROSE_NOISE:
                continue
            if part[:1].isalpha():
                verbs.add(part)
    return verbs


def extract_version(cargo_toml_src: str) -> str:
    m = re.search(r'^version = "([^"]+)"', cargo_toml_src, re.M)
    if not m:
        raise SystemExit("gen_skill_md: no [package] version in Cargo.toml")
    return m.group(1)


def extract_release_features(build_stamp_src: str) -> list[str]:
    m = re.search(r"pub const RELEASE_FEATURES: &\[&str\] = &\[(.*?)\n\];", build_stamp_src, re.S)
    if not m:
        raise SystemExit("gen_skill_md: RELEASE_FEATURES not found in build_stamp.rs")
    feats = re.findall(r'"([a-z0-9_-]+)"', m.group(1))
    if not feats:
        raise SystemExit("gen_skill_md: RELEASE_FEATURES parsed empty")
    return feats


def extract_cratesio_line(build_stamp_src: str) -> str:
    m = re.search(r'pub const CRATESIO_RELEASE_LINE: &str = "((?:[^"\\]|\\.)*)";', build_stamp_src)
    if not m:
        raise SystemExit("gen_skill_md: CRATESIO_RELEASE_LINE not found in build_stamp.rs")
    return m.group(1).replace('\\"', '"')


def extract_domains(router_src: str) -> list[str]:
    m = re.search(r"pub fn label\(self\) -> &'static str \{(.*?)\n    \}", router_src, re.S)
    if not m:
        raise SystemExit("gen_skill_md: DomainId::label not found in domain_router.rs")
    domains = re.findall(r'=> "([a-z]+)"', m.group(1))
    if not domains:
        raise SystemExit("gen_skill_md: DomainId::label parsed empty")
    return domains


def adjudicate(verbs: set[str]) -> None:
    """Two-sided membership: unknown verb in, classified verb gone — both red.

    Also asserts the split is total: every classified verb is either rendered
    publicly or lives in a declared omitted category — nothing classified
    can silently vanish from both halves.
    """
    unknown = sorted(verbs - set(CATEGORIES) - META)
    if unknown:
        raise SystemExit(
            "gen_skill_md: the CLI declares verbs the skill does not classify: "
            + ", ".join(unknown)
            + " — add them to CATEGORIES (or META) in scripts/gen_skill_md.py, "
              "then regenerate. A new CLI mode must not ship unrendered."
        )
    gone = sorted(v for v in CATEGORIES if v not in verbs)
    if gone:
        raise SystemExit(
            "gen_skill_md: classified verbs the CLI no longer declares (stale rows): "
            + ", ".join(gone)
            + " — remove them from CATEGORIES in scripts/gen_skill_md.py."
        )
    omitted_cats = set(PUBLIC_OMITTED_CATEGORIES)
    unknown_cats = omitted_cats - set(CATEGORY_ORDER)
    if unknown_cats:
        raise SystemExit(
            "gen_skill_md: PUBLIC_OMITTED_CATEGORIES names unknown categories: "
            + ", ".join(sorted(unknown_cats))
        )
    accounted = {
        v for v, (cat, _) in CATEGORIES.items() if cat not in omitted_cats
    } | {v for v, (cat, _) in CATEGORIES.items() if cat in omitted_cats}
    unaccounted = set(CATEGORIES) - accounted
    if unaccounted:
        raise SystemExit(
            "gen_skill_md: verbs outside every declared category: "
            + ", ".join(sorted(unaccounted))
        )


# ---------------------------------------------------------------------------
# Domain territories (authored one-liners; membership comes from code)
# ---------------------------------------------------------------------------

DOMAIN_TERRITORY = {
    "clippy": "lint-shaped style/complexity/perf fixes (the clippy-corpus lane)",
    "perf": "Rust performance rules (Pareto- + alloc-validated)",
    "kernel": "GPU kernel optimization — pregated: joins the fan-out only on kernel-signal files",
    "shader": "WGSL `@vertex`/`@fragment` render-pipeline rules (path-gated off `.rs` spans)",
    "ane": "CoreML/ANE host-orchestration suggestions — pregated, suggestion-only",
    "docker": "Dockerfile bounded fixes",
    "dist": "shipped-artifact build-config advice (suggest-only)",
    "sec": "security-pattern suggestions — SOURCE BUILDS ONLY (`--features sec`), not in the prebuilt binary",
}

# Domains shipped in the prebuilt release binary (the RELEASE_FEATURES set
# compiles kernel/shader/ane via their pregates; sec stays out).
DOMAINS_SHIPPED = {"clippy", "perf", "kernel", "shader", "ane", "docker", "dist"}


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------


def render(verbs: set[str], version: str, release_features: list[str],
           cratesio_line: str, domains: list[str]) -> str:
    by_cat: dict[str, list[tuple[str, str]]] = {c: [] for c in CATEGORY_ORDER}
    for v in sorted(CATEGORIES):
        cat, gloss = CATEGORIES[v]
        by_cat[cat].append((v, gloss))

    domain_rows = "\n".join(
        "| `{d}` | {shipped} | {terr} |".format(
            d=d,
            shipped="shipped" if d in DOMAINS_SHIPPED else "source builds only",
            terr=DOMAIN_TERRITORY.get(d, "⚠ unclassified domain — add it to DOMAIN_TERRITORY"),
        )
        for d in domains
    )
    unclassified_domains = [d for d in domains if d not in DOMAIN_TERRITORY]
    if unclassified_domains:
        raise SystemExit(
            "gen_skill_md: unclassified domains: " + ", ".join(unclassified_domains)
        )

    surface_blocks = []
    for cat in CATEGORY_ORDER:
        if cat in PUBLIC_OMITTED_CATEGORIES:
            continue
        rows = "\n".join("| `cargo refine {v}` | {g} |".format(v=v, g=g) for v, g in by_cat[cat])
        surface_blocks.append(
            "**{t}**\n\n| command | what it does |\n|---|---|\n{rows}".format(
                t=CATEGORY_TITLES[cat], rows=rows
            )
        )
    surface = "\n\n".join(surface_blocks)
    meta_line = ", ".join("`cargo refine {}`".format(m) for m in sorted(META))
    omitted_note = ""
    if PUBLIC_OMITTED_CATEGORIES:
        omitted_note = (
            "Operator / money / devnet-ops verbs also exist "
            "(account, top-up, faucet and kin) but are deliberately NOT listed here — "
            "they are operator lanes, not healing lanes."
        )

    out: list[str] = []
    w = out.append
    w("---")
    w("name: cargo-refine")
    w("description: Drive the riir-refine `cargo refine` healer over its CLI — bare dry run "
      "first, then compile-gated fixes at the real feature set, the divergence classes that "
      "stay manual, bench-file safety, and the contribution loop (join, leave, sync). Use when "
      "a task says fix clippy, heal lints, auto-fix warnings, mechanical fixes, lint hygiene, "
      "cargo refine, riir-refine, or code healing — even when cargo refine is never named.")
    w("---")
    w("")
    w("# cargo-refine — a code healer for your coding agent")
    w("")
    w("`cargo refine` is a modelless code healer that runs inside any Rust crate: a drafter + "
      "pruner over a shipped lint corpus, no LLM, no telemetry of your source. It reviews code, "
      "suggests bounded fixes, and applies them with a compile gate.")
    w("")
    w("- Bare `cargo refine` is the DRY RUN — zero edits, zero sends.")
    w("- `cargo refine fix` WRITES by default — the explicit verb is the consent.")
    w("")
    w("Skill version {v} · {date} · GENERATED by `scripts/gen_skill_md.py` in the "
      "[riir-refine repo](https://github.com/riir-ai/riir-refine) — regenerate it, never "
      "hand-edit. Every command below is extracted from the CLI source and gated by tests: a "
      "verb the CLI gains or loses fails CI until this file is regenerated, so the skill "
      "cannot silently rot. Describes riir-refine {ver}.".format(
          v=SKILL_VERSION, date=SKILL_DATE, ver=version))
    w("")
    w("## Install this skill for your agent")
    w("")
    w("Claude Code reads `.claude/skills/` natively; Zed reads `.agents/skills/`.")
    w("")
    w("```sh")
    w("# Claude Code")
    w("mkdir -p .claude/skills/cargo-refine")
    w("curl -fsSL {url} \\".format(url=SERVED_URL))
    w("  -o .claude/skills/cargo-refine/SKILL.md")
    w("")
    w("# Zed (and any agent that accepts a markdown instruction file)")
    w("mkdir -p .agents/skills/cargo-refine")
    w("curl -fsSL {url} \\".format(url=SERVED_URL))
    w("  -o .agents/skills/cargo-refine/SKILL.md")
    w("```")
    w("")
    w("## 1. Install the healer")
    w("")
    w("```sh")
    w("# Any OS with a Rust toolchain (crates.io — the canonical lane)")
    w("cargo install riir-refine")
    w("")
    w("# macOS / Linux (Homebrew — prebuilt formula, Apple Silicon + Intel)")
    w("brew tap riir-ai/tap && brew trust riir-ai/tap && brew install riir-refine")
    w("")
    w("# Windows (Scoop) + macOS / Linux (direct download — checksum-verified)")
    w("scoop bucket add riir-ai https://github.com/riir-ai/scoop-bucket && scoop install riir-refine")
    w("curl -fsSL https://raw.githubusercontent.com/riir-ai/cargo-refine/main/install.sh | sh")
    w("iwr -useb https://raw.githubusercontent.com/riir-ai/cargo-refine/main/install.ps1 | iex")
    w("```")
    w("")
    w("`cargo install riir-refine` builds the MIT crate from crates.io and fixes code. The "
      "prebuilt binaries (Homebrew, Scoop, direct download) additionally carry the hosted "
      "contribution surface (`cargo refine join`, the leaderboard, latent retrieval). Either "
      "way it runs as `cargo refine` inside a crate.")
    w("")
    w("Check the build stamp — it prints the compiled feature set and refuses to hide an "
      "incomplete one:")
    w("")
    w("```console")
    w("$ cargo refine --version")
    w("cargo-refine {ver}".format(ver=version))
    w("features: <the compiled feature set>")
    w("build: <commit hash>")
    w("release set: complete ({feats})".format(feats=",".join(release_features)))
    w("```")
    w("")
    w("`release set: STALE — missing …` means \"not the shipped binary\": re-install or "
      "rebuild before trusting the network surfaces. The crates.io build prints "
      "`{cio}` instead — that is correct for that lane, not a defect.".format(
          cio=cratesio_line))
    w("")
    w("## 2. The golden law: dry run first, fix second")
    w("")
    w("```sh")
    w("cargo refine                    # DRY RUN: review what would be fixed — zero edits")
    w("cargo refine src/ crates/foo/   # scope the review to paths")
    w("cargo refine suggest            # the explicit review command")
    w("cargo refine fix                # REAL FIX: writes in place + compile-gates")
    w("cargo refine fix --no-verify    # skip the compile gate (fast, syntax-checked only)")
    w("cargo refine fix --verify-args \"--features a,b\"   # verify gated code at its real set")
    w("cargo refine --quiet --fix      # skip the review pass (only show fixes)")
    w("cargo refine --yes --fix        # non-interactive: every y/N prompt answers yes")
    w("```")
    w("")
    w("- The bare dry run is private: no account, nothing written, nothing sent.")
    w("- **Verb-first**: the docs teach the verbs (`fix`/`suggest`/`fix-compile`). The old "
      "`--fix`/`--suggest` flag spellings stay accepted as quiet aliases — existing scripts "
      "keep working — but teach the verbs.")
    w("- rustfmt follows automatically when the file was already rustfmt-clean "
      "(pre-existing drift and CRLF files are skipped) — no formatting churn enters a "
      "mechanical diff. `--no-fmt` disables.")
    w("- Allow-by-default lints (pedantic/nursery) are fixed only where the target crate "
      "enables them or their group; otherwise they are skipped loudly. `fix --groups "
      "pedantic,nursery` restores apply-everything for those groups.")
    w("- `--top-k <N>` caps the per-span suggestion list (default 3 per domain) — raise it "
      "when hunting a fix the ranking missed.")
    w("- Commit the fix separately: `fix: refine <repo> <scope>`.")
    w("")
    w("## 3. The oracle: clippy anchors the suggestions")
    w("")
    w("Over a cargo project, a scan auto-runs clippy "
      "(`cargo clippy --workspace --no-deps --message-format=json`) and uses real diagnostics "
      "to anchor suggestions:")
    w("")
    w("- spans clippy flagged surface FIRST, marked `[oracle]`, with the corpus fix;")
    w("- diagnostics with no corpus rule surface as `[no corpus rule]` (a live gap signal, "
      "not an error) — `--corpus-delta` partitions them into \"not leased yet\" vs \"unsupported\";")
    w("- covered spans clippy said nothing about are stamped `[heuristic]` — the "
      "false-positive datum; treat them as leads, not verdicts.")
    w("")
    w("Knobs: `--all-targets` is the DEFAULT posture (test-module spans are where fixture "
      "code lives). `RIIR_REFINE_AUTO_ORACLE=0` restores never-spawn; "
      "`RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>` re-aims the spawn at a non-default feature "
      "set; gap-arming (the default) runs up to 3 extra passes over targets whose "
      "required-features the base pass did not enable — "
      "`RIIR_REFINE_AUTO_ORACLE_GAP_ARM=0` restores single-pass. "
      "`cargo refine oracle-census` prints the blind-spot report without compiling. Manual "
      "feed: `cargo refine suggest --clippy-json <path>` (the `--message-format=json` "
      "stream). Non-cargo directories run heuristic-only, loud.")
    w("")
    w("## 4. Verify at the real feature set")
    w("")
    w("`fix` compile-gates by default on release builds: baseline check → apply → re-check → "
      "auto-revert the offending edits. The gate runs at the invocation's feature flags — a "
      "default-features verify compiles cfg-gated files EMPTY, so a green check proves "
      "nothing about them. Pass `--verify-args \"--features <set>\"` covering the edited "
      "files' gates (grep the enclosing `#[cfg(...)]`).")
    w("")
    w("Four post-verify disclosures name the exact re-run for kept edits the posture provably "
      "did not compile — read them when they print: the **cfg audit** (edits inside "
      "cfg-gated regions), the **required-features audit** (edits in a test/bench/example "
      "target whose features are unsatisfied — `--all-targets` skips those silently), the "
      "**module-decl audit** (a `#[cfg]` on the file's `mod` declaration), and the "
      "**unverified-kept audit** (a kept edit under a failed final check). Kept edits any of "
      "these name are recorded `verified=false` — never certified.")
    w("")
    w("## 5. What stays manual")
    w("")
    w("The healer is deliberately SILENT (never wrong) on its documented divergence classes — "
      "fix these by hand:")
    w("")
    w("- comment-guarded matches (a match whose arms carry guarded comments),")
    w("- array-literal defaults,")
    w("- named-argument renames,")
    w("- nested macro arguments.")
    w("")
    w("Also manual: semantic lints, anything needing a design decision, and **bench files** — "
      "perf rules are measurement-altering on bench targets, so `fix` declines them with a "
      "loud `bench-guard:` line per file and `suggest` stamps `⚠ bench-guard`. A bench "
      "file's numbers ARE its artifact; hand-edit those.")
    w("")
    w("## 6. The repo-posture fix (heal a repo you do not own the style of)")
    w("")
    w("```sh")
    w("cargo refine fix --clippy-json feed.json --repo-posture")
    w("```")
    w("")
    w("Clippy-lane edits land ONLY where the target repo's own clippy flagged the span. "
      "Requires a feed (the auto-oracle or `--clippy-json`); without one it refuses. Use when "
      "healing a foreign repo: never restyle code its gate never complained about.")
    w("")
    w("## 7. Compile-error repair lane")
    w("")
    w("```sh")
    w("cargo refine fix-compile                 # review: prints the plan")
    w("cargo refine fix-compile --write         # apply: keep/revert per attempt")
    w("cargo refine fix-compile --write --verify-args \"--features <set>\"")
    w("```")
    w("")
    w("Coded rustc errors via playbooks (the E0597 family today; other codes report as "
      "misses). Bounded by `--max-iters` (default 5). Bench targets guarded the same way — "
      "name the file explicitly to override. Every `--write` run dumps an ordered "
      "per-attempt ring to `<repo>/.refine/fixseq/run-<ts>.jsonl` and the misses queue to "
      "`.refine/misses/` — both are files on YOUR disk; read them before re-running a stuck "
      "lane.")
    w("")
    w("## 8. Domains")
    w("")
    w("`suggest --domain <name>` narrows the fan-out; `auto` fans out over the compiled "
      "domains and the router verdict is a per-span ANNOTATION, never a gate:")
    w("")
    w("| domain | in the prebuilt binary | territory |")
    w("|---|---|---|")
    w(domain_rows)
    w("")
    w("## 9. Reports — read-only, never write")
    w("")
    w("`cargo refine --info` (one-command view: the network's fixes, your contribution "
      "state, local work, corpus) · " + " · ".join(
        "`{}`".format(v) for v in sorted(v for v, g in CATEGORIES.items() if g[0] == "reports")
    ) + ". Report-first: they gate nothing and leave the tree untouched.")
    w("")
    w("## 10. The contribution loop (free, opt-in)")
    w("")
    w("Fixing is always local and free — no account, no network. Contributing your redacted "
      "fix records to the shared corpus (and getting public credit) is opt-in:")
    w("")
    w("```sh")
    w("cargo refine join      # opt in: the contribution policy + a read-only GitHub check,")
    w("                       # binding your random client ID (public credit + leaderboard)")
    w("cargo refine rank      # the leaderboard (joined identities only)")
    w("cargo refine info      # what the network fixed, by domain (counts only)")
    w("cargo refine corpus    # corpus state + the freshness offer")
    w("cargo refine update    # corpus lease refresh (auto-runs ≤ once per 12 h)")
    w("cargo refine sync      # push redacted batches + pull the lease")
    w("cargo refine leave     # undo the join — nothing more is sent")
    w("```")
    w("")
    w("- Joining is one command and reversible (`cargo refine leave`); local healing is "
      "identical joined or not. Every record leaves secret-scanned (a finding drops the "
      "row), never whole files, never your paths. Proven pairs join the public MIT corpus; "
      "the corpus leases back to every machine's retrieval.")
    w("- Contribution points are non-transferable, expiring reputation — never payment, "
      "never a token; they can never be bought, sold, or traded.")
    w("- `--stats on` is a separate consent (per-run rule VOCABULARY + counts, never spans, "
      "source, paths, or repo names). Contribution and stats are independent; neither "
      "implies the other.")
    w("- Offline / unreachable service never fails a heal run — the contribution tail "
      "degrades loudly; healing is local. `cargo refine offline` is the hard firewall "
      "(marker-based; env vars and settings.toml cannot override it until `cargo refine "
      "online`).")
    w("- A depleted shared-corpus account gets a soft gate — a refusal naming the two refuel "
      "paths (contribute or top up); the refusal itself names the opt-out env, and the "
      "metering plane has its own disable env.")
    w("")
    w("## 11. Service URL")
    w("")
    w("`cargo refine config get` prints the ACTIVE service URL and its source "
      "(env > config > default). The default dials the production network:")
    w("")
    w("```sh")
    w("cargo refine config set --url devnet    # pinned alias (also: mainnet)")
    w("cargo refine --net devnet ...           # one-invocation env sugar")
    w("```")
    w("")
    w("`RIIR_REFINE_KAT_SERVICE_URL` overrides everything for one invocation. "
      "`cargo refine config adopt-state <devnet|mainnet>` migrates pre-split network-blind "
      "state — oracle-confirmed adopt or quarantine, never auto.")
    w("")
    w("## Traps we already paid for")
    w("")
    w("1. **`fix` writes.** Review = bare `cargo refine` or `suggest`. The pre-2026-09-08 "
      "docs called the fix flag the dry run — they lie.")
    w("2. **A green verify over cfg-gated code proves nothing** without `--verify-args` — "
      "gated files compile empty at default features (§4).")
    w("3. **The four divergence classes never auto-heal** (§5). Forcing them by hand-editing "
      "the corpus rules produces wrong edits; that silence is the design.")
    w("4. **Bench files are declined on purpose** — their numbers are the artifact. Do not "
      "un-guard a `bench-guard:` decline.")
    w("5. **A silent heuristic-only run is the loud fallback's failure** — if clippy could "
      "not run, the run TELLS you; don't cite its ranking as oracle-anchored.")
    w("6. **The crates.io build is code-fixing only.** `cargo install riir-refine` fixes "
      "code; `join`, the leaderboard and latent retrieval ship in the prebuilt binaries; "
      "the installers verify SHA256SUMS.")
    w("7. **The mining surface is gone.** `--mine`/`--unmine`/`--purge` do not exist — old "
      "docs naming them are stale. `cargo refine join` / `leave` is the contribution "
      "surface, and the run-end CTA is join.")
    w("8. **The production network is the default dial.** The prebuilt binary is net-pinned; "
      "`--net devnet` and off-mainnet env overrides refuse at runtime naming the pin. Money "
      "verbs are operator lanes, not healing lanes.")
    w("9. **The installer never logs in for you.** Local healing needs no account; the "
      "first consent question is answered on YOUR terminal, never pre-seeded.")
    w("")
    w("## Full command surface (generated — the healing + contribution CLI)")
    w("")
    w(surface)
    w("")
    if omitted_note:
        w(omitted_note)
        w("")
    w("Meta surface: {meta}.".format(meta=meta_line))
    w("")
    w("## Freshness")
    w("")
    w("This file is served by the network's service worker at {url} — it changes only when "
      "the worker deploys. It is GENERATED by `scripts/gen_skill_md.py` in the "
      "[riir-refine repo](https://github.com/riir-ai/riir-refine) from the CLI source and "
      "gated by `tests/agent_skill_md_fresh.rs` (a verb the CLI gains or loses fails CI "
      "until the file is regenerated). When the file and the shipped binary's `--help` "
      "disagree, trust the binary, and re-fetch this file after the next deploy.".format(
          url=SERVED_URL))
    w("")
    return "\n".join(out)


# ---------------------------------------------------------------------------


def main() -> None:
    args = set(sys.argv[1:])
    heal_src = HEAL_RS.read_text(encoding="utf-8")
    verbs = extract_verbs(heal_src)
    if "--dump-verbs" in args:
        # The rendered (public) set only — the gate test checks containment
        # against the file, and the omitted ops category is deliberately not
        # in it (the zero-token law keeps that vocabulary off the public
        # surface; the script's adjudication covers those verbs two-sided).
        for v in sorted(
            v for v, (cat, _) in CATEGORIES.items()
            if cat not in PUBLIC_OMITTED_CATEGORIES
        ):
            print(v)
        return
    version = extract_version((CARGO_TOML).read_text(encoding="utf-8"))
    stamp_src = BUILD_STAMP_RS.read_text(encoding="utf-8")
    feats = extract_release_features(stamp_src)
    cio = extract_cratesio_line(stamp_src)
    domains = extract_domains(DOMAIN_ROUTER_RS.read_text(encoding="utf-8"))
    adjudicate(verbs)
    content = render(verbs, version, feats, cio, domains)
    if "--stdout" in args:
        sys.stdout.write(content)
        return
    if "--check" in args:
        current = SKILL_PATH.read_text(encoding="utf-8") if SKILL_PATH.exists() else ""
        if current == content:
            print("gen_skill_md: {p} is fresh (skill v{v})".format(p=SKILL_PATH, v=SKILL_VERSION))
            return
        diff = "\n".join(
            list(difflib.unified_diff(
                current.splitlines(), content.splitlines(),
                "committed", "generated", lineterm=""))[:60])
        raise SystemExit(
            "gen_skill_md: {p} is STALE — regenerate "
            "(python3 scripts/gen_skill_md.py)\n\n{diff}".format(p=SKILL_PATH, diff=diff))
    SKILL_PATH.parent.mkdir(parents=True, exist_ok=True)
    SKILL_PATH.write_text(content, encoding="utf-8")
    print("gen_skill_md: wrote {p} (skill v{v}, riir-refine {ver}, {n} verbs)".format(
        p=SKILL_PATH, v=SKILL_VERSION, ver=version, n=len(verbs)))


if __name__ == "__main__":
    main()
