# Plan 110 — Batch 129: the ds4 Qwen Q2 decode-fusion + prefill-structural re-mine

**Status:** PLANNED 2026-09-08 08:40 +07 — dual trigger verified by API. **→ COMPLETE 2026-09-08 ~11:30 +07: Batch 129 LANDED (Research 151) — 5 kernel_opt + 2 rust_perf io + 5 riders, all gates green (batch129 4/4 FIRST-SHOT; 1168/0, 1493/0/1i, 1610/0/2i; retrieval byte-identical; router annotation re-pinned 88/131 per Issue 020; clippy 0 ×3; score-bench #78 FLAT), clones removed, snapshot updated (CloudRift lead re-pointed to Batch 130 candidate — the queue lead was not an allocation). No open tasks.**

## Trigger (verified 2026-09-08 08:35 +07)

- **Upstream main moved: `c0a6119f` → `f62ca29a`** (2 commits, 2 files, 43 ln): `b6af0adf` "Bound GLM SSD prefill cache priorities so decode can adapt" (kernel-adjacent — the direct successor of B123's mined `static-seed-priority-bounded-to-observed-scale`, same SSD eviction-namespace area) + `f62ca29a` README-only. Standing trigger from Plan 106 T7: "upstream movement past `c0a6119f`" — FIRED.
- **PR #991 head moved: `ffd85d42` → `b85a6174`** (updated 2026-09-07T22:07:20Z, AFTER the B125 mine read). **LINEAR — status ahead, behind_by 0, no force-push** (the B125 rebase trap does NOT apply: the compare IS the true delta). 12 commits (1 upstream merge + ~10 content), 42 files: `metal/qwen4.metal` +285/−54, `ds4_metal.m` +381/−43, `ds4.c` +91/−21, `metal/dense.metal` +16, `ds4_gpu.h` +38, speed-bench writeups ~2.9K ln (qwen38-prefill-structural.md 156 + qwen38-q2-decode-fusions.md 122 + 6.7K ln results JSON), `gguf-tools/qwen4_iq2.py` +49/−14, docs/Makefile/download_model.sh. Size class: MEDIUM. Yield estimate: 4–7 rules + riders.
- **prismml master re-checked same pass: identical to the B128 pin `5ea87dda` (0 ahead)** — the P14 watch's "15 commits past the league pin" are tag-ancestry already inside B128's mine. No prismml trigger.
- 4090 checked pre-mine (LAN): 20% / 565 MiB = idle, no batch conflict. M3 box: no cargo processes, load falling (12.2 → 6.4).

## Commit-message survey (messages are NOT evidence — re-verify every file/line at the pin)

Content commits (fork branch `qwen3.8-flash-next`, newest last):
- `b85a6174` Speed up Qwen prefill with reusable weights and block convolution
- `82d03140` Fuse Qwen ordinary decode projections and residual normalization
- `01a94896` Record REJECTED Qwen MTP output-head fusion experiments (negative results — rule/rider gold per the negative-annotated tradition)
- `66b0e3fc` Select Qwen MTP drafts on GPU without full logit readback (league-adjacent: MTP — riir-ai 717/721 are closed-negative; CONFIRM-only unless the shape is a transferable decision-content rule)
- `0bb323aa` Tune paired Qwen MTP mixer grouping on M3 Ultra
- `b4c35507` Improve Qwen Q2 decode geometry and partial prefill tiles
- `19964345` Speed up Qwen low-bit prefill and decode on M3 Ultra
- `b4501c05` Make the smaller Qwen Q2 release the default download (provisioning — B122 `implemented-surface-requires-provisioning-target` family rider territory)
- `5bd87961` Support padded Q2_K down projections for Qwen3.8
- `38d3e45d` Optimize Qwen prefill with quantization-specialized MoE kernels
- `f9d76f9d` Merge upstream main (carries `b6af0adf` + `f62ca29a` into the branch)
- `b6af0adf` Bound GLM SSD prefill cache priorities so decode can adapt (upstream; B123-family successor — rule-or-rider, dedup grep `static-seed|priority|evict|preload|seed` FIRST)

## Lanes

- **Lane 1 (kernel surfaces, kernel_opt):** `metal/qwen4.metal`, `metal/dense.metal`, `ds4_gpu.h`, `ds4_metal.m` — the prefill reusable-weights + block-convolution shape, decode projection+RMSNorm fusion, Q2_K padded down-projection layout, quant-specialized MoE prefill kernels, Q2 decode geometry + partial prefill tiles, MTP mixer grouping + GPU draft selection.
- **Lane 2 (bench methodology + negatives + io, rust_perf/kernel_opt):** `speed-bench/*.md` decision writeups + the rejected-MTP-fusion commit content + `ds4.c` + `download_model.sh` + `gguf-tools/qwen4_iq2.py` + `docs/QWEN38_FLASH_NEXT.md`.
- **Upstream SSD lane (coordinator-level, tiny):** `b6af0adf` at the upstream pin (~43 ln) — read directly.
- **Dedup needles:** `mtp|draft|readback|logit|fusion|residual|rms|norm|conv|q2|padded|down.proj|tile|geometry|moe|quant|seed|priority|evict|preload` against the corpus (esp. B118/B121/B122/B123/B125 entries) before any rule is written.

## Part C — LEAGUE INTEL (named-not-edited)

- MTP GPU draft selection without full logit readback (`66b0e3fc`) — read-only intel for riir-ai 717/721 (both closed-negative); do NOT reopen without measured parity evidence; a transferable decision-content rule (e.g. "select on-GPU, ship the index not the logits") MAY route kernel_opt/rust_perf if hardware-independent.
- M3-Ultra absolute numbers = foreign-hardware class vs our M3 Max — B110 precedent: corpus serves user code on foreign hardware, valid decision-content rules route; tuning constants do not.

## Tasks

- [x] T0 — stage `.raw/ds4` at `f62ca29a` + `.raw/ds4-metal` at `b85a6174` (fresh clones, HEAD == ls-remote, LICENSE MIT verified at both); re-verify the quoted lines at the pins before any entry lands
- [x] T1 — dedup grep + 2 parallel read lanes (kernel surfaces / methodology+negatives+io) proposing rules (pattern, bounded fix-space, category, rationale, source quote); coordinator reads `b6af0adf` at the upstream pin; re-verify every quoted line
- [x] T2 — corpus entries + `batch129_*` parse gates + count pins (kernel_opt total/category; rust_perf total; walk every manifest/doc line)
- [x] T3 — GOAT: batch gates first-shot + lib default/kernel_opt/kernel_opt+latent counts + retrieval re-run byte-identical + clippy 3 states + score-bench FLAT (isolated `CARGO_TARGET_DIR=/tmp/batch129_target`, removed at landing)
- [x] T4 — landing: Research note (next number per `.research/.highwater` = 151) + AGENTS.md corpus counts + `.docs/11_domains/` rows + snapshot row; commit `feat: batch 129: ...` + push
- [x] T5 — Part C league intel: MTP GPU draft selection (66b0e3fc) recorded as CONFIRM-only intel in the argmax rider (riir-ai 717/721 stay closed-negative; the transferable face — on-device selection shipping an index not logits — banked as the B25 rider facet); M3-Ultra absolute numbers treated as foreign-silicon per the B110 precedent throughout
- [x] T6 — remove `.raw` clones at landing; final snapshot update (pins current: PR `b85a6174`, upstream `f62ca29a`). DONE 2026-09-08
