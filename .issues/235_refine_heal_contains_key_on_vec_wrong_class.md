# Issue 235 — wrong-heal class: `unnecessary_get_then_check` fixer rewrites a correct `Vec::get(i).is_none()` into `!contains_key(i)` (E0599) — and the fix lane's compile gate missed the broken posture

**Status:** OPEN — wrong-heal report from a consumer repo (riir-rethink `03ab27a`, 2026-11-11); the healed site is fixed there (`crates`-side bounds check), this issue is the rulewright's repair + the gate-posture lesson.
**Claimed:** zed-b7add734@amo, 1794630584, expires 2026-11-14T12:10:00+00:00

## What happened

`cargo refine fix` (the `clippy_lints` domain, `unnecessary_get_then_check` rule) rewrote:

```rust
if handle.state_strs.get(case_idx).is_none() { ... }   // Vec<String> — CORRECT bounds check
```

into:

```rust
if !handle.state_strs.contains_key(case_idx) { ... }   // Vec has NO contains_key — E0599
```

`contains_key` is a map/set method; on a `Vec` the rewrite does not compile. The receiving
site was riir-rethink `src/decstat_verify.rs:424` (the `decstat`-gated path), and the landing
commit — riir-rethink `03ab27a`, "fix: mechanical clippy fix via cargo refine -
unnecessary_get_then_check (decstat_verify.rs)" — stated its compile gate as
`--features serve-encoder` (that crate's working posture). The `decstat`-gated code compiles
ONLY at `--features decstat`: the gate compiled the wrong posture, the E0599 shipped, and the
consumer repo's pinned `decstat-gates` row (floor 11) has been red since 2026-11-11.

## The two defects

- [ ] **R1 — the rule.** The `unnecessary_get_then_check` fixer must distinguish the receiver
      type: `.get(k).is_none()` on a map-like receiver may rewrite to `!contains_key(k)`;
      on a slice/`Vec` receiver the pattern is a BOUNDS CHECK and the rewrite must be
      `k < receiver.len()` (or the site left for manual repair — a divergence-class entry in
      the `cargo-refine` skill's table is acceptable if the type is not resolvable at the
      rule's check time). Evidence fixture: `Vec<String>` + `get(usize).is_none()` must not
      produce `contains_key`.
- [ ] **R2 — the gate lesson.** A `cargo refine fix` landing whose compile gate names ONE
      feature posture can ship a break in ANOTHER posture's gated code. The record: the fix
      lane compiles baseline→apply→re-check at the GIVEN posture; a file touched under a
      feature the posture does not enable compiles to NOTHING (the green-zero rule one layer
      down). Mitigation to evaluate: the fix lane already prints kept-edit verification
      scoping (P20/P43/P104 audits) — extend the disclosure to name feature-gated regions
      whose feature was NOT in `--verify-args` (the P20 cfg audit may already cover this;
      verify, don't assume).

## Evidence

- Consumer fix landed: riir-rethink commit `03ab27a` (2026-11-11) — the same commit's message
  documents the gate posture.
- Consumer repair: riir-rethink (2026-11-13) — the site now reads
  `if case_idx >= handle.state_strs.len()` with a comment naming this issue; the
  `decstat-gates` row re-reads 11/11.
- Repro of the E0599: `cargo check --features decstat` in riir-rethink at any commit from
  `03ab27a` up to (not including) the repair.
