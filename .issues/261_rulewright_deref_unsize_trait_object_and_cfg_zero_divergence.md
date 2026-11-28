# Issue 261 — rulewright divergence: the deref→unsize trait-object rewrite class compiles to NOTHING at the verifying posture (riir-chain E0277 + a dropped let-binding casualty)

**Status:** Open — filed 2026-11-28 (amo idle lane zed-eb03c1fb@amo) from the riir-chain heal; both consumer-side casualties ALREADY FIXED (riir-chain `45cce96a`); **the proposer-side adjudication LANDED same session (the fixture pins + the divergence disclosure) — the guards that refuse both shapes POSTDATE the 11-24 batch, so no new proposer code was owed.** What remains open: the posture hole (below) as a standing watch.

## Adjudication (2026-11-28, same session)

- **Casualty 1** (`explicit_auto_deref`, `&*self.nft_registry` → `&self.nft_registry` at an `&dyn RecipientRegistry` param): the i258 owned-binding guard (`6401ef1a`, 2026-11-27) refuses the exact shape — the operand is a multi-segment path (unproven binding form → refuse-on-uncertainty). The 11-24 batch ran with the guard-less proposer. **Fixture pinned:** `smart_pointer_field_projection_at_trait_object_param_refuses_i261` (the measured shape through the public `propose_fix` seam). Residual SILENT class disclosed in the module's divergence table: an operand PROVEN owned (owned `let` init) at a `&dyn` param still breaks — parse-only cannot see the parameter type; bounded to owned-let operands, caught by `--verify` only at a posture that compiles the call site.
- **Casualty 2** (the dropped `let status = ` in the chaind e2e canary): the let_unit_value value-break guard (`926d1608`, 2026-11-24 06:56 ICT — 3 h AFTER the batch) refuses `loop { break <value>; }` inits. **Fixture pinned against the MEASURED shape:** `trait_object_param_value_break_loop_let_refuses_i261` (a match arm's `break s` inside the loop body — the guard's wholesale descent covers it; now provably).
- **The P20 question is ANSWERED — no P20 mystery:** both guards postdate the batch; the compile gate at the default posture saw both cfg-gated sites compile to nothing (the green-zero class), and the guards that now refuse the shapes did not exist yet. The standing posture hole that REMAINS: **a cfg-gated edit verified at a posture that compiles it to nothing can ship a break the NEXT posture that arms the gate reds** — P20's module-decl audit (issue-182 class) is the disclosure layer for exactly this; its reach (does it fire on a `#[cfg]` one hop up on `pub mod attribution_agent;`?) is verified by its own tests, and the consumer-side net is the repo's own `--all-features` layer (riir-chain guard Layer 3, which HAD been passing only because nobody ran it between 11-24 and today).

## Standing watch (what stays open)

- The sweep gate for cfg-gated repos: after any mechanical batch on a repo whose sites are feature-gated, the consumer repo's own `--all-features` clippy is the verifying posture. The batch runner cannot know this parse-side — the residue is the consumer repo's guard discipline (riir-chain Layer 3 exists and is green again); no refine-side code change owed.
- The trait-object-callee silent class (owned-let operand + `&dyn` param) stays disclosed-only; re-open bar: a SECOND measured instance whose operand is an owned let.

## Provenance

- Consumer-side fixes: riir-chain `45cce96a` (heal + comments at both sites).
- Guards that close the classes: `6401ef1a` (i258, 2026-11-27) + `926d1608` (255 T0+T2, 2026-11-24 06:56 ICT).
- Standing rule honored: "wrong suggestion → fix the proposer at root cause" — here the root cause was temporal (guards landed after the batch), so the deliverable is the measured-shape fixtures + the disclosure, not new refusal code.
