//! Entries slice — Batch 223 (Plan 222) — the lithos-metal pack, slice C:
//! R10 derived pack immutable original + content-addressed atomic
//! publish (`MemoryAccess`), R11 autotune fidelity (`AutotuneDispatch`),
//! R12 `MoE` readiness queue without a grid barrier (`MoeDispatchBackward`).
//! Source lithos-ai/lithos-metal @ 236475fa (Apache-2.0); every quote
//! re-verified at the pin clone this session.
//!
//! DEDUP (re-run at authoring, 3+ vocabularies): zero prior owners —
//! R10 (`derived` / `repack` / `content.addressed` / `immutable pack`):
//! the six `repack` rules own runtime memoized repack (B127), register
//! handoff (B138), staging-time in-place activation transforms plus
//! jit-band L1-hot repack (B186), and a lossless recode trial (B201);
//! `content-addressed` has ZERO corpus hits; no rule owns the
//! immutable-original + atomic-rename + span-shrink law. R11
//! (`autotune` / `noise margin` / `cache key` / `synthetic bench`):
//! B02/B05 own autotune-cache LIFETIME, B104 the offline swept table,
//! B129 compiled-pipeline cache-key completeness, B193 the
//! DRAM-streaming adoption trial, B207 the profile-window drain — none
//! owns the tuner fidelity clauses. R12 (`readiness` / `ready queue` /
//! `grid barrier` / `retire`): B105 owns the routing no-host-sync
//! contract (adjudicated COMPOSED NEIGHBOR, see its entry), B70
//! host-side barrier elimination, B119 sentinel polls, B194 overwrite
//! gating, B186/B210 the `MoE` admission/workspace/cache axes — the queue
//! discipline has no owner.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        KernelRule {
            rule_id: "derived-pack-immutable-original-content-addressed-atomic".to_string(),
            gpu_concept: "MECHANISM (kernel-friendly weight repack: immutable original + content-addressed derived file + atomic publish + the span-shrink memory half): a weight pack consumed under several access patterns (row tile, reduction iteration, operand slot, lane order) is NEVER repacked in place — the ORIGINAL stays immutable and a DERIVED, page-aligned file holds the SAME codes reordered for the kernel's access pattern. THE SOURCE LAW, verbatim (fp8_tiles.py:3-5): 'The original pack remains immutable. Derived, page-aligned files contain the same codes in (row tile, reduction iteration, operand slot, lane, consecutive codes) order. Conversion happens at compilation, outside the timed GPU program.' — and its sibling (nvfp4_tiles.py:3-5): 'Keep the checkpoint and original BLM pack immutable. Codes and E4M3 scales occupy separate contiguous planes; no dequantization or tensor-scale folding occurs on the CPU. Derived files are content-addressed and atomically published.' FOUR decision clauses. (1) COMPILE-TIME ONLY: the conversion runs in the compiler's weight-preparation pass, never inside the timed GPU program — a runtime repack pays the transform in the serving path and the rule's premise dies; the derived file is a REORDER of the same codes (lossless by construction — 'no dequantization or tensor-scale folding occurs on the CPU'), which is exactly the fence separating this from every lossy-requantization rule: same bits, new order, nothing to trial — the lossless-11bit-repack-trial class needs an adoption margin BECAUSE it recodes; a pure permutation needs none, bit equality is structural. (2) CONTENT-ADDRESSED FILES: the derived file's name is a digest over its construction inputs — the tile lanes digest a versioned domain tag + the packed geometry + the raw codes themselves (fp8_tiles.py:63-73: sha256 over 'monolith-fp8-operand-v2' + struct-packed params + the codes; moe_tiles.py:34-36: 'monolith-expert-operands-v1' + pack info + raw), so two programs over the same weights map ONE file instead of a second layout (nvfp4_tiles.py:21-22: 'Another program over the same weights can map the same content-addressed file instead of a second layout'); the span-compaction lane keys on SOURCE-FILE IDENTITY instead (weight_windows.py:67-69: resolved path + size + mtime_ns + the selected spans) — the honest hybrid: content digest where the bytes are transformed, identity digest where the bytes are copied verbatim. (3) ATOMIC PUBLISH: the derived file is written to a temp sibling and moved into place with an atomic rename (os.replace at fp8_tiles.py:112, moe_tiles.py:50, weight_windows.py:98), guarded by an existence + exact-size check (fp8_tiles.py:78) — a reader sees the old or the new file, never a half-written one; the in-process memoized repack that publishes a map entry after a stream sync (one-time-dense-repack-keyed-sync-publish) is the runtime twin of this clause, not a substitute: the file-level form survives process restarts and shares across programs. (4) THE CLEANUP HALF IS WHERE THE MEMORY WIN LIVES — after projections acquire derived layouts, the original windows shrink to their USED spans: only windows with substantial unused ranges rebuild, 'retaining entry alignment and one binding per original window' and 'No tensor bytes change' (weight_windows.py:4-5), with TWO DELIBERATE SKIPS (weight_windows.py:65: skip when the pack is under 1 MiB — spec.nbytes < (1 << 20); skip when the active fraction is >= 80% — active_bytes >= spec.nbytes * .8) and the clause that makes the shrink mandatory when it applies (weight_windows.py:3-4): 'A handful of norms must not keep gigabytes of replaced matrix payload resident in every ICB.' The gigabytes are the point: the derived layouts let the original pack's dead spans leave residency; a repack that keeps the full original alive bought nothing. One binding per original window is retained because a merged task may address several entries through one binding and its generated source already encodes the internal offsets (weight_windows.py:22-23) — renumbering bindings would mean regenerating kernels the shrink was supposed to leave alone. The immutability assumption is ASSERTED at every read, not hoped for ('expert operand extends beyond immutable weights', moe_tiles.py:27-28). REGIME: engines with compile-time weight preprocessing serving multiple access-pattern consumers of one pack; decode/prefill pairs wanting different orderings of the same checkpoint. ANTI-REGIME: runtime repacking (the conversion must stay out of the timed path); lossy transforms (the same-codes clause is the fence — a requantization needs the trial-and-margin discipline, not this rule); mutable weights (every clause assumes a static pack — derived files and identity digests both go stale the moment weights train). Distinct from one-time-dense-repack-keyed-sync-publish (B127 — the RUNTIME memoized repack: in-process cache, tuple key, publish-after-stream-sync; THIS is the compile-time FILE form with atomic rename + immutable original + the span-shrink half — compose: the file form feeds the process, the memoized form feeds the launch), from derived-index-from-pinned-checkpoint-is-a-cache (B115 — the LEGALITY boundary that derived-from-immutable-source is cache-class; THIS is the file-system instantiation: digest-named derived files from a pack nothing may mutate), from lossless-11bit-repack-trial (B201 — lossless RECODE with a timed adoption trial; THIS is a lossless REORDER needing no trial, and it owns the residency half neither does), from stage-operand-mac-layout-inplace-transpose-at-repack + jit-band-repack-stage-per-use-l1-hot (B186 — runtime IN-PLACE transforms of STAGED ACTIVATIONS for one launch; THIS never mutates anything in place and targets persistent weight packs), and from immutable-operand-upload-at-set-creation (B04 — upload-once placement of one immutable operand at buffer-set construction; THIS is pack-file publication + span reclamation across consumers). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- monolith/compiler/fp8_tiles.py:3-5,63-78,106-114; nvfp4_tiles.py:3-5,21-22,71-78; moe_tiles.py:27-36,47-51; weight_windows.py:3-5,22-23,65,67-98. NotBenchable here (a Python compiler-side file pipeline — no runtime kernel in this corpus to bench; the memory win is structural residency, gigabytes of dead spans reclaimed, and losslessness is by construction, never a wall-clock bar). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: repack the ONLY copy in place, inside the serving path — the
// original layout is gone (every other consumer breaks), a crash
// mid-write leaves an unbootable pack, the transform cost lands in the
// timed program, and the full-size original stays resident forever.
fn repack_in_place(pack: &mut Pack, layout: Layout) {
    let started_in_timed_path = true; // the serving path IS the caller
    permute_for_kernel(&mut pack.codes, layout);
    debug_assert!(!started_in_timed_path); // never holds: the transform
    // pays inside every decode step, and the old layout is unrecoverable
}

fn permute_for_kernel(_codes: &mut [u8], _layout: Layout) {}

struct Pack {
    codes: Vec<u8>, // the only copy: mutate it and every consumer breaks
}

struct Layout;
"
            .to_string(),
            opt_pattern: "// optimized: the original pack is IMMUTABLE; a derived, content-addressed
// file holds the SAME codes reordered for the kernel (lossless by
// construction); conversion is compile-time, outside the timed program;
// publication is write-temp + atomic rename; and once consumers adopt
// the derived layout the original shrinks to its used spans — the two
// deliberate skips first.
struct DerivedPack {
    path: PackPath,  // named by a digest over (domain, params, codes)
    aligned: usize,  // page-aligned length
}

fn build_derived(params: Geom, codes: &[u8]) -> DerivedPack {
    // content address: same inputs -> same file name, so a second
    // program over the same weights maps ONE file, not a second layout
    let digest = content_digest(DERIVED_DOMAIN, &params, codes);
    let path = derived_path(digest);
    let aligned = page_align(codes.len());
    if !path_exists(&path) || file_len(&path) != aligned {
        let tmp = temp_sibling(&path); // NEVER write the target in place
        write_all(&tmp, codes);
        atomic_rename(&tmp, &path); // readers see old or new, never torn
    }
    DerivedPack { path, aligned }
}

// the cleanup half, where the memory win lives: shrink the original to
// its used spans AFTER the derived layout exists.
fn compact_original(spec: &PackSpec, active_bytes: usize) -> Option<PackSpec> {
    if spec.nbytes < 1 << 20 {
        return None; // skip 1: small pack — the copy outweighs the residency
    }
    if active_bytes * 5 >= spec.nbytes * 4 {
        return None; // skip 2: active fraction >= 80% — nothing substantial
    }
    // entry alignment + one binding per original window retained; the
    // derived file is a REORDER, so no tensor bytes change anywhere
    Some(shrink_to_spans(spec))
}

const DERIVED_DOMAIN: u64 = 1;

fn content_digest(_domain: u64, _p: &Geom, _c: &[u8]) -> u64 {
    0xD1CE
}

fn page_align(n: usize) -> usize {
    n.div_ceil(16384) * 16384
}

fn derived_path(_d: u64) -> PackPath {
    PackPath
}

fn path_exists(_p: &PackPath) -> bool {
    false
}

fn file_len(_p: &PackPath) -> usize {
    0
}

fn temp_sibling(_p: &PackPath) -> PackPath {
    PackPath
}

fn write_all(_p: &PackPath, _b: &[u8]) {}

fn atomic_rename(_from: &PackPath, _to: &PackPath) {}

fn shrink_to_spans(_s: &PackSpec) -> PackSpec {
    PackSpec { nbytes: 0 }
}

struct Geom;

struct PackPath;

struct PackSpec {
    nbytes: usize,
}
"
            .to_string(),
        },
        KernelRule {
            rule_id: "autotune-fidelity-synthetic-carries-producer-params".to_string(),
            gpu_concept: "MECHANISM (autotuner measurement fidelity): a synthetic benchmark elects kernel variants only if it reproduces the program's REAL data conditions — four clauses, each a named trap. (1) PRODUCER PARAMS: when a variant's cost depends on a producer's parameterization (the fused-norm GEMV folds stat_parts per-token partial sums in every SIMD-group), the synthetic must allocate the program's REAL count — verbatim (autotune.py:94-95): 'stat_parts: the partial sums per token the norm statistic arrives in (the producer's blocks × its row split) — the fused variant folds them in every SIMD-group, so the benchmark must see the program's count.' A bench that under-allocates the partials measures a DIFFERENT kernel and the tuner caches the wrong winner under a key that looks right. (2) STREAM, NOT CACHE: every timing streams replicated weight copies — the source computes copies = max(1, int((256 << 20) // max(len(data), 1))) with the comment '≥ 256 MB streamed per timing' (autotune.py:106) — so DRAM bandwidth is what is measured, not L2 hits on one resident copy; the same law our adoption-trial lane carries as rotate-operands-per-launch (a burst over cached sets ranks variants the other way than the DRAM-streamed deployment regime). (3) THE BAR IS THE DEFAULT PLUS NOISE: NOISE_MARGIN = 0.03 with the comment 'a variant replaces the default only if faster by more than this fraction' (autotune.py:30); the timing is min-of-reps after a warm-up (reps: int = 5, warmup_ms: float = 30.0, autotune.py:58; the warm loop + min at :71-82), and the replacement check is CONSERVATIVE-VS-BEST: unless the best variant beats the default by MORE than the margin (best_ms > default_ms * (1 - NOISE_MARGIN), autotune.py:161), the DEFAULT is restored (:162-164). The module docstring states the pair as policy (autotune.py:8-9): 'Timings are min-of-N after a warm-up; a variant must beat the default by more than the noise margin to replace it.' Conservative-vs-best is DELIBERATE: an autotuner that flips on noise poisons every downstream cell consuming its choices; the margin buys stability, the warm-up buys a steady clock state, the min-of-5 buys tail-outlier immunity. (4) THE CACHE KEY IS THE FULL MECHANISM SIGNATURE per (chip, pack layout): the base key names EVERY axis that could change the winner — 'gemv|{format}|{n}x{k}|R{rows}|{lane_order}|{scale_placement}|T{t}|{epilogue}|{norm}' (autotune.py:34; epilogue defaults to plain, norm is norm-or-raw) — and three CONDITIONAL tails append when an axis leaves its default: '|shared-scales{d}' when scale_lane_divisor != 1 (:35-36), '|scale-order-{o}' when scale_order is not lane (:37-38), and '|P{stat_parts}' when norm_fed AND stat_parts differs from the default 64 (:39-40), with the in-code comment (autotune.py:40): 'the producer's partial count: the fused norm folds it per SIMD-group'. The elision IS the law: default-valued axes stay out of the key (stable keys, cache hits across programs), but the moment an axis could matter it is NAMED — a stale entry can never answer for a different kernel. The choices cache per (chip, pack layout) per the module docstring (autotune.py:2-3): 'time a small set of kernel variants on the device with synthetic data of the same shape and format, keep the fastest, and cache the choices per (chip, pack layout) so a program compiles with them.' REGIME: any autotuner or bench harness choosing among kernel variants — GEMV/GEMM variant sweeps, tile-selection probes, spec-decode arm races. ANTI-REGIME: none — these are discipline clauses, not a regime choice; where the producer has no parameterization clause 1 has no lever and the other three still bind. KINSHIP: our G2 box-state law (the box is part of every latency claim) and the Bench-948 flip-matrix discipline are the same species host-side; this is the corpus's autotune-facing statement. Distinct from swept-shape-bucket-launch-tables (B104 — the OFFLINE sweep shipped as a data table; THIS governs the runtime autotuner that produces such tables, and clause 2 prices its sweep rows), from pipeline-cache-key-covers-every-function-constant (B129 — cache-key completeness for COMPILED PIPELINE variants keyed on bound function constants; THIS is the same completeness law one level up, keying TUNING DECISIONS on the mechanism signature — compose at any function-constant autotuner), from adoption-trial-rotate-operands-per-launch-dram-streaming (B193 — the DRAM-streaming axis at the in-situ adoption trial; clause 2 is its autotune-side instance, same physics, different site), from autotune-cache-extract-outlives-owner + hot-swap-weights-preserve-autotune-caches (B05/B02 — cache LIFETIME across owners and weight swaps; THIS is cache VALIDITY — what an entry must have measured to exist at all), from profile-window-drains-queued-work-before-open (B207 — the window drains queued work before it opens; the warm-up clause here is the warm-side sibling: steady state before sampling), and cross-domain from whole-round-in-situ-adoption-trial (rust_perf B182 — whole rounds + margin vs bursts; the same never-conclude-from-a-wrong-shaped-measurement instinct). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- monolith/compiler/autotune.py:2-3,8-9,30,33-41,58,71-82,93-95,106,160-164. NotBenchable here (the fix IS measurement methodology — benching the bench is the category error the whole-round rule names; the clauses are discipline, adopted by inspection of any tuner we ship). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority). RIDER (Batch 237 / Plan 239 — the HARDWARE-PATH axis of clause 4, tracel-ai/cubecl @ 5a4f5882c82f0dbb3147c628b0eb4492ec9f3dd0 #1811 'fix(autotune): key the tune cache on the adapter and API a device resolved to', Apache-2.0 OR MIT — dual license verified at the clone root, quotes verbatim at the pin clone): clause 4 named every SOFTWARE axis that could change the winner and left the hardware path implicit — a tune cached under one adapter/API pair silently misprices the same mechanism on another. THE KEY GAINS THE FINGERPRINT: Tuner::new gains a third parameter, verbatim (tuner.rs:205): 'pub fn new(name: &str, device_id: &str, fingerprint: &str)', fed from 'client.properties().identity.fingerprint' (local.rs:188) — DeviceIdentity.fingerprint's doc: 'What this runtime compiles *for* — `hip-kernel_gfx1151`, `ptx_sm90`. Verbatim the `compilation_store` fingerprint, so a namespace read back out of a bundle compares against it directly.' — and the persisted namespace moves from '{device_id}/{name}' to '{fingerprint}/{device_id}/{name}' (tune_cache.rs:177; Namespace::scoped composes autotune/<build version>/<fingerprint>/<device_id>/<name>, the version segment already making entries from one build invisible to another), so a tune measured on one adapter can never answer for another. THE API FACE: the wgpu runtime name now names its API — 'wgpu<wgsl-vulkan>' / 'wgpu<wgsl-metal>' / 'wgpu<wgsl-dx12>' / 'wgpu<wgsl-gl>' beside the compiler-named 'wgpu<spirv>'/'wgpu<msl>'/'wgpu<wgsl>' arms — with the doc, verbatim: 'a device that fell back to WGSL says so, along with its API: DX12 and Vulkan on one card share a fingerprint.' — the fingerprint is the ADAPTER identity (shared across APIs on one card, both APIs compiling for the same silicon), so the adapter half alone cannot separate tunes across APIs; the runtime NAME carries the API and rides cache keys beside the fingerprint (Client::name doc: 'The runtime name on this device, as logs and cache keys show it.'; the throughput cache's device key composes runtime, fingerprint and device name; the new wgsl_runtime_names_its_api test asserts Vulkan != Gl and Vulkan != Dx12), so the composed identity separates the same mechanism's measurements across APIs even on the same card. THE TEST NAMES THE LAW: persisted_answers_are_scoped_to_the_adapter_fingerprint with the doc, verbatim: 'A device id that resolves to another adapter must not read the first adapter's persisted answers.' — the same tuner name + device id under fingerprint fp-b reads TuneCacheResult::Miss (a fresh tune runs), the same fingerprint fp-a re-reads its answer ('Unchecked, not Miss: the answer was hydrated from disk.'). COMPOSED LAW: clause 4's mechanism signature (every winner-changing axis NAMED) + the hardware half (the adapter fingerprint + the API the device resolved to) = the full key — the same mechanism measured on another adapter/API is a DIFFERENT measurement, and this is the G2 box-state law pushed INTO the cache key, so the stale cross-hardware hit is structurally impossible rather than disciplined against. Distinct from this rule's clauses 1-3 (producer params / DRAM streaming / noise margin — measurement SHAPE; THIS is measurement IDENTITY), from pipeline-cache-key-covers-every-function-constant (B129 — the COMPILED-PIPELINE variant of key completeness, keyed on bound function constants; THIS is the TUNE-cache hardware identity — compose at any function-constant autotuner), and from autotune-cache-extract-outlives-owner + hot-swap-weights-preserve-autotune-caches (B05/B02 — cache LIFETIME across owners and weight swaps; THIS is cache VALIDITY across hardware paths — compose: a cache that survives its owner still refuses to answer for another adapter). CITE: crates/cubecl-runtime/src/tune/tuner.rs:205 + tune_cache.rs:177 + local.rs:188; crates/cubecl-wgpu/src/runtime.rs:615-623; crates/cubecl-server/tests/integration_test.rs:691-721; crates/cubecl-ir/src/properties.rs:187-190. NotBenchable here (count-neutral rider, correctness-of-measurement class — the fix is cache-key completeness; its instrument is the persisted-scope test, never a wall-clock bar). Batch 237 rider.".to_string(),
            ref_shape: "// naive: the synthetic under-allocates the producer's partials, times
// one cache-hot weight copy, picks the strict minimum with no noise
// margin, and caches the winner under a key that names no mechanism —
// a stale entry then answers for a different kernel.
fn autotune_naive(candidates: &[Variant]) -> Variant {
    let mut best = candidates[0];
    let mut best_t = f64::INFINITY;
    for c in candidates {
        // 4 partials: NOT the program's stat_parts; 8 MB: cache-hot
        let t = time_variant(c, 4, 8.0);
        if t < best_t {
            best_t = t; // no margin: cache-warm noise elects the winner
            best = *c;
        }
    }
    CHOICES.set(best.key()); // the key is the variant name alone
    best
}

fn time_variant(_v: &Variant, _partials: usize, _mb: f64) -> f64 {
    1.0
}

#[derive(Clone, Copy)]
struct Variant {
    key: u64,
}

impl Variant {
    fn key(self) -> u64 {
        self.key
    }
}

struct Choices;

impl Choices {
    fn set(&self, _k: u64) {}
}

static CHOICES: Choices = Choices;
"
            .to_string(),
            opt_pattern: "// optimized: the synthetic sees the PROGRAM'S producer params, streams
// replicated weights (>= 256 MB per timing), replaces the default only
// beyond a noise margin (min-of-5 after a 30 ms warm), and the cache
// key names EVERY mechanism axis — defaults elided, deltas appended.
const NOISE_MARGIN: f64 = 0.03; // beat the default by MORE than this
const STREAM_BYTES: usize = 256 << 20;
const REPS: usize = 5;
const WARM_MS: f64 = 30.0;
const DEFAULT_PARTS: usize = 64;

fn autotune(
    info: &PackInfo,
    epilogue: Option<Epilogue>,
    norm_fed: bool,
    stat_parts: usize,
) -> Choice {
    // (4) the full signature: base axes always present, conditional
    // tails appended ONLY when an axis leaves its default
    let mut key = Key::base(info, epilogue, norm_fed);
    if info.scale_lane_divisor != 1 {
        key.push_shared_scales(info.scale_lane_divisor); // |shared-scales{d}
    }
    if info.scale_order != ScaleOrder::Lane {
        key.push_scale_order(info.scale_order); // |scale-order-{o}
    }
    if norm_fed && stat_parts != DEFAULT_PARTS {
        key.push_stat_parts(stat_parts); // |P{stat_parts}: the producer's
        // partial count rides the key — a stale entry cannot answer
    }
    let mut choices = Choices;
    if let Some(hit) = choices.get(&key) {
        return hit; // measured under THIS signature, reuse it
    }
    let default_t = measure_default(info, epilogue, norm_fed, stat_parts);
    let mut best = Choice::Default;
    let mut best_t = f64::INFINITY;
    for variant in variants(info, norm_fed) {
        // (1) the program's REAL producer count — never a smaller stand-in
        // (2) >= 256 MB of replicated weights per timing: DRAM, not cache
        let t = min_of(REPS, || {
            warm_then_time(variant, stat_parts, STREAM_BYTES, WARM_MS)
        });
        // (3) conservative-vs-best: beat the default by MORE than noise
        if t < default_t * (1.0 - NOISE_MARGIN) && t < best_t {
            best_t = t;
            best = Choice::Variant(variant);
        }
    }
    choices.insert(key, best); // published only after the bar is applied
    best
}

fn min_of(_n: usize, _f: impl FnMut() -> f64) -> f64 {
    1.0
}

fn warm_then_time(_v: Variant, _parts: usize, _stream: usize, _warm_ms: f64) -> f64 {
    1.0
}

fn measure_default(_i: &PackInfo, _e: Option<Epilogue>, _n: bool, _p: usize) -> f64 {
    1.0
}

fn variants(_i: &PackInfo, _n: bool) -> Vec<Variant> {
    Vec::new()
}

#[derive(Clone, Copy)]
enum Choice {
    Default,
    Variant(Variant),
}

#[derive(Clone, Copy)]
struct Variant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Epilogue {
    Plain,
    Residual,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScaleOrder {
    Lane,
    Payload,
}

#[derive(Clone, Copy)]
struct PackInfo {
    scale_lane_divisor: usize,
    scale_order: ScaleOrder,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key;

impl Key {
    fn base(_i: &PackInfo, _e: Option<Epilogue>, _n: bool) -> Key {
        Key
    }

    fn push_shared_scales(&mut self, _d: usize) {}

    fn push_scale_order(&mut self, _o: ScaleOrder) {}

    fn push_stat_parts(&mut self, _p: usize) {}
}

struct Choices;

impl Choices {
    fn get(&self, _k: &Key) -> Option<Choice> {
        None
    }

    fn insert(&mut self, _k: Key, _c: Choice) {}
}
"
            .to_string(),
        },
        KernelRule {
            rule_id: "moe-readiness-queue-no-grid-barrier-producer-scans-before-retire".to_string(),
            gpu_concept: "MECHANISM (grouped-MoE dispatch on a readiness queue, no grid-wide barrier): the gate/up grouping dispatch and the down projection run as ONE persistent worker program with NO grid-wide barrier between them — verbatim (moe_ready.py:5-7): 'There is no grid-wide barrier or wait for a particular worker. A bounded worker loop chooses ready down work or an unclaimed gate tile. The preceding grouping dispatch resets the counters before every replay.' (Provenance in the same docstring, moe_ready.py:3-4: 'Inspired by Mirage PR #786 (c28cac618b98fda1e0f1d590923b5f69b4ef3603), implemented independently using Metal coherent memory and device fences.') THREE correctness clauses carry it. (1) RESET-BEFORE-REPLAY: the queue counters are zeroed by the PRECEDING dispatch, before the replay starts — never by a consumer mid-flight (resetting an ACTIVE queue corrupts every claim in progress); the route-table producer initializes the bounded work queue as part of ITS dispatch (moe_fusion.py:23: 'The preceding route-table dispatch initializes the bounded work queue.') — setup-vs-flight, the same law the static-fusion queue setup carries. (2) THE BOUNDED WORKER LOOP: each iteration, a worker checks round-robin over experts (moe_ready.py:101: expert = (worker+iteration) % count) whether ITS expert's grouping results are fully published (readiness counters, :102-103) and an unclaimed down tile exists (fetch_add claim, :104-105); else it claims an unclaimed gate tile (:107-110); else it continues within a computed ITERATION BOUND (:96; the no-retire arm caps the loop at min(maxpairs*sum(chunks)*8+32768, 65536), :72) — never a spin on a named peer, never an unbounded poll. (3) LAST-PRODUCER-SCANS-BEFORE-RETIRE — the clause that makes worker EXIT safe, verbatim (moe_ready.py:68-71): 'Once all gate jobs are claimed, each unready expert still has an in-flight producer. Its last producer continues after publication and scans ready work before retiring. Consequently idle workers can leave without losing future down jobs, freeing execution slots instead of polling other groups.' The scan arm is OPT-IN (retire=config.get('retire_idle',False), moe_ready.py:67; the scan loop at :73-85 walks the other experts' readiness + claim counters before the worker returns): the DEFAULT arm bounds the loop instead, and the comment's invariant licenses either — because the worker that publishes an expert's LAST gate result is still resident, IT can pick up that expert's down work, so a worker leaving never orphans ready work. NUMERICS ride the same fusion, verbatim (moe_down.py:3-5): 'The projection rounds each expert result to BF16 in threadgroup memory, then the original ordered FP32 combine adds the shared expert and residual.' — the per-threadgroup rounding site is the fused kernel's private threadgroup ushort partial[topk*rows] buffer (moe_down.py:104; the down writes threadgroup-local, :69, the combine reads it back, :75), and the ORDER is the contract, verbatim (moe_fusion.py:3): 'The router and final weighted reduction retain their original numerical order.' — fusing the dispatch must not fuse the rounding away: BF16 per threadgroup, FP32 combine in the original order, or a tolerance gate becomes the only defense. 'No task waits for another worker' (moe_down.py:5) — oversubscription cannot deadlock, the property the local-expert lane states for itself too (moe_fusion.py:22: 'Workers never wait on other workers, so oversubscription cannot deadlock.'). THE UNDO LAW, verbatim (moe_down.py:6-7): 'Original operations are retained as JSON metadata so explicit tuning recipes can recover the unfused program after automatic chip selection.' — the fusion is REVERSIBLE BY CONSTRUCTION: the pre-fusion down + combine op specs are serialized into the fused op's metadata (moe_unfused, moe_down.py:121-122) and an explicit recipe restores them in place before re-tuning (unfuse_down_combine, moe_down.py:17-33: restore the combine at i, re-insert the down at i-minus-gap). Automatic chip selection may fuse; an explicit human recipe wins — metadata, not hope. REGIME: grouped MoE with ragged expert readiness on persistent/dispatch-free runtimes (megakernels, cooperative launch, any host that cannot cheaply re-dispatch per phase). ANTI-REGIME: dense uniform experts (no readiness skew — a plain grouped GEMM needs no queue); hosts where two ordinary launches with the API's own ordering are cheap. Distinct from gpu-resident-moe-routing-no-host-sync (B105 — ADJUDICATED COMPOSED NEIGHBOR, not a duplicate: that rule owns the HOST-SYNC BOUNDARY of the routing PIPELINE — zero host readback, sentinel-padded capacity map, consumer-side desync guard; THIS owns the DEVICE-SIDE SCHEDULING between grouping and down — the readiness queue, the worker loop policy, producer-scans-before-retire, reset-before-replay, plus numerics placement and undo metadata; the two compose: B105's route table is this queue's INPUT — the table binding at moe_ready.py:58 — and this queue's counters never cross to the host), from barrier-free-per-dependency-wait-from-loop-counters (B70 — host-side multi-worker loops replacing all-worker barriers with monotone counters + closed-form thresholds; same no-barrier philosophy, host orchestration substrate, thresholds computed at issue time vs THIS claim-counter fetch_adds inside the device program), from sentinel-readiness-over-separate-flag (B119 — the readiness POLL mechanism for peer-produced DATA via flag-in-data sentinels; THIS readiness is producer-count atomics gating CLAIMS, and nothing is re-poisoned), from ring-overwrite-gated-k-back-readiness (B194 — readiness counters consumed for overwrite SAFETY at ring distance K; THIS consumes readiness counters for WORK CLAIMS), from moe-per-expert-mixed-dispatch-worst-case-workspace (B186 — per-expert ADMISSION + plan-time worst-case workspace for the tiled matmul; THIS is the dispatch QUEUE over already-admitted experts), and from expert-cache-write-path-regime-gated (B210 — the MoE expert cache's WRITE-path regime gate; the cache-axis neighbor in the same MoE-dispatch family, no queue at all). Source: lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5 (Apache-2.0) -- monolith/compiler/moe_ready.py:1-7,58,67-85,96-127; moe_down.py:1-7,17-33,69,75,104,121-122; moe_fusion.py:3,22-23. NotBenchable here (a Metal megakernel worker loop — no Metal validator in this corpus; the win class is execution-slot freeing + barrier elimination on a persistent dispatch, and the source's own A/B control is the unfused program it retains as metadata). Batch 223 / Plan 222 / Research 249 (drafts; Research 248 is the quote authority).".to_string(),
            ref_shape: "// naive: a GRID-WIDE barrier before the down pass; workers spin on
// NAMED peers until every expert is ready; and whoever notices resets
// the queue counters mid-flight — corrupting the next replay's claims.
const E: usize = 8;

fn moe_naive(ready: &[bool; E], workers: usize) {
    for _w in 0..workers {
        while !ready.iter().all(|&r| r) {
            std::hint::spin_loop(); // poll peers until ALL are ready:
            // execution slots burn on polling, nobody retires early,
            // and one straggler stalls every worker
        }
    }
    grid_barrier(); // every worker must arrive: the straggler serializes all
    reset_counters(); // WRONG site: an ACTIVE queue loses its claims
    down_pass(workers);
}

fn grid_barrier() {}

fn reset_counters() {}

fn down_pass(_workers: usize) {}
"
            .to_string(),
            opt_pattern: "// optimized: a bounded worker loop — ready down-work or an unclaimed
// gate tile, never a named-peer wait; the PRECEDING dispatch resets the
// counters before the replay; the last producer scans ready work before
// retiring; numerics round BF16 per threadgroup, combine FP32 in order.
use std::sync::atomic::{AtomicU32, Ordering};

const E: usize = 8;
const GATES_PER_EXPERT: u32 = 4;
const DOWNS_PER_EXPERT: u32 = 2;
const ITER_BOUND: u32 = 1 << 16; // the bounded loop, never unbounded

struct Queue {
    gate_claim: AtomicU32, // next unclaimed gate tile
    expert_ready: [AtomicU32; E], // grouping results published per expert
    down_claim: [AtomicU32; E], // down tiles claimed per expert
}

impl Queue {
    // clause (1): the producer runs this BEFORE the replay — never a
    // consumer mid-flight (an active queue must not lose its claims)
    fn reset_before_replay(&self) {
        self.gate_claim.store(0, Ordering::Relaxed);
        for i in 0..E {
            self.expert_ready[i].store(0, Ordering::Relaxed);
            self.down_claim[i].store(0, Ordering::Relaxed);
        }
    }

    // clause (2): one bounded-loop iteration for one worker
    fn next_job(&self, worker: usize, iteration: u32) -> Option<Job> {
        let expert = (worker + iteration as usize) % E; // round-robin
        if self.expert_ready[expert].load(Ordering::Relaxed) == GATES_PER_EXPERT
            && self.down_claim[expert].load(Ordering::Relaxed) < DOWNS_PER_EXPERT
        {
            let part = self.down_claim[expert].fetch_add(1, Ordering::Relaxed);
            if part < DOWNS_PER_EXPERT {
                return Some(Job::Down(expert)); // ready down-work first
            }
        }
        let job = self.gate_claim.fetch_add(1, Ordering::Relaxed);
        if job < (E as u32) * GATES_PER_EXPERT {
            return Some(Job::Gate(job)); // else an unclaimed gate tile
        }
        None // keep looping within the bound; the retire arm scans, then leaves
    }
}

enum Job {
    Gate(u32),
    Down(usize),
}

// clause (3): the LAST producer of an expert — the worker whose publish
// completes its grouping — scans ready work ITSELF before retiring, so
// idle workers leave without losing future down jobs.
fn last_producer_scans(q: &Queue, expert: usize) -> bool {
    let published = q.expert_ready[expert].fetch_add(1, Ordering::Relaxed) + 1;
    published == GATES_PER_EXPERT && q.next_job(expert, 0).is_some()
}

// numerics: BF16 rounding PER THREADGROUP, then the original-order FP32
// combine; undo: the unfused ops ride the fused op's metadata.
fn combine_in_order(per_tg_bf16: &[f32], weight: f32) -> f32 {
    per_tg_bf16.iter().map(|x| x * weight).sum::<f32>()
}
"
            .to_string(),
        },
    ]
}
