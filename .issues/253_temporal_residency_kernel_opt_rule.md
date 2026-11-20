# Issue 253 — Temporal-residency kernel_opt rule (MoE expert residency)

**Status:** DEFERRED `- [-]` — relocated 2026-11-20 from katgpt-rs Issue 987 (closed; Research 657 §A1, Temporal MoE [P]). Trigger: an MoE league artifact is named (Bonsai-2-27B is ternary dense — no MoE consumer exists today).

Lead: resident expert set = last tick's set; one-slot monotonic swap (LaneSlot semantics); one prefetch overlapped with compute; demote-only env switch. Lands as a kernel_opt corpus rule through the normal mining batch window.

- [-] T0 — mine the rule against the named MoE artifact's decode loop; fixture + score-bench row.
