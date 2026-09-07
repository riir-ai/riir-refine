# Plan 109 — Batch 128: the PrismML llama.cpp Metal/FA distill

**Status:** LANDED 2026-09-07 — Batch 128 complete: kernel_opt +5 (565 → 570: A 121, K 11, N 41), 3 skip-ledger entries, GOAT full-pass (gates 3/3 first-shot, lib 1167/1489/1606, retrieval byte-identical, clippy 0 ×3, score-bench #77 FLAT). Research 150. Clone removed at landing.

## Source

**PrismML-Eng/llama.cpp master @ `5ea87ddad22541a37053c7ba92b02ec1923617c6`** (MIT, LICENSE verified at the tip). The unmined Metal/FA surface of the fork Batch 127 mined for its CUDA slice. Target commits (all in the Aug 20–25 upstream window present at the tip):

- `63b64a50` — metal: dequant kv cache only for large batches (#27438)
- `f280b269` — metal: per-device tuned (Q, NE) for flash-attn vec (#26570)
- `78ec4c37` — vulkan: FA MMQ should use fp32 for Q quantization calculations (#27413)
- `a3027337` — metal: clamp K extent in tensor API mat-mat kernel (#27450)
- SKIPS: `3af988fa` (epilogue family covers), `369e1cd6` (row-copy rule covers), `b615f5b4` (build-infra)

Lead estimate: 3–6 rules; landed 5 + skip ledger.

## Tasks

- [x] T0 — stage the source: fresh blob-filtered clone at `5ea87dda` (this session, /tmp), LICENSE MIT verified at the tip; 4090 checked (18% util, no batch collision; LAN refused = documented IP drift, Tailscale used). **DONE**
- [x] T1 — trigger re-scan + dedup: nnethercote no new post (Jul-31), ds4/ds4-metal pins unchanged; the PrismML tip move is the trigger. Dedup greps vs the 565-rule id list + 5 KEEP / 3 SKIP verdicts; every quote re-verified at the tip with exact line numbers. **DONE**
- [x] T2 — corpus entries (entries_gpu_34.rs) + `batch128_*` gates (present / concepts / parse) + count pins walked (totals ×4 sites, per-category table A/K/N, mod.rs lists + comments + history block + stale 555 header healed, router 570). **DONE: gates 3/3 FIRST-SHOT; one trailing-comma syntax fix pre-gate; clippy doc_markdown ×3 + the line-start `+` list-marker misparse (the B125 lesson) healed en route**
- [x] T3 — Batch 128 GOAT: lib default 1167/0 + kernel_heal 1489/0/1i + kernel_heal+latent 1606/0/2i; retrieval re-run kernel 100/100/100, blended 94.7/96.9/97.7 (byte-identical to B118–B127); clippy 0 ×3; score-bench #77 FLAT 96.2/1.9/100/100. **DONE**
- [x] T4 — landing: Research 150, AGENTS.md corpus counts + batch line, `.docs/11_domains/kernel_opt.md` header + Batch 128 row, snapshot update (pin current `5ea87dda`); commit `feat: batch 128: ...` + push. **DONE**
- [x] T5 — remove the working clone; final snapshot line. **DONE: /tmp/b128_recon + /tmp/b128_target removed**
