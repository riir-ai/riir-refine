//! Issue 200 — the FIXER TIER (inner crate, root-package workspace member).
//!
//! Charter (the owner trust rule, issue 200): one binary, two postures —
//! join the contributor cluster with open-source-class targets (TUNA point
//! weights, operator-batched cadence, owner-armed), or self-host on private
//! code: the contributor's own hardware, zero egress to anyone. We never
//! promise custody of proprietary code; we provide the tool that makes
//! custody unnecessary. **"Open-source class" is a CHECKED property,
//! fail-closed**: a target is OSS only if its remote is publicly reachable
//! AND carries an OSI license, verified client-side BEFORE any network
//! connection; anything unconfirmable is PRIVATE → self-host route. The
//! check lands in THIS crate; the trainer crate inherits it.
//!
//! Tier discipline (issue 200, verdict round 4): heavy deps live in THIS
//! manifest as forwarded features of the core (default-OFF), never in the
//! root — feature unification cannot reach the default posture. The KAT/
//! commercial surfaces stay default-OFF so a Mozilla-facing build carries
//! none (the issue 190 T14 discipline, carried up a layer). TUNA point
//! weights are "non-transferable, expiring reputation — never purchase,
//! never token value" (Proposal 025 line 47); contributors ACCRUE, they do
//! not earn. NATIVE-ONLY — the tier bins never join a wasm32 combo (the
//! `hero_planner` posture).
//!
//! ⚠ CWD-relative write (disclosed per issue 200 C2): the migrated
//! `fixer_regate` bin appends `.benchmarks/fixer_quality_history.jsonl`
//! RELATIVE TO THE CURRENT WORKING DIRECTORY. Run it from the repo root
//! (as `scripts/kat_sweeper/run_sweep.sh` does) so the quality history
//! lands where `scripts/gen_dashboard.py` reads it.
//!
//! The migrated bin itself lands with issue 200 C2; until then this crate
//! is a charter stub (lib only, no code).
