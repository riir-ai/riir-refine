# Bench 093 — shader_perf G5-real: capture-fidelity protocol (Plan 134 T1.6)

Status: **GO — G5-real PASS** (executed 2026-09-16, M3 Max / Apple GPU / Metal; unity/zed active — exempt per owner call)

Executes the last unticked Phase-1 item of
[Plan 134](../.plans/134_shader_perf_domain_phase0_batch1.md) T1.6: heal a
synthetic Tier-2-shaped shader end-to-end via the consumer protocol —
**scan → fix → naga-validate → capture pre/post at pinned frame in
riir-shader's harness → byte-identity** — with the harness commit pinned
(Proposal 014 caveat 6).

## Pins

| What | Value |
|---|---|
| Harness repo | `riir-shader` @ `ae8123e24d52b658939589c4a419cf8ddce4a108` (worktree clean at run time) |
| Harness build | `cargo build --release -p riir-shader-showcase --features hot-reload` |
| Healer repo | `riir-clippy` @ `6460df2f89773299a8729e01c14d7efa388ff48b` |
| Healer build | `cargo run --features latent_retrieval,rust_perf,clippy_lints,kat_service,shader_perf,shader_verify` (naga 30.0.1) |
| Carrier effect | `triangle-led-front` (tlf), hero mode −1 (lines: no brush, no deploy), capture frame 90 |
| Carrier file | `src/assets/shaders/tlf/led-emitters.wgsl` — the ONE import-free file-backed shader with render entry points (`@vertex vs_main` + `@fragment fs_main`) in the 9 file-backed effects |
| Serving | hot-reload lane: `RIIR_SHADER_HOT=1 RIIR_SHADER_HOT_DIR=<dir>` (the fixture under test is the ONLY file on disk; every other shader falls back to embedded bytes) |

## Fixtures (all under `.benchmarks/data/093_g5real/`)

| File | SHA-256 (16) | Role |
|---|---|---|
| `led_emitters_pre.wgsl` | `00d84f5ca012c38c` | the synthetic Tier-2 defect: a `[g5-defect]` "debug refine" `loop` with no exit, gated on `cfg.resolution.x < 0.0` (never true — the substrate always passes a positive resolution) |
| `led_emitters_post_healed.wgsl` | `58246ec761c8311d` | the healer's output: exactly the 2-edit guard insertion (decl + increment/bound-check), scanner-clean on re-run (idempotence) |
| `led_emitters_canary.wgsl` | `1a786ef18643c3ee` | protocol self-test: same defect + a deliberate `emit * 1.002` (0.2%) behavioral delta the protocol MUST catch |
| `tlf_frame90_valid.png` | `8c193126b1370c3c` | a valid capture (the byte both sides produced) |
| `tlf_black_invalid.png` | `3f4e354b4dec307b` | a black capture — the harness invalid (see below) |
| `trails_pre.wgsl` / `trails_post_healed.wgsl` | `8c1ef29fe1f10e74` / `3a7ee468440e2641` | the po-leg carrier (heal legs passed; capture leg measured NEGATIVE on po — see calibration) |

## The heal legs (all green, both carriers)

1. **Scan**: `cargo heal --domain shader --suggest` fires
   `unbounded-render-loop-guard` at the defect loop — po/trails line 140,
   tlf/led-emitters line 103.
2. **Fix**: `--fix` writes exactly 2 bounded edits (the `shader_heal_guard`
   decl + the increment/256-bound break); re-run fixes 0 (idempotence, both
   the guard-name refusal and scanner-cleanliness).
3. **Naga-validate**: with `shader_verify` compiled in, the fix lane's
   Tier-1 validates the healed module in-process (naga 30.0.1) — no
   refusal. The PRE defect shape is validated by the strongest oracle
   available: the harness itself rendered it (wgpu validates WGSL via naga
   before pipeline creation — a captured frame is a validation pass).

## The capture protocol (and the two harness findings it produced)

Command shape per run (fresh process each):

```sh
RIIR_SHADER_CAPTURE=<out.png> RIIR_SHADER_CAPTURE_MIN_MS=0 \
RIIR_SHADER_EFFECT=triangle-led-front RIIR_SHADER_CAPTURE_FRAME=90 \
RIIR_SHADER_WINDOW=1600x900 RIIR_SHADER_HOT=1 RIIR_SHADER_HOT_DIR=<dir> \
./target/release/riir-shader-showcase
```

### Finding 1 — po is NOT a byte-identity carrier (measured negative)

particle-orbit's clock is EXECUTION-GATED (advances only on confirmed chain
runs), so the effect-frame at a wall-frame-pinned capture drifts with
pipeline-compile timing: three po captures (`RIIR_SHADER_PO_HERO=1`, frame
60, 1280x720) were pairwise byte-different — including warm-to-warm
(cold-pipeline-cache was NOT the mechanism). Honest conclusion: po serves
its bracketed-hero fidelity protocol, not byte-identity. The heal legs on
`po/trails.wgsl` still passed (scan/fix/naga/idempotence) — the fixtures
are kept as the record.

### Finding 2 — the black-capture invalid class (and why MIN_MS matters)

With `MIN_MS=0` (pure frame pinning), ~1 in 5 captures came out completely
BLACK (pixel sum 0): the capture fired before the effect's first real
present — the exact hazard `warmup_min_duration` (default 750 ms) exists to
prevent ("17 of 29 sweep captures came out fully black" without it,
`riir_shader_core/src/capture.rs`). A black frame is byte-identical
regardless of shader content — PROVEN: the canary produced one, and it
hash-equals the PRE black frame. Black captures are information-free
invalids: excluded, and the run extended until k valid captures per side.

The 750 ms wall itself is a knife edge near frame 90 (90 frames ≈ 740 ms
at this resolution): captures under the default drift by a few frames
run-to-run (adjacent settled-phase frames differ by ±1-3 LSB on ~1.4% of
pixels — initially misread here as GPU noise; it is frame drift). The
protocol therefore pins FRAMES (MIN_MS=0) and discards black invalids —
every valid capture is then exactly animation-frame 90, byte-deterministic
across runs and across the heal.

### The verdict rule

Collect ≥4 VALID (non-black) captures per side. **G5 PASSES iff the
hash-sets are equal** (every PRE capture has a byte-identical POST twin
and vice versa; distinct hashes per side ≤ 2). A behavioral change of ANY
size breaks every pair — the canary proves the detection floor.

### Measured results

| Side | runs | valid hashes | invalid (black) |
|---|---|---|---|
| PRE | 5 | `8c193126b1370c3c` ×4 | 1 |
| POST (healed) | 5 | `8c193126b1370c3c` ×4 | 1 |
| Canary (0.2% bump) | 3 | `556aa8bf1cb63fd5`, `b5a2abe0ea0cd2b8` — **no overlap with PRE** | 1 |

- **PRE set == POST set == {`8c193126…`}** — the healed shader renders
  byte-identically to the defect shader at every captured frame: the guard
  insertion is behavior-preserving on the rendered path. G5-real **PASS**.
- **Canary detection**: both valid canary captures differ from every PRE
  hash (pixel deltas: 208,613 px / max 35 and 487,903 px / max 23 — a 0.2%
  `emit` change is amplified by the composite to easily-detectable size).
  The protocol catches changes far below visual threshold.
- Hot-serve was verified live before trusting any of this: a
  solid-red `fs_main` variant rendered (green channel exactly 0.0) —
  the disk file IS the shader the GPU ran (guards against a vacuous pass
  where both sides silently render the embedded bytes).

## Scope notes

- The defect is inert at capture by construction (never-true gate) — this
  proves the HEAL preserves rendering, which is what the mint gate needs:
  the gate's job is capping the TDR class without changing what the author
  sees. It does NOT claim the guard rescues an actually-hung shader (that
  is the guard's own semantics, covered by the unit tests).
- The black-frame exclusion rate (1/5, 1/5, 1/3) is recorded honestly; the
  protocol's run-extension step absorbs it.
- Phase 2 gates unchanged: frame-time tier (needs riir-shader's frame axis),
  f16/MSAA rules (needs ≥6 bounded), dapps S2 mint-gate wiring (waits on the
  shader-toy editor consumer).
