# Issue 177 — fixer_regate's FROZEN arm calls an escalation_manifest-gated fn ungated

**Status:** OPEN — filed 2026-10-26 (shikuwa idle loop), found while compiling the
issue-176 consumer surface at `--features cf_workers_ai,groq_first`.

## The defect

`src/bin/fixer_regate.rs` line ~602 (the `--arm FROZEN` branch) calls
`run_frozen(&args, &verifier)` UNGATED, but `run_frozen` is
`#[cfg(feature = "escalation_manifest")]`-gated (line ~2701). At any feature
posture where the bin compiles (its `required-features = ["cf_workers_ai"]`
is satisfied) but `escalation_manifest` is OFF, the bin fails with E0425
(`cannot find function run_frozen in this scope` — "found an item that was
configured out").

Reproduced: `cargo check -p riir-refine --bins --features cf_workers_ai,groq_first`
→ error[E0425]. The `cf_workers_ai` feature (Cargo.toml line 503) implies
l4_lora/kat_service/etc but NOT `escalation_manifest` — the posture is legal
per the manifest, so this is a reachable compile break, not a hypothetical.

## The shape

The sibling arm (`hosted_dispatch`, line ~2070) does it RIGHT: both branches
cfg-gated (`#[cfg(all(escalation_manifest, any(seat_rerank, hosted_expert)))]`
/ `#[cfg(not(...))]` with the loud eprintln decline). The FROZEN branch needs
the same two-arm treatment: gate the call site on `escalation_manifest` and
add a not-gated arm that prints the loud "requires --features
escalation_manifest" decline (the sibling arm's wording is the precedent).

## Fix sketch

1. Wrap the `if args.arm == "FROZEN" { .. }` block in the same cfg-all pair
   the hosted arm uses (or cfg the `run_frozen` call + add the decline arm).
2. `cargo check -p riir-refine --bins --features cf_workers_ai,groq_first`
   green; the release-set posture unchanged; `--features
   cf_workers_ai,groq_first,escalation_manifest` still compiles the real arm.
3. Optionally pin the posture in the CI guard lane list (the Issue-856 class:
   a required-features-protected target that breaks at a reachable feature
   posture compiles to nothing in every default gate run).

Found on shikuwa during the issue-176 unit; NOT fixed in that unit (unrelated
surface, the discipline says file-then-fix). Session: shikuwa-i176-redact-rows, 1793031546
