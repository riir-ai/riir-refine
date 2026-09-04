# Proposal 003 — Binary-only distribution (source stays secret)

**Status:** proposed (binding policy adopted 2026-08-14; execution awaits a release plan) — **UNBLOCKED 2026-08-26**: `Issue 037` CLOSED (all gates green incl. A1 cadence — record: [AGENTS.md](../AGENTS.md) §Issue 037 + git history). The moat claim is now proven E2E (verdicts → learning → bench, E1–E5). First public release may proceed under its own release plan.
**Branch:** `develop` (per global rule — no feature branches)
**Owner:** unassigned
**Fusion of:** [Research 003](../../riir-ai/.research/003_Commercial_Open_Source_Strategy_Verdict.md) (the boundary verdict) × the rustup installer pattern (curl|sh + install.ps1) × cargo-dist prior art (installer UX + tap automation)
**Related:** [Proposal 001](001_multi_domain_modelless_code_healer.md) §"Product vision: `cargo heal` CLI", [Proposal 002](002_real_file_span_healing.md) §Layer 3 (the CLI this distributes), [Plan 005](../.plans/005_real_file_span_healing.md) Phase 6

## TL;DR

> **Verdict: adopt — `riir-clippy` (and every `riir-*` product built on it) ships
> ONLY prebuilt binaries. Source never leaves the private repos.** Per Research
> 003's absolute rule ("anything `riir-*` is internal. No exceptions."), the
> public surface is a **separate tiny public distribution repo** hosting GitHub
> Release assets + installers — because release assets on a *private* GitHub
> repo are **not publicly downloadable** (the load-bearing constraint that
> forces the two-repo split). macOS/Linux install via `curl | sh` + a Homebrew
> tap; Windows via `install.ps1` + a Scoop bucket (winget later, after
> signing). This is standard release engineering (cargo-dist, rustup, goreleaser
> all do variants of it) — the novelty here is only the private-source twist,
> which cargo-dist does NOT support out of the box, hence a thin custom
> pipeline.

## The problem this solves

`cargo heal` is about to become a user-facing product (Plan 005 Phase 6 `--fix`
is the production-usable milestone). Nothing in the repo currently defines how
it reaches users — and the default paths a future agent might reach for are ALL
leaks:

| Path | Leak |
|---|---|
| `cargo publish` to crates.io | Full source public. **Blocked already**: `publish = false` in `Cargo.toml` L7. |
| GitHub Releases on `gist-rs/riir-clippy` | Repo is PRIVATE (verified 2026-08-14 via `gh repo view`) — assets require auth, so this doesn't work at all, AND if the repo were ever flipped public the entire source leaks. |
| Public mirror / "open-source the tool for adoption" | Violates Research 003's absolute rule. The proposals ([001](001_multi_domain_modelless_code_healer.md), [002](002_real_file_span_healing.md)) never propose this — audited 2026-08-14, zero hits — but nothing *forbids* a future session from "helpfully" doing it. |

The gap: **no documented distribution architecture + no standing guardrails**.
This proposal is the record + the rules.

## The proposed design

### The two-repo pattern (the core mechanism)

```
PRIVATE (source, forever)                 PUBLIC (distribution, tiny)
──────────────────────────                ──────────────────────────────
gist-rs/riir-clippy  (PRIVATE)            gist-rs/cargo-heal  (PUBLIC)
  src/**                                    README.md        (capabilities, NOT how)
  .plans/.proposals/...                     install.sh       (curl | sh)
  CI: tag push → build matrix →             install.ps1      (iwr | iex)
      strip → package → upload ──────────►  Releases: cargo-heal-v0.1.0-*
      (robot PAT, contents:write             {aarch64-apple-darwin,x86_64-unknown-linux-musl,...}
       scoped to cargo-heal repo ONLY)       .sha256 + THIRD_PARTY_LICENSES.md

                                           gist-rs/homebrew-tap  (PUBLIC)
                                             Formula/cargo-heal.rb
                                             (url → cargo-heal release asset)

                                           gist-rs/scoop-bucket  (PUBLIC)
                                             bucket/cargo-heal.json
                                             (url → release zip)
```

**Why the split is mandatory:** GitHub Release assets on a private repo require
authentication to download (verified: cargo-dist issue #2267 exists exactly for
this). The public repo solves it while containing **zero Rust source** — only
markdown, shell, PowerShell, a Ruby formula, and JSON manifests.

### The canonical install one-liners (Issue 061 T4.3, 2026-09-02)

**This proposal owns the install copy.** Every other document mirrors these
two lines verbatim; none of them invents a URL. The single source of truth:

```text
macOS/Linux:  curl -fsSL https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.sh | sh
Windows:      iwr -useb https://raw.githubusercontent.com/gist-rs/cargo-heal/main/install.ps1 | iex
```

Enforced by `tests/install_copy_parity.rs`: any URL in any tracked `*.md`
that ends in `install.sh`/`install.ps1` and is not one of these two FAILS.
It is a source-text audit, and it exists because the copy had already
drifted — [Proposal 005](005_kat_heal_service_network.md)'s TL;DR served both
installers from the **vanity host `cargo-heal.gist.rs`, which does not
exist**, while [Proposal 011](011_kat_pay_request_qr_genesis_and_onboarding.md)
§8.1 advertised the `raw.githubusercontent.com` form. Two documents, two
one-liners, one of them unrunnable, and nothing to catch it: the copy is a
string literal in prose and there was no structure for it to disagree with.

**Why `raw.githubusercontent.com` and not the vanity host:** it works the
moment the dist repo exists, with no DNS record, no TLS certificate and no
redirect to maintain. `cargo-heal.gist.rs` is a **deferred alias**, not the
canonical form — promote it here (and re-run the audit) if and when it is
actually served.

**Neither line is runnable yet, and that is a repo-creation gap, not a copy
gap.** `gh repo view gist-rs/cargo-heal` on 2026-09-02 answered *"Could not
resolve to a Repository"* — T1.1 below has never been executed. The audit
pins what the copy WILL point at; publishing binaries built from private
source is an owner act.

### Platform matrix

| Platform | Channel 1 (zero-infra) | Channel 2 (package manager) | Notes |
|---|---|---|---|
| macOS (aarch64) | `curl -fsSL …/install.sh \| sh` | `brew tap gist-rs/tap && brew install cargo-heal` | Tap repo MUST be named `homebrew-*` → `gist-rs/homebrew-tap`. Formula downloads the **prebuilt** release tarball (binary formula — allowed in third-party taps; only homebrew/core requires build-from-source). |
| macOS (x86_64) | same | same | Either a separate asset or a `universal2-apple-darwin` lipo build. |
| Linux (x86_64, aarch64) | same install.sh | (tap n/a) | **Static musl builds** (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`) — no glibc-version coupling. |
| Windows (x86_64) | `iwr -useb …/install.ps1 \| iex` | `scoop bucket add gist-rs https://github.com/gist-rs/scoop-bucket && scoop install cargo-heal` | Scoop is the brew-equivalent for portable zips + closed source. **winget later**: winget-pkgs accepts closed-source packages (manifests just point at URLs + hashes) but unsigned exes hit SmartScreen — add after Authenticode signing + stable versioning. Chocolatey possible, not worth the moderation overhead initially. |

Installer scripts detect OS/arch, download the matching asset, verify SHA256,
install to `~/.cargo/bin` (already on PATH for Rust devs — the target audience)
or `/usr/local/bin` fallback. Bin name `cargo-heal` gives the `cargo heal`
subcommand UX for free (cargo subcommand convention, already the shipped bin
name in `Cargo.toml`).

### Release pipeline (CI sketch, in the PRIVATE repo)

Tag push `v*` → matrix build: `aarch64-apple-darwin`, `x86_64-apple-darwin` (or
universal2), `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-pc-windows-msvc` → `strip` → tar.gz (unix) / zip (windows) → generate
`SHA256SUMS` + `THIRD_PARTY_LICENSES.md` (cargo-about; MIT/Apache-2.0 deps
require license-text inclusion for **binary** distribution) → `gh release create
-R gist-rs/cargo-heal` with a fine-grained robot PAT (contents:write on
`cargo-heal` ONLY) → tap + bucket manifest bumps (PR from the same job).

### Binary leak-surface checklist (what the shipped binary reveals)

1. **Symbols**: `[profile.release] strip = true` — removes the symbol table
   (function names would otherwise map the internal architecture).
2. **Panic paths**: panic messages embed relative `src/...` paths + line
   numbers. Module names become visible; source does not. Acceptable. Optional
   hardening: `RUSTFLAGS="--remap-path-prefix=$(pwd)=/build"` in release CI.
3. **Corpus texts**: rule names + `ref_shape`/`opt_pattern` texts are compiled
   in AND printed by `--suggest` by design — that's the WHAT (capabilities),
   not the HOW. The moat (drafter ternary weights, routing directions,
   self-evolve loop architecture) stays compiled behavior, per Research 003
   "capabilities, not methods".
4. **Feature set of the public binary**: `default + latent_retrieval +
   rust_perf` (exactly the `cargo-heal` bin's required-features). **NEVER**
   `ternary_inference` (needs GGUF at runtime, pulls the inference stack —
   wrong product surface) and never dev/debug profiles.
5. **`env!()`/`option_env!()` audit** at release time — nothing secret baked in.
6. **Dist-repo guard**: CI in `gist-rs/cargo-heal` fails if any `*.rs` file is
   ever committed there. The tap/bucket repos contain only `.rb`/`.json`.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **cargo-dist does NOT fit as-is.** It is the obvious prior art (generates
   the exact shell/powershell installers + homebrew tap we want), but its model
   assumes a PUBLIC source repo: installers point at the source repo's release
   URLs. Adapting it means fighting the tool. The recommendation is a ~200-line
   custom workflow doing the same 5 steps. Re-evaluate if cargo-dist grows
   first-class "build private, publish to separate repo" support (issue #2267
   is adjacent, not it).
2. **Binaries are reverse-engineerable.** Binary-only distribution raises the
   bar, it does not make extraction impossible (strings, disassembly). The
   corpus + architecture are partially recoverable by a determined RE effort.
   This matches Research 003's threat model (raise replication cost, keep the
   adaptive/learning layers as the moat) — but state it honestly: the
   `--suggest` corpus is effectively public the day the binary ships.
3. **Homebrew tap maintenance is manual-ish.** Formula version bumps per
   release; `brew audit` style points (stable URLs, checksums) apply. If the
   tap rots, `curl | sh` still works — the tap is convenience, not the spine.
4. **winget is deferred for real reasons**: unsigned exes → SmartScreen
   warnings; winget-pkgs submission wants stable URLs + ideally signed
   installers (MSIX/Inno + signtool). Don't rush it; install.ps1 + scoop cover
   Windows day one.
5. **The robot PAT is a new secret** with write access to the public dist repo.
   Fine-grained, single-repo, contents:write only; rotate on any suspicion.
   (Worth noting: that token canNOT leak source — it only writes to the
   source-free repo. Worst case is a tampered binary release, which SHA256SUMS
   signing discipline + (later) artifact signing mitigates.)
6. **macOS notarization is NOT needed** for CLI binaries via tar.gz + brew +
   curl (quarantine xattr is set by browsers/LaunchServices, not curl). It
   BECOMES required if we ever ship `.pkg`/`.dmg`/`.app` bundles. Don't add
   Apple Developer cert complexity until then.

## Fusion lineage

1. **[Research 003](../../riir-ai/.research/003_Commercial_Open_Source_Strategy_Verdict.md)**
   — the boundary verdict: "anything `riir-*` is internal. No exceptions." +
   the capabilities-not-methods rule that governs what the public README of the
   dist repo may say.
2. **rustup's installer pattern** — `curl … | sh` + `install.ps1` with
   OS/arch detection, checksum verification, PATH installation. Copied shape,
   not code.
3. **cargo-dist** (axodotdev) — prior art for the installer UX + auto-updated
   homebrew tap; explicitly NOT adopted wholesale (caveat 1).

## GOAT gate (distribution gates, not perf gates)

- **G1 (install works from a clean machine):** fresh macOS (brew + curl|sh),
  fresh Linux (curl|sh, musl static — verify on a glibc-2.17-era container),
  fresh Windows (ps1 + scoop). `cargo heal --suggest` runs end-to-end on a
  fixture crate on all five assets.
- **G2 (no source leak in artifacts):** `strings`/`nm` spot-check on shipped
  binaries — no absolute build-machine paths, no symbol table (strip verified);
  dist repos contain zero `.rs` files (CI guard green).
- **G3 (no-regression):** `publish = false` intact in `Cargo.toml`; private
  repos' visibility unchanged (PRIVATE); repo test suite unaffected.
- **G4 (integrity + compliance):** SHA256SUMS verifies for every asset;
  `THIRD_PARTY_LICENSES.md` present in every archive (MIT/Apache obligation).

## What ships now (riir-clippy) vs deferred

### Ships now — policy + guards (this proposal)
- This proposal + the AGENTS.md "Source secrecy & distribution" section
  (binding for future agents) + the Plan 005 Phase 6 distribution note +
  README status line.
- Already-present guard cited: `publish = false` (`Cargo.toml` L7).

### Deferred — execution (needs its own plan when a release is scheduled)
- `[profile.release] strip = true` (Cargo.toml — deferred deliberately: the
  file is mid-edit by a sibling session today; the change lands with the
  release plan, not mixed into unrelated work).
- The CI workflow, the three public repos, installers, tap/bucket manifests,
  cargo-about license generation, the dist-repo `*.rs` guard.

### Explicitly NOT shipped by this proposal
- Any source publication of any `riir-*` repo — forever (Research 003).
- crates.io publication (also blocked by `publish = false`).
- winget/Chocolatey channels, macOS notarization, code signing, self-update
  (axoupdater-style) — all later, each with its own trigger.

## Phased rollout (sketch — a plan would expand this)

### Phase 1 — Scaffolding (when first release is wanted)
- [ ] T1.1 Create `gist-rs/cargo-heal` (public): README (capabilities-not-how), install.sh, install.ps1, `*.rs` CI guard.
- [ ] T1.2 Create `gist-rs/homebrew-tap` + `gist-rs/scoop-bucket` (public, manifest-only).
- [ ] T1.3 Fine-grained robot PAT (contents:write on `cargo-heal` only) as an org secret in the private repo.

### Phase 2 — Release pipeline (private repo CI)
- [ ] T2.1 Tag-triggered matrix workflow (5 targets; musl static for linux; universal2 or per-arch for mac).
- [ ] T2.2 `[profile.release] strip = true` + optional `--remap-path-prefix`.
- [ ] T2.3 Package: tar.gz/zip + SHA256SUMS + THIRD_PARTY_LICENSES.md (cargo-about).
- [ ] T2.4 `gh release create -R gist-rs/cargo-heal` + tap/bucket manifest bump PRs.
- [ ] T2.5 `env!()`/secret-bake audit step.

### Phase 3 — GOAT gate
- [ ] T3.1 G1–G4 per the gate table above; record in `.benchmarks/`.

## Risks

1. **Token compromise** → tampered public binaries. Mitigation: fine-grained
   single-repo PAT, rotation discipline, checksums, later artifact signing.
2. **Tap/bucket rot** → broken brew/scoop installs while the spine (curl|sh)
   keeps working. Mitigation: manifest bumps automated in the release job.
3. **Accidental source push to a public repo** (the original concern).
   Mitigation: the AGENTS.md rule + the dist-repo `*.rs` CI guard + this
   proposal's record.
4. **License non-compliance** (MIT/Apache binary distribution requires
   notices). Mitigation: cargo-about generated file shipped in every archive.

## Out of scope

- katgpt-rs / katgpt-core distribution (already public MIT on crates.io — the
  adoption funnel per Research 003; unchanged by this proposal).
- Distribution of any other `riir-*` product (each gets its own dist repo when
  it has a user-facing binary; same pattern, same rules).
- Telemetry/update servers, pricing, licensing of the binary itself (EULA
  choice is a business decision recorded elsewhere).

## References

1. [Research 003 — Commercial Open Source Strategy Verdict](../../riir-ai/.research/003_Commercial_Open_Source_Strategy_Verdict.md) — the boundary rule this enforces (distilled in-house; the governing document).
2. [cargo-dist](https://github.com/axodotdev/cargo-dist) + [its book](https://axodotdev.github.io/cargo-dist/book/introduction.html) — prior art: prebuilt-binary release engineering for Rust (shell/powershell installers, homebrew tap automation). Cited-only; NOT adopted (assumes public source repo).
3. [cargo-dist issue #2267 — Homebrew Private Repos](https://github.com/axodotdev/cargo-dist/issues/2267) — evidence that private-repo release assets need auth strategies; the constraint forcing the two-repo split.
4. [rustup](https://github.com/rust-lang/rustup) — the `curl | sh` / `install.ps1` installer pattern (cited-only, shape copied).
5. `gh repo view gist-rs/riir-clippy` (2026-08-14): `visibility: PRIVATE` — verified no existing leak; full org audit: all 12 `riir-*` repos PRIVATE.

## TL;DR

**Adopt: source stays secret forever; users get prebuilt `cargo-heal` binaries from a small public dist repo (`gist-rs/cargo-heal`) via brew tap / curl|sh / install.ps1 / scoop; winget + signing later. Next action: none until a release is actually scheduled — then open a plan from §Phased rollout, and add `strip = true` with it.**
