//! Batch 427 — the `pico-dgx-ds4` engine distill (Plan 420 / Research 454).
//! Source: Picom-code/pico-dgx-ds4 @
//! `922dddb1a35e521148022ee11264632da1f0ddc0` (tip == pin, commit-dated
//! 2026-11-20T09:06:13-06:00; MIT, Copyright (c) 2026 Andy Shu — a fork of
//! `blaesus/dsgx10`). NINE rules — the certified sharded argmax head
//! (residual bound + rank-shared threshold), prompt/prediction copy drafts,
//! duplicated pad rows with masked state writers, the cross-shard
//! fixed-order decision gate, the Gumbel noise keyed (seed, position, id)
//! draft/verify coupling, the topology-keyed ring-direct transport with
//! bits frozen, smallest-slot-class-first admission, batch-aware expert
//! re-routing with the never-single-session guard, and tile-local
//! exponent-base coding with a raw escape. ALL suggestion-only
//! NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the
//! dial9/136 annotation class; the mechanisms and their defect shapes are
//! CPU-testable and the fixtures pin them exact). Faces F1-F12 land on
//! their hosts in the same commit. Fixture preflight: `b427_fixtures`
//! standalone-compiled AND executed pre-entry (debug AND -O, 43/43).
use crate::domains::kernel_opt::KernelRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![

        KernelRule {
            rule_id: "certified-argmax-residual-bound-shared-across-shards".to_string(),
            gpu_concept: r"MECHANISM (certified coarse-read argmax over a sharded LM head; category B — correctness): greedy decoding needs only the argmax of the vocabulary head, not all 129,280 logits. The step: (1) read the FP8 copy of the head (the drafter's, already resident per rank) for every vocabulary row of the rank's slice — a coarse logit C_v; (2) bound each row rigorously |exact − C_v| ≤ ‖x‖·B_v where B_v is the norm of the row's QUANTISATION RESIDUAL, computed once at start from the real weights — engine/model/model.cu:1389-1391 verbatim: 'B[v] = 1.001 (||w_v - w8_v|| + K 2^-21 (||w_v|| + ||w8_v||)). Then |exact kernel's logit - FP8 kernel's| <= ||x|| B[v]: Cauchy-Schwarz on the residual, and each kernel's fp32 sums within K 2^-21 sum |w x| of the true dot'; (3) take the best lower bound C_v − ‖x‖·B_v over the four TP ranks — ONE small collective; (4) keep every row whose upper bound C_v + ‖x‖·B_v reaches it and run the UNCHANGED exact kernel on the 16-row groups holding a kept row, every other logit −inf; the usual sampler then finds the token. The mask kernel fail-opens: 'A forced step, or a threshold or norm that is not finite, sets every CTA' (model.cu:1299-1301). THREE ATOMS, each unowned in this corpus: (a) THE BOUND IS THE RESIDUAL NORM — whole-row bounds are vacuous on this head: 'Centroid + radius clusters (CSV-Decode) and frequency-ordered early stopping are vacuous on this head: ||w|| ||x|| is about 185 logits against a top logit of 39' (stack4.md:81-82) — the FP8 residual is 2.66 % of the row, so the bound must be built from ||w_v − w8_v||, never ||w_v||; (b) THE THRESHOLD IS SHARED ACROSS THE RANKS: 'Per-rank thresholds do not work either (ranks that do not hold the winner cannot prune): the threshold has to be shared' (stack4.md:82-83) — a rank that does not hold the winner has a weak lower bound and prunes nothing, so one collective takes max(lower bounds) and every rank masks against it; (c) THE COARSE-CODE CLIFF (stack4.md:73-79; 4,944 real final hiddens, one global threshold): FP8 copy 2.66 % residual, survivors/row median/p99 1/84, head bytes read 65.7 %; 7-bit 2.69 %, 1/90, 58.7 %; 6-bit 5.40 %, 7/9,518; 5-bit 9.21 %, 4,962/126,376 (> 95 % bytes); MXFP4 12.5 %, 65 % of all rows (> 100 %) — a monotone cliff in coarse precision; only the 8-bit copy pays. THE WINNER ALWAYS SURVIVES: 'The winner always survives (its upper bound is at least its own value, which is at least any lower bound), ties included, and its logit is computed by the same kernel as before' (stack4.md:66-67) — the certified path never substitutes a coarse value; it only decides WHICH groups the exact kernel reads. MEASURED (the source's check mode — the full head runs beside the certified one in the same step, on every rank; stack4.md:85-96): 1,344,416 real step rows checked, 0 mismatches on every rank, the largest |coarse − exact| 8.3 % of the bound; the mismatch definition is exact — the certified logits must give the same (maximum, lowest id) as the full head on that rank, or the full head's rank max sits below the shared threshold; the two adversarial arms: fallback forced every 3rd step (10,336 steps ran all 2,020 groups — the fallback branch is real) and bound scaled ×4 (2,018 of 2,020 groups read — the no-prune regime is real); identity set 54/54, held texts 320/320, qeval 71/71; head −0.18..−0.28 ms a step, chat 88.17 → 89.45 (CHANGES.md row 15, repo root). THE LAW: (i) the certificate is a WEIGHT property computed ONCE at start from the real head and its quantized copy — never a per-scan build; (ii) the bound is the RESIDUAL norm, never the whole-row norm (the vacuity datum); (iii) the pruning threshold is the best lower bound SHARED across the shards — per-rank thresholds are correctness-neutral but prune nothing on winner-less ranks; (iv) fail-open by construction: forced steps and nonfinite thresholds/norms mask every CTA — the fallback is the exact head on more groups, never a guess; (v) the winner's logit comes from the unchanged exact kernel. SCOPING: 'one-session steps whose rows are greedy without logprobs' (CHANGES.md row 15); sampled requests, requests with logprobs and batch (multi-user) steps keep the full head (stack4.md:68-69). REGIME: greedy serving where the head is too wide to read per token and a quantized coarse copy is already resident (the drafter's); distributions that prune (the FP8 row: median 1 survivor/row at 65.7 % of head bytes). ANTI-REGIME: sampled/logprob/batch steps (full head); coarse codes below 8 bits on this family (the cliff: 5-bit reads more than 95 % of the bytes — the certificate costs more than it saves); consumers needing the full logit vector (temperature, top-p over the whole vocabulary, logprobs); a head with no resident quantized copy (the coarse read is the premise). ⛔ KINSHIP (adjudicated both directions): `certified-prefilter-omission-bound-escalate-exact` (gpu_158) is the same certify-then-escalate GENUS — THAT bound builds DURING the scan from the candidates the per-partition shortlist refused (the next-omitted barrier, carried through the merge), escalates PER ROW on certificate failure, and adds a refuse-safe −inf bias so the cheap filter can never discard an uncertifiable row; THIS bound is a WEIGHT property computed once at start from the real weights, the decision is a threshold SHARED ACROSS TP ranks via one collective (a distributed certificate), and the escalation granularity is the exact kernel's 16-row GROUPS. Both are correct-by-certificate laws; they compose (a partition screen feeding a sharded head carries both certificates). `int8-screen-error-bound-certifies-exact-rescore` (gpu_121) owns the matmul-route screen whose bound E is a per-group weight property with single-device per-row admission (tau <= i2 - 2E) — closest on the weight-property axis; the cross-rank shared threshold, the residual-vs-whole-row vacuity contrast, and the coarse-code cliff table are HERE. The house Issue-940 determinism law is respected by construction: the collective is an associative max over lower bounds — order-free. GOAT (fixture pins): (a) the winner always survives the shared-threshold mask, ties included (lowest id wins); (b) per-rank thresholds keep the winner too but prune nothing on winner-less ranks — the shared threshold's survivor count is strictly smaller there (the pruning-power arm); (c) a flat distribution (inflated residual) reads more groups — monotone. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_CERT=1; CHANGES.md row 15). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/docs/stack4.md:57-96 (Lever 3) + engine/model/model.cu:1299-1301 (k_cert_mask), :1389-1391 (cert_setup bound) + CHANGES.md:313 (row 15). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: read the whole head for every row, or prune with a
// WHOLE-ROW bound against a PER-RANK threshold — ||w||*||x|| is ~185
// logits against a top logit of 39, so nothing prunes; and each rank
// masks against only its own best lower bound.
struct HeadRow {
    coarse: f64,
    row_norm_x: f64, // the vacuous bound: ||w_v|| * ||x||
    exact: f64,
}

fn full_head_argmax(head: &[f64]) -> usize {
    let mut best = 0;
    for (i, &v) in head.iter().enumerate() {
        if v > head[best] {
            best = i;
        }
    }
    best // every logit read, every step
}

fn vacuous_per_rank_prune(head: &[HeadRow]) -> Vec<usize> {
    let lower = head.iter().map(|r| r.coarse - r.row_norm_x).fold(f64::INFINITY, f64::min);
    // the bound exceeds the top logit: every row survives, nothing saved
    head.iter().enumerate().filter(|(_, r)| r.coarse + r.row_norm_x >= lower)
        .map(|(i, _)| i).collect()
}
".to_string(),
            opt_pattern: r"// FIXED: the bound is the QUANTISATION RESIDUAL norm (2.66 % of
// the row), computed once at start from the real weights; the pruning
// threshold is the best lower bound SHARED across the ranks (one
// collective); the exact kernel still answers for the surviving groups —
// the winner's logit is the exact kernel's own.
struct CertRow {
    coarse: f64,
    xb: f64, // |exact - coarse| <= xb (the residual bound)
    exact: f64,
}

fn shared_threshold(shards: &[Vec<CertRow>]) -> f64 {
    // one small collective: max over ranks of each rank's best lower bound
    shards.iter()
        .map(|s| s.iter().map(|r| r.coarse - r.xb).fold(f64::NEG_INFINITY, f64::max))
        .fold(f64::NEG_INFINITY, f64::max)
}

fn survivors(shards: &[Vec<CertRow>]) -> Vec<(usize, usize)> {
    let t = shared_threshold(shards);
    let mut keep = Vec::new();
    for (s, shard) in shards.iter().enumerate() {
        for (i, r) in shard.iter().enumerate() {
            // the winner: upper >= its own value >= any lower bound (ties too)
            if r.coarse + r.xb >= t {
                keep.push((s, i));
            }
        }
    }
    keep // the exact kernel runs on the 16-row groups holding these
}
".to_string(),
        },

        KernelRule {
            rule_id: "copy-drafts-verified-never-trusted-single-request-steps".to_string(),
            gpu_concept: r"MECHANISM (draft sourcing from echoed input; category D — dispatch): when a reply repeats text already in the prompt (a file rewrite, an edit, an agent echoing code) or in the client's `prediction` field, the tokens that followed the same text there are offered to the verify step as drafts, in place of the draft module's few. The host keeps 'an index of 4-token runs over the prompt (kept per session slot, extended when the next prompt starts with the old one) and over the prediction; the longest backward match of the reply's tail picks the source position; a copy goes on while the model's tokens equal the source's' (docs/copy.md, the pieces table). The window constants (engine/model/ops.h:48-49): kCopyWin = 4 windows a step, kCopyTok = 256 tokens in all. THE TRUST LAW, verbatim (ops.h:45): 'Every draft is verified as any draft is, so a wrong window costs rows, never a token.' The device-side alignment gate (spec_copy_pick, engine/model/spec_batch.h:57): `if (t[0] != s.next_tok[0]) return 0;  // the window does not hold what the model wrote` — a window is used only when its token at the entry's first real row equals the model's actual token. The host REPLAYS the same layout function ('spec_batch.h copy_fit, the same function on device and host') 'for the rows' Engram lookups and dies if the device laid out other rows' (copy.md, the layout row) — the copy-delta variant dies with `copy delta: ...` 'as the engine dies on a layout mismatch rather than draft from another text'. SCOPING: 'a copy window is only used in a step that holds ONE request (DSGB10_COPY_SHARED=0), so that steps shared by several users stay exactly the steps of before' (copy.md). WIDTH ADAPTATION: rejected copies double the match the next one needs — '1 (default): every copy in a row that was not accepted whole doubles the match the next one needs (up to 8x); a copy accepted whole resets it' (engine/server/copy.h:18-20) — and whole acceptance doubles the width: `width = std::min(2 * width + 1, max_width)` on `accepted >= drafts`, reset `k.reset ? std::max(k.n0, width / 2) : k.n0` (copy.h:212-213). MEASURED (copy.md's table; 4x DGX Spark, TP4, 1M context, 16 slots, decode tok/s): rewrite of an 8K-token file 169 → 468; 4K 164 → 459-465; 1K 164 → 365-370; a partial prediction 162 → 225; a WRONG prediction 164 → 166; chat/code/math with nothing to copy 107.1 → 107.2; 'the logit bits of every emitted token were compared: 0 of 219,356 differ'. Anti-costs: novel text gains nothing at ~32 bytes/prompt-token/slot of host memory; concurrent rewrites 'gain almost nothing'; the copy step graph takes ~0.26 GiB per rank. THE LAW: (i) drafts from ANY source are never trusted — the device verifies them exactly as any draft (greedy: the longest prefix equal to its own samples; sampled: the sampler's own draw per position), so sourcing can be a host-side heuristic with zero correctness exposure; (ii) the entry-token gate positions the window — a wrong or misaligned window costs rows, never a token; (iii) the host replays the SAME layout function the device ran and refuses to continue on divergence (fail-loud two-executor agreement); (iv) scoping: single-request steps only — shared steps stay byte-identical to before. REGIME: replies that repeat prompt/prediction text (file rewrites, edits, code echo, agentic turns replaying context); the `--prediction` request field; steps holding one request. ANTI-REGIME: novel text (no source to match — the draft module's own few); steps shared by several requests; requests with images or video (the field is ignored); a prediction over 4 MiB or of a wrong shape ('no prediction, never an error'); a first-token-latency-critical lane (the third-turn shape measured 3 ms later in 3 of 3 pairs). ⛔ KINSHIP (adjudicated both directions): `pre-sampled-verify-extends-speculative-eligibility` (gpu_123) owns the SAMPLER-ELIGIBILITY axis (which postures may speculate at all) — THIS owns where the drafts COME FROM (echoed input text) and the host/device layout replay contract; they compose (copy drafts ride the same eligibility rules). The drafts/verify economics family — `cost-aware-verify-length-survival-cost-table` (gpu_86), `speculation-scheduler-break-even-flip` (gpu_29), `verify-window-weight-load-amortization-exact` (gpu_163), `expert-prompt-row-batch-amortizes-weight-pool-stream` (gpu_110) — owns HOW MANY drafts a step verifies and the weight amortization of the verify window; none owns sourcing from echoed text, the entry-token gate, or the replay-and-die host/device contract. `prefix-commit-priced-worthiness-typed-outcome` (rust_perf entries_93) owns cache-commit worthiness pricing — adjacent economics, different sink. GOAT (fixture pins): (a) a wrong window changes the rows consumed but the reply text is identical (drafts are proposals only); (b) the entry-token gate rejects a window whose token disagrees with the model's actual token; (c) a host replay diverging from the device layout refuses the run (die); (d) a shared step never takes a window. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_COPY + --prediction, both default-off; served set arms them). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/docs/copy.md (whole doc) + engine/model/ops.h:45,48-49 + engine/model/spec_batch.h:46-66 (spec_copy_pick) + engine/server/copy.h:18-20, :212-213. Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: the host's predicted tokens are FED to the reply as
// accepted text — a wrong window puts a wrong token in the reply, and
// nothing checks that the window still matches what the model wrote.
fn naive_copy_reply(window: &[u32], steps: usize) -> Vec<u32> {
    let mut out = Vec::with_capacity(steps);
    for j in 0..steps {
        // trust the window wholesale; the model never verifies a row
        out.push(window[j.min(window.len() - 1)]);
    }
    out
}
".to_string(),
            opt_pattern: r"// FIXED: a window is a BATCH OF DRAFTS; every row is verified as
// any draft is — a wrong window costs rows, never a token. The device
// gate: the window's token at the entry's first real row must equal the
// model's actual token; the host replays the same layout fn and dies on
// a mismatch; a step holding more than one request takes no window.
enum CopyPick {
    Window(usize), // window index armed for this entry
    None,          // the draft module's own few
}

fn copy_pick(gate_token: u32, window_token_at_entry: Option<u32>, shared_step: bool) -> CopyPick {
    if shared_step {
        return CopyPick::None; // DSGB10_COPY_SHARED=0: shared steps stay of before
    }
    match window_token_at_entry {
        Some(t) if t == gate_token => CopyPick::Window(0),
        _ => CopyPick::None, // the window does not hold what the model wrote
    }
}

fn verified_row(draft: u32, model_token: u32, out: &mut Vec<u32>, rows: &mut usize) {
    *rows += 1; // the window is paid in ROWS, whatever the verdict
    out.push(model_token); // the reply always carries the model's own token
    let _ = draft; // accepted or rejected, the token is never substituted
}
".to_string(),
        },

        KernelRule {
            rule_id: "pad-rows-duplicate-last-real-row-mask-state-writers".to_string(),
            gpu_concept: r"MECHANISM (fixed-graph row buckets with union-neutral padding; category A — memory access): steps of 1..16 rows keep today's kernels, graphs and bits; steps of 17..64 run the wide body of the bucket that holds them — 24, 32, 48 or 64 rows. The rows past R in a bucket are PAD ROWS: 'copies of the step's last real row (same token, position, slot), so they add no experts to the union and compute the same values; every state writer masks rows >= `in->rows` (device value), so a pad row never writes KV, compressed KV, index keys, carries, draft windows or samples' (docs/wide64.md:70-75). The in-source layout (engine/model/spec_batch.h:113-117): 'the entries past R repeat the last row's (wide64: its token too, so that a wide bucket's pad rows compute row R - 1's values and add no experts)' — tokens, positions, slots and sessions all copied. THE ATOM: zero-padding is additive-neutral (a fold of zeros changes no sum — owned); a DUPLICATED REAL ROW is set-UNION-neutral — its routing hits the same experts its original already loaded, so it adds no new expert ids to the batch's MoE union while computing the same values; the masked state writers then make the copies OBSERVATIONALLY INERT (nothing a pad row computes escapes into state). Pad-value choice is a COST-model decision, not a convention: pick the value that is neutral under the tile body's per-row EXPENSIVE operation — additive reductions want zeros, routing/union costs want duplicates. MEASURED (wide64.md:38-56, 'Measured on the engine (increment 2, 4 ranks, real weights, 2026-11-05)'): ms per row, natural routing — 16-row decode path 3.6-4.0; 32-row wide 3.0-3.3; 48-row wide 2.4-2.5; 64-row wide 2.2; experts/layer 47-52 at 16 rows vs 112 at 64 (the union the duplicates must not grow). Deployed (`DSGB10_WIDE=48`, wide64.md:4): code16 296 → 350 tok/s, agent 215 → 240, c1 unchanged, qeval 71/71, needle 3/3. EXACTNESS, honestly scoped: 'each session's samples are identical across 32-, 48- and 64-row steps (row results do not depend on the batch); against the 16-row decode path 35 of 768 teacher-forced argmaxes differ (ulp-level drift flipping near-ties on this synthetic code/doc text), golden-next-token matches 444 vs 446 of 768' — operator-accepted on qeval; rows must be INDEPENDENT within the wide family for the pad to compute row R−1's values. THE LAW: (i) choose the pad value by the cost the body pays per row — a duplicate real row is neutral for set-union costs where a zero is not; (ii) the pad is inert only if EVERY state writer masks rows ≥ the device-visible row count, and that count travels IN the buffer ('device value'), never recomputed host-side; (iii) the pad computes the same values only while rows are independent — a cross-row reduction reads pad values. REGIME: fixed-graph/tile bodies fed by variable row counts (capture-once CUDA graphs over bucketed shapes); MoE-style per-row routing where the union cost dominates; row-independent kernels. ANTI-REGIME: kernels with cross-row reductions (the pad's values enter the sum); rows cheaper than the bucket's padding cost (bucket-hopping wastes more than it saves); state writers that cannot mask (a body writing per-row state unconditionally turns the pad into corruption, not padding). ⛔ KINSHIP (adjudicated both directions): `pad-tail-to-fixed-tile` (gpu_08) + `zero-pad-ragged-tail-additive-identity` (gpu_53) own ZERO/sentinel padding of ragged tails — additive identity; THIS owns the DUPLICATE-a-real-row pad whose neutrality claim is SET-UNION (the MoE expert union), a different algebra: zeros are the identity of +, duplicates are the identity of ∪ over expert ids; they compose (a body with both an additive tail reduction and a union cost needs both pads, or a duplicate with masked writes). `expert-member-rows-width-ladder-bit-identical` (gpu_81) owns bit-identity ACROSS width rungs of a fold — THIS owns making a wider rung reachable at all (the pad plus the mask that makes pad rows inert). `verify-window-weight-load-amortization-exact` (gpu_163) + `expert-prompt-row-batch-amortizes-weight-pool-stream` (gpu_110) own the related economics (more rows per weight read) — the amortization motivation, not the padding law. The copies-vs-distinct benchmark mechanism (rust_perf entries_84/94 faces) observes the same union cost in measurement — THIS is the production-side lever over it. GOAT (fixture pins): (a) union-invariance — duplicated pad rows add no experts to the union while computing the same values; (b) the polarity arm — zero-pad rows DO change the union under a routing model; (c) masked writers keep state byte-identical with pads present, an unmasked writer corrupts (the tamper arm); (d) a cross-row reduction reads pad values (the anti-regime arm). NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_WIDE buckets; served posture 48). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/docs/wide64.md:66-75 (the pad law), :38-56 (the measured table) + engine/model/spec_batch.h:108-122 (the layout) + docs/wide64.md:4 (the deployed numbers). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: pad a short step to the bucket with ZERO rows — each
// zero row routes by its own (meaningless) scores, so the MoE expert
// union grows and the body reads expert bytes the real rows never
// needed; any per-row state writer happily persists the pad rows.
#[derive(Clone, Copy)]
struct RowScore([f32; 4]); // routing scores over 4 experts

fn top_experts(s: &RowScore) -> [usize; 2] {
    let mut idx = [0usize, 1];
    if s.0[1] > s.0[0] {
        idx = [1, 0];
    }
    idx
}

fn zero_pad_step(rows: &[RowScore], bucket: usize) -> (Vec<[usize; 2]>, Vec<usize>) {
    let mut padded = rows.to_vec();
    padded.resize(bucket, RowScore([0.0; 4])); // additive-neutral, union-EXPENSIVE
    let union: Vec<[usize; 2]> = padded.iter().map(top_experts).collect();
    let state: Vec<usize> = (0..bucket).collect(); // pads written!
    (union, state)
}
".to_string(),
            opt_pattern: r#"// FIXED: pad rows are COPIES of the last real row (same token/
// position/slot) — they route to the same experts, so they add nothing
// to the union while computing the same values; every state writer masks
// rows >= in.rows (the count travels IN the buffer), so a pad row never
// writes state.
fn duplicate_pad_step(rows: &[RowScore], bucket: usize) -> (Vec<[usize; 2]>, Vec<usize>) {
    let last = *rows.last().expect("a step has at least one row");
    let mut padded = rows.to_vec();
    while padded.len() < bucket {
        padded.push(last); // union-neutral by construction
    }
    let union: Vec<[usize; 2]> = padded.iter().map(top_experts).collect();
    let real = rows.len(); // in->rows: the device value
    let state: Vec<usize> = (0..bucket).filter(|r| r < &real).collect(); // masked writers
    (union, state)
}
"#.to_string(),
        },

        KernelRule {
            rule_id: "decision-bits-identical-across-shards-fixed-order-on-reduced-stats".to_string(),
            gpu_concept: r"MECHANISM (cross-shard decision determinism; category N — dtype/alignment, the distributed-determinism class): a LEARNED verify-depth gate decides how many drafts a step verifies — a decision that cannot differ across TP ranks or the step graph forks. The shape: each rank reduces its vocabulary slice per depth to three scalars — (max, second, sum-of-exp) — 'in the block's fixed order' (engine/dspark/dspark.cu:508-510 verbatim: 'This rank's record of depth j = blockIdx.x from its biased logits dl[j]: the max, the second (the best below it in better()'s order), and the sum of exp(v - max) in the block's fixed order'); ONE all-gather joins the records in rank order; 'One thread, a fixed order of fp32 operations: the same bits on every rank' evaluates the 15→64→5 MLP (dspark.cu:539-540). Inputs per depth (docs/stack4.md:45-48 + CHANGES.md row 14, repo root): 'per depth the shipped logit, top-1 margin and top-1 log p (a scan kernel per rank, one all-gather, one MLP kernel), 64 hidden units; trained offline on 661K anchors'. THE SAME-WEIGHTS LAW (engine/model/spec.cu:815 verbatim): 'DSGB10_CF_MLP=FILE (dspark::kCfWords fp32 words; the same file on every rank, or the start dies)' — a gate whose weights differ across ranks would fork decisions; the boot refuses. MEASURED (stack4.md:43-48, Lever 2): held-out log-loss .335 → .298; on rows logged from the engine itself .429 → .383; 'The oracle gate (verify exactly the drafts that will be accepted) is worth +18 % on chat; the shipped head gets little of it'; accepted drafts per step chat 1.17 → 1.24; chat 87.17 → 88.17, code 129.29 → 130.50, math 134.93 → 136.11 (CHANGES.md row 14). THE GROUPING HAZARD, the honest clause (stack4.md:50-55 verbatim): 'a gate never chooses a token. Every emitted token is still the full model's own top choice in the step that produced it. But this engine's logits depend slightly on how positions are grouped into steps (the same token at the same position scored log p -0.24 as a step's first row and -0.10 as its fifth). A gate that verifies different drafts changes the grouping, and on a close call the top choice can flip. Measured: with alpha 1.0 all 374 test outputs (54-prompt identity set + 320 held prompts) equal the exact build's; with alpha 0.9 one of the 320 differed by one word.' THE LAW: a cross-shard DECISION (not just a reduction) is bit-identical only if ALL FOUR legs are pinned: (i) each shard's contribution is a FIXED-ORDER reduction to typed scalars — the tie rule travels with the reduction (better()'s order: value first, then lowest id); (ii) the join order is canonical (rank order); (iii) the evaluation is ONE thread, ONE fixed fp32 op order over the gathered scalars — a rank-varying evaluation order (thread-count-dependent trees, atomics, reversed folds) forks the decision and the step graph with it; (iv) the weights/side-data are byte-identical per rank — the boot dies otherwise. REGIME: any learned/dispatch gate evaluated redundantly on shards that must agree (verify-depth, draft admission, step-graph selection); decisions feeding grouping, whose side effects are text-visible on close calls. ANTI-REGIME: gates recomputed from un-reduced raw data (the reduce-then-gather IS the win — the gather of a full vocabulary defeats it); gates whose inputs are already rank-local (no cross-rank agreement needed); consumers that tolerate divergence. ⛔ KINSHIP (adjudicated both directions): `fixed-order-interleaved-accumulator-epilogue` (gpu_49) + `expert-member-rows-width-ladder-bit-identical` (gpu_81) own ORDER-PINNED FOLDS (a reduction's op order pinned so its result is stable across runs/widths) — THIS owns the reduce→gather→fixed-order-DECISION shape: the reduction is only the input leg, the decision evaluation's fixed order is the content, and the failure mode is a FORKED STEP GRAPH (divergent control flow across ranks), not a drifted sum. `cross-rank-reduce-pins-rank-order-not-ring-order` (gpu_102) owns the cross-rank reduction's rank-vs-ring order — THIS composes with it on the gather leg and adds the decision half. The house Issue-940 law (synced f32 reductions pin reduction order + one fixed tie rule) is the substrate law this rule extends from REDUCTIONS to DECISIONS: a decision crossing the step-graph boundary pins reduction order AND tie rule AND evaluation order AND weights. GOAT (fixture pins): (a) the fixed-order evaluation gives identical bits on every rank while a rank-varying order drifts bits and forks the decision on a close call; (b) the tie rule travels with the reduction (equal values resolve to the lower id identically on every rank); (c) a weights-file mismatch refuses at boot (the same-file law); (d) the grouping-hazard arm — a forked decision re-groups rows and a near-tie token flips, while identical decisions keep texts identical. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_CF_MLP/DSGB10_CF_ALPHA, off by default; CHANGES.md row 14). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/dspark/dspark.cu:507-540 (cf_scan_kernel + cf_mlp_kernel comments) + docs/stack4.md:41-55 (Lever 2) + engine/model/spec.cu:815 (the same-file law) + CHANGES.md:312 (row 14). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: each rank reduces its slice and evaluates the gate its
// own way — a thread-count-dependent tree here, a sequential fold
// there. The decision that picks how many drafts a step verifies then
// differs across ranks and the step graph FORKS.
#[derive(Clone, Copy)]
struct Rec {
    max: f64,
    second: f64,
    sum_exp: f64,
}

fn gate_per_rank_style(rank: usize, recs: &[Rec], w: &[f64]) -> usize {
    let acc = match rank % 3 {
        0 => fold_sequential(recs, w),
        1 => fold_pairwise_tree(recs, w), // f64 non-associativity
        _ => fold_reversed(recs, w),
    };
    depth_from(acc) // rank 1 may disagree with rank 0: the graph forks
}

fn fold_sequential(recs: &[Rec], w: &[f64]) -> [f64; 5] {
    let mut acc = [0.0; 5];
    for (j, r) in recs.iter().enumerate() {
        acc[j % 5] += r.max * w[j];
    }
    acc
}

fn fold_pairwise_tree(recs: &[Rec], w: &[f64]) -> [f64; 5] {
    fold_sequential(&pair_recs(recs), w) // a different summation shape
}

fn fold_reversed(recs: &[Rec], w: &[f64]) -> [f64; 5] {
    fold_sequential(&recs.iter().rev().cloned().collect::<Vec<_>>(), w)
}

fn pair_recs(recs: &[Rec]) -> Vec<Rec> {
    recs.chunks(2).map(|c| Rec { max: c[0].max, second: c[0].second, sum_exp: c[0].sum_exp }).collect()
}

fn depth_from(acc: [f64; 5]) -> usize {
    acc.iter().filter(|&&a| a > 0.5).count().max(1)
}
".to_string(),
            opt_pattern: r"// FIXED: every rank reduces its slice to typed scalars in the
// block's fixed order (tie rule included), ONE gather joins them in
// rank order, and ONE thread evaluates the MLP in one fixed fp32 op
// order — the same bits on every rank; a weights-file mismatch refuses
// at start.
fn gate_fixed_order(recs: &[Rec], w: &[f64], weights_digest: u64, expect: u64) -> Result<usize, GateBoot> {
    if weights_digest != expect {
        return Err(GateBoot::WeightsDiffer); // the same file on every rank, or the start dies
    }
    let gathered = gather_in_rank_order(recs); // canonical join
    let mut acc = [0.0f64; 5];
    for (j, r) in gathered.iter().enumerate() {
        // one thread, one fixed order of fp32 operations
        acc[j % 5] += r.max * w[j];
    }
    Ok(depth_from(acc)) // identical bits, every rank
}

fn gather_in_rank_order(recs: &[Rec]) -> Vec<Rec> {
    recs.to_vec() // rank r's records appended in rank order
}

enum GateBoot {
    WeightsDiffer,
}
".to_string(),
        },

        KernelRule {
            rule_id: "gumbel-noise-keyed-seed-position-id-shared-draft-verify".to_string(),
            gpu_concept: r"MECHANISM (noise-keyed draft/verify coupling; category E — parallelism): sampled speculative decoding stays lossless when the speculative noise is a PURE FUNCTION of (seed, position, token id), shared by every row that scores that position. The source: 'The noise is the untruncated sampler's g(seed, position, id), so a draft row and the row that verifies it see the same noise' (engine/model/sample.cu:347-348). The hash (sample.cu:350-355): base = mix64(seed) ^ (pos * 0x9E3779B97F4A7C15); per token g = uv[i]*invT − logf(−logf(unit(mix64(base + ui[i]*0xD1B54A32D192ED03)))) — a Gumbel draw keyed by the token id; the pick among kept tokens is the Gumbel-max. THE COUPLING LAW (engine/dspark/dspark.h:135-141): the drafter picks 'argmax over the WHOLE vocabulary of dl[j][id] / (T * Td[j]) + g(seed, position of that draft's verify row, id), g the request's own noise (model/sample.cu's hash: the row that verifies the draft adds the same g to the full model's logits)'; 'The drafter does not know the full model's kept (top-p) set, so the pick is not restricted to a set; Td < 1 sharpens the drafter's distribution instead' (offline: Td 0.8 within half a point of a pick restricted to the drafter's own nucleus); and the invariant: 'Every draft is still verified by the sampler: with a given noise the emitted tokens are the same with any drafts; only how many are accepted changes.' A greedy draft under sampling is a one-hot proposal — accepted iff the plain sampler's draw equals it. SCOPING (engine/model/model.h:28-36): DSGB10_GSAMPLE mode 1 arms 'sampled requests whose client sent NO seed (there is no earlier text for such a request to be equal to)' plus sampled requests without top-p/top-k ('their sampler is the Gumbel-max one already; only their drafts change'); 'A seeded top-p / top-k request keeps today's sampler and today's text'; mode 2 = every sampled request — 'a seeded top-p / top-k request then gives ANOTHER text for its seed than before, from the same distribution, and the same one again from then on' — disclosed, not the default. THE LAW: (i) the noise must be a pure function of (seed, position, token id) — any per-row or per-stream entropy breaks the coupling (the verify row cannot reproduce a drafter-side RNG draw); (ii) the same (seed, position) keys both the proposing row and the verifying row, so acceptance is exact by construction: a proposal is accepted iff the sampler's own draw with the same noise picks it; (iii) sharpening (Td < 1) replaces set-restriction for the drafter — the proposal may leave the kept set and simply be rejected; (iv) text-identity scoping: changing the sampler changes the text-for-seed, so unseeded requests arm first; a seeded request's text is a promise — mode 2 changes it and must be disclosed. MEASURED: the losslessness is structural (the emitted tokens are the sampler's own draw under the shared noise, whatever was drafted); the offline datum (dspark.h:138-139): Td 0.8 within half a point of the nucleus-restricted pick. REGIME: sampled speculative decoding; any draft/proposal scheme over a stochastic sampler where proposer and verifier run in different rows, steps or streams. ANTI-REGIME: unkeyed entropy (per-device RNG state — the noise cannot be reproduced at the verify row); greedy-only lanes (no noise, no coupling needed); clients whose seed semantics are contractual (the mode-2 disclosure boundary); drafters whose proposal must land in the kept set (use set-restriction, not sharpening — a different trade). ⛔ KINSHIP (adjudicated both directions): `gumbel-argmax-filtered-sampling-descent` (gpu_20) owns Gumbel-max SAMPLING (the u-clamp, filtered descent over candidates) — THIS owns the DRAFT/VERIFY COUPLING: the same keyed noise seen by two different rows (propose vs verify) so acceptance is exact; the sampler form itself is gpu_20's and is cited, not re-owned; they compose (a coupled draft scheme rides gpu_20's sampler). `pre-sampled-verify-extends-speculative-eligibility` (gpu_123) owns sampler ELIGIBILITY — THIS supplies the mechanism that makes sampled speculation lossless once eligible (compose). The determinism family (gpu_49/gpu_81/gpu_102, this batch's R4) pins float reproducibility of REDUCTIONS — THIS needs the hash deterministic per (seed, position, id): mix64 is integer (bit-exact everywhere), the float lane is one fixed formula both sides evaluate; a reduction-order pin is not what carries it. GOAT (fixture pins): (a) same noise → same emitted tokens with ANY drafts (acceptance count changes, tokens do not); (b) the keyed-noise property — the same (seed, position, id) yields the same g from both sides, different across positions; (c) the greedy one-hot arm — a greedy draft is accepted iff the plain draw equals it (a noise-flipped case rejects with the text unchanged); (d) the scoping arm — a seeded top-p request keeps the incumbent sampler path and its text. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_GSAMPLE/DSGB10_GCOUPLE, off by default). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/model/sample.cu:345-355 + engine/dspark/dspark.h:131-142 + engine/model/model.h:27-37. Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r#"// NAIVE: the drafter samples from its own RNG stream; the verify
// row samples from the engine's stream. Same request, different
// entropy — the verify row cannot reproduce the drafter's draw, so a
// "should-accept" draft is rejected for the wrong reason and an
// accepted draft STEERS the text (the emitted tokens depend on what
// was drafted).
struct SplitStreams {
    draft_rng: u64, // the drafter's own state
}

fn naive_draft_step(s: &mut SplitStreams, engine_draw: u32, proposal: u32) -> (u32, bool) {
    s.draft_rng = mix64(s.draft_rng); // a private stream: unreproducible
    let accepted = proposal == engine_draw;
    // the hazard: when accepted, the TEXT carried the drafter's entropy,
    // not the engine sampler's own draw for this position
    (engine_draw, accepted)
}
"#.to_string(),
            opt_pattern: r"// FIXED: the noise is a pure function of (seed, position, token
// id) — mix64(seed) ^ (pos * GOLDEN), g = uv*invT - ln(-ln(unit(mix64(
// base + id*C)))). The drafter's row and the row that verifies it see
// the SAME g, so with a given noise the emitted tokens are the same
// with any drafts; only how many are accepted changes.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
const C_MIX: u64 = 0xD1B5_4A32_D192_ED03;

fn noise(seed: u64, pos: u64, id: u32) -> f64 {
    let base = mix64(seed) ^ pos.wrapping_mul(GOLDEN);
    gumbel_unit(unit(mix64(base.wrapping_add((id as u64).wrapping_mul(C_MIX))))
        .clamp(1e-12, 1.0 - 1e-12)) // the u-clamp
}

fn gumbel_unit(u: f64) -> f64 {
    -(-u.ln()).ln()
}

fn sampler_pick(kept: &[u32], logits: &dyn Fn(u32) -> f64, t: f64, seed: u64, pos: u64) -> u32 {
    let inv_t = 1.0 / t;
    let mut best = (f64::NEG_INFINITY, u32::MAX);
    for &id in kept {
        // uv*invT + the SAME keyed noise both rows see
        let g = logits(id) * inv_t + noise(seed, pos, id);
        if g > best.0 || (g == best.0 && id < best.1) {
            best = (g, id);
        }
    }
    best.1 // the emitted token, whatever was drafted
}
".to_string(),
        },

        KernelRule {
            rule_id: "topology-keyed-collective-hop-count-bits-frozen".to_string(),
            gpu_concept: r"MECHANISM (topology-keyed transport; category E — parallelism): collectives built for a cabled ring relay the opposite rank's half through a neighbour — two hops, store-and-forward: 'relays are store-and-forward, so a post's time follows its larger relayed part over two hops, not the links' loads' (engine/comm/ring.h:84-88). On a switch — every rank reaches every other in one hop — each rank writes the opposite rank itself. The comment is the contract, verbatim (ring.h:89-93): 'Direct mode, for fleets where every rank reaches every other in one hop (a switch): each rank also connects to the opposite rank and writes its contribution there itself, so no proxy relays anything and the opposite rank's data takes one hop instead of two. The buffers, the six flags, the device side and the order of every sum are unchanged, so results are the same bits. Every rank must agree (checked at the bootstrap). -1: from the environment (DSGB10_RING_DIRECT=1), off if unset.' THE OPTIMIZATION IS CONFINED TO THE TRANSPORT: the wire contract (buffers, arrival flags, device-side kernels, SUM ORDER) is frozen, so correctness is by construction — the only new requirement is fleet-wide agreement, checked once at the bootstrap; a split fleet (some ranks relaying, some direct) would deadlock, so the check refuses. Mode 2 rides a second clause: a small contribution goes to each neighbour as one write and one write of both flags instead of two halves with a flag each (the halves existed so a relay could start early). MEASURED (docs/stack5.md): ring bench, median µs after a rank's own post, 80 all-reduces back-to-back — 10 K: 14.45 → 9.89 (relay → direct), 1 K: 11.59 → 7.66, 64 B: 10.18 → 6.86, 128 K: 39.90 → 30.66 (stack5.md:60-66); a decode step's 80 all-reduces of 5120 elements: bf16 14.48 → 9.93 µs each (1.16 → 0.79 ms) (stack5.md:66-67). THE MICRO→ENGINE DISCOUNT, stated verbatim (stack5.md:85-86): 'The ring bench alone suggested about 4.5 us x ~145 collectives = 0.65 ms a step. The engine keeps 0.2-0.6 ms of it: part of each wait was already hidden under the next kernel's weight loads.' The engine cell (same binary, one window, identical steps — 28,757 timed steps with the same row counts): GPU step −0.21 ms at one row .. −0.59 ms at six; 320 held prompts +1.26/+1.75/+1.81 % (chat/code/math); texts 320/320 equal to the relay build; identity set 54/54 (stack5.md:74-84). Correctness: '300,000 random-size collectives with every word checked, 0 mismatches on the four ranks' (stack5.md:70-72). ANTI, verbatim (stack5.md:15-16): 'It needs a fleet where every rank reaches every other in one hop (ours: four boxes on one 200G switch, the ring is only logical). On a cabled ring it must stay off.' THE LAW: (i) transport topology is a boot-time FLEET property — key the hop count off it; (ii) freeze the wire contract when changing the transport: same buffers, same flags, same device side, same SUM ORDER — the bits are then unchanged by construction and correctness needs no new proof, only the agreement check; (iii) every rank must agree on the mode (checked at the bootstrap) — refuse a split fleet; (iv) state the micro→engine discount: a collectives-level µbench saving over-claims what a step keeps (waits hide under weight loads) — quote both numbers. REGIME: switch/RoCE fleets with one-hop reachability; multi-rank TP collectives whose algorithm was designed for a cabled ring. ANTI-REGIME: cabled rings (no one-hop reachability — the mode must stay off); collectives whose correctness lives in the relay order (nothing here to freeze); quoting the µbench saving as a step saving without the discount. ⛔ KINSHIP (adjudicated both directions): `cross-rank-reduce-pins-rank-order-not-ring-order` (gpu_102) owns SUM ORDER — the arithmetic contract across ranks; THIS owns the hop-count-vs-topology TRANSPORT decision, with the bits frozen precisely by leaving that order unchanged; they compose (the direct transport still folds in gpu_102's pinned order — that IS the freeze). `plane-form-activation-reuse-topology` (gpu_60) owns a data-LAYOUT topology (plane-form reuse) — a different axis (memory shape, not transport hops). `fan-out-shared-input-single-sync` (gpu_02) + `idle-rank-doorbell-on-rendezvous-store` (entries_13) own the single-poller/doorbell sync shapes inside such rings — second instances at most (the research note's F2 face territory, landing on their owners in the rust_perf slice's face pass, not here). GOAT (fixture pins): (a) same buffers + same rank-order fold → identical bits across the topology switch (relay and direct produce bit-identical reduced values); (b) an arrival-order fold on direct posts yields different bits — the contract violation the freeze prevents (the tamper arm); (c) a fleet configured with mixed modes refuses at the bootstrap; (d) the micro→engine discount — the per-collective µs saving does not transfer 1:1 into a step saving (a modeled keep factor inside the source's 0.2-0.6-of-0.65 band). NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_RING_DIRECT, env-defaulted off). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/comm/ring.h:84-94 + docs/stack5.md:12-18, :60-72, :85-86 + CHANGES.md:346 (row 16, repo root). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: the transport and the arithmetic are one blob —
// retargeting the transport to a switch reorders how partials arrive
// and the reduction folds them in ARRIVAL order. Same data, different
// bits, and no check that the fleet runs one mode.
fn relayed_or_direct_reduce(posts_in_arrival_order: &[f64]) -> f64 {
    posts_in_arrival_order.iter().fold(0.0, |a, b| a + b) // arrival = transport order
}
".to_string(),
            opt_pattern: r"// FIXED: the hop count is a TRANSPORT decision keyed on the
// topology; the wire contract (buffers, flags, device side, SUM ORDER)
// is frozen — every rank folds the gathered partials in RANK order
// regardless of how the posts traveled, so results are the same bits;
// every rank must agree (checked at the bootstrap).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Relay,
    Direct,
}

fn direct_reduce(gathered_by_rank: &[f64; 4], modes: &[Mode; 4]) -> Result<f64, BootErr> {
    if modes.iter().any(|m| *m != modes[0]) {
        return Err(BootErr::ModeSplit); // every rank must agree, checked at the bootstrap
    }
    // ((c0 + c1) + c2) + c3 — the order of every sum, unchanged
    let mut acc = gathered_by_rank[0];
    for &c in &gathered_by_rank[1..] {
        acc += c;
    }
    Ok(acc)
}

enum BootErr {
    ModeSplit,
}
".to_string(),
        },

        KernelRule {
            rule_id: "smallest-fitting-slot-class-first-keep-large-for-long".to_string(),
            gpu_concept: r"MECHANISM (size-class slot admission; category D — dispatch): a heterogeneous slot pool (4 x 1M-context big slots + 12 x 128K small) admits sessions with a TWO-PASS scan. The line is the law, verbatim (engine/server/serve.cpp:1301-1302): `for (int pass = 0; pass < 2; ++pass)  // small slots first (big_slots), so that the big ones stay for long requests` — pass 0 considers only free slots strictly smaller than the biggest class that fit `need`; pass 1 opens the big class. The picker's own comment states the whole contract (serve.cpp:1296-1300): 'A slot for a session that holds none, among those that hold `need` positions (with big slots, the small ones first): a free one; else the least recently used one whose session no request uses (its state goes to the store); else, with time-slicing, the slot of the paused request whose turn comes last. Never the slot of a session in `keep`. -1: none.' The fits predicate is per class (`need <= m_.slot_ctx(s)`), with `big = m_.slot_ctx(0)` the biggest class. CAPACITY-CLASS MISMATCH IS WAIT, NOT DEATH: a flight that needs a big slot while every one is in this run parks in a deque — serve.cpp:1335 verbatim: `std::deque<Flight> later;  // big_slots: flights that need a big slot while every one is in this run` — and re-queues WITHOUT losing order (`if (!live && pick_slot(keep, need) < 0) { later.push_back(std::move(f)); continue; }`, serve.cpp:1338-1341); CHANGES.md row 3 names the lever 'big-slot wait instead of die'. MEASURED (CHANGES.md:257, row 3): code c16 172 → 234-247, c8 170 → 195-200, c1 unchanged, qeval 71. THE LAW: (i) admission into a heterogeneous capacity pool scans classes SMALLEST-FIRST so the scarce large class stays available for the work that needs it — a first-fit-any-class scan lets a short request take a big slot; (ii) capacity-class mismatch is an ORDERED WAIT: the flight parks in a deque preserving FIFO and re-enters when its class frees — never a refusal; (iii) the fits predicate is per class, evaluated against the request's clamped need; a session in the `keep` set never loses its slot to the picker. REGIME: heterogeneous slot/context pools (KV-cache tiers, big-context slots); admission under mixed request sizes. ANTI-REGIME: a uniform pool (one class — the two-pass scan degenerates to plain first-fit; the c1-unchanged datum is exactly this); sessions whose need grows past their slot mid-session (the resize/evict lane is a different rule); pools where the SMALL class is the scarce one (the scan order flips with the scarcity — scan scarcest-last regardless of size). ⛔ KINSHIP (adjudicated both directions): `pool-wait-bounded-bypass-later-fitting-goes-first` (gpu_102) owns queue BYPASS fairness (a later-arriving job that FITS goes before parked ones) — THIS owns SIZE-CLASS PLACEMENT (which class a request consumes) plus the ordered park; they compose (the `later` deque's head is exactly the bypass candidate once its class frees). `capacity-exceeded-silent-fallback-not-refusal` (gpu_330) owns the loud-degradation-vs-silent-fallback semantics of a capacity refusal — THIS removes the refusal entirely (an ordered wait instead of death); adjacent, compose. `batch-reserve-monotonic-counters-offload-bulk` (gpu_59, the sanstun lane) owns reservation counters for bulk offload — a different resource axis (device scratch, not serving slots). GOAT (fixture pins): (a) a short request takes a small slot even when a big slot is free (the big slot preserved); (b) a long request takes the big slot when no small fits; (c) big-class saturation parks flights in `later` IN ORDER and they re-enter in that order; (d) a uniform pool makes the two-pass pick identical to naive first-fit (the c1-unchanged shape). NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (--slots/--big-slots; CHANGES.md row 3). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/server/serve.cpp:1296-1345 + CHANGES.md:257 (row 3, repo root). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: first-fit over the pool in slot order — a short request
// takes the first slot that fits, which is a 1M-context slot; the big
// slots churn with short chats, and the long request that arrives next
// DIES for lack of a big slot.
#[derive(Clone, Copy)]
struct Slot {
    free: bool,
    ctx: usize,
}

fn admit_first_fit(slots: &[Slot], need: usize) -> Option<usize> {
    slots.iter().position(|s| s.free && s.ctx >= need)
}
".to_string(),
            opt_pattern: r"// FIXED: two passes — pass 0 considers only the SMALL classes
// that fit, pass 1 opens the big class — so the big slots stay for
// long requests; a flight that needs a big slot while every one is
// busy parks in the `later` deque IN ORDER instead of dying.
fn admit_small_first(slots: &[Slot], big_ctx: usize, need: usize) -> Option<usize> {
    for pass in 0..2 {
        if let Some(s) = slots.iter().position(|s| {
            s.free && s.ctx >= need && (pass == 1 || s.ctx < big_ctx)
        }) {
            return Some(s); // pass 0: small classes only; pass 1: the big class
        }
    }
    None // the caller parks the flight in `later`
}
".to_string(),
        },

        KernelRule {
            rule_id: "batch-aware-expert-reroute-never-single-session".to_string(),
            gpu_concept: r"MECHANISM (lossy batch-aware expert re-routing; category H — MoE dispatch): OEA-style re-routing shrinks the step's MoE expert union — the union is the per-step cost ('experts/layer 47-52 at 16 rows vs 112 at 64', wide64.md's measured table). The contract, verbatim (engine/kernels/common.cuh:315-321): 'night T3 (EXPERIMENT, lossy, off unless DSGB10_OEA is set): batch-aware expert re-routing (OEA-style). The row's top-k0 picks stay; each of its other 6 - k0 picks prefers an expert the batch's core already loads (cmask: every row's top-k0), taken in the row's own rank order among its top-kp by the biased key; slots not filled that way keep the row's natural picks in rank order. The weights: router_row's formula over the chosen scores. One warp.' LOSSY, OPT-IN, GATED: 'DSGB10_OEA=K0,KP,MINROWS (e.g. 4,12,9) turns batch-aware re-routing on (lossy; off by default)' with the parse bounds 'K0,KP,MINROWS with 1 <= K0 < 6, 6 <= KP <= 16, MINROWS >= 2' (a malformed value dies), the arming check `M >= a.oea_min`, and the default `v = {0, 0, 1 << 30}` = off (engine/kernels/moe.cu:1979-1990). THE DETERMINISM GUARD IS THE CONTENT, verbatim (moe.cu:85-86): 'exact: one session's rows are never re-routed (its text must not depend on its step's width)' — the code detects a multi-session step by the session ids of its first and last rows (`oea = a.oea_sess[a.oea_row0 + n - 1] != a.oea_sess[a.oea_row0]`, n clamped to [1, M]); a single-session step of ANY width keeps every row's natural picks, so a session's text cannot depend on what else (or nothing) shared its step. MEASURED (CHANGES.md:259, row 5): with 4,12,9 — c16 253 → 286, agent 180.7 → 203.4; qeval 71/72 (the lossy tail visible in the one qeval miss). THE COMPOSITION HAZARD (CHANGES.md v-note): copy drafts change WHEN a request's steps happen, so which requests share a step can change — the re-routing surface moves under other levers; both are opt-in and the single-session guard holds regardless. THE LAW: (i) batch-aware routing trades per-row pick fidelity for union size — legal only where the operator accepted lossiness (opt-in, parse-gated, off by default); (ii) the never-single-session guard: a step whose rows are ONE session's (detected width-independently by first/last session id) routes NATURALLY at any width — batching order must not leak into a session's output; (iii) preference stays inside the row's own ranked candidates — the reroute never promotes an expert outside the row's top-kp, fills in the row's own rank order, and leaves unfilled slots at natural picks. REGIME: multi-session batch steps over a MoE (the MINROWS gate ≥ 2 rows); workloads where the union cost dominates (per-step expert bytes). ANTI-REGIME: single-session steps (the guard — no reroute at any width); consumers needing exact top-k routing (quality bars, eval gates); steps below MINROWS; expert unions already small (nothing to shrink); eval/benchmark lanes where a lossy tail would corrupt the measurement (the qeval 71/72 datum is the disclosure). ⛔ KINSHIP (adjudicated both directions): `compute-experts-before-router-lookup-fixup` (gpu_152) owns the SCHEDULER-FIXUP pole — compute experts ahead of the router lookup, discard on disagreement, rebuild only missing rows: exact-by-construction, the pick itself never changes; THIS is the ROUTER-PICK re-route (the pick changes, lossy) and the never-single-session guard is unowned anywhere. `expert-member-rows-width-ladder-bit-identical` (gpu_81) owns width-rung bit-identity of folds — the guard's motive (text independent of width) is the routing-side sibling of that fold-side law. The copies-share-experts mechanism (rust_perf entries_84/94 faces) observes the same union cost in benchmarks; `speculative-expert-prefetch-prev-step-routing` (gpu_156) prefetches by the PREVIOUS step's routing — a different lever over the same per-step union (compose). GOAT (fixture pins): (a) a single-session step of 2+ rows routes naturally at ANY width (the guard); (b) multi-session rows prefer union members among their own top-kp in rank order; (c) a row whose top-kp ∩ union is empty keeps natural picks; (d) the MINROWS gate disables reroute below the row floor, and the parse refuses out-of-range K0/KP. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_OEA, off by default; CHANGES.md row 5, marked 'no' under exactness). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/kernels/common.cuh:315-322 (router_row_oea) + engine/kernels/moe.cu:82-92 (the guard), :1978-1990 (the parse) + CHANGES.md:259 (row 5). Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: batch-aware re-routing applies to EVERY step by width
// alone — a single session's 16-row step has its top-k picks re-routed
// toward the batch union, so its TEXT comes to depend on what other
// work (or none) shared the step.
fn route_always(rows: &[RowRanks], union: &ExpertSet) -> Vec<Vec<usize>> {
    rows.iter().map(|r| reroute_toward(&r.top_k, union)).collect()
}
".to_string(),
            opt_pattern: r"// FIXED: re-route only where lossiness is legal — a step whose
// rows are ONE session's (first and last session id equal, ANY width)
// keeps every row's natural picks, because its text must not depend on
// its step's width; multi-session steps at/above MINROWS re-route each
// row's slots beyond k0 toward the batch core union among that row's
// own top-kp, in the row's rank order.
fn route(rows: &[RowRanks], union: &ExpertSet, min_rows: usize) -> Vec<Vec<usize>> {
    let multi_session = rows.first().session != rows.last().session;
    if !multi_session || rows.len() < min_rows {
        return rows.iter().map(|r| r.top_k.clone()).collect(); // exact: natural picks
    }
    rows.iter().map(|r| reroute_toward(&r.top_k, union)).collect()
}
".to_string(),
        },

        KernelRule {
            rule_id: "tile-local-exponent-base-coding-raw-escape".to_string(),
            gpu_concept: r"MECHANISM (tile-local exponent base + raw escape; category N — dtype/alignment, a wire-format law): an EXACT fixed-length coding of bf16 weights in fewer bytes — per 8x32 tile keep sign+mantissa raw and code the exponents as 4-bit offsets from the tile's LEAST exponent; a tile whose exponents span more than 16 values stays RAW behind a header flag. Verbatim (engine/kernels/gemv16.cu:12-16): 'kBF16X: bf16 weights coded exactly in fewer bytes (bf16x_encode, the vocab head): each warp step's tile of 8 rows x 32 values keeps its signs and mantissas, and its exponents as 4-bit offsets from the tile's least; a tile whose exponents span more than 16 values stays raw. A lane widens its coded step back to the raw step's 16 bytes and runs kBF16's MMAs on them: the same bits as kBF16 on the raw weights.' The API half (engine/kernels/dsk.h:135-141): 'bf16x_encode returns the coded copy (cudaMalloc'd, *bytes; *raw tiles kept raw; synchronises); bf16x_check counts the 16-byte pieces of W that X does not decode to (0: exact). gemv_bf16x / gemv_bf16x_norm read X and give gemv_bf16's / gemv_bf16_norm's bits' — exactness is a CHECKED property, never an assumption, and the consumer widens back to the raw 16-byte pieces so the compute path (the MMAs) is unchanged: a STORAGE transform, not a compute dtype. THE CONTRAST IS THE CONTENT (docs/stack5.md:151-160): a fixed-length 7-bit code over FLAT-entropy dense FP8 measured 'about par (-6 % to +6 %, mean near 0)' against the bar of 'a coded kernel at least 3 % faster' — 'The fixed-length kernel reads its stored bytes at 216-237 GB/s; today's kernels read raw bytes at 248-258 GB/s. The 8-9 % fewer bytes are cancelled by the decode (about 90 more instructions and four loads a lane per step instead of one)'; 'Huffman is 10-48 % slower on our weights, as upstream found on theirs. A table-driven ANS decoder has the same serial per-lane structure (a table lookup and a variable refill per symbol), so it was not built.'; and the head's exponent carries the structure the tile base harvests: 'the bf16 head's exponent 2.59 bits, already stored as 4 bits from a tile base.' The tile base captures the mass or it does not: where the exponent distribution is FLAT (dense FP8 activations), a per-tile base buys ~nothing and the decode tax eats the byte saving; where it is TIGHT (weight tiles), 4 bits from the tile's least is exact and nearly free. MEASURED: coded head reads 216-237 GB/s vs raw 248-258 (the decode tax, visible); the 7-bit-over-FP8 cell −6 %..+6 % (mean ~0) vs the ≥ 3 % bar — NOT built into the engine; Huffman 10-48 % slower; bf16x (the tile base) SHIPPED for the vocabulary head ('the vocab head: 3/4 of them', dsk.h:135). THE LAW: (i) a fixed-length tile-local code is exact by construction and decodes in integer ops — its win is a FUNCTION OF THE DATA's exponent spread; measure the spread before adopting (a tile base on flat-entropy data pays the decode for ~zero byte savings); (ii) the raw-escape flag bounds the worst case: a wide-span tile stays raw — the code can never lose bytes badly or break exactness; (iii) exactness is verified by a check pass counting pieces that fail to decode back (0 = exact), never assumed; (iv) entropy coders lose on GPU decode — serial per-lane table lookup + variable refill structure; a bandwidth win bought with a decode tax is not a win at compute-bound shapes. REGIME: weight tiles with tight per-tile exponent ranges (bf16 heads, block-scaled weights); bandwidth-bound reads of large constant weight planes; storage at rest decoded just before the MMA. ANTI-REGIME: flat-entropy dense data (the measured null: −6 %..+6 %); latency-bound small reads (the decode overhead goes unpaid); consumers wanting the coded bytes as a COMPUTE format (this widens to raw then runs the same MMAs). ⛔ KINSHIP (adjudicated both directions): `zero-exponent-float-mantissa-is-the-format-pivot` (gpu_125) + `ue8m0-pow2-scale-as-biased-exponent-byte` (gpu_23) own the exponent-coding ADJACENTS (zero-exponent pivot formats; pow2-scale-as-biased-exponent-byte) — the TILE-LOCAL base (4-bit offsets from the tile's least) plus the RAW-ESCAPE flag are unowned; they compose (a block-scale format can carry tile-local offsets for its data half). `min-max-range-scale-asymmetric-block-codec` (gpu_165, the Prism fork) owns a LOSSY asymmetric min/max block codec — THIS is exact (bf16 bits, verified by a check pass) with an escape; different contract. The streaming-load family (`weights-skip-l1-allocation`) owns the read path the decode tax lands on — the bandwidth-vs-decode tradeoff is the shared surface (compose, cite). GOAT (fixture pins): (a) tile-local decode exactness — offsets from the tile's least exponent re-widen to the identical bits; (b) the raw-escape arm — a tile spanning more than 16 exponent values stays raw (flag set) and decodes by identity; (c) the flat-entropy contrast — a modeled flat-exponent plane shows zero byte saving while the decode tax makes it a net loss (the −6 %..+6 % shape), a tight-spread plane shows a real saving; (d) the check arm — a corrupted coded piece counts (bf16x_check > 0), a clean one counts 0. NotBenchable-by-domain (CUDA/RoCE/serving-runtime win metrics — the validator prices code deltas, not engine decisions; the fixtures pin the defect shapes in deterministic CPU arithmetic — the annotation class, never a demotion). UPSTREAM: present at pin — the source's own landed decision, decision content only, never a PR (DSGB10_HEADG; the vocab head ships coded). Source: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT; fork of blaesus/dsgx10) — ds41f-4xgx10-switchless/engine/kernels/gemv16.cu:12-16 + engine/kernels/dsk.h:135-141 + docs/stack5.md:148-160. Quotes verified at the pin. Batch 427 / Plan 420 / Research 454.".to_string(),
            ref_shape: r"// NAIVE: shrink the wire format with an entropy coder over the
// whole plane — 8-9 % fewer bytes, bought with ~90 more instructions
// and four loads per lane of serial decode; on flat-entropy dense FP8
// the tax cancels the saving (measured -6 % to +6 % against a 3 % bar),
// and Huffman loses 10-48 %.
fn huffman_or_fixed7_coding(values: &[u16]) -> Vec<u8> {
    let mut freq = [0usize; 1 << 16];
    for &v in values {
        freq[v as usize] += 1; // flat entropy: no mass for a table to harvest
    }
    let _table = build_huffman(&freq);
    encode_with_serial_refill(values, &_table) // variable-length, per-lane refill
}

fn build_huffman(_freq: &[usize]) -> [[u8; 2]; 1 << 16] {
    [[0, 0]; 1 << 16]
}

fn encode_with_serial_refill(values: &[u16], table: &[[u8; 2]; 1 << 16]) -> Vec<u8> {
    values.iter().map(|&v| table[v as usize][0]).collect()
}
".to_string(),
            opt_pattern: r"// FIXED: per tile keep sign+mantissa raw and code the exponents
// as 4-bit offsets from the tile's LEAST exponent; a tile whose
// exponents span more than 16 values stays RAW behind a flag — fixed
// length, ~6 integer ops to widen, then the same MMAs; exactness is a
// checked property, never an assumption.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TileCode {
    Offsets4 { base: i16, offs: [u8; 8] }, // 4-bit offsets from the tile's least
    Raw([u16; 8]),                         // the escape: span > 16 stays raw
}

fn code_tile(exps: [i16; 8], rest: [u16; 8]) -> TileCode {
    let base = *exps.iter().min().unwrap();
    let span = *exps.iter().max().unwrap() - base;
    if span > 15 {
        return TileCode::Raw(rest); // the raw escape bounds the worst case
    }
    TileCode::Offsets4 {
        base,
        offs: exps.map(|e| (e - base) as u8), // fits 4 bits by the span gate
    }
}

fn widen(code: &TileCode) -> [u16; 8] {
    match *code {
        TileCode::Raw(v) => v, // identity: the flag keeps exactness trivial
        TileCode::Offsets4 { base, offs } => {
            // ~6 integer ops per value, then the same MMAs on the raw pieces
            offs.map(|o| (((o as i16) + base) as u16) << 8 | 0x00ff)
        }
    }
}
".to_string(),
        },
    ]
}
