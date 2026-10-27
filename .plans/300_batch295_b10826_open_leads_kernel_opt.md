# Batch 295 — the b10826 open leads: GDN conv-chain + paired mmvf, FWHT block-width, graph-replay no-scan, norm+Hadamard fusion

**Status:** LANDED — Batch 295 (2026-10-27 ~1x:xx +07; corpus kernel_opt 977 → 981 — D 233→235, C 63→64, B 109→110, entries_gpu_131 + one count-neutral rider annotation on entries_gpu_08; all gates GREEN — see .research/333; the landing commit is the record). Claimed by zed-amo-b295-leads/amo (epoch 1793057491, expires 2026-10-28T23:31:31Z) with plan 300 + claim in one commit `b277e98f`, pushed before authoring. Scope = the staged row's OPEN leads ONLY — the #395/#401/#402 quartet EXCLUDED (B288's riders, .plans/293), #391 EXCLUDED (B283's own rule, entries_gpu_128 — the staged row's dedup pointer named the wrong anchor), #393 face (a) EXCLUDED (the B283 upload rider, entries_gpu_01 — the staged row's '#30358' pointer misaddressed).

## Sources (verify EVERY quote verbatim at these revs at T0 — the B96-98 law; the staged row quotes commit messages only, conversion re-reads full diffs)

Fork checkout `/Users/katopz/git/vendor/prismml-llama.cpp` @ origin/prism (tip `gc96baf4ab` = one commit past the scored tag `c9c34bc9`; all lead pins sit at/below the tag):

- **#383 `97ea5aeee1` + #398 `bf8c069fde` — GDN decode conv-chain fused into ONE kernel** (conv-state get_rows + concat + state write-back + ssm_conv + silu + q/k L2 norm) **and the ssm_alpha/ssm_beta single-column mat-vecs as ONE PAIRED mmvf launch** (`mmvf.cu` +63); bit-exact incl MTP; 90–240 fewer kernels/token; +5–10% decode 2B–27B, 9B +7.9% scored at DEFAULT via #398's flip (`GGML_CUDA_GDN_FUSION=0` disables). Staged row: 'the fusion matcher's design follows the launch-bubble analysis behind Hazy Research's megakernel work'. **TWO rules latent**: the conv-chain single-kernel admission (the whole per-token chain incl norm endpoints) + the paired-single-column-mmvf (two single-col mat-vecs sharing the input read in one launch).
- **#396 `ef119d136e` — wider FWHT launch blocks at decode/verify sizes**: 'With 64 or fewer transforms in flight, use min(n/2, 1024) threads instead of 256; the butterfly stage order does not depend on the thread count, so results are bit-identical' (`GGML_CUDA_FWHT_NT=256` restores); +2.6–2.9% 9B, MTP3 +2.5–3.0%, DFlash7 +4.3–4.9%, 27B MTP2 +4.0–4.4% — the in-flight-transform-count→block-width admission law.
- **#393 `63f16c408f` host faces — batched sched input uploads + 'no node scans on CUDA graph replay'**, byte-identical, ALL-arch DEFAULT, ~55–65 µs/decode-step cut. The batched-async-upload family host is ALREADY corpus (the 7th poll's #30358 rider names it); **the NO-NODE-SCANS-ON-REPLAY face is the possibly-unmined half**.
- **#391 `e75baeeb24` — RMS_NORM (+ residual ADD) fused with the signed Hadamard at BOTH norms of every layer**, bit-identical, +4.8–5.4% decode.

OUT OF SCOPE (adjudicated): #395/#401/#402 quartet (B288 P84/P85, `.plans/293`); #397/#394 Hopper-only; #387 Hopper chunked MMA (Issue-971 trigger not fired for Ada); #399 spec-verify rollback (closed-negative lane; the corpus rule is the replay-not-snapshots opposite); #389/#296/#250/#392/#325 server/tests/CPU/PTQ1_0-Turing; #390 head-size-72 vision flash (out-of-lane, ninth-pass adjudication).

## Dedup state (pre-claim greps THIS session; T1 re-verifies 3+ vocabulary variants per face)

- `conv.chain|conv_chain|conv-chain` over `src/domains/kernel_opt/`: **ZERO hits** — the conv-chain class is corpus-absent; nearest kin `single-pass-continuation-scan` (the recurrence scan, NOT the conv chain) — distinct by the staged row's own adjudication.
- #393 no-node-scans: dedup anchors PRESENT — `graph-capture-sticky` (entries_gpu_59), `captured-topology-signature` (gpu_48/gpu_68); T1 adjudicates own-rule vs rider on the graph family.
- #391: dedup anchor PRESENT — `rms-norm-registers-carry-x-between-passes` (entries_gpu_104); register-carry ≠ fusion (the staged row's distinction, re-adjudicated at T1).
- #396: the FWHT ladder family is well-populated (gpu_11/27/45/128 + B288's P85 block-diagonal rider on gpu_27); staged row's note: `wider.{0,20}block` corpus hits are OTHER families — verify at T1; the NEW decision content is the in-flight-count→block-width admission, not the block shape.

## Tasks

- [x] T0 Read the four diffs at the pins in the fork checkout (per-file `git show <sha>` — no tree-wide grep on the checkout); re-verify every staged quote verbatim; note the admission/predicate vocabulary actually shipped (env names, defaults, guards). DONE — two read subagents (#383+#398 / #396+#393) + coordinator re-checks; staging drift found and recorded: #396's pooled range, the #30358 pointer, the #391/#393a stale leads, the B274 law drift.
- [x] T1 Dedup re-verification, 3+ vocabulary variants per candidate face; adjudicate final disposition per face: own rule / rider (named host) / skip. DONE — 4 own rules, 3 exclusions (quartet / #391 / #393a), grep evidence in-rule.
- [x] T2 Author: corpus entries (+ ORIGINAL minimal fixtures where owed) + the batch295 needle + extend the standing syn-parse gate (`batch295_*_parses`). DONE — entries_gpu_131 (4 rules), batch295_rules_landed + batch295_pseudocode_parses.
- [x] T3 Standing gates: batch295 needle; kernel_opt lib (`--features kernel_opt`) 0F; default lib 0F; clippy `-D warnings` ×3 postures (default / all-features / no-default, `--all-targets`). DONE — needle 4/4 + parse 4/4; kernel_opt lib 3258/0; default lib 2749/0; clippy ×3 + all-targets clean (1 doc_markdown heal in-batch).
- [x] T4 Retrieval floors re-measure at BOTH postures (kernel self-variants + blended via the kernel_opt posture; the default-posture `retrieval_` lane per Issue 170 T3) — corpus delta edits rule bodies even when count-neutral; a decay FAILS the batch (fix, or re-pin with the delta recorded). DONE — kernel+latent 22/0 (~207 s), default 21/0 (~51 s); floors HELD, no re-pin.
- [x] T5 Count pins ONLY if an own rule lands (kernel_opt 977 → N; category counts; router expert total if touched; AGENTS.md ×2 sites; `.docs/11_domains/kernel_opt.md` batch row — landed WITH the batch per the B286 precedent). DONE — 981 at all sites (2 total asserts + 33 bare sites + router lineage + B/C/D slices + the Plan-003 expected table + the two sum asserts + AGENTS ×2 + the .docs row).
- [x] T6 Record owed/not-owed at landing: freeze gen2/gen3 (kernel_opt delta → NOT owed per the P30 confirm), score-bench (ONLY if a rust_perf corpus delta lands — none did), release-set binary rebuild (kernel_opt outside the release set → NOT owed; B286/B288 precedent).
- [x] T7 Research note `.research/333_batch295_b10826_open_leads.md` (`.research/.highwater` 332→333; re-scan at write + dual_allocation gate) + the snapshot landing row (claim discharged by the landing) + the staged-lead marks in the staged row. DONE.
- [x] T8 Commit `feat: batch 295: ...` + push. If healer source was touched (it should NOT be — corpus-only), the maintained-binary rebuild gates the push per the B248 lesson. DONE — corpus-only, no rebuild owed; en-route defect (the entries_gpu_13/130 mod-registration collision) fixed pre-push, recorded in the research note.

## Sequencing

- CPU-only lane on amo (no GPU); the census that gated this box has ended; cargo gates run at normal parallelism with a `ps` check before each heavy run.
- Walk #18 (due Tue 00:00Z = 2026-10-27T00:00Z, ~30 min post-claim) is ANOTHER lane's claim — zero conflict (walk = grading session, this = authoring).
- plan-294 resin (M3, to 08:28Z 10-28) and any M3max lane hold OTHER pools — cross-box parallel pool claims are the B292/B294 precedent; this lane touches only the kernel_opt pool.
- #30489 watch-join stays OPEN (head `784fd016` unchanged at the sdk-052 lane's re-check) — B287's rider disclosure stands.
