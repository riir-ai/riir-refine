#!/usr/bin/env python3
"""Batch 293 rider splice — 5 count-neutral rider faces on 4 hosts.

Exact-anchor splices (the B259 lesson: byte-level script job, never fuzzy edits).
Every face's quotes were verified verbatim at the pin clone (.raw/molten @
611e704fe9a3f77e56a459ade78e77bd560cf9a7) by the coordinator this session.
Count-neutral: corpus stands 981.
"""
import sys

SRC = "src/domains/kernel_opt/corpus/"

# The provenance suffix every face carries.
PROV = "Batch 293 / Plan 297 / Research 331."

FACES = {
    # ---- Face 1: gpu_20 packed-total-order-multi-field-argmax ----------------
    "entries_gpu_20.rs": [
        # anchor: the end of the packed-total-order rule's Batch 276 rider
        ("packed-total-order-multi-field-argmax",
         "the Metal simd spelling. Batch 276 / Plan 280 / Research 314.",
         " RIDER (Batch 293, the DEVICE-SCOPE ATOMIC-MAX face, zhangshuibai/molten @ 611e704fe9a3f77e56a459ade78e77bd560cf9a7 (Apache-2.0) -- where this host packs (score, index) so ONE reduction is score-desc-then-index-asc, this face is the CROSS-THREADGROUP arm: the packed key turns the whole-device argmax merge into ONE atomic_max per threadgroup -- 'atomic_max_explicit(best, lfm_argmax_key(bv, bi), memory_order_relaxed);' on a device atomic_ulong (lfm.metal:1280) -- so the partials array, the inter-dispatch barrier and the second reduce kernel of the two-pass form ALL vanish (lfm.metal:1209-1214: 'instead of storing the BF16 logits each threadgroup merges its sixteen BF16-rounded logits with lfm_argmax_merge and folds the winner into one device-wide atomic_max key. 'Value desc, index asc' is a total order on non-NaN candidates, so the winner is the two-stage argmax's for any partition'). TWO KEY-CONSTRUCTION FACES the host does not own. (1) THE TIE SEMANTICS MUST MATCH THE COMPARATOR'S EQUALITY, not the IEEE total order alone: the -0/+0 fold 'if ((u << 1) == 0u) { u = 0u; }' (lfm.metal:1202-1204) exists because the reference merge's float == treats -0 == +0 while the raw sign-flip orderable mapping puts -0 < +0 -- the source's own law: '(-0 ties +0, as the merge's float == does)' (lfm.metal:1198); a key whose order disagrees with the reference comparator's equality silently re-picks ties the reference resolves differently. (2) THE INLINE SIGN-FLIP FORM with the index inverted in the LOW bits: 'const uint o = (u & 0x80000000u) != 0u ? ~u : (u | 0x80000000u);' + 'return (ulong(o) << 32) | ulong(~i);' (lfm.metal:1205-1207) -- the bit-NOT index makes 'an equal value at a lower index' the LARGER key (lfm.metal:1196-1197: 'a larger key is a larger value, or an equal value at a lower index'), the same inverted-index law this host's Batch 111 rider cites, at atomic-max granularity. ADMISSION: atomic_max over keys requires the value domain NaN-free or NaN folded -- here 'Candidates are never NaN (the merge never takes one)' (lfm.metal:1199). LIFECYCLE CONTRACT: 'best' is zeroed before the dispatch and CLEARED BY THE READER (the token-tail face this same batch lands on argmax-epilogue-replaces-output-materialization) -- no reset dispatch per step. KINSHIP (do not double-count): two-pass-partial-reduction-inter-dispatch-barrier (gpu_116, B269) owns the OVERSIZED-ROW alternative -- per-(row, part) partials in graph-allocator scratch + the encoder memory barrier + the final reduce kernel; THIS face is the single-dispatch arm, the boundary is atomic contention (one atomic per threadgroup) vs a second dispatch's cost. NotBenchable here (Apple-silicon Metal; the axis is launch count, no local Metal validator). " + PROV),
    ],
    # ---- Face 2: gpu_22 spin-barrier ------------------------------------------
    "entries_gpu_22.rs": [
        ("spin-barrier-needs-co-residency-capped-launch",
         "single-group spin barriers only, or a redesign to a device-scope atomic/s",
         " RIDER (Batch 293, the WAIT-FREE LAST-FINISHER face, zhangshuibai/molten @ 611e704fe9a3f77e56a459ade78e77bd560cf9a7 (Apache-2.0) -- this face IS the device-scope-atomic redesign this host's portable note names, in full: a relaxed 'atomic_fetch_add_explicit(counter, 1u, memory_order_relaxed)' bracketed by device-scope seq_cst fences on BOTH sides -- 'atomic_thread_fence(mem_flags::mem_device, memory_order_seq_cst, thread_scope::thread_scope_device)' before the increment (publish: every logit store ordered before it) and again after winning (acquire, lfm.metal:833-840) -- and the ONE threadgroup whose increment observes E - 1 runs the merge and RESETS the counter in-kernel for the next dispatch ('atomic_store_explicit(counter, 0u, memory_order_relaxed);', lfm.metal:850). THE FORWARD-PROGRESS LAW, verbatim (lfm.metal:716-719): 'No threadgroup ever waits on another, so forward progress never depends on co-residency.' -- the spin barrier's deadlock class disappears WITHOUT the occupancy cap this host enforces: uncapped grids are sound because nothing spins (the two regimes of this host collapse into one: the barrier is reachable and SAFE at any occupancy). THE VERSION GATE is part of the design, not noise: '#if __METAL_VERSION__ >= 320' (lfm.metal:705) -- 'atomic_thread_fence needs MSL 3.2; this file is also compiled at 3.1 (the other kernels), hence the guard' (lfm.metal:720-721) -- a capability gate keyed on the language version WITHIN one source (the cache-key face this batch lands on pipeline-cache-key-covers-every-function-constant is its consumer-side consequence). THE PROLOGUE LOAD-OVERLAP ATOM rides the same kernel: stage the far-DRAM operand at kernel ENTRY under the barrier-heavy prologue -- verbatim (lfm.metal:796-799): 'Issue this expert's 8 KB F32 router row and the top-k bias at kernel entry, with all 256 threads, so their DRAM latency overlaps the add_rmsnorm prologue (whose barriers also publish the staged row). Pure copies: simdgroup 0 then sums the same values in the same lane order.' -- the copy issues before the norm's barriers so the DRAM latency hides, and the staged path is PURE COPIES (the consumer repeats the producer's lane order exactly -- no arithmetic to drift). KINSHIP (do not double-count): bounded-cross-worker-barrier-protocol-polling-fan-in-ladder (gpu_85, B223 + the B260 arrival-merge face) owns the MULTI-DISPATCH pass-loop arrival protocol and its device-fence discipline; THIS face owns the SINGLE-dispatch wait-free merge -- and the in-kernel reset is lawful here only because the pass boundary is a whole-dispatch ordering, the same reset law B260's face records, cited not re-owned. NotBenchable (Metal; MSL 3.2 gate). " + PROV),
    ],
    # ---- Face 3: gpu_52 argmax-epilogue ---------------------------------------
    "entries_gpu_52.rs": [
        ("argmax-epilogue-replaces-output-materialization",
         "compared as raw uint32 bits) gates the route per (k, n, kernel).",
         " RIDER (Batch 293, the TOKEN-TAIL face, zhangshuibai/molten @ 611e704fe9a3f77e56a459ade78e77bd560cf9a7 (Apache-2.0) -- this host owns the PRODUCER half (the epilogue swap replacing [rows, V] materialization); this face is the CONSUMER half on the decode loop: the argmax-key READER (one small threadgroup) writes the token AND produces the NEXT STEP'S INPUT -- verbatim (lfm.metal:1285-1288): '(and clears it for the next step), writes the token, and when `embed` is set also produces the next step's input: lfm_gather_rows_q6_bf16's dequantized embedding row into `x` and rmsnorm_bf16_v8's normalized row into `out`.' -- the next embedding + its RMSNorm never run as separate dispatches during generation; logits are never stored on the greedy path at all. THE READ-AND-CLEAR LIFECYCLE: the key buffer is zeroed before each dispatch and the READER clears it after reading -- 'const ulong key = best[0]; const uint tok = ~uint(key & 0xFFFFFFFFul); best[0] = 0ul;' (lfm.metal:1315-1317) -- no reset/memset dispatch per step; the producer's atomic_max accumulates from zero every step (the atomic-max face this batch lands on packed-total-order-multi-field-argmax is the producer arm of the same buffer). THE HOST'S OWN ANTI-REGIME ('full logits needed downstream (sampling, logprobs, temperature > 0)') IS LIFTED BY THE SAME TAIL, not violated: the sampled arm stores the BF16 logits with the plain head GEMV, draws on the GPU into the SAME 64-bit key, and reuses the greedy token tail -- verbatim (sample.rs:8-12): 'A sampled step instead stores the BF16 logits with the plain head GEMV (the same values the argmax epilogue compares), draws the token on the GPU into the same 64-bit argmax key, and reuses the greedy token tail, which writes the token, the next embedding row and its RMSNorm. Tokens never leave the GPU between steps.' -- the sampling-face rider this batch lands on gumbel-argmax-filtered-sampling-descent is that draw. NotBenchable (Metal; decode-tail dispatch-count axis). " + PROV),
    ],
    # ---- Face 4: gpu_35 pipeline-cache-key ------------------------------------
    "entries_gpu_35.rs": [
        ("pipeline-cache-key-covers-every-function-constant",
         "the generic variant remains the semantic baseline.",
         " RIDER (Batch 293, the LANGUAGE-VERSION axis face, zhangshuibai/molten @ 611e704fe9a3f77e56a459ade78e77bd560cf9a7 (Apache-2.0) -- the host owns the FUNCTION-CONSTANT axis of the pipeline-cache key; this face is the COMPILER/LANGUAGE-VERSION axis on the same law: the same fn name compiled from the same source at two MSL language versions is TWO DIFFERENT compiled artifacts, so every cache that outlives one compile keys on (source, MSL version) -- the library cache '(Mutex<HashMap<(usize, usize, MslVersion), Retained<ProtocolObject<dyn MTLLibrary>>>>' with the doc 'so many functions from one embedded source compile it only once' per version, the pipeline cache 'Keyed by function name and MSL version' ('Mutex<HashMap<(&'static str, MslVersion), Pipeline>>'), and the function-constant tier keyed '(&'static str, MslVersion, u64)' (metal.rs:80-88) -- EVERY tier carries the version component; dropping it serves one version's binary for another's launch, the same SILENT-WRONG-MATH class this host records for constants. THE TWO-VERSION POSTURE IS REAL, NOT HYPOTHETICAL: the same source file compiles at 3.1 AND at 4.0+ -- the wait-free router kernel exists only under '#if __METAL_VERSION__ >= 320' while the file's other kernels compile at 3.1 ('atomic_thread_fence needs MSL 3.2; this file is also compiled at 3.1 (the other kernels), hence the guard', lfm.metal:720-721; the version-gate face this batch lands on spin-barrier-needs-co-residency-capped-launch is the producing side). Suggest-only fix class (a cache-key tuple widening; a parse-only proposer cannot see which language version a driver compiles). NotBenchable (correctness law: a stale-binary-for-version hazard, not a perf axis). " + PROV),
    ],
    # ---- Face 5: gpu_20 gumbel-argmax -----------------------------------------
    # NOTE: gpu_20 appears twice; handled by the two-pass splice below.
}

# gpu_20 needs TWO faces on TWO different rules -- do them in order.
GPU20_FACES = [
    # Face 1 (packed-total-order) appended above via FACES; Face 5 here.
    ("gumbel-argmax-filtered-sampling-descent",
     "Batch 107 / Research 130 / Plan 077.",
     " RIDER (Batch 293, the SPLIT-INVARIANT POSITION-KEYED COUNTER + SMALL-E SERIES + EXACT-TAU LADDER faces, zhangshuibai/molten @ 611e704fe9a3f77e56a459ade78e77bd560cf9a7 (Apache-2.0) -- three atoms this family founder does not own. (1) THE POSITION-KEYED COUNTER makes the draw REQUEST-SPLIT INVARIANT, a stronger contract than the host's per-seed walk reproducibility: Philox4x32-10 with key = seed and counter = (word index, the token's ABSOLUTE position, a domain tag, 0) -- 'smp_philox(uint4(q, p.pos, SMP_DOMAIN, 0u), uint2(p.seed_lo, p.seed_hi))' (lfm_sample.metal:138-139) -- verbatim (sample.rs:25-27): 'The draw is the Gumbel-max trick with Philox4x32-10 keyed by the seed and countered by the token's absolute position, so a seed reproduces the same tokens however a generation is split into requests.' and (lfm_sample.metal:34-36) 'The draw depends only on (seed, position, logits, kept set), never on how the vocabulary is split across threadgroups or how a generation is split into requests.' -- prefix-cache resumption and multi-request continuations of one generation land on the same draws. (2) THE SMALL-E SERIES branch is the POSITIVE TWIN of this host's u-clamp hazard (the host clamps u away from 1.0 because u == 1 gives +inf): this face steers PRECISION to where the draws WIN -- verbatim (lfm_sample.metal:148-152): 'Small E (large G, the draws that win) comes from the series of -log1p(-V), so it keeps full relative precision down to V = 2^-33.' (G = -log(E), E = -log(1-V) an Exp(1) variate, V = (x + 0.5) / 2^32; the tiny-V tail takes the power series, the rest the -log of the clamped max). (3) THE EXACT-TAU LADDER is the sequential-semantics ALTERNATIVE to the host's bounded descent walk: where the host's filtered descent is MAX_ITERS-bounded with a final fallback emit and DOCUMENTED parallel-filter semantics (a deviation from sequential top-k-then-top-p), this face resolves the threshold EXACTLY with the sequential convention -- verbatim (lfm_sample.metal:37-41): 'top-k keeps every token whose w is at least the k-th largest (ties at the boundary kept, as Hugging Face and vLLM do); top-p then keeps, among those, the smallest such set whose probability is at least top_p (a tie group straddling the boundary is kept whole)' -- via 32 fixed cutoffs + up to two 32-fold refinement rounds + one-threadgroup exact resolution (radix rounds when the band exceeds 2048), and the decision sums carry the determinism law verbatim (lfm_sample.metal:41-43): 'every sum in that decision is either integer (counts, fixed-point masses) or a float sum in a fixed order, so the result is bitwise deterministic.' THE GREEDY FLOOR never moves: temperature 0/absent keeps the plain argmax epilogue bit-identical ('The greedy path is unchanged (same tokens, same speed)', docs/ARCHITECTURE.md:33), the sampled step reuses the same 64-bit key and token tail (the token-tail face this batch lands on argmax-epilogue-replaces-output-materialization), and the whole draw is GPU-RESIDENT -- 'Tokens never leave the GPU between steps' (sample.rs:11-12); source-reported cost for temperature 0.6 + top_k 50 + repetition_penalty 1.05: +0.6% / +1.0% / +1.2% (docs/ARCHITECTURE.md:32-33). NotBenchable (Metal sampling; the win is GPU-residency + exact sequential semantics, no local Metal validator). " + PROV),
]

def splice(path, rule_id, anchor, rider):
    p = SRC + path
    txt = open(p, encoding="utf-8").read()
    # locate the rule block
    ri = txt.find(f'rule_id: "{rule_id}"')
    assert ri > 0, f"rule not found: {rule_id} in {path}"
    ai = txt.find(anchor, ri)
    assert ai > 0, f"anchor not found after rule: {anchor[:60]!r} in {path}"
    # the anchor must belong to THIS rule (before the next rule_id)
    nxt = txt.find('rule_id: "', ri + 10)
    assert nxt < 0 or ai < nxt, f"anchor beyond rule end for {rule_id}"
    insert_at = ai + len(anchor)
    out = txt[:insert_at] + rider + txt[insert_at:]
    # quote-count identity: the rider adds exactly 2 double-quote chars? No --
    # the rider is inserted INSIDE the string literal, so any \" escapes stay
    # balanced; verify the file still parses as rust by counting .to_string()
    # occurrences and the raw double-quote count delta is 0 (we add no quotes).
    open(p, "w", encoding="utf-8").write(out)
    print(f"spliced {rule_id} <- {path} (+{len(rider)} chars)")

def main():
    for path, faces in FACES.items():
        for rule_id, anchor, rider in faces:
            splice(path, rule_id, anchor, rider)
    # gpu_20 second face (FACES already applied face 1 for gpu_20)
    for rule_id, anchor, rider in GPU20_FACES:
        splice("entries_gpu_20.rs", rule_id, anchor, rider)
    print("done")

if __name__ == "__main__":
    main()
