# Plan 083 — Batch 112: nnethercote July-2026 byval-move memcpy-threshold rule (rust_perf, layout)

**Status:** RECORD — Batch 112 LANDED 2026-09-05 (T1–T8 all done; the rule landed with
its measured M3 verdict AND the same-day x86 re-gate: the threshold is a backend constant
and ARCH-SHAPED — aarch64 inlines 136 B (ldp/stp + 8 B tail; ceiling in (136, 320] by
the 320 B `bl _memcpy` probe), x86-64 crosses at 128 B (asm: `movl $136; callq memcpy`
in the 136-byte loop; 48 B inlines). M3 bench Mixed 1/0/1 recorded honestly; default
lib 1086/0, bench 1104/0, latent 1193/0/1i floors HELD, clippy 0 three states, score-
bench #39 flat.

Source: `.research/134_nnethercote_july2026_perf_search.md` — rust-lang/rust **#160005**
(nnethercote, July 2026): **LLVM lowers any by-value move of a >128-byte type to a `memcpy`
CALL** (not inline load/store pairs) — a hot 136-byte type paid ~half its compile time in
memcpy; the fix shrank the type to 104 bytes so the remaining moves inlined (plus avoiding
2/3 of the moves) → **40% wall-time** on the two worst crates. DHAT copy-profiling was the
finding instrument. The 128-byte threshold IS the decision content.

- [x] T0 — `.plans/.highwater` repair: said 81 while plan 082 existed (the missed-bump
      class, B94/B106 precedent); this plan takes 83, highwater written to 83.
- [x] T1 — Corpus entry `hot-byval-move-over-memcpy-threshold` (layout family, corpus
      100 → 101) via the Python-generator discipline. Bench shapes mirror the
      source EXACTLY (136 B = 13 hot + 4 cold u64; fixed = 104 B + cold tail by
      ref/in-place; equal work pinned; `#[inline(never)]` + `black_box`).
- [x] T2 — Bench spec + G1: `sizes [1_000, 1_000_000]`; alloc axis zero both arms.
      All four batch112 gates GREEN FIRST-SHOT (rules_present / pseudocode_parses /
      concepts_covered / byval_folds_agree_bit_identical + the 136/104 size pins).
      clippy lint fixes during the sweep: needless_range_loop → enumerate/fold,
      explicit_iter_mut → `&mut`, items-after-statements → hoisted to fn top,
      redundant field names → named locals.
- [x] T3 — Riders landed (name-append): `usize-index-to-u32` += #158720
      cache-line-boundary confirmation; `sort-key-cache-precompute-out-of-lock` +=
      #157179 short-sort-key confirmation. Retrieval floors re-run: HELD, no re-pins.
- [x] T4 — M3 BENCH VERDICT (no MLX cell was live; the wait-pass resets on ambient
      Zed/RustDesk compute anyway — recorded per the yield rule):
      **Measured Mixed 1/0/1 — 1.29x/0.99x at 1K/1M (verdict-stable ×2; allocs 0/0,
      alloc-det stable). THE ARCH FINDING (this batch's real yield): the threshold
      is a backend constant and it is ARCH-SHAPED. The corpus gate's asm reading
      (`cargo rustc --emit asm` on the bench shapes) shows a 136-byte by-value move
      INLINING to ldp/stp pairs (4 pairs + an 8 B tail) on aarch64 + current rustc —
      NO memcpy call — while a 320-byte move on the same box emits `bl _memcpy`:
      the aarch64 inline ceiling sits in (136, 320], ABOVE x86-64's 128 B. The 1M
      tie is therefore the honest mechanism cell (both arms inline at 136 B); the
      1K 1.29x cell is warm-up-dominated (per-iter cost halves 1K→1M in BOTH arms)
      and is recorded as never-citable. Annotation authored from the measurement
      with the Mixed verdict + the regime + the armed re-gate; context_notes
      43 → 44, unannotated pinned at 57 (first-pass arithmetic said 56; the runtime
      count refused — the new rule joined the ANNOTATED side).**
- [x] T5 — Count pins walked: corpus.rs doc header 100 → 101 + layout 9 → 10;
      `entries.len()` 101; benchable 86 → 87; annotated 44 (membership added);
      unannotated 57 (corrected after the runtime count refused 56); serde 44;
      propagates 44/57; the stale per-category test prose (layout 7 → 10);
      AGENTS.md 101 + Batch 112 line; `.docs/11_domains/rust_perf.md` 101 + block
      + layout 10; `.distill/001_mining_queue_snapshot.md` Last-landed + NEXT-UP;
      `.research/134` LANDED status addendum.
- [x] T6 — x86 cross-arch re-gate: **DISCHARGED SAME-DAY** (4090 box, quiet — GPU
      5%/660 MiB, no cargo/rustc processes; rustc 1.93.0 x86_64-pc-windows-msvc).
      The probe binary (same 48B/136B/320B arms) built + run over SSH; the asm
      shows the step DIRECTLY: the 48-byte copy has NO memcpy call while the
      136-byte loop's hot loop carries `movl $136; callq memcpy` per iteration
      (the exact #160005 shape). Wall clock moves the same direction: 136 B loop =
      1.32–1.35× the 48 B loop per iteration on x86 (3 reps, verdict-stable) vs
      1.00–1.13× on the M3. The 320-byte arm memcpys on BOTH arches. Annotation
      + entry comment + all doc surfaces updated from ARMED to the discharged
      state; the concepts gate needle updated to pin the discharged contract
      ("x86 RE-GATED" + "callq memcpy"). Cleanup: the box-side probe tree removed.
- [x] T7 — Validation (the B95 honest-skip shape): batch112 gates green (4/4);
      default lib 1086/0 (+4 gates); bench lib 1104/0; latent lib 1193/0/1i +
      `retrieval_precision_regression_gate` PASS + 13/13 retrieval tests (floors HELD,
      no re-pins); clippy 0 in default `--all-targets` + bench + latent states;
      score-bench run #39 FLAT at the floor (96.2/1.9/100/100, miss map unchanged:
      map_unwrap_or ×1, useless_format ×1 — the documented parse-only frontier).
- [x] T8 — Commit `feat: batch 112: byval-move memcpy-threshold rule (rust_perf layout,
      corpus 100->101) + 2 confirmation riders` + push on develop. No `.raw/` (blog
      source, consumed at R134); 4090-side probe artifacts cleaned.

Deferred / out of scope:
- [-] The partitioned-visitor lane (clippy #17124: pass combination over no-op default
      visitor methods, 10-30% runtime) — architectural, no bounded site rewrite;
      recorded as the post-mining watch in R134 §row 4. Re-open only on a real in-tree
      instance.
- [-] `-Zmacro-stats` pricing pass over `neuron-db-sdk`'s derive macro (R134 §B) —
      nightly toolchain; keep-quiet-box discipline defers to a scheduled build window.
- [-] XProtect Developer-Tool exemption (R134 §A) — OWNER ACT (security trade-off;
      never executed unilaterally). Named suspect if gate wall-times inflate on the M3.
