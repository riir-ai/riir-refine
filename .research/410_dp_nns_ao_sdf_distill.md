# Research 410 — DP-NNS-AO-for-threejs distill verdict (SDF ambient occlusion; the DP-NNS negative)

**Status:** DISTILLED — lane executed as seal-sprints sprint#032 (owner go 2026-11-14: port + bench + replace-if-GOAT + grass exclusion); the corpus-mining batch rides a later batch window (candidates staged below, plan-first law binds when it opens).

## Source

`wrong-nebula/DP-NNS-AO-for-threejs` — PIN `e48b563b26146d498f705f55071156543c38aa33` (3 commits, tip commit-dated 2026-11-14T18:48:39+09:00, **NO LICENSE file** → decision-content-only transcription posture, outbound-PR fail-closed). Shallow clone `.raw/dp-nns-ao`, REMOVED at close of the verdict session. Surface: `index.html` (505 lines — the three.js demo, AO + bake shaders inline), `tools/bake_sdf.cpp` (321 lines, dependency-free C++17 SDF baker), `benchmark/` (methodology + numbers + the DP-NNS ablation), `README.md`. The user's headline premise is INVERTED by the source itself: the repo is named for the DP-NNS paper (arXiv 2611.15320, Delaunay NN search w/ grid-accelerated queries + parallel construction), but its own precompute doc records DP-NNS **as a measured negative** — "As a seed for this baker it cost more to build than it saved, so the tool doesn't use it" (sample bake 0.61 s + 3.5 s index build, 3.7 cm p95 error; ablation table: BVH+prev-voxel-triangle 115/307 ms vs BVH+DP-NNS-seed 571/1,610 ms at 1.9k/488k tris).

## The measured claims (source's own tables, Apple M1, 1280×720)

| arm | static median | MAE vs 512-ray ref |
|---|---:|---:|
| four-tap SDF AO | 0.161 ms | 0.0421 |
| reweighted SDF | 0.139 ms | 0.0439 |
| SDF + wall correction | 0.159 ms | 0.0401 |
| Three.js SSAO | 3.797 ms | 0.0792 |
| Three.js GTAO | 8.970 ms | 0.0835 |

SDF is faster AND more accurate on this scene — not a lossy trade. Caveats carried by the source itself: one machine, one camera, one scene; wall weights tuned on Archive; SSAO/GTAO ran with full-res depth+normal inputs ("an engine with reusable buffers may have different ratios").

## The mechanisms (decision content at the pin)

1. **SDF-AO vs screen-space ray-march**: occlusion from four taps against a baked distance volume (static) + per-object analytic/local fields (movers), min-composed per tap. No blur, no temporal accumulation, no half-res ladder — noise-free by construction.
2. **Baker, prev-voxel seed**: exact closest-point-on-triangle distance (Ericson 5.1.5); BVH query starts from the PREVIOUS voxel's nearest triangle as the bound ("a free bound that is almost as tight" — beat nanoflann 393 ms and DP-NNS 571 ms vs its own 115 ms). Scanline-coherence reuse; the sample-index class LOSES because build cost dominates (trim 8.6→7.2 tested triangles = 16%, cost 3.4×).
3. **Baker, 1-Lipschitz far-block skip**: 8³ blocks; block center queried at clip + half-diagonal; miss fills the whole block — 241→109 ms (Archive), 365→146 ms (dense).
4. **Winding-number sign**: one z-ray per voxel column, ±1 sorted crossings, nonzero = inside — handles overlapping closed shells without per-object attribution; non-watertight robust via generalized winding (solid-angle sum) on the GPU bake.
5. **Mover architecture**: movers never baked into the static field; per-tap min of analytic SDFs (box/sphere/capsule/cylinder/rounded/torus) or 64³ per-mesh local fields (GPU-baked at load, exact point-triangle + solid-angle winding, one atlas, padding ≥ tap reach); per-pixel farthest-tap bounding-sphere cull ("|p+n·t−c|−t never increases with t, so the farthest tap is the easiest to satisfy: if it fails, every tap does") + draw-instance reach cull + convex self-skip; pixel-for-pixel matched with culls on/off (source's claim).
6. **The measured negative**: DP-NNS as an exact-query seed for grid-ordered distance queries — index build ≫ savings vs a free coherence bound. Kin: `sorted-input-previous-row-fast-path` (rust_perf — the previous-answer reuse, but EQUALITY-check axis; this source is the METRIC-BOUND axis: a distance upper bound that prunes a search), `eager-pointer-collect-lazy-fetch` (the index-build amortization regime, K/N≪1).

## Domain routing + ranked candidates (staged for a later batch; dedup greps run this session)

Routing: shader mechanisms → **shader_perf** (WGSL-class rules; GLSL source transcribes per the law; the corpus's 8 rules own SampleBudget/LoopBoundedness/StageTraffic/BranchCoherence/DerivativeSafety — zero AO/occlusion/culling owners, grep recorded); baker mechanisms → **rust_perf** (compute/io; zero voxel/far-block/scanline/coherence-NN owners). kernel_opt: zero occlusion/atlas/bounding-sphere owners. The tempting wrong domain (this is GPU-adjacent) — the pass IS a fragment shader (shader_perf, not kernel_opt); the baker is pure CPU (rust_perf).

| # | candidate | domain | grade | notes |
|---|---|---|---|---|
| R1 | `sdf-ao-distance-field-four-tap` (the architecture: baked field + per-tap min vs ray-marched AO) | shader_perf | B | decision content = the algorithm-selection + the min-composition + noise-free-no-denoise law; the "replace the AO algorithm" fix-space is architecture-wide → lands as decision-content, suggest-lean |
| R2 | `farthest-tap-dominance-cull` (+ convex self-skip + instance reach cull as faces) | shader_perf | B+ | the monotonicity argument is provable; pixel-identical claim; zero owners |
| R3 | `mover-min-into-static-field-no-rebake` | shader_perf | B− | dynamic occluders enter a baked field via per-tap analytic/local min — motion needs no rebake |
| R4 | `prop-local-field-atlas-padding-past-reach` | shader_perf | B− | 64³ local-space fields, one atlas, pad ≥ effect reach ("nothing outside it can occlude") |
| R5 | `lipschitz-far-block-skip` | rust_perf | **B+ (benchable GOAT lead)** | exactness provable (fill value = clipped bound, sign independent), 2.2× measured in-source, CPU-fixturable on the Pareto validator |
| R6 | `prev-query-coherence-bound-beats-index-seed` | rust_perf / measurement | B | the DP-NNS negative as the measurement-category rule: build-cost amortization gate for exact grid-ordered NN queries; in-rule adjudication vs sorted-input-previous-row (equality vs metric bound) + eager-pointer-collect (the regime) |
| R7 | winding-number column sweep | rust_perf | C+/face | correctness pattern; likely a face on R6's entry or annotation |

UPSTREAM check: not applicable — every row is the source's own landed decision (no fix-shaped leads; outbound-PR lane empty by construction, and license-less anyway).

Yield estimate: 3–5 rules after ~50% falsification (R5/R6 near-certain; R2 strong; R1/R3/R4 decision-content class).

## Lane intel (the axis that fired — owner-confirmed)

**SEAL LANE CONFIRMED-HIGH (owner directive 2026-11-14)**: the canvas's AO view effect (`seal-render bevy_canvas_post` — GTAO/SSAO three.js ports, the composite multiplies the opaque pass between opaque and transparent) is the direct consumer. Executing as **seal-sprints sprint#032** (`eaeaf09`): `AoKind::Sdf` + baked static volume + per-prop 64³ atlas + body capsules + four-tap pass + grass receive-exclusion (grass draws opaque → receives the multiply and casts GTAO/SSAO depth — the owner's "looks bad on grass") + distance falloff + pre-registered GOAT bars (G1 MAE ≤ 0.06 and ≤ GTAO's; G2 median ≤ 0.35× GTAO's; G3 other kinds byte-identical; G4 alloc-free bake ≤ 500 ms) + replace-the-recommendation-if-GOAT (owner pre-approved). League/riir-ai lane mapping: NONE (grep `occlu|ambient.*ao|sdf` over riir-ai/.issues = inference-side hits only, no AO lane; no league row touches this). NOT transferable: the source's absolute ms (different scene/silhouette; our zones are ~100×100 tiles vs its 26×18×8 m — sprint#032's voxel-budget risk carries it); DP-NNS for anything (the negative).

## Caveats

NO LICENSE (decision-content transcription only; never vendor code from it). 3-commit repo, tip moved the day of the verdict (re-clone + re-pin at mine time). The shader half was read from the clone (index.html L30-255); bake_sdf.cpp read in full; benchmark JSONs not opened (the .md carries the tables). The source's "renders matched pixel for pixel with culls on/off" is its own claim, untested here. NOTHING INVENTED — every quote read at the pinned clone or the live benchmark pages this session. Session: zed-m3@m3, epoch 1794668000.
