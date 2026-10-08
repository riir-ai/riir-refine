# Plan 211 — Batch 211: the 4-lead conversion mini-batch (M3)

**Status:** COMPLETE — all gates green, landed

Converted the four staged leads from the 2026-10-08 intake + upstream sweep
(`63cacd06` + `d5178621`) in ONE session, ONE verify cycle. Landed on the M3
under the live Plan-337 hyperthink census (nice-19; the battery is correctness
gates, not timing gates — no GOAT bench owed this batch, so the census's
yielding posture makes the cargo-quiet precondition's INTENT — measurement
validity — moot; wall-time cost only, bounded with CARGO_BUILD_JOBS=8 + nice).

- [x] Verify all four upstream pins verbatim (patch + PR bodies fetched):
      PrismML #290 @ `4fbda1292`; llama.cpp #29100 @ `7d9305fb`;
      llama.cpp #28205 @ `6ed24bce`; bevy #26042 @ `a144d280`
      (merge sha `a144d28034a229c6bc1ce80e1a2c8d955cd72cbc`) +
      rust #163916 @ `aaa57c9a8beec536ad85b6b117ef8c5e1e96f0ba`
- [x] Rider 1 — B116 `fwht-wide-block-row-per-threadgroup-exchange-ladder`
      (entries_gpu_27.rs) += #29100: the upstream CUDA width gate verbatim
      ('Widths 64 to 512 keep the warp kernel. 1024 through 8192 use the new
      one') + the SUPERSET-ADMISSION FALL-THROUGH law (composes-with-tension
      B176: superset admission sound IFF the refusal path is a safe
      fall-through to a correct generic kernel, never assert-after-admission)
      + the stage-tier confirmation (owned)
- [x] Rider 2 — B204 ISA arm `swar-lane-widened-base3-dp4a-dot`
      (entries_gpu_27.rs) += PrismML #290: the i8mm arm VALIDATES in-tree,
      the arch-fallback.h alias-back LIFTS (4 lines deleted); the
      division-free 8-bit-lane identity '((q + (q >> 1)) >> 1) >> 6 == (3q)
      >> 8' verbatim + vhadd halving-add form; the nrc==2 vmmlaq_s32 2x2
      tile reusing decoded trits (vzip interleave); nrows=2 gated
      __ARM_FEATURE_MATMUL_INT8; the Snapdragon 7 Gen 4 A/B (pp64 1.22→2.33,
      tg16 1.09→1.31 — the same device family that measured the plain-NEON
      negative, isolating the arm); greedy output matches generic
- [x] Rider 3 — B208 `fa-fast-path-validated-exact-shape-admission-v-as-strided-view`
      (entries_gpu_78.rs) += #28205: the DTYPE half of the aliasing conjunct
      (V-as-K when KV overlaps → both arrays one type; PR body verbatim) +
      the NEGATION-SCOPE law: '!q(a) && !q(b)' distributed vs
      '!(q(a) && q(b))' compound — a distributed negation admits every
      mixed pair
- [x] New rule — rust_perf #184 `repr-align-register-partition-store-forwarding`
      (entries_18.rs, layout 13→14): the bevy #26042 register-partition law
      (8+4+4 → 4+4+8, world_entity/50000 x0.26, get_entity_mut_slice/20 x1.49
      regression → parity recovered, the honest still-worse observer row) +
      the rust #163916 alignment-drop face as the rider (usize→u32 drops
      PartialRes alignment; (NodeId, PartialRes) 32→20 bytes, −102 MiB
      max-rss) — ONE law, two faces: field width/alignment decides BOTH the
      register partition (codegen) and the container footprint
- [x] Pins: tests.rs total 183→184 + Batch 211 comment; layout 14 prose;
      without_spec 84→85; batch211_rule_present_and_parse (rust_perf);
      batch211_riders_present (kernel_opt, 3 sections); AGENTS.md ×2;
      .docs/11_domains/rust_perf.md
- [x] Gate battery ALL GREEN: batch211 gates first shot (both postures);
      default lib **2477/0**; rust_perf_bench lib **2508/0** (without_spec
      85, annotated 130, total 184); kernel_opt+latent_retrieval lib
      **3088/0** (3 ignored = the documented freeze-regen e2es, 1717 s;
      retrieval floors HELD); clippy -D ×3 postures clean. En-route: the
      unescaped-quote parse-gate catch; three annotation pins 129→130
      (missed in the first pass — the full-lib run is the authority); the
      PRE-EXISTING default-posture clippy red on develop (build_stamp
      assertions_on_constants, latent since 772cae41) fixed with a
      reason-comment allow
- [x] Snapshot row (CRLF preserved, battery results filled) + research
      note 246
- [x] Commit + push

Count pins moved ONLY for the rust_perf rule (riders are count-neutral).
Score-bench/freeze NOT owed (zero clippy-corpus delta, P30).
