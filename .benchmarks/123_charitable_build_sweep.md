# Bench 123 — the charitable (Posture A) build sweep: issue 190 T14 regular bar, as a named posture

**Status:** PASS — the T14 regular bar is met and re-runnable (`scripts/charitable_gate.sh`, landed `dd3efe8c`); the STRICT bar stays owner-gated (the `kat_redact` decision, issue 190 T14a row).
**Date:** 2026-11-01 · **Session:** zed-m3-t14-bar (lease on issue 190, expires 2026-11-03T07:35Z) · **Issue:** riir-refine 190 T14 (Mozilla Human-Scale AI Incubator readiness)

## What ran

`scripts/charitable_gate.sh` — builds the charitable posture
(default + `latent_retrieval,rust_perf,clippy_lints`, the T14a-min verified
shape, `e7cf72ea`) into an ISOLATED target dir with
`--profile dist` + `RUSTFLAGS="--remap-path-prefix $HOME=/build"`, then
adjudicates the T14 regular bar against the artifact. The release set is
never touched (`release_set_matches_workflow` stays pinned; this is the
second posture, by name).

## Bar verdicts (all green at `dd3efe8c` + working tree = same)

| Bar | Surface | Verdict |
|---|---|---|
| 1 | `--help` | zero pattern hits |
| 1 | `--version` | 2 disclosure-class lines exempt (verbatim in the gate output; cap 2) |
| 1 | bare dry-run (fixture dir) | zero pattern hits |
| 1 | `--fix --no-verify` scan (fixture) | zero pattern hits |
| 2 | `cargo tree -e features -i riir-kat` | `redact` ONLY |
| 2 | `cargo tree -e features` (forward) | no client/transport/service feature resolved |
| 3 | hard-zero classes (`X-Kat-`, KAT host literals, `/mining/`, soft-gate/burn/meter machinery) | **zero** — no exemption path exists for this class |
| 3 | R1 disclosed class (help/refusal text carrying `kat_service\|kat_transport`) | 29 strings, cap 40 |
| 3 | R2 env-name census | 2/2 pins present (`RIIR_REFINE_KAT_ACCOUNT_DIR`, `RIIR_REFINE_KAT_CRANK_TOKEN`); 1 merged-const artifact disclosed |
| + | `--mine` at the charitable posture | refuses rc=2 naming the missing features |
| + | `binary_leak_scan.sh` on the artifact | PASS (no symbols, no machine paths, no secret shapes) |

Pattern canary (run explicitly, both directions): a sentence carrying
join/service/earn/credit matches (rc 0); the tagline's "learns" does NOT
match (rc 1) — the word-boundary form is neither blind nor over-broad.

## The two disclosure classes (T14a's recorded carve-out, now enforced + capped)

1. **Bar-1 class (cap 2):** the build stamp's release-set STALE disclosure and
   its rebuild hint — posture honesty printed by `--version`, naming exactly
   what is compiled out. Printed verbatim by the gate on every run.
2. **R1 class (cap 40):** the `--help-all` maintenance listings + long-form
   command help fragments + typed-refusal texts (`--candidates requires a
   build with --features kat_service,score_bench`, `stats: built without
   kat_service`) + the build-stamp release-set const blob. None reachable
   from the four bar-1 surfaces. 29 strings at this landing; the class
   GROWING past the cap fails the gate loud.

## Notes for the record

- **`.rodata` byte adjacency:** `RIIR_REFINE_KAT_ACCOUNT_DIR` and
  `XDG_CONFIG_HOME` pack with no NUL between them, so a naive census reads
  `RIIR_REFINE_KAT_ACCOUNT_DIRXDG_CONFIG_HOME`. The gate's R2 classifier
  accepts a merged row only when the remainder carries no second KAT token
  (any other `RIIR_REFINE_KAT_*` name contains one) and discloses the
  artifact verbatim.
- **The gate found real things on its first run** (kept as evidence in this
  session): the `--version` STALE disclosure hits, the `--help-all` listing
  strings baked into `.rodata` at the charitable posture, and a first-draft
  R2 census that passed VACUOUSLY through a POSIX-sh process-substitution
  syntax error — fixed to a temp-file exclusion; a broken exclusion must
  never be a pass.
- **POSIX sh discipline:** no process substitution, no EXIT trap (the macOS
  bash 3.2 abort-launder class), grep rc adjudicated at every decision point.
- **Hermetic build:** `CHARITABLE_TARGET_DIR` defaults to a fresh `mktemp -d`
  per run — a charitable build never shares the release-set cache. The
  sweep's build dir is deliberately NOT the repo `target/` (sibling cargo
  contention + posture pollution).

## Box state (m3, correctness sweep — no perf claims)

MacBook Pro M3 Max 16-core, 64 GB, AC power (charged), load avg 31.6/25.5/20.3
(sibling agents actively building — irrelevant to a string/grep/tree sweep;
no latency number is claimed here). Commits: gate script `dd3efe8c`; the
binary's embedded build hash at sweep time was `0e0117d5b790` (the issue-190
claim commit — pre-dates the script, the posture's feature set is unchanged
by both).

## Strict bar residue (owner-gated, unchanged)

`kat_redact` decision: gate it behind `kat_service` (charitable builds lose
secret-scanning) vs inline the table into riir-refine (two-repo sync class)
vs vendor a shared leaf. Until decided, `riir-kat` stays non-optional at its
feature-free+`redact` tier — which T14 check (2) explicitly blesses. The
strict bar (zero `kat_*` crates) is deliberately NOT asserted by the gate.

## Next

- T14's public README disclosure row ("an optional credit/networking lane
  exists upstream; these builds exclude it — an inert accounting-types crate
  remains linked") — dist-repo surface, drafted alongside this record.
- The sweep must stay green before T4's pilot soft-launch installers ship
  (issue 190 T4).
