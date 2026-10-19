# Plan 259 — Batch 255: the #30637 MoE SLRU host-cache law

**Status:** LANDED 2026-10-19 (all tasks [x]; claim `fbcb7bbf` discharged by the landing commit; Research 290 = RECORD)

Source: `ggml-org/llama.cpp` @
`33d6135cbe20e90e67d9db1c874fbb4a9dc64b19` (#30637, MIT, Evan Huus,
merged 2026-10-19, "moe-cache : replace MoE LRU cache with a 50/50 SLRU
cache"; src/llama-moe-cache.cpp +60/-26, .h +2/-2). Solo rust_perf batch —
the B209/B210/B254 thin-watch-day precedent (exactly 1 new mechanism commit
in the window past the B254 pin `098ce526`). Quotes verified at the pin this
session via `git show` in F:/llama.cpp (clone HEAD since moved to `d7fa69b7`;
the pin is present in history and every read was AT the pin).

The law: a host-side recency cache holding EXPENSIVE entries (whole expert
weight uploads) should be a 50/50 SLRU, not a plain LRU — first touch lands
in the PROBATION half (`move(s, false)` on fill), the SECOND touch promotes
to PROTECTED (`touch` -> `move(s, true)`), protected is capped at
`n_slots/2` with overflow demoting the protected-LRU head; victims come from
an ORDERED scan, probation-head-first then protected, while a request-scoped
generation pin (`seen[key % n_expert] != seen_gen`) skips every expert the
CURRENT request selected — including earlier fills; hits are promoted BEFORE
victims are chosen for misses; `GGML_ABORT("no SLRU victim")` is the loud
impossibility terminal (unreachable while plan()'s uniq <= n_slots
admission holds). Plain LRU thrashes: one large batch sweeping many
single-use experts evicts the repeatedly-used working set; SLRU keeps the
repeats protected and makes the one-touch entries evict among themselves.
Routing: rust_perf, caching category. NotBenchable (upstream ships no
numbers; the win metric is host-resident upload traffic under request
interleave — the B136 decision-content class).

## Tasks

- [x] T0 Watch poll: llama.cpp delta past B254 pin = exactly 1 commit
      (#30637); prism tip `1180918489` UNCHANGED, no tag past
      `prism-b10777` (decode re-pin NOT fired); lithos `5485ad6` / splash
      `be3b083c` unchanged; TensorFold `9ccf4727` -> `d1018b31` = +988
      commits STAGED-NOT-CONVERTED (the B219/B249 multi-slice campaign
      precedent — named leads `018e73c6` digest-cache + `3c0d6d3c` GLM
      prefix-move-before-evict, re-verify at pin at the next slot); bevy
      +8 (`c38fa547bf` #25419 watch-class); wgpu/cubecl deltas unexamined;
      uzu/mlxfast ls-remote timed out x2 (network; mlxfast floor ~10-24).
- [x] T1 Claim row in `.distill/001_mining_queue_snapshot.md`, commit+push
      BEFORE authoring (the push is the arbiter) — `fbcb7bbf`. EN-ROUTE
      REPAIR: the edit lane rewrote the snapshot CRLF (the sixth-time
      hazard, seventh instance) — fixed-forward in the SAME amended
      commit (byte-clean +2 lines, force-with-lease over my own
      seconds-old push; no sibling work existed at that tip).
- [x] T2 Dedup greps re-run at authoring (slru|probation|protected.*lru|
      victim.*selection|second.*touch over the rust_perf + kernel_opt
      corpora — zero owners confirmed; distinct-from set verified in-tree:
      B103 evict-oldest, B241 empty-slot-first-victim-order (entries_28),
      B210 #29887, B237 capacity admission, kernel_opt B189
      shape-bucketed-graph-cache-lru (cache keying, not policy)).
- [x] T3 Author `entries_35.rs` — 1 caching rule
      `slru-probation-protected-promote-on-second-touch`, quotes verbatim
      at the pin (inner double-quotes as single quotes per house
      string-literal safety, disclosed in the module header). EN-ROUTE:
      the B182 Some( lesson re-fired (missing closer paren — rustc blamed
      the STRING, four probe rounds); the fixture `#[test]`s EXECUTED once
      in a throwaway crate pre-landing (both traces proven).
- [x] T4 Wire: `mod.rs` (mod entries_35 + entries() extend + module-header
      batch chain + build-fn doc count 223->232 era update — the header
      was stale at 230 through B250s1, the doc at 223 through
      B246/248/250s1, both repaired with honest labels); `tests.rs`
      count pins: total 232; **with_spec STALE-PIN REPAIR 119->120**
      (PROVEN at HEAD pre-landing: failed `got 120` under rust_perf_bench
      — the B239/B207 pin-hiding class); without_spec 112; annotated
      178->179 at FOUR sites (1914 forty_three, 2055 drafter_propagates,
      the serde test, the unannotated messages x2); batch255 parse +
      needle gate tests.
- [x] T5 Gates: batch255 gates 2/2 FIRST-RUN PASS; default lib 2698/0;
      rust_perf_bench lib 2729/0 (6 ignored = documented freeze-regen
      e2es); clippy -D x4 postures clean (default / all-features /
      no-default / rust_perf, all --all-targets; en-route: 9 doc_markdown
      healed via cargo refine + the F:/ path-as-URL reword).
- [x] T6 Retrieval floors re-measured — HELD no re-pin: blended 125/141
      (88.7%) top-1 >= 124, 131/141 top-3 >= 130, 133/141 top-5 >= 132;
      per-domain all above floors (clippy 87.5/89.6/89.6, perf
      94.6/96.4/98.2).
- [x] T7 Score-bench appended (run #190; heal_rate 0.964 FLAT; sha
      recorded at the claim commit).
- [x] T8 Maintained binary rebuilt at the release set (2m23s dist
      profile) -> `~/.cargo/bin/cargo-refine.exe`; `--version` build
      `fbcb7bbf104c-dirty` == HEAD, release set complete (dirty = the
      uncommitted batch content, resolves at this commit); isolated
      target dirs removed; both new .md files CRLF-corrected to LF before
      the commit (the seventh-instance hazard, caught by the check).
- [x] T9 Research note `.research/290_batch255_slru_host_cache_rule.md`
      (highwater 289->290, dual_allocation_gate clean 0/0/0).
- [x] T10 Docs: AGENTS.md x2 bullets (Role + feature) 231->232; README
      domain-table row; `.docs/11_domains/rust_perf.md` count row + BOTH
      stale `BENCH-VERDICT-PENDING-x86` tokens repaired (the B250-T6
      close updated AGENTS.md + kernel_opt.md only; README + this doc
      kept the PENDING token — repaired with attribution; kernel_opt.md's
      copy stays as a dated historical row). Queue snapshot LANDING row
      (claim discharged).
- [x] T11 Commit `feat: batch 255: ...` + push; verify clean tree after.
