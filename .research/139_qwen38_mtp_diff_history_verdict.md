# Research 139 — Layr-Labs/qwen-3.8-mtp-challenge: submission-diff history is the asset (corrected verdict + walk spec)

**Status:** DISTILLED — pending owner decision (Batch 115 via Plan 088). This note RETRACTS the same-day queue-snapshot line grading the source "CORPUS DEDUP-CLOSED / LEAGUE INTEL MODERATE" — that grading compounded three errors recorded below. The tip-tree mine (Batch 100 / Research 120) stands and is COMPLEMENTARY, not superseded.

## Source

- `Layr-Labs/qwen-3.8-mtp-challenge`, tip `0863b06` (public; harness Apache-2.0 per LICENSE/THIRD_PARTY_NOTICES; no weights in-tree — fixtures pin digests). 103 commits total; the **submission trail is ~50 Accept/Validate commits, 08-17..08-23** (inspected via GitHub API + commit pages; no clone yet — the walk clones at the pin, Plan 088 T1).
- Each submission = one reviewed, scored, leaderboard-ranked diff with a manifest note stating the technique and its repricing. That diff history is the mineable object. **Nobody has walked it** — Batch 100 read a file subset of the tip tree only.

## Why the first verdict was wrong (recorded, not buried)

1. **One decode-only diff stood in for the arena.** The tip commit (`0863b06`, the xsums fusion) says "prefill run the shipped kernel byte for byte" — true of THAT diff, meaningless for the other ~49. The +273% record is the cumulative of all of them.
2. **"Tip == Batch 100's pin" was read as "mined."** A tip-tree mine (B100: pagedattention.metal, kernel-research.md, LagunaDFlashBlockSession.swift + 5 files) and a diff-history mine are different objects. The diffs contain whole techniques absent from the tip-tree read (below).
3. **The fairness caveat became the verdict.** "Not same-box" is a *number* discipline (851's own rule: format/model-crossed rows are non-scoring reference bars — never a headline, never a gate change). It never licenses skipping the *techniques*. The lane-intel skill rule says exactly this.

## The load-bearing fact: geometry match

README (authoritative Qwen-MTP sections; the Laguna/DFlash prose is inherited from the split-off sibling repo): the backbone is **Qwen 3.8 27B, MLX 4-bit affine group-64, converted from `Qwen/Qwen3.8-27B` bf16** — "64 layers on the same 4-layer hybrid repeat, **vocab 248320, hidden 5120**, 24 attention heads / 4 KV heads, head_dim 256."

That is **Bonsai-27B's exact shape class** (5120 / 248320 / 64-layer GDN-hybrid; dense — the MoE prose in the README belongs to the retired Laguna track, and the "6B-active MoE" in my earlier line was a misreading of the *humanrouter recipe name's* "6bit" quant marker). 851 arm (e) had already flagged `alibaba:qwen3.8:27b` as the family-matched reference. The 248320-wide lm_head — where the arena's biggest decode-side structures live — is **exactly our lm_head width**.

## What the score actually is (fairness facts)

- Track `qwen3.8-27b-mtp-v1`: score = **median over 8 hidden prompts of the raw serial-relative DECODE speedup**; serial depth-0 = 1.0 by measurement; floor 0.90; **ceiling 3.0**; token fidelity absolute (every emitted token must equal the serial trajectory).
- **"The published score is decode-only … There is no separate prefill component — the seed prefill is charged inside the decode window."** So **273.6% = 2.736× serial-relative decode** (speculation apparatus + kernels, MTP is in the editable surface since the 08-14 contract change) on a **self-hosted M5 Max 128 GB** runner. Never citable as a prefill gain; never same-box headroom (M5 vs our M3; 4-bit affine vs PQ2_0 ternary).
- The yukon.org leaderboard displays per-arena prefill/decode TPS informationally (the gemma4 sibling tab's headline combines prefill^0.25 · decode^0.75; this repo's README pins decode-only median — either way decode-anchored).
- **Prefill is score-bearing**: the seed prefill is inside the timed window, and the README's own priority line is "Prioritize kernels reached by **the seed prefill** and the timed decode phase." Pre-fill-touching diffs are rewarded — the walk will surface them.
- Protocol: paired same-session serial/candidate legs, fixed 40 °C thermal gate, random hidden prompt, stall guardrail, one gated retry, `passed_correctness: false` recorded rather than hidden. Our league fairness manifest's shape, independently re-invented — worth a one-line cite in the league doc's methodology notes.

## Diffs read (3 of ~50) + candidates

| # | candidate | diff | grade | dedup (both vocabularies grepped over `src/domains/kernel_opt/`) |
|---|---|---|---|---|
| 1 | `two-level-centroid-shortlist-before-exact-rerank` — 98,336 compact readout rows → 12,292 leaf-mean affine-2/group-64 centroids → score centroids → `probe_sort` top leaves (fraction 0.15–0.25) → gather + exact affine-4 rerank of the 32-row shortlist; pruning gated by a **measured worst-domain argmax miss rate (2.3e-4)**; "0.25 removes 23.0% of the declared head's per-draft bytes" | `eb5eadc` + `0863b06` manifest | **B (D)** — applies to any huge-readout argmax consumer; our 248320 lm_head is the exact shape (consumer-gated: our spec-decode is closed per 721/717, so name the consumer honestly) | B77 packed-total-order (selection ORDER machinery — different concern), B107 radix-select (top-k substrate), B110 degenerate-selection (robustness). Zero hits: centroid / leaf / probe fraction / shortlist. NEW decision content: coarse-to-fine index + measured miss-rate gate |
| 2 | `activation-digit-scale-fold-makes-nibble-extraction-free` — x pre-scaled per nibble stride (`x/1, x/16, x/256, x/4096` per lane), so 4-bit accumulation is raw masked adds (`packed & 0x000f … 0xf000`) with NO shift/lookup extraction | `41bad1c6` | **B− (N)** — likely a **B96 rider** (4-bit sibling of `telescoped-digit-coefficients-byte-fma-dot`; B96's own note already cites the PrismML Q2_0 2-bit spelling) | B96 telescoped-digit (byte coefficients — the 4-bit masked-add spelling is the delta), B92 lop3/prmt (EXTRACT-then-convert — this ELIMINATES extraction), B44 bit-cursor. R unless the walk finds it recurring standalone |
| 3 | `in-kernel-input-side-offset-sum` — the raw activation block-sum accumulated in the SAME loop and applied per group as `result += scale*accum + sum*bias` (offset leaves the inner loop entirely) | `41bad1c6` | **rider** — the third placement arm of the derived-operand family (standalone fill / producer epilogue (the xsums diff) / **in-consumer recompute**). Candidate consolidation: ONE `derived-operand-placement-ladder` rule naming all three arms with their regimes | B96 `bit-plane-plane-sums-input-side-offset-dot` (the MATH — computed "once per slice"; this is in-consumer per-block), B77 quantize-memo, B30 fuse-count-into-mutation |
| 4 | `one-dispatch-gather-score-reduce-preserving-rounding-contract` — `qwen_mtp_draft_selected_affine4_rerank_g64_v1`: replaces `gather_qmm` + separate value/id reducer with ONE dispatch (256 tg = 8 simdgroups × 4-row dot tiles sharing one activation walk), preserving `qmv_fast` BF16 rounding (`float(InT(reduced))`) and the incumbent (value, id) total order | `41bad1c6` | **B− (D)** | B97 bit-exact-dispatch-fusion (boundary EMULATION of a chain — this fuses gather+GEMV+reduce while preserving the rounding CONTRACT; kin not duplicate), B58 same-input segments, B25 device-side-argmax. Confirm recurrence in the walk before landing |
| 5 | `dead-jit-factory-skip-on-first-hot-call` — "Live E87 select never consumes `_draftProbeSort`. Constructing it still JITs `qwen_mtp_probe_sort` on the first draft. Skip that factory while E87 is the first arm." (an UNCONSUMED lazy construction whose build cost = a JIT compile lands on the first hot call) | `eb5eadc` | **C (D/rider on B38)** — lazy factories must be gated on consumption, not merely on config | B38 env-flag-resolved-once (flag READS; this is flag-consistent UNCONSUMED CONSTRUCTION), B39 cost model, B56 caches |
| 6 | `env-gate-name-must-survive-env-sanitizer` — "the `MLX_` prefix is load-bearing: the trusted worker's environment sanitizer drops `MLXFAST_*`, so an `MLXFAST_`-spelled gate would never reach the process that runs the scored round, and **both arms of an A/B would silently measure the same code**" | `41bad1c6` (deleted gate's doc) | **A− (measurement-honesty, B109 family)** — the fake-A/B class: a config-plumbing fault that makes an experiment VACUOUS rather than wrong. Our league/GOAT harnesses pass env vars through wrappers (`env -u`, launchd, DO workers) — directly load-bearing for us | B109 dead-result-timing-loop-folded-under-lto (the FIRST measurement-honesty rule; this is its config-plumbing twin). Zero hits: sanitizer / gate prefix / A-B arms |
| 7 | `head-trim-needs-committed-argmax-id-coverage` — measured negative encoded: "A 49,152 halving was measured on the public longcopy gate and **REGRESSED**: three of its committed argmax ids live in [49_152, 248_044), the head could no longer propose them" | tip tree (Qwen35.swift comment) | **C (D, guard rule)** — any readout trim must sweep the committed id set before shipping | zero hits: trim / committed ids. Small; possibly a rider on #1 |

Sample bias note: 3 of ~50 diffs, chosen by author activity, all decode/draft-side. The prefill-touching subset (seed prefill is score-bearing) is UNREAD — it is Lane A of the walk and the owner's actual question.

## Non-transferable (boundary evidence, record-only)

- The speculation-apparatus diffs (draft schedules 0..8, probe fractions, E-numbered arms, MTP head variants) — 721/717 closed our speculation path structurally; these are economics evidence for that verdict, not adoption candidates. EXCEPTION: the measurement-process practices (resample tickets, zero-diff frontier draws, repricing-vs-frontier notes) are domain-general and feed the measurement-honesty axis.
- Absolute numbers (973 prefill TPS, M5) — never a same-box cell; 851's non-scoring discipline owns this.

## Walk spec (executes as Plan 088 / Batch 115 — plan-first, before any corpus edit)

1. Clone at pin `0863b06` into `.raw/qwen38-mtp` (history needed — diffs ARE the data; `--filter=blob:none` + full history). Verify HEAD, record sha + license.
2. Enumerate the ~50 submission commits; classify each by touched surface from the diff stat + manifest note: **seed-prefill / decode-kernel / draft-head / schedule-apparatus / manifest-only**.
3. **Lane A = prefill-touching diffs FIRST** (the owner's question). Lane B = decode-kernel + draft-head kernels. Two-lane split per the B77+ discipline; coordinator re-verifies every quote at the pin.
4. Dedup greps per candidate (table above is the seed list); author entries with ORIGINAL fixtures (license hygiene — no source text in corpus; the ported-numeric exhaustive-equivalence mandate applies to anything arithmetic).
5. Target yield after ~50% falsification: **3–6 rules + 4–8 riders** across the full walk (B100 took 8 rules from the tip tree; the diff history is strictly richer per-diff).

## Caveats

- ~47 diffs unread; everything above is a 3-diff sample biased toward high-activity authors.
- Swift/Metal, MLX 4-bit affine group-64 — patterns transfer, code does not (our kernels are CubeCL/Metal ternary); ported-numeric rules need the exhaustive gate.
- MTP-apparatus and schedule diffs are boundary evidence only (721/717).
- The scoring re-anchor (08-14) means early-arena diffs optimized under the OLD normalization — per-diff gains are comparable only via their own repricing notes, not across the contract change.
- The first queue-snapshot line and the 851/876 addenda written earlier the same day carry the wrong grading + the "6B-active MoE" misread — all three get dated correction lines in the same commit as this note.

## Queue slot

**Batch 115 via Plan 088**, queued behind Batch 114 (oMLX ANE/GDN — also a prefill lane; owner may reorder, the geometry match argues for priority). On landing: this note's Status → RECORD; snapshot + AGENTS.md counts; `.raw/` removed.
