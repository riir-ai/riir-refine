# Bench 133 — the kernel_opt GPU-host G2 SOURCES PIN (Bench 130's companion; Issue 191 T6 step 2)

> **RENUMBERED 131 → 133 (2026-11-05, zed-m3-i208pregate):** a dual allocation — this
> document and `131_kernel_pregate_ab_preregistration.md` (the pregate lane, Issue 208)
> each claimed 131 from concurrent sessions. First allocation owns the number
> (`1894374f` 20:08 vs this commit's parent-side `82b5aaf2` 21:16; citation_weight's WEAK
> +1 concurs, 42/46 sites UNRESOLVED). Every citing site moved in the same commit; the
> sources themselves (the 10 pinned files + their BLAKE3 digests) are UNCHANGED — the
> pin's identity is the digests + the sources commit `82b5aaf2`, both stable.

**Status:** SOURCES PINNED 2026-11-05 (zed-m3-i191t6, the issue-191 lease) — **NO TIMING RUN HAS HAPPENED.** This document materializes the exact sources the 4090 run compiles, pinned per file by BLAKE3 (recomputed at emit time by the regeneration script preserved in this file's git history). The runner at its scratch home (`kernel_bench_g2_gpu/` beside the siblings, untracked) is the working copy; THIS file is the pin. Any source change after this commit = a NEW sources pin (a fresh bench number), never an edit here.

Metal verification (m3, Apple M3 Max, wgpu-msl, `--release`): `cargo check` + `cargo clippy --all-targets` zero warnings; `--correctness` **23 passed / 0 failed**; `cargo check --features cuda_runtime` type-checks clean (cudarc compiles driver-less on macOS — the house Issue-949 posture). Indicative Metal leg median 1.0299 — DISCLOSED AS NOT-A-MEASUREMENT (no exclusivity claim; the G2 verdict belongs to the 4090).

## Digest table (BLAKE3, b3sum)

| file | blake3 | bytes |
|---|---|---|
| `Cargo.toml` | `76248cbe17b5ea98447c54446a0033419d9915660078e9c9c8ff6baed37f60ae` | 714 |
| `Cargo.lock` | `c5913e8721327c00a9efa69a98993cf73d741c3c4137dfc7a71f34a17d535b63` | 96467 |
| `src/common.rs` | `df7ce39cfec9034983a461f72de7c60110b5e34345dccef6d22d308d513ff07d` | 10463 |
| `src/main.rs` | `7b3dd71a70e115dc52d1bca079f123b8f4864fac89bb0744c340dd311ee80317` | 9913 |
| `src/pairs.rs` | `fbd44b1f5e7d3a755c546839e33085fb630af0fca3078b62629c83dbacf0cacf` | 2587 |
| `src/pairs_gemm.rs` | `c46df043ebcd9907d23188c3c0311f6564795ee06b8c1460988d378ab7bca269` | 39446 |
| `src/pairs_reduce.rs` | `5c5b1ae2a43a117955ff07c246dedc5dc98c47e409345003c4ee0f7576f5adeb` | 35674 |
| `src/pairs_special.rs` | `d0eb21fa4ba4209c68fcbab322bebd3d82b72dc20a70aa6ae440f3e9835855c7` | 49649 |
| `src/pairs_stream.rs` | `7dbbaf9526c90a541133a2634ddccfa1eb4da84a27c04fa869fe868c922bde5a` | 43694 |
| `src/probe.rs` | `3b61321604370f3ddc34af76ea12be9c8557010648f31883e3d47689ddea1ab2` | 7415 |

Total: 10 files, 296022 bytes. The `Cargo.lock` pin is load-bearing: `cubecl =0.11.0-pre.2` + the three vendored `[patch.crates-io]` rows (the drop_queue #1359 fix, the cubecl-common idle-park fix, the cubecl-wgpu fork) — the patch paths are RELATIVE (`../riir-ai/vendor/...`, `../riir-infer/vendor/...`) and resolve because the materialized crate sits beside the sibling checkouts on both boxes (m3: `/Users/kat/git/*`, 4090: `E:/git/*`).

## Materialization protocol (the 4090 run lane)

1. Extract the fenced blocks below into `kernel_bench_g2_gpu/<path>` (the `### File:` headers name the paths; a small python extractor suffices — parse `### File: <path>` then the 5-backtick fence body).
2. Verify: `b3sum <every file>` matches the digest table above. A mismatch = STOP (the pin is broken; no compile, no run).
3. `CARGO_TARGET_DIR=/tmp/kbg2-target cargo run --release --features cuda_runtime -- --correctness` — all 23 gates must PASS on the 4090 BEFORE any timing (the correctness gate is the modelless-gain law: a wrong kernel cannot win).
4. Timing: `CARGO_TARGET_DIR=/tmp/kbg2-target cargo run --release --features cuda_runtime -- --bench --provenance` with `BENCH_SOURCES_COMMIT=<this file's commit hash>` and `BENCH_BOX_LABEL=4090` exported.
5. The rig hold: `agent-mail rig run 4090 --exclusive --needs cuda --timeout 2h -- <the above>`; PROVENANCE fields per Bench 130 (nvidia-smi query at start AND end, the rig QUIET receipt, toolchain version, the digests). Thermal gate: > 10 degC over the box's ambient band or anomalous idle power => the run's numbers are PROVISIONAL (cannot pass the bar; can only fail or read UNSEEN).

## Pre-run amendments to Bench 130 (accepted 2026-11-05, BEFORE any run — the T1 law's legal window)

1. **Pair count: 21 -> 23.** Bench 130's prose under-counted its own table (23 rows: P117x3, P125x1, P111x2, P136x2, P138x2, P134x2, P110x1, P137x2, P112x2, P133x1, P135x1, P121x1, P75x3). All 23 are implemented and Metal-verified; MIN_MEASURED_PAIRS stays 12.
2. **Accepted realized-shape deviations** (each disclosed in-source with `// DEVIATION:`, none changes the measured axis): P117-A opt reads one concatenated W buffer (branchless); P117-C 32-output register tile (>= the pinned 4 chains); P111-B + P134-B i32 fixed-point atomics instead of f32 atomicAdd (Metal lacks device f32 atomic add; the atomic-storm axis is preserved, the rounding cut identical in both arms); P136-A hi/lo split performed host-side into 4 buffers; P136-B 16-bit store emulated as in-kernel f32->f16-RNE->f32, the reduction realized as block smem trees (the sm_89-legal mapping, disclosed); P135-A stages 2048 candidates (256 threads x top-8); P121-A selection = single block, 256 threads, 256 selection passes (honest but slow — the run record discloses its verdict); P110-A ref = global stats arrays + normalize launch (exp computed 2x/element); P137-A the strided axis sits on the WRITE side under the pinned thread mapping (symmetric — the smem tile fixes either side); P133-A opt smem k-half = 64 elems (256 B) per the "capped at 64" geometry.
3. **Harness-law realizations** (Bench 130's laws, implemented): WARMUP=3, PAIRS=15 interleaved with per-pair median, ref-arm calibration to >= 2 ms chunks (clamp 1..=10000) applied to BOTH arms, zero allocation inside timed regions, zero-time chunk = pair FAIL, post-timing correctness re-run, per-chunk launch count black_box'ed, NO autotune (`launch_unchecked` only), deterministic per-pair-seeded LCG inputs, correctness gate BEFORE timing. Bench 130's "sources embedded in the record" law is realized as THIS file (the digest-pinned materialization source); the scratch crate is the working copy.

## The sources

### File: `Cargo.toml`

blake3: `76248cbe17b5ea98447c54446a0033419d9915660078e9c9c8ff6baed37f60ae`

`````toml
[package]
name = "kernel_bench_g2_gpu"
version = "0.1.0"
edition = "2021"

[features]
default = []
cuda_runtime = []

[dependencies]
pollster = "0.4"
cubecl = { version = "=0.11.0-pre.2", features = ["wgpu-msl", "cuda"] }

# The house vendored patches (the drop_queue arithmetic-overflow bug, tracel-ai/cubecl#1359;
# the cubecl-common idle-park fix). These relative paths resolve because this dir sits
# beside the sibling repos (m3: /Users/kat/git/*, 4090: E:\git\*).
[patch.crates-io]
cubecl-runtime = { path = "../riir-ai/vendor/cubecl-runtime-0.11.0-pre.2" }
cubecl-common = { path = "../riir-infer/vendor/cubecl-common-0.11.0-pre.2" }
cubecl-wgpu = { path = "../riir-infer/vendor/cubecl-wgpu-0.11.0-pre.2" }

`````

### File: `Cargo.lock`

blake3: `c5913e8721327c00a9efa69a98993cf73d741c3c4137dfc7a71f34a17d535b63`

`````toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "addr2line"
version = "0.27.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e567177890eb1617b1f774005b66b26b2377afd138a2ca37aae7d8f0c81429d4"
dependencies = [
 "gimli",
]

[[package]]
name = "adler2"
version = "2.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa"

[[package]]
name = "ahash"
version = "0.8.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5a15f179cd60c4584b8a8c596927aadc462e27f2ca70c04e0071964a73ba7a75"
dependencies = [
 "cfg-if",
 "once_cell",
 "version_check",
 "zerocopy",
]

[[package]]
name = "aho-corasick"
version = "1.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c982642fa9e8606056828ee9a8505737230110bb1099153c79efe865c59d12ba"
dependencies = [
 "memchr",
]

[[package]]
name = "allocator-api2"
version = "0.2.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "683d7910e743518b0e34f1186f92494becacb047c7b6bf616c96772180fef923"

[[package]]
name = "android_system_properties"
version = "0.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ae221649c9976a6f6c56ae1facf410f3ddb33cc661c4b7b61020a912d4237fbc"
dependencies = [
 "libc",
]

[[package]]
name = "anyhow"
version = "1.0.104"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "330a5ed07fa54e4702c9d6c4174f74427fc0ef6e214bbd677ae50a5099946470"

[[package]]
name = "arrayvec"
version = "0.7.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d3fb67a6e08acf24fdeccbac2cb6ac4305825bd1f117462e0e6f2f193345ad56"

[[package]]
name = "ash"
version = "0.38.0+1.3.281"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0bb44936d800fea8f016d7f2311c6a4f97aebd5dc86f09906139ec848cf3a46f"
dependencies = [
 "libloading 0.8.9",
]

[[package]]
name = "async-channel"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "924ed96dd52d1b75e9c1a3e6275715fd320f5f9439fb5a4a11fa51f4221158d2"
dependencies = [
 "concurrent-queue",
 "event-listener-strategy",
 "futures-core",
 "pin-project-lite",
 "portable-atomic",
 "portable-atomic-util",
]

[[package]]
name = "atomic-waker"
version = "1.1.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1505bd5d3d116872e7271a6d4e16d81d0c8570876c8de68093a09ac269d8aac0"

[[package]]
name = "autocfg"
version = "1.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53"

[[package]]
name = "backtrace"
version = "0.3.77"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "42f4220217f3288537b83684130f42fad43320b1c3b805f1910d683d52ded24f"
dependencies = [
 "addr2line",
 "cfg-if",
 "libc",
 "miniz_oxide",
 "object",
 "rustc-demangle",
 "windows-link",
]

[[package]]
name = "base64"
version = "0.22.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6"

[[package]]
name = "base64"
version = "0.23.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ac07cdecf99051d9a5238b80f35af32cdeba5b336e55d957b318b50137e18da5"

[[package]]
name = "bit-set"
version = "0.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "09ec2f926cc3060f09db9ebc5b52823d85268d24bb917e472c0c4bea35780a7d"
dependencies = [
 "bit-vec",
]

[[package]]
name = "bit-vec"
version = "0.9.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b71798fca2c1fe1086445a7258a4bc81e6e49dcd24c8d0dd9a1e57395b603f51"

[[package]]
name = "bitflags"
version = "2.13.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3ded4057c258ba199e2d26386d3af3780957ecaee6c4ef4041c6b4b8b97c0b06"
dependencies = [
 "serde_core",
]

[[package]]
name = "blake3"
version = "1.8.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6d9e454fc11f76977dc803893aff6304ed33d6a26efae8696573bea74baa27ae"
dependencies = [
 "arrayvec",
 "cc",
 "cfg-if",
 "constant_time_eq",
 "cpufeatures 0.3.1",
]

[[package]]
name = "block-buffer"
version = "0.10.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71"
dependencies = [
 "generic-array",
]

[[package]]
name = "block2"
version = "0.6.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cdeb9d870516001442e364c5220d3574d2da8dc765554b4a617230d33fa58ef5"
dependencies = [
 "objc2",
]

[[package]]
name = "bumpalo"
version = "3.20.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649"

[[package]]
name = "bytemuck"
version = "1.25.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "95832e849adfb21180ccb6826a99da14e5d266ae5c2e668e1602cf234f153797"
dependencies = [
 "bytemuck_derive",
]

[[package]]
name = "bytemuck_derive"
version = "1.12.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6a1f896587b6f2c069c73d2f0913e2d590c3990285cd2f0b6aa02b786b4c679c"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "bytes"
version = "1.12.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fc652a48c352aef3ea3aed32080501cf3ef6ed5da78602a020c991775b0aff04"

[[package]]
name = "caseless"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8b6fd507454086c8edfd769ca6ada439193cdb209c7681712ef6275cccbfe5d8"
dependencies = [
 "unicode-normalization",
]

[[package]]
name = "cc"
version = "1.11.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "375989decec1a0fa424442bed520fa12ede43a48475ce8c7417d2c69b32b5f6b"
dependencies = [
 "find-msvc-tools",
 "jobserver",
 "libc",
 "shlex",
]

[[package]]
name = "cfg-if"
version = "1.0.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600"

[[package]]
name = "cfg_aliases"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f079e83a288787bcd14a6aea84cee5c87a67c5a3e660c30f557a3d24761b3527"

[[package]]
name = "chacha20"
version = "0.10.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "65c35e4b699c7e15ccbe7ee35c005e4fc0a278d22238a2857e6ce2dadeda1b06"
dependencies = [
 "cfg-if",
 "cpufeatures 0.3.1",
 "rand_core",
]

[[package]]
name = "ciborium"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "42e69ffd6f0917f5c029256a24d0161db17cea3997d185db0d35926308770f0e"
dependencies = [
 "ciborium-io",
 "ciborium-ll",
 "serde",
]

[[package]]
name = "ciborium-io"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "05afea1e0a06c9be33d539b876f1ce3692f4afea2cb41f740e7743225ed1c757"

[[package]]
name = "ciborium-ll"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "57663b653d948a338bfb3eeba9bb2fd5fcfaecb9e199e87e1eda4d9e8b240fd9"
dependencies = [
 "ciborium-io",
 "half",
]

[[package]]
name = "codespan-reporting"
version = "0.13.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "af491d569909a7e4dee0ad7db7f5341fef5c614d5b8ec8cf765732aba3cff681"
dependencies = [
 "serde",
 "termcolor",
 "unicode-width",
]

[[package]]
name = "comrak"
version = "0.39.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2fefab951771fc3beeed0773ce66a4f7b706273fc6c4c95b08dd1615744abcf5"
dependencies = [
 "caseless",
 "entities",
 "memchr",
 "slug",
 "typed-arena",
 "unicode_categories",
]

[[package]]
name = "concurrent-queue"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4ca0197aee26d1ae37445ee532fefce43251d24cc7c166799f4d46817f1d3973"
dependencies = [
 "crossbeam-utils",
 "portable-atomic",
]

[[package]]
name = "constant_time_eq"
version = "0.4.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3d52eff69cd5e647efe296129160853a42795992097e8af39800e1060caeea9b"

[[package]]
name = "constcat"
version = "0.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "136d3e02915a2cea4d74caa8681e2d44b1c3254bdbf17d11d41d587ff858832c"

[[package]]
name = "convert_case"
version = "0.8.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "baaaa0ecca5b51987b9423ccdc971514dd8b0bb7b4060b983d3664dad3f1f89f"
dependencies = [
 "unicode-segmentation",
]

[[package]]
name = "convert_case"
version = "0.10.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "633458d4ef8c78b72454de2d54fd6ab2e60f9e02be22f3c6104cdc8a4e0fceb9"
dependencies = [
 "unicode-segmentation",
]

[[package]]
name = "cpufeatures"
version = "0.2.17"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "59ed5838eebb26a2bb2e58f6d5b5316989ae9d08bab10e0e6d103e656d1b0280"
dependencies = [
 "libc",
]

[[package]]
name = "cpufeatures"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5ca28b0ae3115b884660db4118d803791fd6756b6e88f39c0f3f7859060d7566"
dependencies = [
 "libc",
]

[[package]]
name = "critical-section"
version = "1.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "790eea4361631c5e7d22598ecd5723ff611904e3344ce8720784c93e3d83d40b"

[[package]]
name = "crossbeam-utils"
version = "0.8.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a31eee39dddec8330830986fcd7625edb5a24ec90ea038215273bbc3adb08ac6"

[[package]]
name = "crunchy"
version = "0.2.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "460fbee9c2c2f33933d720630a6a0bac33ba7053db5344fac858d4b8952d77d5"

[[package]]
name = "crypto-common"
version = "0.1.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "78c8292055d1c1df0cce5d180393dc8cce0abec0a7102adb6c7b1eef6016d60a"
dependencies = [
 "generic-array",
 "typenum",
]

[[package]]
name = "cubecl"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2029987fbb8d073eabc5ca09887846c2db6928fa81f90afc6b883ce819425884"
dependencies = [
 "cubecl-core",
 "cubecl-cpu",
 "cubecl-cuda",
 "cubecl-environment",
 "cubecl-hip",
 "cubecl-ir",
 "cubecl-runtime",
 "cubecl-std",
 "cubecl-wgpu",
 "half",
]

[[package]]
name = "cubecl-common"
version = "0.11.0-pre.2"
dependencies = [
 "bytemuck",
 "cfg_aliases",
 "cubecl-environment",
 "derive-new",
 "derive_more",
 "float4",
 "float8",
 "half",
 "hashbrown 0.17.1",
 "log",
 "num-traits",
 "portable-atomic",
 "portable-atomic-util",
 "serde",
 "spin",
 "tynm",
 "xxhash-rust",
]

[[package]]
name = "cubecl-core"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6d6fd5fb5fbf719e1a86df673dddae9e5183b08f0744308b2558fcc0632f5fe9"
dependencies = [
 "bitflags",
 "bytemuck",
 "cubecl-common",
 "cubecl-environment",
 "cubecl-ir",
 "cubecl-macros",
 "cubecl-runtime",
 "cubecl-zspace",
 "derive-new",
 "derive_more",
 "enumset",
 "float-ord",
 "half",
 "hashbrown 0.17.1",
 "itertools",
 "log",
 "num-integer",
 "num-traits",
 "paste",
 "serde",
 "serde_json",
 "variadics_please",
]

[[package]]
name = "cubecl-cpp"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1f43128c27f790ff22012679945c2b8972296979c52e5c323b54be830069e6e9"
dependencies = [
 "bytemuck",
 "cubecl-common",
 "cubecl-core",
 "cubecl-environment",
 "cubecl-opt",
 "cubecl-runtime",
 "derive-new",
 "half",
 "itertools",
 "log",
]

[[package]]
name = "cubecl-cpu"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8274cac78ff1528b075eb0240b576690c419e13c4621dc1c596ea0a6fce809ca"
dependencies = [
 "bytemuck",
 "crossbeam-utils",
 "cubecl-common",
 "cubecl-core",
 "cubecl-environment",
 "cubecl-opt",
 "cubecl-runtime",
 "cubecl-std",
 "derive-new",
 "half",
 "libc",
 "log",
 "paste",
 "serde",
 "smallvec",
 "spin",
 "sysinfo",
 "tracel-llvm",
 "tracel-llvm-bundler",
 "winapi",
]

[[package]]
name = "cubecl-cuda"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3e51bbfb4834e79972d7248bb8f69a747e848415f6e06e01aa8529cc8a29ad69"
dependencies = [
 "bytemuck",
 "cubecl-common",
 "cubecl-core",
 "cubecl-cpp",
 "cubecl-environment",
 "cubecl-runtime",
 "cudarc",
 "derive-new",
 "half",
 "log",
 "serde",
]

[[package]]
name = "cubecl-environment"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "61c7e23b2d90817886a9dc7c0a4527343def447ddbba1bae39f4f9f12c367385"
dependencies = [
 "async-channel",
 "backtrace",
 "bytemuck",
 "cfg-if",
 "cfg_aliases",
 "ciborium",
 "embassy-futures",
 "embassy-time",
 "etcetera",
 "foldhash 0.2.0",
 "futures-lite",
 "hashbrown 0.17.1",
 "log",
 "oneshot",
 "parking_lot",
 "portable-atomic",
 "portable-atomic-util",
 "rand",
 "serde",
 "serde_bytes",
 "serde_json",
 "spin",
 "toml",
 "wasm-bindgen-futures",
 "web-time",
]

[[package]]
name = "cubecl-hip"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7eaad6c93c5c48aa075457f03d8eeeca6f3ccb01f19a7da4940c664d48a14248"
dependencies = [
 "bytemuck",
 "cubecl-common",
 "cubecl-core",
 "cubecl-cpp",
 "cubecl-environment",
 "cubecl-hip-sys",
 "cubecl-runtime",
 "derive-new",
 "half",
 "log",
 "paste",
 "serde",
]

[[package]]
name = "cubecl-hip-sys"
version = "7.14.6085001"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3ba92853ebdf935ce68fdf96e4631a121ab72ce078201371ce7ace26603597f6"
dependencies = [
 "libc",
 "libloading 0.8.9",
 "regex",
]

[[package]]
name = "cubecl-ir"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "17ca2b43b9cfb6c4013592fa023a9f9158e344074d9dbba040b45461cc0842bc"
dependencies = [
 "bumpalo",
 "cubecl-common",
 "cubecl-environment",
 "cubecl-macros-internal",
 "derive-new",
 "derive_more",
 "enumset",
 "float-ord",
 "fnv",
 "foldhash 0.2.0",
 "half",
 "hashbrown 0.17.1",
 "internment",
 "itertools",
 "num-traits",
 "portable-atomic",
 "serde",
 "variadics_please",
]

[[package]]
name = "cubecl-macros"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f6b8fa7c11f7e311af88fc12883eedef8e0350541121fe8cc634db335c6e49ce"
dependencies = [
 "cubecl-common",
 "darling 0.23.0",
 "derive-new",
 "derive_more",
 "ident_case",
 "inflections",
 "prettyplease",
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "cubecl-macros-internal"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7082876a49f18dce60f20c34c42f6368d7753ea570238f2a4924fa0701fbfa82"
dependencies = [
 "darling 0.23.0",
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "cubecl-opt"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d0904a499f3ccbbfb24c2c8668d516f351c3e50c9178068be87d92bb3b8e1478"
dependencies = [
 "cubecl-common",
 "cubecl-core",
 "cubecl-environment",
 "cubecl-ir",
 "float-ord",
 "hashbrown 0.17.1",
 "log",
 "num",
 "petgraph",
 "smallvec",
 "stable-vec",
 "type-map",
]

[[package]]
name = "cubecl-runtime"
version = "0.11.0-pre.2"
dependencies = [
 "ahash",
 "bytemuck",
 "cfg-if",
 "cfg_aliases",
 "cubecl-common",
 "cubecl-environment",
 "cubecl-ir",
 "cubecl-zspace",
 "derive-new",
 "derive_more",
 "enumset",
 "futures-util",
 "hashbrown 0.17.1",
 "itertools",
 "log",
 "md5",
 "serde",
 "serde_json",
 "thiserror",
 "toml",
]

[[package]]
name = "cubecl-std"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bd029b15a150c3326b71a436f20b81973a2898ae9406a349c726918efdacf455"
dependencies = [
 "cubecl-common",
 "cubecl-core",
 "cubecl-environment",
 "cubecl-runtime",
 "half",
 "num-traits",
 "paste",
 "serde",
 "spin",
 "variadics_please",
]

[[package]]
name = "cubecl-wgpu"
version = "0.11.0-pre.2"
dependencies = [
 "blake3",
 "bytemuck",
 "cfg-if",
 "cfg_aliases",
 "cubecl-common",
 "cubecl-core",
 "cubecl-cpp",
 "cubecl-environment",
 "cubecl-ir",
 "cubecl-runtime",
 "derive-new",
 "derive_more",
 "half",
 "hashbrown 0.17.1",
 "itertools",
 "log",
 "sanitize-filename",
 "wasm-bindgen-futures",
 "wgpu",
]

[[package]]
name = "cubecl-zspace"
version = "0.11.0-pre.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f0513aa2d1997d1049305c5dae7f95bdcf808fc06ced02111075b46a67233950"
dependencies = [
 "derive-new",
 "serde",
 "smallvec",
]

[[package]]
name = "cudarc"
version = "0.19.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7359bf1de037ddada72729c3acec9b040f3468400c6e9e740b01ba0cd2e89562"
dependencies = [
 "libloading 0.9.0",
]

[[package]]
name = "darling"
version = "0.21.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9cdf337090841a411e2a7f3deb9187445851f91b309c0c0a29e05f74a00a48c0"
dependencies = [
 "darling_core 0.21.3",
 "darling_macro 0.21.3",
]

[[package]]
name = "darling"
version = "0.23.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "25ae13da2f202d56bd7f91c25fba009e7717a1e4a1cc98a76d844b65ae912e9d"
dependencies = [
 "darling_core 0.23.0",
 "darling_macro 0.23.0",
]

[[package]]
name = "darling_core"
version = "0.21.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1247195ecd7e3c85f83c8d2a366e4210d588e802133e1e355180a9870b517ea4"
dependencies = [
 "fnv",
 "ident_case",
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "darling_core"
version = "0.23.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9865a50f7c335f53564bb694ef660825eb8610e0a53d3e11bf1b0d3df31e03b0"
dependencies = [
 "ident_case",
 "proc-macro2",
 "quote",
 "strsim",
 "syn 2.0.119",
]

[[package]]
name = "darling_macro"
version = "0.21.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d38308df82d1080de0afee5d069fa14b0326a88c14f15c5ccda35b4a6c414c81"
dependencies = [
 "darling_core 0.21.3",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "darling_macro"
version = "0.23.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ac3984ec7bd6cfa798e62b4a642426a5be0e68f9401cfc2a01e3fa9ea2fcdb8d"
dependencies = [
 "darling_core 0.23.0",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "derive-new"
version = "0.7.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2cdc8d50f426189eef89dac62fabfa0abb27d5cc008f25bf4156a0203325becc"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "derive_more"
version = "2.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d751e9e49156b02b44f9c1815bcb94b984cdcc4396ecc32521c739452808b134"
dependencies = [
 "derive_more-impl",
]

[[package]]
name = "derive_more-impl"
version = "2.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "799a97264921d8623a957f6c3b9011f3b5492f557bbb7a5a19b7fa6d06ba8dcb"
dependencies = [
 "convert_case 0.10.0",
 "proc-macro2",
 "quote",
 "rustc_version",
 "syn 2.0.119",
 "unicode-xid",
]

[[package]]
name = "deunicode"
version = "1.6.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abd57806937c9cc163efc8ea3910e00a62e2aeb0b8119f1793a978088f8f6b04"

[[package]]
name = "digest"
version = "0.10.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9ed9a281f7bc9b7576e61468ba615a66a5c8cfdff42420a70aa82701a3b1e292"
dependencies = [
 "block-buffer",
 "crypto-common",
]

[[package]]
name = "dirs"
version = "6.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c3e8aa94d75141228480295a7d0e7feb620b1a5ad9f12bc40be62411e38cce4e"
dependencies = [
 "dirs-sys",
]

[[package]]
name = "dirs-sys"
version = "0.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e01a3366d27ee9890022452ee61b2b63a67e6f13f58900b651ff5665f0bb1fab"
dependencies = [
 "libc",
 "option-ext",
 "redox_users",
 "windows-sys 0.61.2",
]

[[package]]
name = "dispatch2"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1e0e367e4e7da84520dedcac1901e4da967309406d1e51017ae1abfb97adbd38"
dependencies = [
 "bitflags",
 "objc2",
]

[[package]]
name = "displaydoc"
version = "0.2.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c6232dd377dcc64799954cbd3a9bb882e9cdc1308ccd87b1c098f1fb2eaf82a8"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "dlib"
version = "0.5.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ab8ecd87370524b461f8557c119c405552c396ed91fc0a8eec68679eab26f94a"
dependencies = [
 "libloading 0.8.9",
]

[[package]]
name = "document-features"
version = "0.2.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d4b8a88685455ed29a21542a33abd9cb6510b6b129abadabdcef0f4c55bc8f61"
dependencies = [
 "litrs",
]

[[package]]
name = "either"
version = "1.19.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0e9c71c2167ca323c882b99918929403426e2373ea17242ff5653e0d5e1058be"

[[package]]
name = "embassy-futures"
version = "0.1.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "dc2d050bdc5c21e0862a89256ed8029ae6c290a93aecefc73084b3002cdebb01"

[[package]]
name = "embassy-time"
version = "0.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "592b0c143ec626e821d4d90da51a2bd91d559d6c442b7c74a47d368c9e23d97a"
dependencies = [
 "cfg-if",
 "critical-section",
 "document-features",
 "embassy-time-driver",
 "embedded-hal 0.2.7",
 "embedded-hal 1.0.0",
 "embedded-hal-async",
 "futures-core",
]

[[package]]
name = "embassy-time-driver"
version = "0.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6ee71af1b3a0deaa53eaf2d39252f83504c853646e472400b763060389b9fcc9"
dependencies = [
 "document-features",
]

[[package]]
name = "embedded-hal"
version = "0.2.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "35949884794ad573cf46071e41c9b60efb0cb311e3ca01f7af807af1debc66ff"
dependencies = [
 "nb 0.1.3",
 "void",
]

[[package]]
name = "embedded-hal"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "361a90feb7004eca4019fb28352a9465666b24f840f5c3cddf0ff13920590b89"

[[package]]
name = "embedded-hal-async"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0c4c685bbef7fe13c3c6dd4da26841ed3980ef33e841cddfa15ce8a8fb3f1884"
dependencies = [
 "embedded-hal 1.0.0",
]

[[package]]
name = "entities"
version = "1.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b5320ae4c3782150d900b79807611a59a99fc9a1d61d686faafc24b93fc8d7ca"

[[package]]
name = "enumset"
version = "1.1.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2e6bd2d7c76d078cd7a460fa169f778c42c026fe16d90e37d7a45d37181bdce5"
dependencies = [
 "enumset_derive",
 "serde",
]

[[package]]
name = "enumset_derive"
version = "0.15.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f60587c2044cabd91c4d4c73e3912e9f1ea7d76905f31c7eec51869305dae044"
dependencies = [
 "darling 0.21.3",
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "equivalent"
version = "1.0.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "00d174d5400e5e8fd687ad1049e2f578285fa914201b1af7e8b112a4546bd826"

[[package]]
name = "errno"
version = "0.3.14"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb"
dependencies = [
 "libc",
 "windows-sys 0.61.2",
]

[[package]]
name = "etcetera"
version = "0.11.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "de48cc4d1c1d97a20fd819def54b890cadde72ed3ad0c614822a0a433361be96"
dependencies = [
 "cfg-if",
 "windows-sys 0.61.2",
]

[[package]]
name = "event-listener"
version = "5.4.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5a23add41df1562121a9393cb065eab5146a1242410f23a644851e90cfd669d2"
dependencies = [
 "parking",
 "pin-project-lite",
 "portable-atomic",
 "portable-atomic-util",
]

[[package]]
name = "event-listener-strategy"
version = "0.5.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8be9f3dfaaffdae2972880079a491a1a8bb7cbed0b8dd7a347f668b4150a3b93"
dependencies = [
 "event-listener",
 "pin-project-lite",
]

[[package]]
name = "fastrand"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "da7c62ceae207dd37ea5b845da6a0696c799f85e97da1ab5b7910be3c1c80223"

[[package]]
name = "filetime"
version = "0.2.29"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5c287a33c7f0a620c38e641e7f60827713987b3c0f26e8ddc9462cc69cf75759"
dependencies = [
 "cfg-if",
 "libc",
]

[[package]]
name = "find-msvc-tools"
version = "0.1.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1c66915cc5391097de7a2b0b972e8505aa33c0df401fd3a6811dbdb32881ba48"

[[package]]
name = "fixedbitset"
version = "0.5.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1d674e81391d1e1ab681a28d99df07927c6d4aa5b027d7da16ba32d1d21ecd99"

[[package]]
name = "float-ord"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8ce81f49ae8a0482e4c55ea62ebbd7e5a686af544c00b9d090bba3ff9be97b3d"

[[package]]
name = "float4"
version = "0.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9a5404bf31d22893d61cf24d4dda149d8e6b2ff07601c3cb3be651031f61a4ed"

[[package]]
name = "float8"
version = "0.7.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c2d1f04709a8ac06e8e8042875a3c466cc4832d3c1a18dbcb9dba3c6e83046bc"
dependencies = [
 "half",
]

[[package]]
name = "fnv"
version = "1.0.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3f9eec918d3f24069decb9af1554cad7c880e2da24a9afd88aca000531ab82c1"

[[package]]
name = "foldhash"
version = "0.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d9c4f5dac5e15c24eb999c26181a6ca40b39fe946cbe4c263c7209467bc83af2"

[[package]]
name = "foldhash"
version = "0.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "77ce24cb58228fbb8aa041425bb1050850ac19177686ea6e0f41a70416f56fdb"

[[package]]
name = "form_urlencoded"
version = "1.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cb4cb245038516f5f85277875cdaa4f7d2c9a0fa0468de06ed190163b1581fcf"
dependencies = [
 "percent-encoding",
]

[[package]]
name = "futures-channel"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b1f9e3d69d39e4862ffed03ed071a76f9a13ba1d9109d355b0f0aa6b15e393c4"
dependencies = [
 "futures-core",
 "futures-sink",
]

[[package]]
name = "futures-core"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "92d699e522242e69e3003b94ecc1f960f3a5e015aa7c5d7486e65ad01dd94f5e"

[[package]]
name = "futures-io"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "53c0fa8157de1303bfffdaa1cc2a673bfffb60102f76b0ef4441659124373fed"

[[package]]
name = "futures-lite"
version = "2.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f78e10609fe0e0b3f4157ffab1876319b5b0db102a2c60dc4626306dc46b44ad"
dependencies = [
 "fastrand",
 "futures-core",
 "futures-io",
 "parking",
 "pin-project-lite",
]

[[package]]
name = "futures-sink"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1944426bf7d03f1d14f708785e4b33efd750b36d48a157b836b3efc15ede8e1d"

[[package]]
name = "futures-task"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cd417de3d1d015fc3bfd2b1ea46dfc7bab72ef86f1cc7cc9c78e728b34a6d1fd"

[[package]]
name = "futures-util"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0d50a92467f8ba5dd6e3ee5d4bd04d73ab2e4e1c44474a0674821dfce14b79bc"
dependencies = [
 "futures-core",
 "futures-io",
 "futures-sink",
 "futures-task",
 "memchr",
 "pin-project-lite",
 "slab",
]

[[package]]
name = "generic-array"
version = "0.14.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "85649ca51fd72272d7821adaf274ad91c288277713d9c18820d8499a7ff69e9a"
dependencies = [
 "typenum",
 "version_check",
]

[[package]]
name = "getrandom"
version = "0.2.17"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ff2abc00be7fca6ebc474524697ae276ad847ad0a6b3faa4bcb027e9a4614ad0"
dependencies = [
 "cfg-if",
 "js-sys",
 "libc",
 "wasi",
 "wasm-bindgen",
]

[[package]]
name = "getrandom"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "300e883d756b2e4ec94e02791f39b04b522276138852cfc41d9fb7e904106099"
dependencies = [
 "cfg-if",
 "js-sys",
 "libc",
 "r-efi",
 "rand_core",
 "wasm-bindgen",
]

[[package]]
name = "gimli"
version = "0.34.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1033caf0b349c518623b5396bfb2cf0bddf44f0306d543a250e5743297aafd10"

[[package]]
name = "gl_generator"
version = "0.14.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1a95dfc23a2b4a9a2f5ab41d194f8bfda3cabec42af4e39f08c339eb2a0c124d"
dependencies = [
 "khronos_api",
 "log",
 "xml-rs",
]

[[package]]
name = "glow"
version = "0.17.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "29038e1c483364cc6bb3cf78feee1816002e127c331a1eec55a4d202b9e1adb5"
dependencies = [
 "js-sys",
 "slotmap",
 "wasm-bindgen",
 "web-sys",
]

[[package]]
name = "glutin_wgl_sys"
version = "0.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2c4ee00b289aba7a9e5306d57c2d05499b2e5dc427f84ac708bd2c090212cf3e"
dependencies = [
 "gl_generator",
]

[[package]]
name = "gpu-allocator"
version = "0.28.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "51255ea7cfaadb6c5f1528d43e92a82acb2b96c43365989a28b2d44ee38f8795"
dependencies = [
 "ash",
 "hashbrown 0.16.1",
 "log",
 "presser",
 "thiserror",
 "windows",
]

[[package]]
name = "h2"
version = "0.4.20"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7d29020232d6aa3fb1daca64c1127cf662cf97f254ae16c18c05b8ab635fc118"
dependencies = [
 "atomic-waker",
 "bytes",
 "fnv",
 "futures-core",
 "futures-sink",
 "http",
 "indexmap",
 "slab",
 "tokio",
 "tokio-util",
 "tracing",
]

[[package]]
name = "half"
version = "2.7.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6ea2d84b969582b4b1864a92dc5d27cd2b77b622a8d79306834f1be5ba20d84b"
dependencies = [
 "bytemuck",
 "cfg-if",
 "crunchy",
 "num-traits",
 "serde",
 "zerocopy",
]

[[package]]
name = "hashbrown"
version = "0.15.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9229cfe53dfd69f0609a49f65461bd93001ea1ef889cd5529dd176593f5338a1"
dependencies = [
 "allocator-api2",
 "equivalent",
 "foldhash 0.1.5",
]

[[package]]
name = "hashbrown"
version = "0.16.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "841d1cc9bed7f9236f321df977030373f4a4163ae1a7dbfe1a51a2c1a51d9100"
dependencies = [
 "allocator-api2",
 "equivalent",
 "foldhash 0.2.0",
]

[[package]]
name = "hashbrown"
version = "0.17.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ed5909b6e89a2db4456e54cd5f673791d7eca6732202bbf2a9cc504fe2f9b84a"
dependencies = [
 "allocator-api2",
 "equivalent",
 "foldhash 0.2.0",
 "serde",
 "serde_core",
]

[[package]]
name = "http"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "918d3568bebf352712bc2ef3d46a8bcf1a75b373be6539de198e9105cbbf9ce0"
dependencies = [
 "bytes",
 "itoa",
]

[[package]]
name = "http-body"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ca2a8f2913ee65f60facd6a5905613afaa448497a0230cc41ce022d93290bc2c"
dependencies = [
 "bytes",
 "http",
]

[[package]]
name = "http-body-util"
version = "0.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "23169fe34a5fbcdd3f3862e78fb9b6fccd5f02a6dc6f732547005d45631ce71c"
dependencies = [
 "bytes",
 "futures-core",
 "http",
 "http-body",
 "pin-project-lite",
]

[[package]]
name = "httparse"
version = "1.10.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6dbf3de79e51f3d586ab4cb9d5c3e2c14aa28ed23d180cf89b4df0454a69cc87"

[[package]]
name = "hyper"
version = "1.12.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2c3e324da4c95177d6291d4c8730197c0d1822f8a9766814a4a44fa5ab797c9c"
dependencies = [
 "atomic-waker",
 "bytes",
 "futures-channel",
 "futures-core",
 "h2",
 "http",
 "http-body",
 "httparse",
 "itoa",
 "pin-project-lite",
 "smallvec",
 "tokio",
 "want",
]

[[package]]
name = "hyper-rustls"
version = "0.27.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "dfa8e654703247911e29c23fbeaa261834bd9bb74efba2f9acddc37bfb127f53"
dependencies = [
 "http",
 "hyper",
 "hyper-util",
 "rustls",
 "tokio",
 "tokio-rustls",
 "tower-service",
 "webpki-roots",
]

[[package]]
name = "hyper-util"
version = "0.1.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ddc03d96684f9226b8a787cdb71488417b53ab5ea8fdb1dac946cb9431cc8bff"
dependencies = [
 "base64 0.23.1",
 "bytes",
 "futures-channel",
 "futures-util",
 "http",
 "http-body",
 "httparse",
 "hyper",
 "ipnet",
 "libc",
 "percent-encoding",
 "pin-project-lite",
 "socket2",
 "tokio",
 "tower-service",
 "tracing",
]

[[package]]
name = "icu_collections"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fe090a42a0d3140fed8c63263ed1fecc02422cb2f4bfe55cd8de2de1b8e105dd"
dependencies = [
 "displaydoc",
 "potential_utf",
 "utf8_iter",
 "yoke",
 "zerofrom",
 "zerovec",
]

[[package]]
name = "icu_locale_core"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "25d29b57c17c6528d0dc4c2141cfa2d08200465acb537a3909c8b9b8a4a984c0"
dependencies = [
 "displaydoc",
 "litemap",
 "tinystr",
 "writeable",
 "zerovec",
]

[[package]]
name = "icu_normalizer"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "404a3b8c486f205c87fe7f23ab6894d84a4ad76d6e4671ae1981ba7def87c599"
dependencies = [
 "icu_collections",
 "icu_normalizer_data",
 "icu_properties",
 "icu_provider",
 "smallvec",
 "zerovec",
]

[[package]]
name = "icu_normalizer_data"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "20f0088bfb7441f7a37fa0c9e20cb1e9cba1cac6b9ce8cd2a6dc44294f8e8cbc"

[[package]]
name = "icu_properties"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2ea83d991c56158f7cf898bf86436c1dfe7c07ee82fe88951f2695c9f37a379f"
dependencies = [
 "displaydoc",
 "icu_collections",
 "icu_locale_core",
 "icu_properties_data",
 "icu_provider",
 "zerotrie",
 "zerovec",
]

[[package]]
name = "icu_properties_data"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b98f29af82d02b163fe50161388350e743bb3107cc8e8c2f7314cb505faddfcf"

[[package]]
name = "icu_provider"
version = "2.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "413099315636c588ee8f011190b329b971a09a8fae9d157e01dcae8379468514"
dependencies = [
 "displaydoc",
 "icu_locale_core",
 "writeable",
 "yoke",
 "zerofrom",
 "zerotrie",
 "zerovec",
]

[[package]]
name = "ident_case"
version = "1.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b9e0384b61958566e926dc50660321d12159025e767c18e043daf26b70104c39"

[[package]]
name = "idna"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3b0875f23caa03898994f6ddc501886a45c7d3d62d04d2d90788d47be1b1e4de"
dependencies = [
 "idna_adapter",
 "smallvec",
 "utf8_iter",
]

[[package]]
name = "idna_adapter"
version = "1.2.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cb68373c0d6620ef8105e855e7745e18b0d00d3bdb07fb532e434244cdb9a714"
dependencies = [
 "icu_normalizer",
 "icu_properties",
]

[[package]]
name = "indexmap"
version = "2.14.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855"
dependencies = [
 "equivalent",
 "hashbrown 0.17.1",
]

[[package]]
name = "inflections"
version = "1.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a257582fdcde896fd96463bf2d40eefea0580021c0712a0e2b028b60b47a837a"

[[package]]
name = "internment"
version = "0.8.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "636d4b0f6a39fd684effe2a73f5310df16a3fa7954c26d36833e98f44d1977a2"
dependencies = [
 "hashbrown 0.15.5",
 "serde",
]

[[package]]
name = "ipnet"
version = "2.12.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "791930b43c0d5973160d90a8f3894509f2b273430f5c5c73b668636d0287c5c0"

[[package]]
name = "itertools"
version = "0.14.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2b192c782037fadd9cfa75548310488aabdbf3d2da73885b31bd0abd03351285"
dependencies = [
 "either",
]

[[package]]
name = "itoa"
version = "1.0.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682"

[[package]]
name = "jni-sys"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41a652e1f9b6e0275df1f15b32661cf0d4b78d4d87ddec5e0c3c20f097433258"
dependencies = [
 "jni-sys 0.4.1",
]

[[package]]
name = "jni-sys"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c6377a88cb3910bee9b0fa88d4f42e1d2da8e79915598f65fb0c7ee14c878af2"
dependencies = [
 "jni-sys-macros",
]

[[package]]
name = "jni-sys-macros"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "38c0b942f458fe50cdac086d2f946512305e5631e720728f2a61aabcd47a6264"
dependencies = [
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "jobserver"
version = "0.1.35"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1c00acbd29eabad4a2392fa0e921c874934dbbf4194312ad20f04a0ed67a3cb3"
dependencies = [
 "getrandom 0.4.3",
 "libc",
]

[[package]]
name = "js-sys"
version = "0.3.107"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8dc09d2154098bbf4ef54eab6b117a3c24cbbffe94046b16ee6d6d4510584d3a"
dependencies = [
 "cfg-if",
 "futures-util",
 "wasm-bindgen",
]

[[package]]
name = "kernel_bench_g2_gpu"
version = "0.1.0"
dependencies = [
 "cubecl",
 "pollster",
]

[[package]]
name = "khronos-egl"
version = "6.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6aae1df220ece3c0ada96b8153459b67eebe9ae9212258bb0134ae60416fdf76"
dependencies = [
 "libc",
 "libloading 0.8.9",
 "pkg-config",
]

[[package]]
name = "khronos_api"
version = "3.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e2db585e1d738fc771bf08a151420d3ed193d9d895a36df7f6f8a9456b911ddc"

[[package]]
name = "libc"
version = "0.2.191"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a"

[[package]]
name = "libloading"
version = "0.8.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d7c4b02199fee7c5d21a5ae7d8cfa79a6ef5bb2fc834d6e9058e89c825efdc55"
dependencies = [
 "cfg-if",
 "windows-link",
]

[[package]]
name = "libloading"
version = "0.9.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "754ca22de805bb5744484a5b151a9e1a8e837d5dc232c2d7d8c2e3492edc8b60"
dependencies = [
 "cfg-if",
 "windows-link",
]

[[package]]
name = "liblzma"
version = "0.4.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "aa980ea6744074a5b5e0ac6b164aca94fc970121869b3e78ae12a2b0097dbea7"
dependencies = [
 "liblzma-sys",
]

[[package]]
name = "liblzma-sys"
version = "0.4.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7c7e3581f367a78d7b7e7ae948d023310556f0cfc13156c2e4e00e25616492b9"
dependencies = [
 "cc",
 "libc",
 "pkg-config",
]

[[package]]
name = "libm"
version = "0.2.16"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981"

[[package]]
name = "libredox"
version = "0.1.25"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "61ff90caf6077a803a240f62fdbe88645a890bbca49ef8174c3cb0404362171d"
dependencies = [
 "libc",
]

[[package]]
name = "linux-raw-sys"
version = "0.12.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53"

[[package]]
name = "litemap"
version = "0.8.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "47d9d19d1d6efa0109d2f65ff4c85cddd50bd572e5a00127ab10987290bcefae"

[[package]]
name = "litrs"
version = "1.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4744e383959f0db86ede514b809b1c53251889093803c05267acc7d4e7030d70"

[[package]]
name = "lock_api"
version = "0.4.14"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "224399e74b87b5f3557511d98dff8b14089b3dadafcab6bb93eab67d3aace965"
dependencies = [
 "scopeguard",
]

[[package]]
name = "log"
version = "0.4.35"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5d15f86d12da44cd8d4ec3c73984537eb7f4cadcb9776a575a0a8a05ca11005c"

[[package]]
name = "lru-slab"
version = "0.1.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4050469837a6ff301cd14c1f8f24f88549e6d548f24f64e2148eb0f72cebc51f"

[[package]]
name = "md5"
version = "0.8.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7ebb8d8732c6a6df3d8f032a82911cfc747e00efb95cc46e8d0acd5b5b88570c"

[[package]]
name = "memchr"
version = "2.8.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98"

[[package]]
name = "miniz_oxide"
version = "0.9.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b63fbc4a50860e98e7b2aa7804ded1db5cbc3aff9193adaff57a6931bf7c4b4c"
dependencies = [
 "adler2",
]

[[package]]
name = "mio"
version = "1.2.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1788edb87fdc09c7e26304471e2f5be8cdefb1b6930d6e3985fc02ff53bf86ee"
dependencies = [
 "libc",
 "wasi",
 "windows-sys 0.61.2",
]

[[package]]
name = "mutants"
version = "0.0.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bc0287524726960e07b119cebd01678f852f147742ae0d925e6a520dca956126"

[[package]]
name = "naga"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a616d2fb8c89516ac2723a581f69d6c18576046bed761bd6b305e5618e6ae130"
dependencies = [
 "arrayvec",
 "bit-set",
 "bitflags",
 "cfg-if",
 "cfg_aliases",
 "codespan-reporting",
 "half",
 "hashbrown 0.17.1",
 "indexmap",
 "libm",
 "log",
 "naga-types",
 "num-traits",
 "once_cell",
 "rustc-hash 1.1.0",
 "spirv",
 "thiserror",
 "unicode-ident",
]

[[package]]
name = "naga-types"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "590afbf58a6f4f62873cd5cff4468061844bafa1cdf399cc954537c22d768d49"
dependencies = [
 "hashbrown 0.17.1",
 "indexmap",
 "rustc-hash 1.1.0",
 "thiserror",
]

[[package]]
name = "nb"
version = "0.1.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "801d31da0513b6ec5214e9bf433a77966320625a37860f910be265be6e18d06f"
dependencies = [
 "nb 1.1.0",
]

[[package]]
name = "nb"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8d5439c4ad607c3c23abf66de8c8bf57ba8adcd1f129e699851a6e43935d339d"

[[package]]
name = "ndk-sys"
version = "0.6.0+11769913"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ee6cda3051665f1fb8d9e08fc35c96d5a244fb1be711a03b71118828afc9a873"
dependencies = [
 "jni-sys 0.3.1",
]

[[package]]
name = "nom"
version = "8.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "df9761775871bdef83bee530e60050f7e54b1105350d6884eb0fb4f46c2f9405"
dependencies = [
 "memchr",
]

[[package]]
name = "ntapi"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c3b335231dfd352ffb0f8017f3b6027a4917f7df785ea2143d8af2adc66980ae"
dependencies = [
 "winapi",
]

[[package]]
name = "num"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "35bd024e8b2ff75562e5f34e7f4905839deb4b22955ef5e73d2fea1b9813cb23"
dependencies = [
 "num-bigint",
 "num-complex",
 "num-integer",
 "num-iter",
 "num-rational",
 "num-traits",
]

[[package]]
name = "num-bigint"
version = "0.4.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c89e69e7e0f03bea5ef08013795c25018e101932225a656383bd384495ecc367"
dependencies = [
 "num-integer",
 "num-traits",
]

[[package]]
name = "num-complex"
version = "0.4.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "73f88a1307638156682bada9d7604135552957b7818057dcef22705b4d509495"
dependencies = [
 "num-traits",
]

[[package]]
name = "num-integer"
version = "0.1.47"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7ce2d95d4b3734dc35aa2f45e1aa22cd416814592a4f9d9205e11affd5b8e10b"
dependencies = [
 "num-traits",
]

[[package]]
name = "num-iter"
version = "0.1.46"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c92800bd69a1eac91786bcfe9da64a897eb72911b8dc3095decbd07429e8048b"
dependencies = [
 "num-integer",
 "num-traits",
]

[[package]]
name = "num-rational"
version = "0.4.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f83d14da390562dca69fc84082e73e548e1ad308d24accdedd2720017cb37824"
dependencies = [
 "num-bigint",
 "num-integer",
 "num-traits",
]

[[package]]
name = "num-traits"
version = "0.2.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "071dfc062690e90b734c0b2273ce72ad0ffa95f0c74596bc250dcfd960262841"
dependencies = [
 "autocfg",
 "libm",
]

[[package]]
name = "objc2"
version = "0.6.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "08849bbd4767dfae9457696856ae1c84fe4e0281bbe4a7abff2d0e06fb7981f8"
dependencies = [
 "objc2-encode",
]

[[package]]
name = "objc2-core-foundation"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2a180dd8642fa45cdb7dd721cd4c11b1cadd4929ce112ebd8b9f5803cc79d536"
dependencies = [
 "bitflags",
 "dispatch2",
 "objc2",
]

[[package]]
name = "objc2-core-graphics"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e022c9d066895efa1345f8e33e584b9f958da2fd4cd116792e15e07e4720a807"
dependencies = [
 "dispatch2",
 "objc2",
 "objc2-core-foundation",
 "objc2-io-surface",
]

[[package]]
name = "objc2-encode"
version = "4.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ef25abbcd74fb2609453eb695bd2f860d389e457f67dc17cafc8b8cbc89d0c33"

[[package]]
name = "objc2-foundation"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e3e0adef53c21f888deb4fa59fc59f7eb17404926ee8a6f59f5df0fd7f9f3272"
dependencies = [
 "bitflags",
 "objc2",
 "objc2-core-foundation",
]

[[package]]
name = "objc2-io-kit"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "33fafba39597d6dc1fb709123dfa8289d39406734be322956a69f0931c73bb15"
dependencies = [
 "libc",
 "objc2-core-foundation",
]

[[package]]
name = "objc2-io-surface"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "180788110936d59bab6bd83b6060ffdfffb3b922ba1396b312ae795e1de9d81d"
dependencies = [
 "bitflags",
 "objc2",
 "objc2-core-foundation",
]

[[package]]
name = "objc2-metal"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a0125f776a10d00af4152d74616409f0d4a2053a6f57fa5b7d6aa2854ac04794"
dependencies = [
 "bitflags",
 "block2",
 "objc2",
 "objc2-foundation",
]

[[package]]
name = "objc2-quartz-core"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "96c1358452b371bf9f104e21ec536d37a650eb10f7ee379fff67d2e08d537f1f"
dependencies = [
 "bitflags",
 "objc2",
 "objc2-core-foundation",
 "objc2-core-graphics",
 "objc2-foundation",
 "objc2-metal",
]

[[package]]
name = "object"
version = "0.40.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "dd229a0361b9d0d4396176e02d65897f487eebeab7caa6d443855ee152ca0b9c"
dependencies = [
 "memchr",
]

[[package]]
name = "once_cell"
version = "1.21.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9f7c3e4beb33f85d45ae3e3a1792185706c8e16d043238c593331cc7cd313b50"

[[package]]
name = "oneshot"
version = "0.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cfe21416a02c693fb9f980befcb230ecc70b0b3d1cc4abf88b9675c4c1457f0c"

[[package]]
name = "option-ext"
version = "0.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "04744f49eae99ab78e0d5c0b603ab218f515ea8cfe5a456d7629ad883a3b6e7d"

[[package]]
name = "ordered-float"
version = "5.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7f59006ce1e204c30956638535b74d317bca99477b9356dfaec54c37f6f600ab"
dependencies = [
 "num-traits",
]

[[package]]
name = "parking"
version = "2.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f38d5652c16fde515bb1ecef450ab0f6a219d619a7274976324d5e377f7dceba"

[[package]]
name = "parking_lot"
version = "0.12.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "93857453250e3077bd71ff98b6a65ea6621a19bb0f559a85248955ac12c45a1a"
dependencies = [
 "lock_api",
 "parking_lot_core",
]

[[package]]
name = "parking_lot_core"
version = "0.9.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2621685985a2ebf1c516881c026032ac7deafcda1a2c9b7850dc81e3dfcb64c1"
dependencies = [
 "cfg-if",
 "libc",
 "redox_syscall",
 "smallvec",
 "windows-link",
]

[[package]]
name = "paste"
version = "1.0.15"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "57c0d7b74b563b49d38dae00a0c37d4d6de9b432382b2892f0574ddcae73fd0a"

[[package]]
name = "percent-encoding"
version = "2.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9b4f627cb1b25917193a259e49bdad08f671f8d9708acfd5fe0a8c1455d87220"

[[package]]
name = "petgraph"
version = "0.8.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8701b58ea97060d5e5b155d383a69952a60943f0e6dfe30b04c287beb0b27455"
dependencies = [
 "fixedbitset",
 "hashbrown 0.15.5",
 "indexmap",
 "serde",
]

[[package]]
name = "pin-project-lite"
version = "0.2.17"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a89322df9ebe1c1578d689c92318e070967d1042b512afbe49518723f4e6d5cd"

[[package]]
name = "pkg-config"
version = "0.3.34"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f6b464fbc74e149a392436b17d523f769e057cb6877f6a5c4618bc6f11800548"

[[package]]
name = "pollster"
version = "0.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2f3a9f18d041e6d0e102a0a46750538147e5e8992d3b4873aaafee2520b00ce3"

[[package]]
name = "portable-atomic"
version = "1.15.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "05c8b63e8d9609db387f0324918f81d68fe27748f084ef092fb35954d0539a85"
dependencies = [
 "serde",
]

[[package]]
name = "portable-atomic-util"
version = "0.2.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "10ab3eb7f3becc3a1cbc4f2c6f20267996cfc1a6467a873763411b136a122715"
dependencies = [
 "portable-atomic",
]

[[package]]
name = "potential_utf"
version = "0.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d83eb9bc6d8e5cf568e7a1101d60ee05e81ed50ea106026f3d18deeb046d7661"
dependencies = [
 "zerovec",
]

[[package]]
name = "presser"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e8cf8e6a8aa66ce33f63993ffc4ea4271eb5b0530a9002db8455ea6050c77bfa"

[[package]]
name = "prettyplease"
version = "0.2.37"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "479ca8adacdd7ce8f1fb39ce9ecccbfe93a3f1344b3d0d97f20bc0196208f62b"
dependencies = [
 "proc-macro2",
 "syn 2.0.119",
]

[[package]]
name = "proc-macro2"
version = "1.0.107"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "profiling"
version = "1.0.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3d595e54a326bc53c1c197b32d295e14b169e3cfeaa8dc82b529f947fba6bcf5"

[[package]]
name = "quinn"
version = "0.11.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4051e23e9185c255a7e33ef59cdbca87a22d359052eecd22fc6b901fb37d9d11"
dependencies = [
 "bytes",
 "cfg_aliases",
 "pin-project-lite",
 "quinn-proto",
 "quinn-udp",
 "rustc-hash 2.1.3",
 "rustls",
 "socket2",
 "thiserror",
 "tokio",
 "tracing",
 "web-time",
]

[[package]]
name = "quinn-proto"
version = "0.11.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0e750cca55fe4f0439a15d0bb529da9651e79993e8e72c61a899a36d462befbe"
dependencies = [
 "bytes",
 "getrandom 0.4.3",
 "lru-slab",
 "rand",
 "rand_pcg",
 "ring",
 "rustc-hash 2.1.3",
 "rustls",
 "rustls-pki-types",
 "slab",
 "thiserror",
 "tinyvec",
 "tracing",
 "web-time",
]

[[package]]
name = "quinn-udp"
version = "0.5.16"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "af66907df18639dcf4db56ca65490cabc4b27a97dbadd96f2926cca73298f016"
dependencies = [
 "cfg_aliases",
 "libc",
 "once_cell",
 "socket2",
 "tracing",
 "windows-sys 0.61.2",
]

[[package]]
name = "quote"
version = "1.0.47"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "r-efi"
version = "6.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f8dcc9c7d52a811697d2151c701e0d08956f92b0e24136cf4cf27b57a6a0d9bf"

[[package]]
name = "rand"
version = "0.10.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "65c9fb96cbc91e3478eaae79a69fcd3f1ae4ad052e471fe6732fff548984b4af"
dependencies = [
 "chacha20",
 "getrandom 0.4.3",
 "rand_core",
]

[[package]]
name = "rand_core"
version = "0.10.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "63b8176103e19a2643978565ca18b50549f6101881c443590420e4dc998a3c69"

[[package]]
name = "rand_pcg"
version = "0.10.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "caa0f4137e1c0a72f4c651489402276c8e8e1cf081f3b0ba156d2cbeef09e86a"
dependencies = [
 "rand_core",
]

[[package]]
name = "range-alloc"
version = "0.1.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ca45419789ae5a7899559e9512e58ca889e41f04f1f2445e9f4b290ceccd1d08"

[[package]]
name = "raw-window-handle"
version = "0.6.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "20675572f6f24e9e76ef639bc5552774ed45f1c30e2951e1e99c59888861c539"

[[package]]
name = "raw-window-metal"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "40d213455a5f1dc59214213c7330e074ddf8114c9a42411eb890c767357ce135"
dependencies = [
 "objc2",
 "objc2-core-foundation",
 "objc2-foundation",
 "objc2-quartz-core",
]

[[package]]
name = "redox_syscall"
version = "0.5.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ed2bf2547551a7053d6fdfafda3f938979645c44812fbfcda098faae3f1a362d"
dependencies = [
 "bitflags",
]

[[package]]
name = "redox_users"
version = "0.5.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "60dc65c0ff1a7ae1294b0c67b9f14baf70b644404010370171787bfac1038fc0"
dependencies = [
 "libredox",
 "thiserror",
]

[[package]]
name = "regex"
version = "1.13.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f020237b6c8eed93db2e2cb53c00c60a8e1bc73da7d073199a1180401450218d"
dependencies = [
 "aho-corasick",
 "memchr",
 "regex-automata",
 "regex-syntax",
]

[[package]]
name = "regex-automata"
version = "0.4.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ad8553b9b26413251cbf30e620595c7a41b3887f03da04579c0e6b0d6a06b4b2"
dependencies = [
 "aho-corasick",
 "memchr",
 "regex-syntax",
]

[[package]]
name = "regex-syntax"
version = "0.8.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d6f6ff9a378485b298a5286656da665ba74413d36db0979633275d2e708145d4"

[[package]]
name = "renderdoc-sys"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "19b30a45b0cd0bcca8037f3d0dc3421eaf95327a17cad11964fb8179b4fc4832"

[[package]]
name = "reqwest"
version = "0.12.28"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "eddd3ca559203180a307f12d114c268abf583f59b03cb906fd0b3ff8646c1147"
dependencies = [
 "base64 0.22.1",
 "bytes",
 "futures-channel",
 "futures-core",
 "futures-util",
 "h2",
 "http",
 "http-body",
 "http-body-util",
 "hyper",
 "hyper-rustls",
 "hyper-util",
 "js-sys",
 "log",
 "percent-encoding",
 "pin-project-lite",
 "quinn",
 "rustls",
 "rustls-pki-types",
 "serde",
 "serde_json",
 "serde_urlencoded",
 "sync_wrapper",
 "tokio",
 "tokio-rustls",
 "tower",
 "tower-http",
 "tower-service",
 "url",
 "wasm-bindgen",
 "wasm-bindgen-futures",
 "web-sys",
 "webpki-roots",
]

[[package]]
name = "ring"
version = "0.17.14"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7"
dependencies = [
 "cc",
 "cfg-if",
 "getrandom 0.2.17",
 "libc",
 "untrusted",
 "windows-sys 0.52.0",
]

[[package]]
name = "rustc-demangle"
version = "0.1.28"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b74b56ffa8bb2830709a538c2cbcae9aa062db0d2a42563bfb09bdaae44020eb"

[[package]]
name = "rustc-hash"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "08d43f7aa6b08d49f382cde6a7982047c3426db949b1424bc4b7ec9ae12c6ce2"

[[package]]
name = "rustc-hash"
version = "2.1.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6b1e7f9a428571be2dc5bc0505c13fb6bf936822b894ec87abf8a08a4e51742d"

[[package]]
name = "rustc_version"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cfcb3a22ef46e85b45de6ee7e79d063319ebb6594faafcf1c225ea92ab6e9b92"
dependencies = [
 "semver",
]

[[package]]
name = "rustix"
version = "1.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e5e76742e5294153d4dde1207dbec5bdd81eef6fabdc0d2118babadabfd858aa"
dependencies = [
 "bitflags",
 "errno",
 "libc",
 "linux-raw-sys",
 "windows-sys 0.61.2",
]

[[package]]
name = "rustls"
version = "0.23.46"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "48e13bd8c0e9365c43cfa5c9e8f9ad49d3c8444926c9aac819e0e4dc503c8fdf"
dependencies = [
 "once_cell",
 "ring",
 "rustls-pki-types",
 "rustls-webpki",
 "subtle",
 "zeroize",
]

[[package]]
name = "rustls-pki-types"
version = "1.15.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2f4925028c7eb5d1fcdaf196971378ed9d2c1c4efc7dc5d011256f76c99c0a96"
dependencies = [
 "web-time",
 "zeroize",
]

[[package]]
name = "rustls-webpki"
version = "0.103.16"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281"
dependencies = [
 "ring",
 "rustls-pki-types",
 "untrusted",
]

[[package]]
name = "rustversion"
version = "1.0.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cf54715a573b99ac80df0bc206da022bcd442c974952c7b9720069370852e21f"

[[package]]
name = "ryu"
version = "1.0.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9774ba4a74de5f7b1c1451ed6cd5285a32eddb5cccb8cc655a4e50009e06477f"

[[package]]
name = "same-file"
version = "1.0.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "93fc1dc3aaa9bfed95e02e6eadabb4baf7e3078b0bd1b4d7b6b0b68378900502"
dependencies = [
 "winapi-util",
]

[[package]]
name = "sanitize-filename"
version = "0.6.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bc984f4f9ceb736a7bb755c3e3bd17dc56370af2600c9780dcc48c66453da34d"
dependencies = [
 "regex",
]

[[package]]
name = "scopeguard"
version = "1.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "94143f37725109f92c262ed2cf5e59bce7498c01bcc1502d7b9afe439a4e9f49"

[[package]]
name = "semver"
version = "1.0.28"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8a7852d02fc848982e0c167ef163aaff9cd91dc640ba85e263cb1ce46fae51cd"

[[package]]
name = "serde"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4148590afebada386688f18773da617792bf2ef03ffc1e4cbd2b1d45b023e0ba"
dependencies = [
 "serde_core",
 "serde_derive",
]

[[package]]
name = "serde_bytes"
version = "0.11.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a5d440709e79d88e51ac01c4b72fc6cb7314017bb7da9eeff678aa94c10e3ea8"
dependencies = [
 "serde",
 "serde_core",
]

[[package]]
name = "serde_core"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "67dca2c9c51e58a4791a4b1ed58308b39c64224d349a935ab5039aa360942a48"
dependencies = [
 "serde_derive",
]

[[package]]
name = "serde_derive"
version = "1.0.229"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e7a5d71263a5a7d47b41f6b3f06ba276f10cc18b0931f1799f710578e2309348"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "serde_json"
version = "1.0.154"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e7e9cc8b1b85264074fbcc02a88680c4096b1e47df8f739dceb03bf482f04bd6"
dependencies = [
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]

[[package]]
name = "serde_spanned"
version = "1.1.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4e7523beb55eece201a2356bee0bbca0d1ab466c14c07703b2e0ee6d42cb0c2c"
dependencies = [
 "serde_core",
]

[[package]]
name = "serde_urlencoded"
version = "0.7.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d3491c14715ca2294c4d6a88f15e84739788c1d030eed8c110436aafdaa2f3fd"
dependencies = [
 "form_urlencoded",
 "itoa",
 "ryu",
 "serde",
]

[[package]]
name = "sha2"
version = "0.10.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283"
dependencies = [
 "cfg-if",
 "cpufeatures 0.2.17",
 "digest",
]

[[package]]
name = "shlex"
version = "2.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f8fadd59c855ef2080decdef8ff161eb6661b86933c9d82e5ba29dc602a55aba"

[[package]]
name = "slab"
version = "0.4.12"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0c790de23124f9ab44544d7ac05d60440adc586479ce501c1d6d7da3cd8c9cf5"

[[package]]
name = "slotmap"
version = "1.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bdd58c3c93c3d278ca835519292445cb4b0d4dc59ccfdf7ceadaab3f8aeb4038"
dependencies = [
 "version_check",
]

[[package]]
name = "slug"
version = "0.1.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "882a80f72ee45de3cc9a5afeb2da0331d58df69e4e7d8eeb5d3c7784ae67e724"
dependencies = [
 "deunicode",
 "wasm-bindgen",
]

[[package]]
name = "smallvec"
version = "1.16.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d0d5d4b1981b8bbbb7aed204cae5726d683d5d6ac7e74ec54122ea28f315e7c8"
dependencies = [
 "serde",
]

[[package]]
name = "socket2"
version = "0.6.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c3d1e2c7f27f8d4cb10542a02c49005dbd6e93095799d6f3be745fae9f8fedd4"
dependencies = [
 "libc",
 "windows-sys 0.61.2",
]

[[package]]
name = "spin"
version = "0.10.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "023a211cb3138dbc438680b32560ad89f699977624c9f8dbb95a47d5b4c07dd3"
dependencies = [
 "lock_api",
 "portable-atomic",
]

[[package]]
name = "spirv"
version = "0.4.0+sdk-1.4.341.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d9571ea910ebd84c86af4b3ed27f9dbdc6ad06f17c5f96146b2b671e2976744f"
dependencies = [
 "bitflags",
]

[[package]]
name = "stable-vec"
version = "0.4.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "047720fbc1c4cc7daa75c925adba9ce3fa8e1f7b039337c281b142d4ecc6d86b"

[[package]]
name = "stable_deref_trait"
version = "1.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6ce2be8dc25455e1f91df71bfa12ad37d7af1092ae736f3a6cd0e37bc7810596"

[[package]]
name = "static_assertions"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a2eb9349b6444b326872e140eb1cf5e7c522154d69e7a0ffb0fb81c06b37543f"

[[package]]
name = "strsim"
version = "0.11.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f"

[[package]]
name = "subtle"
version = "2.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "13c2bddecc57b384dee18652358fb23172facb8a2c51ccc10d74c157bdea3292"

[[package]]
name = "syn"
version = "2.0.119"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "872831b642d1a07999a962a351ed35b955ea2cfc8f3862091e2a240a84f17297"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "syn"
version = "3.0.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d78c8dee4c7bf0e14673097256fed6142ce9d3b85a408189d07482442145823b"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "sync_wrapper"
version = "1.0.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0bf256ce5efdfa370213c1dabab5935a12e49f2c58d15e9eac2870d3b4f27263"
dependencies = [
 "futures-core",
]

[[package]]
name = "synstructure"
version = "0.14.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "901704edd0dfe137f1987838ee4f259e4e063c31371bdb423f7ae38ec6f77f02"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "sysinfo"
version = "0.38.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "92ab6a2f8bfe508deb3c6406578252e491d299cbbf3bc0529ecc3313aee4a52f"
dependencies = [
 "libc",
 "memchr",
 "ntapi",
 "objc2-core-foundation",
 "objc2-io-kit",
 "windows",
]

[[package]]
name = "tar"
version = "0.4.46"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3f6221d9a6003c78398e3b239969f352578258df48c8eb051caadae0015bc840"
dependencies = [
 "filetime",
 "libc",
 "xattr",
]

[[package]]
name = "termcolor"
version = "1.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "06794f8f6c5c898b3275aebefa6b8a1cb24cd2c6c79397ab15774837a0bc5755"
dependencies = [
 "winapi-util",
]

[[package]]
name = "thiserror"
version = "2.0.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "09e52cb86a36cede5cb101bf8908837b3e4c6e5e59fe7fd85c23fb56200d189e"
dependencies = [
 "thiserror-impl",
]

[[package]]
name = "thiserror-impl"
version = "2.0.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fe5197923287db20a58125f0bc85c062f7f2c892de97b18c356f9efb14b28524"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "tinystr"
version = "0.8.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc840c238154d58f9e86b2792261edfde565a17696e1cda1a449de27979801a"
dependencies = [
 "displaydoc",
 "zerovec",
]

[[package]]
name = "tinyvec"
version = "1.13.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fd3ca314f692efd6c868f8408f53fe444634a845f96c028b97d35f6a1f79f0ee"

[[package]]
name = "tokio"
version = "1.54.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b52fd3255258d26fac7264ab5a76b53c13367136dc011d3d6ad4f8ed070396a3"
dependencies = [
 "bytes",
 "libc",
 "mio",
 "pin-project-lite",
 "socket2",
 "windows-sys 0.61.2",
]

[[package]]
name = "tokio-rustls"
version = "0.26.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c9cc2678c2cdd569ef8215e2afd7954ada2ae20b4fdd2c5fe6139a3b02d105db"
dependencies = [
 "rustls",
 "tokio",
]

[[package]]
name = "tokio-util"
version = "0.7.20"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e464cf451ba96ebfc6f9b6542f17ee8b8956e33f1e40d9690624e59d7a7f8a4b"
dependencies = [
 "bytes",
 "futures-core",
 "futures-sink",
 "libc",
 "pin-project-lite",
 "tokio",
]

[[package]]
name = "toml"
version = "1.2.0+spec-1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c5ffcdf8f50436dbbfd675c1fe1b8e0854f42b7defea8584bdebe1aee900e561"
dependencies = [
 "indexmap",
 "serde_core",
 "serde_spanned",
 "toml_datetime",
 "toml_parser",
 "toml_writer",
 "winnow",
]

[[package]]
name = "toml_datetime"
version = "1.1.2+spec-1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2b86d767906c6c42421dcba507eb9d203e779497710a47782a224bb871653053"
dependencies = [
 "serde_core",
]

[[package]]
name = "toml_parser"
version = "1.1.5+spec-1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "baa693a8032d7e1cada7d0041e96126df243179ff061456783ac7f12bda4744c"
dependencies = [
 "winnow",
]

[[package]]
name = "toml_writer"
version = "1.2.0+spec-1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "009dbb7700f11c55ef83aebbe4c0e9cf1f1ac0844bd8d6f00dc38f302026e13a"

[[package]]
name = "tower"
version = "0.5.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ebe5ef63511595f1344e2d5cfa636d973292adc0eec1f0ad45fae9f0851ab1d4"
dependencies = [
 "futures-core",
 "futures-util",
 "pin-project-lite",
 "sync_wrapper",
 "tokio",
 "tower-layer",
 "tower-service",
]

[[package]]
name = "tower-http"
version = "0.6.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4cfcf7e2740e6fc6d4d688b4ef00650406bb94adf4731e43c096c3a19fe40840"
dependencies = [
 "bitflags",
 "bytes",
 "futures-util",
 "http",
 "http-body",
 "pin-project-lite",
 "tower",
 "tower-layer",
 "tower-service",
 "url",
]

[[package]]
name = "tower-layer"
version = "0.3.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "121c2a6cda46980bb0fcd1647ffaf6cd3fc79a013de288782836f6df9c48780e"

[[package]]
name = "tower-service"
version = "0.3.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8df9b6e13f2d32c91b9bd719c00d1958837bc7dec474d94952798cc8e69eeec3"

[[package]]
name = "tracel-llvm"
version = "22.1.4-5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1fdf8a19f43dfaa86f20c7e7f22376b1bb47f4846cd89f4e0bb9f1a4fa50a9e9"
dependencies = [
 "tracel-mlir-rs",
 "tracel-mlir-sys",
]

[[package]]
name = "tracel-llvm-bundler"
version = "22.1.4-6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "de376825263c1a1e0af395cb7e04e82099c2fe16724235d313ab803b33bba87b"
dependencies = [
 "anyhow",
 "bytes",
 "cc",
 "constcat",
 "dirs",
 "liblzma",
 "regex",
 "reqwest",
 "serde",
 "serde_json",
 "sha2",
 "tar",
 "walkdir",
]

[[package]]
name = "tracel-mlir-rs"
version = "22.1.4-5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b0b1cf4683d1bf518040a8e13ebb2c1b18a8c2bb1db257f47603edbd99b094f8"
dependencies = [
 "tracel-mlir-rs-macros",
 "tracel-mlir-sys",
]

[[package]]
name = "tracel-mlir-rs-macros"
version = "22.1.4-5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "04d4cea4e89fcaddbcc7fdeba0720b62d8bda63e23326ff28463d4b864ef9945"
dependencies = [
 "comrak",
 "convert_case 0.8.0",
 "proc-macro2",
 "quote",
 "regex",
 "syn 2.0.119",
 "tracel-llvm-bundler",
 "tracel-tblgen-rs",
 "unindent",
]

[[package]]
name = "tracel-mlir-sys"
version = "22.1.4-5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "fb813a9f259bb2112a5a1186fa366e6075e6dc1c041b5ae38a116535b9daca64"
dependencies = [
 "tracel-llvm-bundler",
]

[[package]]
name = "tracel-tblgen-rs"
version = "22.1.4-5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d93699345d51e8bcb0d968165bf024ea28af87fee776a60d55875bdb1b898f2d"
dependencies = [
 "paste",
 "thiserror",
 "tracel-llvm-bundler",
]

[[package]]
name = "tracing"
version = "0.1.44"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "63e71662fa4b2a2c3a26f570f037eb95bb1f85397f3cd8076caed2f026a6d100"
dependencies = [
 "pin-project-lite",
 "tracing-core",
]

[[package]]
name = "tracing-core"
version = "0.1.36"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "db97caf9d906fbde555dd62fa95ddba9eecfd14cb388e4f491a66d74cd5fb79a"
dependencies = [
 "once_cell",
]

[[package]]
name = "try-lock"
version = "0.2.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e421abadd41a4225275504ea4d6566923418b7f05506fbc9c0fe86ba7396114b"

[[package]]
name = "tynm"
version = "0.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a21cdb0fc8f85c98b1ec812bc4cd69faf6c0fa2fc17d44ea3c2cdd38dc08e999"
dependencies = [
 "nom",
]

[[package]]
name = "type-map"
version = "0.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "cb30dbbd9036155e74adad6812e9898d03ec374946234fbcebd5dfc7b9187b90"
dependencies = [
 "rustc-hash 2.1.3",
]

[[package]]
name = "typed-arena"
version = "2.0.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6af6ae20167a9ece4bcb41af5b80f8a1f1df981f6391189ce00fd257af04126a"

[[package]]
name = "typenum"
version = "1.20.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b6f5e870be6c3b371b77fe0ee0bafb859fa4964b4404c27de1d380043c4dda20"

[[package]]
name = "unicode-ident"
version = "1.0.27"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a2c754d6c33795a1c324727428e5a7dedb5b06195f9890bdbcba760d3e246563"

[[package]]
name = "unicode-normalization"
version = "0.1.26"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "75cd3fb03d5288b3ba9a1ee103ac7e9cbb59f0b03d64f8ddfcc04dd42758a85d"
dependencies = [
 "tinyvec",
]

[[package]]
name = "unicode-segmentation"
version = "1.13.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c6f5d3c3b1bf09027a88a6bc961fc00497d651009560b5463668dc81b0fa87a8"

[[package]]
name = "unicode-width"
version = "0.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ca718471876992059f251e33aafc334346ada08042b7661dd1fe313a9bced557"

[[package]]
name = "unicode-xid"
version = "0.2.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ebc1c04c71510c7f702b52b7c350734c9ff1295c464a03335b00bb84fc54f853"

[[package]]
name = "unicode_categories"
version = "0.1.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "39ec24b3121d976906ece63c9daad25b85969647682eee313cb5779fdd69e14e"

[[package]]
name = "unindent"
version = "0.2.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7264e107f553ccae879d21fbea1d6724ac785e8c3bfc762137959b5802826ef3"

[[package]]
name = "unsynn"
version = "0.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "501a7adf1a4bd9951501e5c66621e972ef8874d787628b7f90e64f936ef7ec0a"
dependencies = [
 "mutants",
 "proc-macro2",
 "rustc-hash 2.1.3",
]

[[package]]
name = "untrusted"
version = "0.9.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8ecb6da28b8a351d773b68d5825ac39017e680750f980f3a1a85cd8dd28a47c1"

[[package]]
name = "url"
version = "2.5.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ff67a8a4397373c3ef660812acab3268222035010ab8680ec4215f38ba3d0eed"
dependencies = [
 "form_urlencoded",
 "idna",
 "percent-encoding",
 "serde",
]

[[package]]
name = "utf8_iter"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "af3bd9d3e571c26fcce9d033b87ea5ab061b8d2ffe9b7a5675d810d68f03edf0"
dependencies = [
 "utf_types",
]

[[package]]
name = "utf_types"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3a3b425286980a367278453f3b6d28cdcb134b574e5a35824a6ff5eebad50bb6"

[[package]]
name = "variadics_please"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b53d0f7f2d0182759a549f1f313ce16b07d8f9ba93098157b5fa10ee3e61117f"
dependencies = [
 "quote",
 "unsynn",
]

[[package]]
name = "version_check"
version = "0.9.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0b928f33d975fc6ad9f86c8f283853ad26bdd5b10b7f1542aa2fa15e2289105a"

[[package]]
name = "void"
version = "1.0.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6a02e4885ed3bc0f2de90ea6dd45ebcbb66dacffe03547fadbb0eeae2770887d"

[[package]]
name = "walkdir"
version = "2.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "29790946404f91d9c5d06f9874efddea1dc06c5efe94541a7d6863108e3a5e4b"
dependencies = [
 "same-file",
 "winapi-util",
]

[[package]]
name = "want"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ec4cdd0dd910afe868b7ef477227d8d538b46b3075031afee8a9f2acb0a2ed0b"
dependencies = [
 "try-lock",
]

[[package]]
name = "wasi"
version = "0.11.1+wasi-snapshot-preview1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b"

[[package]]
name = "wasm-bindgen"
version = "0.2.130"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a0181c14ddd6c7b8b0b0132a5f1ea2f32be0d893cd3ea97b6e3b28886b8a3d1c"
dependencies = [
 "cfg-if",
 "rustversion",
 "wasm-bindgen-macro",
 "wasm-bindgen-shared",
]

[[package]]
name = "wasm-bindgen-futures"
version = "0.4.80"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5e14e0327cb5368ad60ccb22d04a9827a693c95a926d8d125e9fa39e0baec501"
dependencies = [
 "js-sys",
 "tokio",
 "wasm-bindgen",
]

[[package]]
name = "wasm-bindgen-macro"
version = "0.2.130"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6e931a02a39abdbf66966fc35902b5ed58344750a70885bed4ebb3d8eaf28ed9"
dependencies = [
 "quote",
 "wasm-bindgen-macro-support",
]

[[package]]
name = "wasm-bindgen-macro-support"
version = "0.2.130"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "2ca4b54280e455bd831e33a6b8e734baf432fe63fbc35c42ac14ce67a043a53a"
dependencies = [
 "bumpalo",
 "proc-macro2",
 "quote",
 "syn 3.0.9",
 "wasm-bindgen-shared",
]

[[package]]
name = "wasm-bindgen-shared"
version = "0.2.130"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3b1f9c3798725016c6ee5f2f57869ec0831e8bbc0ab0310a86e787e97f41389e"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "wayland-sys"
version = "0.31.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d8eab23fefc9e41f8e841df4a9c707e8a8c4ed26e944ef69297184de2785e3be"
dependencies = [
 "dlib",
 "log",
 "once_cell",
 "pkg-config",
]

[[package]]
name = "web-sys"
version = "0.3.107"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d7d179596ff5ec0f6de4db64d54011bde6289cc49603d3a6e4ce3f329207ab88"
dependencies = [
 "js-sys",
 "wasm-bindgen",
]

[[package]]
name = "web-time"
version = "1.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5a6580f308b1fad9207618087a65c04e7a10bc77e02c8e84e9b00dd4b12fa0bb"
dependencies = [
 "js-sys",
 "wasm-bindgen",
]

[[package]]
name = "webpki-roots"
version = "1.0.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f5f280bbb2b287bc8e7de7e3852945a0ff50d1bbcb5ed42b6c7a568c546df171"
dependencies = [
 "rustls-pki-types",
]

[[package]]
name = "wgpu"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "527ccdf43dd5b2e8676eed9984ce00e2bbb0a1b85b70c1969dcb6cd2eb55ab9e"
dependencies = [
 "arrayvec",
 "bitflags",
 "bytemuck",
 "cfg-if",
 "cfg_aliases",
 "document-features",
 "hashbrown 0.17.1",
 "js-sys",
 "log",
 "naga",
 "parking_lot",
 "portable-atomic",
 "profiling",
 "raw-window-handle",
 "smallvec",
 "static_assertions",
 "wasm-bindgen",
 "wasm-bindgen-futures",
 "web-sys",
 "wgpu-core",
 "wgpu-hal",
 "wgpu-types",
]

[[package]]
name = "wgpu-core"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "14c018fce9b6270aa203c2fdd56f3cce996713534bd757e4ea58c8560b121f14"
dependencies = [
 "arrayvec",
 "bit-set",
 "bit-vec",
 "bitflags",
 "bytemuck",
 "cfg_aliases",
 "document-features",
 "hashbrown 0.17.1",
 "indexmap",
 "log",
 "naga",
 "naga-types",
 "once_cell",
 "parking_lot",
 "portable-atomic",
 "profiling",
 "raw-window-handle",
 "rustc-hash 1.1.0",
 "smallvec",
 "thiserror",
 "wgpu-core-deps-apple",
 "wgpu-core-deps-emscripten",
 "wgpu-core-deps-windows-linux-android",
 "wgpu-hal",
 "wgpu-naga-bridge",
 "wgpu-types",
]

[[package]]
name = "wgpu-core-deps-apple"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "061f3d319a40d39d00b1ecc2c33b89fe21d4e6fe01859df3500a3a8ecccd6b68"
dependencies = [
 "wgpu-hal",
]

[[package]]
name = "wgpu-core-deps-emscripten"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d98b86cf4abf524a902dd35f18ca6a3f08fc2ae9847c8f10b48e30491b1f0b86"
dependencies = [
 "wgpu-hal",
]

[[package]]
name = "wgpu-core-deps-windows-linux-android"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7586165fd5f6d881cb9ce4bb71f40d6caab2c0f1837e3fc1d9788a197fb6004f"
dependencies = [
 "wgpu-hal",
]

[[package]]
name = "wgpu-hal"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b6b7fb58561a792bc237628ba0792e332de418fefe145f13b5ed8201e6d52f58"
dependencies = [
 "android_system_properties",
 "arrayvec",
 "ash",
 "bit-set",
 "bitflags",
 "block2",
 "bytemuck",
 "cfg-if",
 "cfg_aliases",
 "glow",
 "glutin_wgl_sys",
 "gpu-allocator",
 "hashbrown 0.17.1",
 "js-sys",
 "khronos-egl",
 "libc",
 "libloading 0.8.9",
 "log",
 "naga",
 "naga-types",
 "ndk-sys",
 "objc2",
 "objc2-core-foundation",
 "objc2-core-graphics",
 "objc2-foundation",
 "objc2-metal",
 "objc2-quartz-core",
 "once_cell",
 "ordered-float",
 "parking_lot",
 "portable-atomic",
 "portable-atomic-util",
 "profiling",
 "range-alloc",
 "raw-window-handle",
 "raw-window-metal",
 "renderdoc-sys",
 "smallvec",
 "static_assertions",
 "thiserror",
 "wasm-bindgen",
 "wayland-sys",
 "web-sys",
 "wgpu-naga-bridge",
 "wgpu-types",
 "windows",
 "windows-core",
 "windows-result",
]

[[package]]
name = "wgpu-naga-bridge"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d2f62e73117bb7a62bfd9c5a5841438a823f6566c6442a808ee269d2d055c081"
dependencies = [
 "naga",
 "wgpu-types",
]

[[package]]
name = "wgpu-types"
version = "30.0.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "99dad6f1fbdbbdb4c278a6508b059d44688f5cebddf78d005a46a31340269286"
dependencies = [
 "bitflags",
 "bytemuck",
 "js-sys",
 "log",
 "naga-types",
 "raw-window-handle",
 "static_assertions",
 "web-sys",
]

[[package]]
name = "winapi"
version = "0.3.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5c839a674fcd7a98952e593242ea400abe93992746761e38641405d28b00f419"
dependencies = [
 "winapi-i686-pc-windows-gnu",
 "winapi-x86_64-pc-windows-gnu",
]

[[package]]
name = "winapi-i686-pc-windows-gnu"
version = "0.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ac3b87c63620426dd9b991e5ce0329eff545bccbbb34f3be09ff6fb6ab51b7b6"

[[package]]
name = "winapi-util"
version = "0.1.11"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c2a7b1c03c876122aa43f3020e6c3c3ee5c05081c9a00739faf7503aeba10d22"
dependencies = [
 "windows-sys 0.61.2",
]

[[package]]
name = "winapi-x86_64-pc-windows-gnu"
version = "0.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "712e227841d057c1ee1cd2fb22fa7e5a5461ae8e48fa2ca79ec42cfc1931183f"

[[package]]
name = "windows"
version = "0.62.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "527fadee13e0c05939a6a05d5bd6eec6cd2e3dbd648b9f8e447c6518133d8580"
dependencies = [
 "windows-collections",
 "windows-core",
 "windows-future",
 "windows-numerics",
]

[[package]]
name = "windows-collections"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "23b2d95af1a8a14a3c7367e1ed4fc9c20e0a26e79551b1454d72583c97cc6610"
dependencies = [
 "windows-core",
]

[[package]]
name = "windows-core"
version = "0.62.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b8e83a14d34d0623b51dce9581199302a221863196a1dde71a7663a4c2be9deb"
dependencies = [
 "windows-implement",
 "windows-interface",
 "windows-link",
 "windows-result",
 "windows-strings",
]

[[package]]
name = "windows-future"
version = "0.3.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e1d6f90251fe18a279739e78025bd6ddc52a7e22f921070ccdc67dde84c605cb"
dependencies = [
 "windows-core",
 "windows-link",
 "windows-threading",
]

[[package]]
name = "windows-implement"
version = "0.60.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "053e2e040ab57b9dc951b72c264860db7eb3b0200ba345b4e4c3b14f67855ddf"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "windows-interface"
version = "0.59.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3f316c4a2570ba26bbec722032c4099d8c8bc095efccdc15688708623367e358"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "windows-link"
version = "0.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5"

[[package]]
name = "windows-numerics"
version = "0.3.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "6e2e40844ac143cdb44aead537bbf727de9b044e107a0f1220392177d15b0f26"
dependencies = [
 "windows-core",
 "windows-link",
]

[[package]]
name = "windows-result"
version = "0.4.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7781fa89eaf60850ac3d2da7af8e5242a5ea78d1a11c49bf2910bb5a73853eb5"
dependencies = [
 "windows-link",
]

[[package]]
name = "windows-strings"
version = "0.5.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7837d08f69c77cf6b07689544538e017c1bfcf57e34b4c0ff58e6c2cd3b37091"
dependencies = [
 "windows-link",
]

[[package]]
name = "windows-sys"
version = "0.52.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "282be5f36a8ce781fad8c8ae18fa3f9beff57ec1b52cb3de0789201425d9a33d"
dependencies = [
 "windows-targets",
]

[[package]]
name = "windows-sys"
version = "0.61.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc"
dependencies = [
 "windows-link",
]

[[package]]
name = "windows-targets"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9b724f72796e036ab90c1021d4780d4d3d648aca59e491e6b98e725b84e99973"
dependencies = [
 "windows_aarch64_gnullvm",
 "windows_aarch64_msvc",
 "windows_i686_gnu",
 "windows_i686_gnullvm",
 "windows_i686_msvc",
 "windows_x86_64_gnu",
 "windows_x86_64_gnullvm",
 "windows_x86_64_msvc",
]

[[package]]
name = "windows-threading"
version = "0.2.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3949bd5b99cafdf1c7ca86b43ca564028dfe27d66958f2470940f73d86d75b37"
dependencies = [
 "windows-link",
]

[[package]]
name = "windows_aarch64_gnullvm"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "32a4622180e7a0ec044bb555404c800bc9fd9ec262ec147edd5989ccd0c02cd3"

[[package]]
name = "windows_aarch64_msvc"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "09ec2a7bb152e2252b53fa7803150007879548bc709c039df7627cabbd05d469"

[[package]]
name = "windows_i686_gnu"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8e9b5ad5ab802e97eb8e295ac6720e509ee4c243f69d781394014ebfe8bbfa0b"

[[package]]
name = "windows_i686_gnullvm"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0eee52d38c090b3caa76c563b86c3a4bd71ef1a819287c19d586d7334ae8ed66"

[[package]]
name = "windows_i686_msvc"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "240948bc05c5e7c6dabba28bf89d89ffce3e303022809e73deaefe4f6ec56c66"

[[package]]
name = "windows_x86_64_gnu"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "147a5c80aabfbf0c7d901cb5895d1de30ef2907eb21fbbab29ca94c5b08b1a78"

[[package]]
name = "windows_x86_64_gnullvm"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "24d5b23dc417412679681396f2b49f3de8c1473deb516bd34410872eff51ed0d"

[[package]]
name = "windows_x86_64_msvc"
version = "0.52.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "589f6da84c646204747d1270a2a5661ea66ed1cced2631d546fdfb155959f9ec"

[[package]]
name = "winnow"
version = "1.0.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8c3b93199df0ab874cdf0c6a94f0a0dc3423fe41f1cafb62b8bd7b1deb277aef"

[[package]]
name = "writeable"
version = "0.6.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3ad82d2a33cdc9674dc7465672f271e096168fcdbe0f799d9e6db8c5892679dc"

[[package]]
name = "xattr"
version = "1.7.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "963d6fab92d5504b971dc8bd90ca743c2575e590bc8071ea5bc571598667ae08"
dependencies = [
 "libc",
 "rustix",
]

[[package]]
name = "xml-rs"
version = "0.8.29"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e450f9b2ed1dff33c94c12589a87338689467b9c4f5d8a5710bd09a847d2c8a7"

[[package]]
name = "xxhash-rust"
version = "0.8.19"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "550a2b930b62486a393c52d5c3b84bff264b28aa437ed64694d31e93b1757af7"

[[package]]
name = "yoke"
version = "0.8.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "417ef3fcf6c300b7dbd25c85531a746931c02ad536f3cf4786ed1e88223340b3"
dependencies = [
 "stable_deref_trait",
 "yoke-derive",
 "zerofrom",
]

[[package]]
name = "yoke-derive"
version = "0.8.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "ec8ebde2db3681e8c9980cc27822030e68752690ddfa9473e739aeb4dbde6d71"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
 "synstructure",
]

[[package]]
name = "zerocopy"
version = "0.8.63"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e5fe1f8f1b06191a00962174c61aa5005e0bb391a6d80d07e24d115c01a92ed8"
dependencies = [
 "zerocopy-derive",
]

[[package]]
name = "zerocopy-derive"
version = "0.8.63"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "863ad3ac83293fb4d740aedbfdc9240dd8d1a50c1099acd76ce80ce7c7230c7f"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 2.0.119",
]

[[package]]
name = "zerofrom"
version = "0.1.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0ec05a11813ea801ff6d75110ad09cd0824ddba17dfe17128ea0d5f68e6c5272"
dependencies = [
 "zerofrom-derive",
]

[[package]]
name = "zerofrom-derive"
version = "0.1.8"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f75b4683f6c7f45248d4d64056a24298c6281e0993356d7d1b4a1a962ef10d4a"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
 "synstructure",
]

[[package]]
name = "zeroize"
version = "1.9.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e13084392c5e4bc371903e2935a5eaeed24905a7511356b883835e18a78f6879"

[[package]]
name = "zerotrie"
version = "0.2.5"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4ea269c3bd32f0a32c321907a2ae912ba6f4649bb0fc764a15627e99a7095a3f"
dependencies = [
 "displaydoc",
 "yoke",
 "zerofrom",
]

[[package]]
name = "zerovec"
version = "0.11.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f58a2799cfe9a1670c7ea8edbd52ef8edb93cb37a7f83bf31ba9e35d9dae6a40"
dependencies = [
 "yoke",
 "zerofrom",
 "zerovec-derive",
]

[[package]]
name = "zerovec-derive"
version = "0.11.7"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c3743ddbf47738d929cd1ae2e199d40d0162979dfe31aef8ae646d2bf00bf97a"
dependencies = [
 "proc-macro2",
 "quote",
 "syn 3.0.9",
]

[[package]]
name = "zmij"
version = "1.0.23"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b"

`````

### File: `src/common.rs`

blake3: `df7ce39cfec9034983a461f72de7c60110b5e34345dccef6d22d308d513ff07d`

`````rust
//! Shared harness substrate for the kernel_opt GPU-host G2 re-gate runner
//! (riir-refine Issue 191 T6, Bench 130 pre-registration).
//!
//! Harness law (binding): per pair WARMUP = 3 untimed chunks per arm, then
//! 15 interleaved (ref chunk, opt chunk) timed pairs; per-pair speedup =
//! t_ref / t_opt; the pair verdict is the MEDIAN of the 15. A chunk is
//! `iters` kernel-launch rounds then ONE `client.sync()`; `iters` is
//! calibrated once per pair on the ref arm to a >= 2 ms chunk (clamp
//! 1..=10000) and the SAME iters is handed to the opt arm. Buffers are
//! created once per pair and reused across every chunk — zero allocation
//! inside timed regions. Loud-zero defense: a chunk timing 0 ms fails the
//! pair; the per-chunk launch count and any host-side accumulators are
//! `std::hint::black_box`ed; after timing the pair's correctness gate
//! re-runs to catch state corruption.

use cubecl::prelude::*;
use cubecl::server::Handle;

// ── Active runtime selection (the house ActiveRuntime law, mirroring
//    riir-infer crates/riir-infer-gpu/src/cubecl_runtime.rs L213-236:
//    `cuda_runtime` feature → native CUDA, default → wgpu/Metal on macOS) ──
#[cfg(feature = "cuda_runtime")]
pub type ActiveRuntime = cubecl::cuda::CudaRuntime;
#[cfg(not(feature = "cuda_runtime"))]
pub type ActiveRuntime = cubecl::wgpu::WgpuRuntime;

pub type Client = ComputeClient<ActiveRuntime>;

// ── Deterministic inputs (harness law 5) ──
// Tiny host-side LCG; the constants are fixed by the registration. The
// per-pair seed is derived from the pair id (documented derivation):
// FNV-1a 64 over the id bytes, XOR a fixed bench tag — a pure function of
// the pair id, so it is a constant of the sources commit.

pub const LCG_MUL: u64 = 6364136223846793005;
pub const LCG_INC: u64 = 1442695040888963407;

pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(LCG_MUL).wrapping_add(LCG_INC);
        self.0
    }

    /// Uniform [0, 1). The top 24 bits of the u64 are exactly representable
    /// as f32, so the division by 2^24 is exact and the interval half-open.
    pub fn next_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16777216.0
    }

    /// Uniform [lo, hi).
    pub fn next_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_unit()
    }

    pub fn fill(&mut self, n: usize, lo: f32, hi: f32) -> Vec<f32> {
        (0..n).map(|_| self.next_f32(lo, hi)).collect()
    }
}

pub fn seed_for_pair(id: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^ 0x5EED_4B47_3320_2607
}

// ── Buffer helpers (bytes-based handles, the house pattern) ──

pub fn upload_f32(client: &Client, data: &[f32]) -> Handle {
    client.create_from_slice(f32::as_bytes(data))
}

pub fn upload_i32(client: &Client, data: &[i32]) -> Handle {
    client.create_from_slice(i32::as_bytes(data))
}

pub fn upload_u32(client: &Client, data: &[u32]) -> Handle {
    client.create_from_slice(u32::as_bytes(data))
}

pub fn upload_i8(client: &Client, data: &[i8]) -> Handle {
    client.create_from_slice(i8::as_bytes(data))
}

pub fn empty_f32(client: &Client, n: usize) -> Handle {
    client.empty(n * core::mem::size_of::<f32>())
}

pub fn empty_i32(client: &Client, n: usize) -> Handle {
    client.empty(n * core::mem::size_of::<i32>())
}

pub fn empty_u32(client: &Client, n: usize) -> Handle {
    client.empty(n * core::mem::size_of::<u32>())
}

pub fn sync_ok(client: &Client) -> Result<(), String> {
    pollster::block_on(client.sync()).map_err(|e| format!("device sync failed: {e:?}"))
}

pub fn read_f32(client: &Client, h: &Handle) -> Result<Vec<f32>, String> {
    let bytes = client
        .read_one(h.clone())
        .map_err(|e| format!("read_one failed: {e:?}"))?;
    Ok(f32::from_bytes(&bytes).to_vec())
}

pub fn read_i32(client: &Client, h: &Handle) -> Result<Vec<i32>, String> {
    let bytes = client
        .read_one(h.clone())
        .map_err(|e| format!("read_one failed: {e:?}"))?;
    Ok(i32::from_bytes(&bytes).to_vec())
}

pub fn read_u32(client: &Client, h: &Handle) -> Result<Vec<u32>, String> {
    let bytes = client
        .read_one(h.clone())
        .map_err(|e| format!("read_one failed: {e:?}"))?;
    Ok(u32::from_bytes(&bytes).to_vec())
}

// ── Compare gates ──
// Default: relative L∞ 1e-3 (|a-b| <= tol * max(1, |b|) elementwise);
// elementwise-only pairs 1e-5; pure-movement / fixed-point-combine pairs
// bit-exact.

pub fn rel_linf(a: &[f32], b: &[f32], tol: f64) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    let mut worst = 0.0f64;
    let mut worst_i = 0usize;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if !x.is_finite() || !y.is_finite() {
            return Err(format!("non-finite at [{i}]: {x} vs {y}"));
        }
        let d = ((*x - *y) as f64).abs();
        let r = d / f64::max(1.0, (*y as f64).abs());
        if r > worst {
            worst = r;
            worst_i = i;
        }
    }
    if worst > tol {
        Err(format!(
            "rel Linf {worst:.3e} > {tol:.0e} at [{worst_i}] (a={} b={})",
            a[worst_i], b[worst_i]
        ))
    } else {
        Ok(())
    }
}

pub fn abs_linf(a: &[f32], b: &[f32], tol: f64) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    let mut worst = 0.0f64;
    let mut worst_i = 0usize;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if !x.is_finite() || !y.is_finite() {
            return Err(format!("non-finite at [{i}]: {x} vs {y}"));
        }
        let d = ((*x - *y) as f64).abs();
        if d > worst {
            worst = d;
            worst_i = i;
        }
    }
    if worst > tol {
        Err(format!(
            "abs Linf {worst:.3e} > {tol:.0e} at [{worst_i}] (a={} b={})",
            a[worst_i], b[worst_i]
        ))
    } else {
        Ok(())
    }
}

pub fn bits_exact(a: &[f32], b: &[f32]) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x.to_bits() != y.to_bits() {
            return Err(format!(
                "bit mismatch at [{i}]: {} ({:#x}) vs {} ({:#x})",
                x,
                x.to_bits(),
                y,
                y.to_bits()
            ));
        }
    }
    Ok(())
}

pub fn i32_exact(a: &[i32], b: &[i32]) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            return Err(format!("i32 mismatch at [{i}]: {x} vs {y}"));
        }
    }
    Ok(())
}

pub fn u32_exact(a: &[u32], b: &[u32]) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("length mismatch: {} vs {}", a.len(), b.len()));
    }
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            return Err(format!("u32 mismatch at [{i}]: {x} vs {y}"));
        }
    }
    Ok(())
}

// ── Timing engine ──

pub struct BenchOutcome {
    pub ref_ms: f64,
    pub opt_ms: f64,
    pub median_speedup: f64,
    pub iters: u32,
}

pub fn time_chunk_ms(f: impl FnOnce()) -> f64 {
    let t0 = std::time::Instant::now();
    f();
    t0.elapsed().as_secs_f64() * 1000.0
}

fn chunk_time(chunk: &dyn Fn(u32), iters: u32) -> f64 {
    let n = std::hint::black_box(iters);
    time_chunk_ms(move || chunk(n))
}

/// Smallest (power-of-two-leaning) iters making a ref chunk >= target_ms
/// wall, clamp 1..=10000. Exponential probe up, then halve while the half
/// still meets the bar — the documented approximation of "smallest iters"
/// (each calibration step doubles or halves a full timed chunk).
pub fn calibrate_iters(chunk: &dyn Fn(u32), target_ms: f64) -> u32 {
    let mut iters: u32 = 1;
    let mut t = chunk_time(chunk, iters);
    while t < target_ms && iters < 10000 {
        iters = iters.saturating_mul(2).min(10000);
        t = chunk_time(chunk, iters);
    }
    while iters > 1 {
        let half = (iters / 2).max(1);
        if chunk_time(chunk, half) >= target_ms {
            iters = half;
        } else {
            break;
        }
    }
    iters.clamp(1, 10000)
}

pub fn interleave_bench(
    ref_chunk: &dyn Fn(u32),
    opt_chunk: &dyn Fn(u32),
    iters_override: Option<u32>,
) -> Result<BenchOutcome, String> {
    const TARGET_MS: f64 = 2.0;
    const WARMUP: usize = 3;
    const N_PAIRS: usize = 15;
    // One untimed call per arm BEFORE calibrating: the first launch of each
    // kernel pays the backend's JIT compile, which would otherwise inflate
    // the calibration probe and strand the pair at iters=1.
    ref_chunk(1);
    opt_chunk(1);
    let iters = match iters_override {
        Some(n) => n.clamp(1, 10000),
        None => calibrate_iters(ref_chunk, TARGET_MS),
    };
    for _ in 0..WARMUP {
        ref_chunk(iters);
    }
    for _ in 0..WARMUP {
        opt_chunk(iters);
    }
    let mut ratios: Vec<f64> = Vec::with_capacity(N_PAIRS);
    let mut ref_ms_all: Vec<f64> = Vec::with_capacity(N_PAIRS);
    let mut opt_ms_all: Vec<f64> = Vec::with_capacity(N_PAIRS);
    let mut guard: u64 = 0;
    for _ in 0..N_PAIRS {
        let tr = chunk_time(ref_chunk, iters);
        let to = chunk_time(opt_chunk, iters);
        if tr <= 0.0 || to <= 0.0 {
            return Err(format!(
                "loud zero: chunk timing 0 ms (t_ref={tr:.6} t_opt={to:.6}) — pair FAILS, never passes"
            ));
        }
        guard = guard.wrapping_add(tr.to_bits()).wrapping_add(to.to_bits());
        ratios.push(tr / to);
        ref_ms_all.push(tr);
        opt_ms_all.push(to);
    }
    std::hint::black_box(guard);
    Ok(BenchOutcome {
        ref_ms: median(&mut ref_ms_all),
        opt_ms: median(&mut opt_ms_all),
        median_speedup: median(&mut ratios),
        iters,
    })
}

pub fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("timing NaN"));
    v[v.len() / 2]
}

`````

### File: `src/main.rs`

blake3: `7b3dd71a70e115dc52d1bca079f123b8f4864fac89bb0744c340dd311ee80317`

`````rust
//! kernel_bench_g2_gpu — standalone CubeCL GPU bench runner for the
//! kernel_opt GPU-host G2 re-gate (riir-refine Issue 191 T6, Bench 130
//! pre-registration). 23 ref-vs-opt kernel pairs (the pre-reg table in
//! full); interleaved A/B timing per the harness law; correctness-gated.
//!
//! Backend: Metal (WgpuRuntime) here — every pair must PASS --correctness
//! on this box; CUDA (CudaRuntime) via `--features cuda_runtime` for the
//! 4090 timing run. NO autotune anywhere: every launch goes through
//! `launch_unchecked` (never `launch`), the geometry is pinned per pair.

use cubecl::Runtime as _;
use common::{ActiveRuntime, Client};
use pairs::PAIRS;

mod common;
mod pairs;
mod pairs_gemm;
mod pairs_reduce;
mod pairs_special;
mod pairs_stream;
mod probe;

fn print_help() {
    println!("kernel_bench_g2_gpu — CubeCL GPU G2 re-gate runner (riir-refine Issue 191 T6 / Bench 130)");
    println!();
    println!("USAGE: cargo run --release -- [FLAGS]");
    println!("  --correctness        run every selected pair's correctness gate only; exit 1 on any FAIL");
    println!("  --bench              full interleaved bench (correctness gate first per pair)");
    println!("  --pairs <a,b,c>      comma-separated pair-id subset (default: all)");
    println!("  --iters <n>          override per-arm chunk calibration (clamp 1..=10000)");
    println!("  --provenance         print runtime/device/env provenance lines");
    println!("  --help               this text");
    println!();
    println!("Harness laws (Bench 130): warmup 3 untimed chunks per arm; 15 interleaved");
    println!("(ref chunk, opt chunk) timed pairs; per-pair speedup = t_ref/t_opt; the pair");
    println!("verdict = MEDIAN of the 15; chunk = iters launch rounds + ONE device sync;");
    println!("iters calibrated once per pair on the ref arm to a >= 2 ms chunk (clamp");
    println!("1..=10000), the SAME iters handed to the opt arm; buffers created once per");
    println!("pair (zero allocation inside timed regions); correctness gate BEFORE timing.");
    println!();
    println!("Loud-zero defense (GPU shape): a chunk timing 0 ms FAILS the pair; inputs are");
    println!("deterministic (per-pair LCG seed = fnv1a64(pair_id) ^ 0x5EED4B4733202607); after");
    println!("timing, the pair's correctness gate re-runs to catch state corruption; the");
    println!("per-chunk launch count is std::hint::black_box'ed. NO autotune (launch_unchecked");
    println!("only).");
    println!();
    println!("Posture: --release only (a debug build warns; its numbers are void).");
    println!();
    println!("Pairs: {}", pairs::PAIRS.len());
}

#[cfg(not(feature = "cuda_runtime"))]
fn init_client() -> Client {
    use cubecl::wgpu::{AutoGraphicsApi, RuntimeOptions, WgpuDevice, WgpuRuntime, init_setup};
    // House law (riir-infer cubecl_runtime.rs `new_uncached`): init_setup
    // registers the wgpu server; the client resolves off the same device.
    // Headroom for the many launches a chunk performs between syncs.
    if std::env::var("CUBECL_WGPU_MAX_TASKS").is_err() {
        std::env::set_var("CUBECL_WGPU_MAX_TASKS", "1024");
    }
    let device = WgpuDevice::DefaultDevice;
    let _ = init_setup::<AutoGraphicsApi>(&device, RuntimeOptions::default());
    WgpuRuntime::client(&device)
}

#[cfg(feature = "cuda_runtime")]
fn init_client() -> Client {
    let device = cubecl::cuda::CudaDevice::new(0);
    cubecl::cuda::CudaRuntime::client(&device)
}

fn print_provenance(client: &Client) {
    println!("runtime: {}", ActiveRuntime::name(client));
    let props = client.properties();
    println!("device.name: {}", props.identity.name);
    println!("device.fingerprint: {}", props.identity.fingerprint);
    println!("hw.plane_size_max: {}", props.hardware.plane_size_max);
    println!("hw.max_units_per_cube: {}", props.hardware.max_units_per_cube);
    println!("hw.max_cube_dim: {:?}", props.hardware.max_cube_dim);
    println!("hw.max_cube_count: {:?}", props.hardware.max_cube_count);
    println!(
        "hw.max_shared_memory_size: {}",
        props.hardware.max_shared_memory_size
    );
    println!(
        "hw.num_streaming_multiprocessors: {:?}",
        props.hardware.num_streaming_multiprocessors
    );
    println!("mem.max_page_size: {}", props.memory.max_page_size);
    println!("mem.alignment: {}", props.memory.alignment);
    println!(
        "BENCH_SOURCES_COMMIT={}",
        std::env::var("BENCH_SOURCES_COMMIT").unwrap_or_else(|_| String::from("<unset>"))
    );
    println!(
        "BENCH_BOX_LABEL={}",
        std::env::var("BENCH_BOX_LABEL").unwrap_or_else(|_| String::from("<unset>"))
    );
}

fn parse_pairs(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut correctness = false;
    let mut bench = false;
    let mut want_provenance = false;
    let mut iters_override: Option<u32> = None;
    let mut pair_filter: Option<Vec<String>> = None;
    let mut bad = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--correctness" => correctness = true,
            "--bench" => bench = true,
            "--provenance" => want_provenance = true,
            "--pairs" => {
                i += 1;
                match args.get(i) {
                    Some(v) => pair_filter = Some(parse_pairs(v)),
                    None => {
                        eprintln!("--pairs needs a value");
                        bad = true;
                    }
                }
            }
            "--iters" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(n) => iters_override = Some(n),
                    None => {
                        eprintln!("--iters needs a u32");
                        bad = true;
                    }
                }
            }
            other => {
                eprintln!("unknown arg: {other}");
                bad = true;
            }
        }
        i += 1;
    }
    if bad || (!correctness && !bench && !want_provenance) {
        if !bad {
            eprintln!("nothing to do: pass --correctness and/or --bench (see --help)");
        }
        std::process::exit(2);
    }
    if cfg!(debug_assertions) {
        eprintln!(
            "WARNING: debug_assertions is ON — dev build; benchmark numbers from it are void (--release is the posture)."
        );
    }
    let client = init_client();
    probe::run_all(&client);
    if want_provenance {
        print_provenance(&client);
    }
    let selected: Vec<&pairs::PairDef> = PAIRS
        .iter()
        .filter(|p| {
            pair_filter
                .as_ref()
                .is_none_or(|f| f.iter().any(|x| x.eq_ignore_ascii_case(p.id)))
        })
        .collect();
    if selected.is_empty() {
        eprintln!("no pairs selected");
        std::process::exit(2);
    }

    if correctness {
        let mut failed: Vec<&str> = Vec::new();
        for p in &selected {
            match (p.correctness)(&client) {
                Ok(()) => println!("PASS {}", p.id),
                Err(e) => {
                    println!("FAIL {} :: {}", p.id, e);
                    failed.push(p.id);
                }
            }
        }
        println!(
            "--correctness: {} passed / {} failed / {} selected",
            selected.len() - failed.len(),
            failed.len(),
            selected.len()
        );
        if !failed.is_empty() {
            std::process::exit(1);
        }
    }

    if bench {
        println!("id            t_ref_ms     t_opt_ms  median_speedup   iters  status");
        let mut speedups: Vec<f64> = Vec::new();
        let mut failed: Vec<&str> = Vec::new();
        for p in &selected {
            // correctness gate BEFORE timing (Bench 130 law: a wrong kernel
            // cannot win).
            if let Err(e) = (p.correctness)(&client) {
                println!(
                    "{:<8} {:>12} {:>12} {:>14} {:>7}  {}",
                    p.id,
                    "-",
                    "-",
                    "-",
                    "-",
                    format!("FAIL-GATE: {e}")
                );
                failed.push(p.id);
                continue;
            }
            match (p.bench)(&client, iters_override) {
                Ok((tr, to, sp, it)) => {
                    println!(
                        "{:<8} {:>12.4} {:>12.4} {:>14.4} {:>7}  ok",
                        p.id, tr, to, sp, it
                    );
                    speedups.push(sp);
                }
                Err(e) => {
                    println!(
                        "{:<8} {:>12} {:>12} {:>14} {:>7}  {}",
                        p.id, "-", "-", "-", "-", format!("FAIL: {e}")
                    );
                    failed.push(p.id);
                }
            }
        }
        if !speedups.is_empty() {
            let leg = common::median(&mut speedups);
            println!(
                "leg median over {} measured pairs: {:.4} (G2 pass bar 1.06; MIN_MEASURED_PAIRS=12 enforced by the 4090 runner)",
                speedups.len(),
                leg
            );
        } else {
            println!("leg median: UNSEEN — zero measured pairs");
        }
        if !failed.is_empty() {
            println!("failed/gated pairs ({}): {}", failed.len(), failed.join(", "));
        }
        if speedups.len() < 12 {
            println!(
                "NOTE: {} < MIN_MEASURED_PAIRS(12) — the leg reads UNSEEN, never a pass",
                speedups.len()
            );
        }
    }
}

`````

### File: `src/pairs.rs`

blake3: `fbd44b1f5e7d3a755c546839e33085fb630af0fca3078b62629c83dbacf0cacf`

`````rust
//! The pair registry — the Bench 130 pre-registration pair table, verbatim.
//! Note: the pre-registration prose says "21 named pairs" while its own
//! table (and the task spec) carry 23 rows; all 23 rows are implemented and
//! registered here.

use crate::common::Client;

/// (t_ref_ms, t_opt_ms, median_speedup, calibrated_iters)
pub type BenchResult = Result<(f64, f64, f64, u32), String>;

pub struct PairDef {
    pub id: &'static str,
    pub correctness: fn(&Client) -> Result<(), String>,
    pub bench: fn(&Client, Option<u32>) -> BenchResult,
}

use crate::pairs_gemm::*;
use crate::pairs_reduce::*;
use crate::pairs_special::*;
use crate::pairs_stream::*;

pub const PAIRS: &[PairDef] = &[
    PairDef { id: "P117-A", correctness: p117a_correctness, bench: p117a_bench },
    PairDef { id: "P117-B", correctness: p117b_correctness, bench: p117b_bench },
    PairDef { id: "P117-C", correctness: p117c_correctness, bench: p117c_bench },
    PairDef { id: "P125-A", correctness: p125a_correctness, bench: p125a_bench },
    PairDef { id: "P111-A", correctness: p111a_correctness, bench: p111a_bench },
    PairDef { id: "P75-A", correctness: p75a_correctness, bench: p75a_bench },
    PairDef { id: "P111-B", correctness: p111b_correctness, bench: p111b_bench },
    PairDef { id: "P136-B", correctness: p136b_correctness, bench: p136b_bench },
    PairDef { id: "P138-A", correctness: p138a_correctness, bench: p138a_bench },
    PairDef { id: "P110-A", correctness: p110a_correctness, bench: p110a_bench },
    PairDef { id: "P133-A", correctness: p133a_correctness, bench: p133a_bench },
    PairDef { id: "P134-A", correctness: p134a_correctness, bench: p134a_bench },
    PairDef { id: "P134-B", correctness: p134b_correctness, bench: p134b_bench },
    PairDef { id: "P112-A", correctness: p112a_correctness, bench: p112a_bench },
    PairDef { id: "P112-B", correctness: p112b_correctness, bench: p112b_bench },
    PairDef { id: "P138-B", correctness: p138b_correctness, bench: p138b_bench },
    PairDef { id: "P75-B", correctness: p75b_correctness, bench: p75b_bench },
    PairDef { id: "P75-C", correctness: p75c_correctness, bench: p75c_bench },
    PairDef { id: "P136-A", correctness: p136a_correctness, bench: p136a_bench },
    PairDef { id: "P137-A", correctness: p137a_correctness, bench: p137a_bench },
    PairDef { id: "P137-B", correctness: p137b_correctness, bench: p137b_bench },
    PairDef { id: "P135-A", correctness: p135a_correctness, bench: p135a_bench },
    PairDef { id: "P121-A", correctness: p121a_correctness, bench: p121a_bench },
];

`````

### File: `src/pairs_gemm.rs`

blake3: `c46df043ebcd9907d23188c3c0311f6564795ee06b8c1460988d378ab7bca269`

`````rust
//! GEMM-shaped pairs: P117-A (dispatch fusion), P117-B (split-K), P117-C
//! (register tile), P125-A (operand-role swap), P111-A (weight-panel smem
//! share), P75-A (resident accumulator).
//!
//! Geometry constants are the Bench 130 pre-registration pins. Tolerances
//! per pair; every pair = two kernels (or kernel sets) implementing the
//! SAME math under the named optimization pattern.


// The #[cube] kernel bodies deliberately use explicit index arithmetic and
// nested ifs (the house convention, e.g. riir-infer norms_cubecl.rs): clippy's
// assign_op_pattern / collapsible_if rewrites target shapes the cubecl macro
// expansion has not proven on indexed places / short-circuit chains.
#![allow(clippy::assign_op_pattern, clippy::collapsible_if)]

use crate::common::{
    ActiveRuntime, empty_f32, interleave_bench, read_f32, rel_linf, seed_for_pair, sync_ok, upload_f32, Client,
    Lcg,
};
use cubecl::prelude::*;
use cubecl::server::Handle;

// ── shared kernels ─────────────────────────────────────────────────────

/// Naive matmul: one thread per output, k serial ascending.
#[cube(launch_unchecked)]
fn mm_naive(a: &[f32], b: &[f32], c: &mut [f32], m: u32, n: u32, k: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    if row >= m || col >= n {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        acc += a[(row * k + kk) as usize] * b[(kk * n + col) as usize];
        kk += 1u32;
    }
    c[(row * n + col) as usize] = acc;
}

// ── P117-A dispatch fusion ─────────────────────────────────────────────
// ref: TWO naive launches (A·W1 → O1 then A·W2 → O2), 16×16 threadgroups.
// opt: ONE launch `mm_concat` writing the concatenated [O1|O2] tiles,
//      32×16 threadgroups spanning the N1 boundary, reading ONE
//      concatenated weight buffer w_cat = [W1|W2] ([K][N1+N2]) — the
//      branchless realization of "input read once" (the first cut picked
//      w1/w2 by an in-k-loop branch, which codegen could not hoist and
//      which cost the fusion its win; disclosed in the run record).
// Geometry: M=1024 K=1024 N1=N2=512. Tolerance: rel L∞ 1e-3.
// DEVIATION (pre-registration tile hints 16×16 / 16×32): realized as the
// threadgroup SHAPES above with thread-per-output bodies — the measured
// axis is launch count + the consolidated weight stream, per the task
// text ("naive matmul kernel" on the ref side).

#[cube(launch_unchecked)]
fn mm_concat(a: &[f32], w_cat: &[f32], out: &mut [f32], m: u32, k: u32, n_cat: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    if row >= m || col >= n_cat {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        acc += a[(row * k + kk) as usize] * w_cat[(kk * n_cat + col) as usize];
        kk += 1u32;
    }
    out[(row * n_cat + col) as usize] = acc;
}

pub struct P117A {
    a: Handle,
    w1: Handle,
    w2: Handle,
    w_cat: Handle,
    o1: Handle,
    o2: Handle,
    o_cat: Handle,
    m: u32,
    k: u32,
    n1: u32,
    n2: u32,
}

impl P117A {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, k, n1, n2) = (1024usize, 1024usize, 512usize, 512usize);
        let mut rng = Lcg::new(seed_for_pair("P117-A"));
        let a = upload_f32(client, &rng.fill(m * k, -0.5, 0.5));
        let w1v = rng.fill(k * n1, -0.5, 0.5);
        let w2v = rng.fill(k * n2, -0.5, 0.5);
        let w1 = upload_f32(client, &w1v);
        let w2 = upload_f32(client, &w2v);
        // the opt arm's single concatenated weight buffer: row k = [W1 row k | W2 row k]
        let mut w_cat_v = vec![0.0f32; k * (n1 + n2)];
        for kk in 0..k {
            w_cat_v[kk * (n1 + n2)..kk * (n1 + n2) + n1].copy_from_slice(&w1v[kk * n1..(kk + 1) * n1]);
            w_cat_v[kk * (n1 + n2) + n1..(kk + 1) * (n1 + n2)].copy_from_slice(&w2v[kk * n2..(kk + 1) * n2]);
        }
        let w_cat = upload_f32(client, &w_cat_v);
        let o1 = empty_f32(client, m * n1);
        let o2 = empty_f32(client, m * n2);
        let o_cat = empty_f32(client, m * (n1 + n2));
        Ok(Self {
            a,
            w1,
            w2,
            w_cat,
            o1,
            o2,
            o_cat,
            m: m as u32,
            k: k as u32,
            n1: n1 as u32,
            n2: n2 as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let bn1 = self.n1.div_ceil(16u32);
        let bn2 = self.n2.div_ceil(16u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_naive::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn1, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.w1.clone(), (self.k * self.n1) as usize),
                BufferArg::from_raw_parts(self.o1.clone(), (self.m * self.n1) as usize),
                self.m,
                self.n1,
                self.k,
            );
            mm_naive::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn2, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.w2.clone(), (self.k * self.n2) as usize),
                BufferArg::from_raw_parts(self.o2.clone(), (self.m * self.n2) as usize),
                self.m,
                self.n2,
                self.k,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let bn = (self.n1 + self.n2).div_ceil(32u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_concat::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(32, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.w_cat.clone(), (self.k * (self.n1 + self.n2)) as usize),
                BufferArg::from_raw_parts(self.o_cat.clone(), (self.m * (self.n1 + self.n2)) as usize),
                self.m,
                self.k,
                self.n1 + self.n2,
            );
        }
    }
}

pub fn p117a_correctness(client: &Client) -> Result<(), String> {
    let s = P117A::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let o1 = read_f32(client, &s.o1)?;
    let o2 = read_f32(client, &s.o2)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let cat = read_f32(client, &s.o_cat)?;
    let m = s.m as usize;
    let n1 = s.n1 as usize;
    let n2 = s.n2 as usize;
    let mut expected = Vec::with_capacity(m * (n1 + n2));
    for row in 0..m {
        expected.extend_from_slice(&o1[row * n1..(row + 1) * n1]);
        expected.extend_from_slice(&o2[row * n2..(row + 1) * n2]);
    }
    rel_linf(&cat, &expected, 1e-3)
}

pub fn p117a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p117a_correctness(client)?;
    let s = P117A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    // state-corruption re-check (harness law 4): the correctness gate again
    p117a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P117-B split-K ─────────────────────────────────────────────────────
// ref: mm_naive over K=8192 (k serial in-thread). opt: split-K over 16
//      k-partitions (grid z), partials into P[16][M*N], one drain kernel
//      sums once. M=N=1024. Tolerance: rel L∞ 1e-3 (same k-ascending
//      association → expect bit-exact).

#[cube(launch_unchecked)]
fn mm_splitk_partial(a: &[f32], b: &[f32], p: &mut [f32], m: u32, n: u32, kt: u32, kc: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    let z = CUBE_POS_Z;
    if row >= m || col >= n {
        terminate!();
    }
    let k0 = z * kc;
    let mut acc = f32::new(0.0f32);
    let mut e = 0u32;
    while e < kc {
        acc += a[(row * kt + k0 + e) as usize] * b[((k0 + e) * n + col) as usize];
        e += 1u32;
    }
    let pi = z * m * n + row * n + col;
    p[pi as usize] = acc;
}

#[cube(launch_unchecked)]
fn mm_splitk_drain(p: &[f32], c: &mut [f32], mn: u32, parts: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= mn {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut z = 0u32;
    while z < parts {
        acc += p[(z * mn + e) as usize];
        z += 1u32;
    }
    c[e as usize] = acc;
}

pub struct P117B {
    a: Handle,
    b: Handle,
    p: Handle,
    c_ref: Handle,
    c_opt: Handle,
    m: u32,
    n: u32,
    kt: u32,
}

impl P117B {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, kt) = (1024usize, 1024usize, 8192usize);
        let parts = 16usize;
        let mut rng = Lcg::new(seed_for_pair("P117-B"));
        let a = upload_f32(client, &rng.fill(m * kt, -0.5, 0.5));
        let b = upload_f32(client, &rng.fill(kt * n, -0.5, 0.5));
        let p = empty_f32(client, parts * m * n);
        let c_ref = empty_f32(client, m * n);
        let c_opt = empty_f32(client, m * n);
        Ok(Self {
            a,
            b,
            p,
            c_ref,
            c_opt,
            m: m as u32,
            n: n as u32,
            kt: kt as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let bn = self.n.div_ceil(16u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_naive::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.kt) as usize),
                BufferArg::from_raw_parts(self.b.clone(), (self.kt * self.n) as usize),
                BufferArg::from_raw_parts(self.c_ref.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.kt,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let parts = 16u32;
        let kc = self.kt / parts;
        let bn = self.n.div_ceil(32u32);
        let bm = self.m.div_ceil(32u32);
        let mn = self.m * self.n;
        let drain_blocks = mn.div_ceil(256u32);
        unsafe {
            mm_splitk_partial::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, parts),
                CubeDim::new_2d(32, 32),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.kt) as usize),
                BufferArg::from_raw_parts(self.b.clone(), (self.kt * self.n) as usize),
                BufferArg::from_raw_parts(self.p.clone(), (16 * self.m * self.n) as usize),
                self.m,
                self.n,
                self.kt,
                kc,
            );
            mm_splitk_drain::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(drain_blocks, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.p.clone(), (16 * self.m * self.n) as usize),
                BufferArg::from_raw_parts(self.c_opt.clone(), mn as usize),
                mn,
                parts,
            );
        }
    }
}

pub fn p117b_correctness(client: &Client) -> Result<(), String> {
    let s = P117B::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let c_ref = read_f32(client, &s.c_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let c_opt = read_f32(client, &s.c_opt)?;
    rel_linf(&c_opt, &c_ref, 1e-3)
}

pub fn p117b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p117b_correctness(client)?;
    let s = P117B::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p117b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P117-C register tile ───────────────────────────────────────────────
// ref: mm_naive (1 thread/output) over 2048³. opt: `mm_rowtile` — a thread
//      computes TILE_N=32 consecutive outputs of one row from 32 register
//      accumulators, k-loop reading 32 consecutive W elements (coalesced).
// Geometry: 2048×2048×2048. Tolerance: rel L∞ 1e-3.
// DEVIATION ("4 independent accumulators for ILP"): realized as the
// 32-output register tile whose FMAs target 32 distinct registers (≥ the
// pinned 4 independent FMA chains — the ILP axis preserved).

#[cube(launch_unchecked)]
fn mm_rowtile(a: &[f32], b: &[f32], c: &mut [f32], m: u32, n: u32, k: u32) {
    // 128 threads/block = 2 rows × 64 n-groups; thread computes 32
    // consecutive outputs of its row.
    let t = UNIT_POS;
    let row = CUBE_POS_X * 2u32 + (t >> 6u32);
    let n0 = (t & 63u32) << 5u32;
    if row >= m {
        terminate!();
    }
    let a_base = (row * k) as usize;
    let c_row = row * n + n0;
    let mut r0 = f32::new(0.0f32);
    let mut r1 = f32::new(0.0f32);
    let mut r2 = f32::new(0.0f32);
    let mut r3 = f32::new(0.0f32);
    let mut r4 = f32::new(0.0f32);
    let mut r5 = f32::new(0.0f32);
    let mut r6 = f32::new(0.0f32);
    let mut r7 = f32::new(0.0f32);
    let mut r8 = f32::new(0.0f32);
    let mut r9 = f32::new(0.0f32);
    let mut r10 = f32::new(0.0f32);
    let mut r11 = f32::new(0.0f32);
    let mut r12 = f32::new(0.0f32);
    let mut r13 = f32::new(0.0f32);
    let mut r14 = f32::new(0.0f32);
    let mut r15 = f32::new(0.0f32);
    let mut r16 = f32::new(0.0f32);
    let mut r17 = f32::new(0.0f32);
    let mut r18 = f32::new(0.0f32);
    let mut r19 = f32::new(0.0f32);
    let mut r20 = f32::new(0.0f32);
    let mut r21 = f32::new(0.0f32);
    let mut r22 = f32::new(0.0f32);
    let mut r23 = f32::new(0.0f32);
    let mut r24 = f32::new(0.0f32);
    let mut r25 = f32::new(0.0f32);
    let mut r26 = f32::new(0.0f32);
    let mut r27 = f32::new(0.0f32);
    let mut r28 = f32::new(0.0f32);
    let mut r29 = f32::new(0.0f32);
    let mut r30 = f32::new(0.0f32);
    let mut r31 = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        let av = a[a_base + kk as usize];
        let b_row = kk * n + n0;
        r0 += av * b[b_row as usize];
        r1 += av * b[(b_row + 1u32) as usize];
        r2 += av * b[(b_row + 2u32) as usize];
        r3 += av * b[(b_row + 3u32) as usize];
        r4 += av * b[(b_row + 4u32) as usize];
        r5 += av * b[(b_row + 5u32) as usize];
        r6 += av * b[(b_row + 6u32) as usize];
        r7 += av * b[(b_row + 7u32) as usize];
        r8 += av * b[(b_row + 8u32) as usize];
        r9 += av * b[(b_row + 9u32) as usize];
        r10 += av * b[(b_row + 10u32) as usize];
        r11 += av * b[(b_row + 11u32) as usize];
        r12 += av * b[(b_row + 12u32) as usize];
        r13 += av * b[(b_row + 13u32) as usize];
        r14 += av * b[(b_row + 14u32) as usize];
        r15 += av * b[(b_row + 15u32) as usize];
        r16 += av * b[(b_row + 16u32) as usize];
        r17 += av * b[(b_row + 17u32) as usize];
        r18 += av * b[(b_row + 18u32) as usize];
        r19 += av * b[(b_row + 19u32) as usize];
        r20 += av * b[(b_row + 20u32) as usize];
        r21 += av * b[(b_row + 21u32) as usize];
        r22 += av * b[(b_row + 22u32) as usize];
        r23 += av * b[(b_row + 23u32) as usize];
        r24 += av * b[(b_row + 24u32) as usize];
        r25 += av * b[(b_row + 25u32) as usize];
        r26 += av * b[(b_row + 26u32) as usize];
        r27 += av * b[(b_row + 27u32) as usize];
        r28 += av * b[(b_row + 28u32) as usize];
        r29 += av * b[(b_row + 29u32) as usize];
        r30 += av * b[(b_row + 30u32) as usize];
        r31 += av * b[(b_row + 31u32) as usize];
        kk += 1u32;
    }
    c[c_row as usize] = r0;
    c[(c_row + 1u32) as usize] = r1;
    c[(c_row + 2u32) as usize] = r2;
    c[(c_row + 3u32) as usize] = r3;
    c[(c_row + 4u32) as usize] = r4;
    c[(c_row + 5u32) as usize] = r5;
    c[(c_row + 6u32) as usize] = r6;
    c[(c_row + 7u32) as usize] = r7;
    c[(c_row + 8u32) as usize] = r8;
    c[(c_row + 9u32) as usize] = r9;
    c[(c_row + 10u32) as usize] = r10;
    c[(c_row + 11u32) as usize] = r11;
    c[(c_row + 12u32) as usize] = r12;
    c[(c_row + 13u32) as usize] = r13;
    c[(c_row + 14u32) as usize] = r14;
    c[(c_row + 15u32) as usize] = r15;
    c[(c_row + 16u32) as usize] = r16;
    c[(c_row + 17u32) as usize] = r17;
    c[(c_row + 18u32) as usize] = r18;
    c[(c_row + 19u32) as usize] = r19;
    c[(c_row + 20u32) as usize] = r20;
    c[(c_row + 21u32) as usize] = r21;
    c[(c_row + 22u32) as usize] = r22;
    c[(c_row + 23u32) as usize] = r23;
    c[(c_row + 24u32) as usize] = r24;
    c[(c_row + 25u32) as usize] = r25;
    c[(c_row + 26u32) as usize] = r26;
    c[(c_row + 27u32) as usize] = r27;
    c[(c_row + 28u32) as usize] = r28;
    c[(c_row + 29u32) as usize] = r29;
    c[(c_row + 30u32) as usize] = r30;
    c[(c_row + 31u32) as usize] = r31;
}

pub struct P117C {
    a: Handle,
    b: Handle,
    c_ref: Handle,
    c_opt: Handle,
    m: u32,
    n: u32,
    k: u32,
}

impl P117C {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, k) = (2048usize, 2048usize, 2048usize);
        let mut rng = Lcg::new(seed_for_pair("P117-C"));
        let a = upload_f32(client, &rng.fill(m * k, -0.5, 0.5));
        let b = upload_f32(client, &rng.fill(k * n, -0.5, 0.5));
        let c_ref = empty_f32(client, m * n);
        let c_opt = empty_f32(client, m * n);
        Ok(Self {
            a,
            b,
            c_ref,
            c_opt,
            m: m as u32,
            n: n as u32,
            k: k as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let bn = self.n.div_ceil(16u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_naive::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.b.clone(), (self.k * self.n) as usize),
                BufferArg::from_raw_parts(self.c_ref.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.k,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let blocks = self.m.div_ceil(2u32);
        unsafe {
            mm_rowtile::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks, 1, 1),
                CubeDim::new_1d(128),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.b.clone(), (self.k * self.n) as usize),
                BufferArg::from_raw_parts(self.c_opt.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.k,
            );
        }
    }
}

pub fn p117c_correctness(client: &Client) -> Result<(), String> {
    let s = P117C::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let c_ref = read_f32(client, &s.c_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let c_opt = read_f32(client, &s.c_opt)?;
    rel_linf(&c_opt, &c_ref, 1e-3)
}

pub fn p117c_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p117c_correctness(client)?;
    let s = P117C::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p117c_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P125-A operand-role swap ───────────────────────────────────────────
// ref: mm_naive reading B[K,N] (b[k][j]) — the column-strided stream.
// opt: the swapped formulation computing Cᵀ[N,M]: thread per (j,i) reads
//      B[j][k] rows coalesced, writes Cᵀ[j*M+i]; compare opt[j*M+i] vs
//      ref[i*N+j] (the layout-legality mapping). 1024³.
// Tolerance: rel L∞ 1e-3. DISCLOSED: if this measures ≈1.0 that is an
// honest possible outcome — the swap moves the strided stream from B's
// columns to A's columns; the layout-legality claim is what is priced.

#[cube(launch_unchecked)]
fn mm_swap(a: &[f32], bt: &[f32], ct: &mut [f32], m: u32, n: u32, k: u32) {
    // bt = B stored [N, K] row-major (b[j][k]); ct = Cᵀ [N, M].
    let j = ABSOLUTE_POS_X;
    let i = ABSOLUTE_POS_Y;
    if i >= m || j >= n {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        acc += bt[(j * k + kk) as usize] * a[(i * k + kk) as usize];
        kk += 1u32;
    }
    ct[(j * m + i) as usize] = acc;
}

pub struct P125A {
    a: Handle,
    b_kn: Handle,
    b_nk: Handle,
    c_ref: Handle,
    ct_opt: Handle,
    m: u32,
    n: u32,
    k: u32,
}

impl P125A {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, k) = (1024usize, 1024usize, 1024usize);
        let mut rng = Lcg::new(seed_for_pair("P125-A"));
        let a = upload_f32(client, &rng.fill(m * k, -0.5, 0.5));
        let b_logical = rng.fill(k * n, -0.5, 0.5); // b[k][j]
        let b_kn = upload_f32(client, &b_logical);
        let mut b_nk_vec = vec![0.0f32; n * k]; // b[j][k]
        for kk in 0..k {
            for j in 0..n {
                b_nk_vec[j * k + kk] = b_logical[kk * n + j];
            }
        }
        let b_nk = upload_f32(client, &b_nk_vec);
        let c_ref = empty_f32(client, m * n);
        let ct_opt = empty_f32(client, n * m);
        Ok(Self {
            a,
            b_kn,
            b_nk,
            c_ref,
            ct_opt,
            m: m as u32,
            n: n as u32,
            k: k as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let bn = self.n.div_ceil(16u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_naive::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.b_kn.clone(), (self.k * self.n) as usize),
                BufferArg::from_raw_parts(self.c_ref.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.k,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let bn = self.n.div_ceil(16u32);
        let bm = self.m.div_ceil(16u32);
        unsafe {
            mm_swap::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(16, 16),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.b_nk.clone(), (self.n * self.k) as usize),
                BufferArg::from_raw_parts(self.ct_opt.clone(), (self.n * self.m) as usize),
                self.m,
                self.n,
                self.k,
            );
        }
    }
}

pub fn p125a_correctness(client: &Client) -> Result<(), String> {
    let s = P125A::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let c_ref = read_f32(client, &s.c_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let ct = read_f32(client, &s.ct_opt)?;
    // index-mapped compare: opt[j*m + i] vs ref[i*n + j]
    let m = s.m as usize;
    let n = s.n as usize;
    let mut expected = vec![0.0f32; n * m];
    for i in 0..m {
        for j in 0..n {
            expected[j * m + i] = c_ref[i * n + j];
        }
    }
    rel_linf(&ct, &expected, 1e-3)
}

pub fn p125a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p125a_correctness(client)?;
    let s = P125A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p125a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P111-A weight-panel smem share ─────────────────────────────────────
// ref: block per 128-row tile; W panel [K=1024, N=128] re-read from global
//      for every row (128× re-stream). opt: W staged into Shared once per
//      64-row k-subtile (64×128 f32 = 32 KiB — exactly the M3's reported
//      32768 B cap), reused across the tile's 128 rows. M=4096.
// Tolerance: rel L∞ 1e-3 (k-ascending association both arms → expect
// bit-exact). Fallback twin kernel with k-sub 32 (16 KiB) picks at setup
// only if the device reports < 32768 B shared memory (disclosed).
// NOTE: grids are exact multiples of the block shapes — no thread may
// terminate before a barrier, so the smem kernels carry no guards.

static P111A_NOTE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

#[cube(launch_unchecked)]
fn panel_share_ref(a: &[f32], w: &[f32], out: &mut [f32], _m: u32, n: u32, k: u32, rpb: u32) {
    let col = UNIT_POS; // one thread per column (n == 128 == thread count)
    let row0 = CUBE_POS_X * rpb;
    let mut r = 0u32;
    while r < rpb {
        let row = row0 + r;
        let mut acc = f32::new(0.0f32);
        let mut kk = 0u32;
        while kk < k {
            acc += a[(row * k + kk) as usize] * w[(kk * n + col) as usize];
            kk += 1u32;
        }
        out[(row * n + col) as usize] = acc;
        r += 1u32;
    }
}

#[cube(launch_unchecked)]
fn panel_share_opt(a: &[f32], w: &[f32], out: &mut [f32], _m: u32, n: u32, k: u32, rpb: u32) {
    // k-sub 64: Shared = 64×128 f32 = 32 KiB.
    let col = UNIT_POS;
    let row0 = CUBE_POS_X * rpb;
    let mut smem = Shared::<[f32]>::new_slice(8192usize);
    let mut ks = 0u32;
    while ks < k {
        // stage W[ks..ks+64][0..128) coalesced (n == 128 pinned)
        let w_base = ks * n;
        let mut e = 0u32;
        while e < 64u32 {
            smem[(e * 128u32 + col) as usize] = w[(w_base + e * 128u32 + col) as usize];
            e += 1u32;
        }
        sync_cube();
        let mut r = 0u32;
        while r < rpb {
            let row = row0 + r;
            let mut acc = f32::new(0.0f32);
            let mut ee = 0u32;
            while ee < 64u32 {
                acc += a[(row * k + ks + ee) as usize] * smem[(ee * 128u32 + col) as usize];
                ee += 1u32;
            }
            let oi = (row * n + col) as usize;
            out[oi] = out[oi] + acc;
            r += 1u32;
        }
        sync_cube();
        ks += 64u32;
    }
}

#[cube(launch_unchecked)]
fn panel_share_opt32(a: &[f32], w: &[f32], out: &mut [f32], _m: u32, n: u32, k: u32, rpb: u32) {
    // k-sub 32 fallback: Shared = 32×128 f32 = 16 KiB (device < 32 KiB).
    let col = UNIT_POS;
    let row0 = CUBE_POS_X * rpb;
    let mut smem = Shared::<[f32]>::new_slice(4096usize);
    let mut ks = 0u32;
    while ks < k {
        let w_base = ks * n;
        let mut e = 0u32;
        while e < 32u32 {
            smem[(e * 128u32 + col) as usize] = w[(w_base + e * 128u32 + col) as usize];
            e += 1u32;
        }
        sync_cube();
        let mut r = 0u32;
        while r < rpb {
            let row = row0 + r;
            let mut acc = f32::new(0.0f32);
            let mut ee = 0u32;
            while ee < 32u32 {
                acc += a[(row * k + ks + ee) as usize] * smem[(ee * 128u32 + col) as usize];
                ee += 1u32;
            }
            let oi = (row * n + col) as usize;
            out[oi] = out[oi] + acc;
            r += 1u32;
        }
        sync_cube();
        ks += 32u32;
    }
}

pub struct P111A {
    a: Handle,
    w: Handle,
    out_ref: Handle,
    out_opt: Handle,
    m: u32,
    n: u32,
    k: u32,
    rpb: u32,
    use32: bool,
}

impl P111A {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, k) = (4096usize, 128usize, 1024usize);
        let rpb = 128usize;
        let mut rng = Lcg::new(seed_for_pair("P111-A"));
        let a = upload_f32(client, &rng.fill(m * k, -0.5, 0.5));
        let w = upload_f32(client, &rng.fill(k * n, -0.5, 0.5));
        let out_ref = empty_f32(client, m * n); // full overwrite per row
        // zero-init: the opt kernel ACCUMULATES across k-subtiles (+=)
        let out_opt = upload_f32(client, &vec![0.0f32; m * n]);
        let max_smem = client.properties().hardware.max_shared_memory_size;
        let use32 = max_smem < 32768;
        if use32 {
            P111A_NOTE.get_or_init(|| {
                println!(
                    "DEVIATION P111-A: device max shared memory = {max_smem} B < 32768 B — the \
                     pinned k-sub 64 (32 KiB panel stage) shrunk to k-sub 32 (16 KiB); disclosed, \
                     geometry otherwise per registration."
                );
            });
        }
        Ok(Self {
            a,
            w,
            out_ref,
            out_opt,
            m: m as u32,
            n: n as u32,
            k: k as u32,
            rpb: rpb as u32,
            use32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let blocks = self.m / self.rpb; // 32 — exact
        unsafe {
            panel_share_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks, 1, 1),
                CubeDim::new_1d(128),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                BufferArg::from_raw_parts(self.w.clone(), (self.k * self.n) as usize),
                BufferArg::from_raw_parts(self.out_ref.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.k,
                self.rpb,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let blocks = self.m / self.rpb; // 32 — exact
        unsafe {
            if self.use32 {
                panel_share_opt32::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(128),
                    BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                    BufferArg::from_raw_parts(self.w.clone(), (self.k * self.n) as usize),
                    BufferArg::from_raw_parts(self.out_opt.clone(), (self.m * self.n) as usize),
                    self.m,
                    self.n,
                    self.k,
                    self.rpb,
                );
            } else {
                panel_share_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(128),
                    BufferArg::from_raw_parts(self.a.clone(), (self.m * self.k) as usize),
                    BufferArg::from_raw_parts(self.w.clone(), (self.k * self.n) as usize),
                    BufferArg::from_raw_parts(self.out_opt.clone(), (self.m * self.n) as usize),
                    self.m,
                    self.n,
                    self.k,
                    self.rpb,
                );
            }
        }
    }
}

pub fn p111a_correctness(client: &Client) -> Result<(), String> {
    let s = P111A::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-3)
}

pub fn p111a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p111a_correctness(client)?;
    let s = P111A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p111a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P75-A resident accumulator ─────────────────────────────────────────
// M=N=1024 K=512, tile 32×32, k split into 16 chunks of 32.
// ref: per k-chunk launch (16 launches/round): partial GEMM then
//      read-modify-write of C from global (32×32 loads+stores × 16).
// opt: ONE launch — all chunks accumulated in registers, one store.
// Tolerance: rel L∞ 1e-3 (k-ascending association both arms → expect
// bit-exact).

#[cube(launch_unchecked)]
fn resid_gemm_rmw(a: &[f32], b: &[f32], c: &mut [f32], m: u32, n: u32, kt: u32, kc: u32, k_off: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    if row >= m || col >= n {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut e = 0u32;
    while e < kc {
        acc += a[(row * kt + k_off + e) as usize] * b[((k_off + e) * n + col) as usize];
        e += 1u32;
    }
    let ci = (row * n + col) as usize;
    c[ci] = c[ci] + acc;
}

#[cube(launch_unchecked)]
fn resid_gemm_full(a: &[f32], b: &[f32], c: &mut [f32], m: u32, n: u32, kt: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    if row >= m || col >= n {
        terminate!();
    }
    let mut acc = f32::new(0.0f32);
    let mut e = 0u32;
    while e < kt {
        acc += a[(row * kt + e) as usize] * b[(e * n + col) as usize];
        e += 1u32;
    }
    c[(row * n + col) as usize] = acc;
}

pub struct P75A {
    a: Handle,
    b: Handle,
    c_ref: Handle,
    c_opt: Handle,
    m: u32,
    n: u32,
    kt: u32,
    parts: u32,
}

impl P75A {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, kt) = (1024usize, 1024usize, 512usize);
        let mut rng = Lcg::new(seed_for_pair("P75-A"));
        let a = upload_f32(client, &rng.fill(m * kt, -0.5, 0.5));
        let b = upload_f32(client, &rng.fill(kt * n, -0.5, 0.5));
        let c_ref = upload_f32(client, &vec![0.0f32; m * n]);
        let c_opt = empty_f32(client, m * n);
        Ok(Self {
            a,
            b,
            c_ref,
            c_opt,
            m: m as u32,
            n: n as u32,
            kt: kt as u32,
            parts: 16,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let bn = self.n.div_ceil(32u32);
        let bm = self.m.div_ceil(32u32);
        let kc = self.kt / self.parts;
        for z in 0..self.parts {
            let k_off = z * kc;
            unsafe {
                resid_gemm_rmw::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(bn, bm, 1),
                    CubeDim::new_2d(32, 32),
                    BufferArg::from_raw_parts(self.a.clone(), (self.m * self.kt) as usize),
                    BufferArg::from_raw_parts(self.b.clone(), (self.kt * self.n) as usize),
                    BufferArg::from_raw_parts(self.c_ref.clone(), (self.m * self.n) as usize),
                    self.m,
                    self.n,
                    self.kt,
                    kc,
                    k_off,
                );
            }
        }
    }

    fn launch_opt(&self, client: &Client) {
        let bn = self.n.div_ceil(32u32);
        let bm = self.m.div_ceil(32u32);
        unsafe {
            resid_gemm_full::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(bn, bm, 1),
                CubeDim::new_2d(32, 32),
                BufferArg::from_raw_parts(self.a.clone(), (self.m * self.kt) as usize),
                BufferArg::from_raw_parts(self.b.clone(), (self.kt * self.n) as usize),
                BufferArg::from_raw_parts(self.c_opt.clone(), (self.m * self.n) as usize),
                self.m,
                self.n,
                self.kt,
            );
        }
    }
}

pub fn p75a_correctness(client: &Client) -> Result<(), String> {
    let s = P75A::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let c_ref = read_f32(client, &s.c_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let c_opt = read_f32(client, &s.c_opt)?;
    rel_linf(&c_opt, &c_ref, 1e-3)
}

pub fn p75a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p75a_correctness(client)?;
    let s = P75A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p75a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

`````

### File: `src/pairs_reduce.rs`

blake3: `5c5b1ae2a43a117955ff07c246dedc5dc98c47e409345003c4ee0f7576f5adeb`

`````rust
//! Reduction-shaped pairs: P111-B (head-row smem stage vs atomic combine),
//! P136-B (quad-lane softmax / block-staged vs 3-pass scalar), P138-A
//! (statistic row-split), P110-A (softmax state exchange), P133-A (head-dim
//! smem tile cap).
//!
//! DEVIATION (P111-B, disclosed per the task fallback law): cubecl exposes
//! `Atomic<i32>` device atomics; float atomicAdd is not portable to Metal
//! (MSL has no device float atomic add), so both arms combine through i32
//! fixed point at scale 2^12 (truncation toward zero, applied to the same
//! per-thread partials in BOTH arms — integer addition is order-free, so
//  the arms compare BIT-EXACT). The measured axis stays atomic traffic vs
//! staged smem reduce. On the 4090 (CUDA) the same code runs unchanged;
//! f32-atomicAdd is available there and can be adjudicated as a follow-up.


// The #[cube] kernel bodies deliberately use explicit index arithmetic and
// nested ifs (the house convention, e.g. riir-infer norms_cubecl.rs): clippy's
// assign_op_pattern / collapsible_if rewrites target shapes the cubecl macro
// expansion has not proven on indexed places / short-circuit chains.
#![allow(clippy::assign_op_pattern, clippy::collapsible_if)]

use crate::common::{
    abs_linf, empty_f32, interleave_bench, i32_exact, read_f32, read_i32, rel_linf,
    seed_for_pair, sync_ok, upload_f32, upload_i32, ActiveRuntime, Client, Lcg,
};
use cubecl::prelude::*;
use cubecl::server::Handle;

// ── P111-B head-row combine ────────────────────────────────────────────
// seq=2048 head_dim=128, 64 blocks (32 rows each), 256 threads/block.
// d = t & 127, half = t >> 7: two threads per column split the block's
// rows by parity. ref: each thread atomicAdd's its fixed-point partial
// (2 atomics/d/block = 16384 total). opt: both partials staged in Shared,
// one pairwise combine, thread (half 0) does ONE atomicAdd per column
// (8192 total). Compare: i32 bit-exact.

#[cube(launch_unchecked)]
fn head_combine_ref(s: &[f32], acc: &[Atomic<i32>], rpb: u32, seq: u32, hd: u32) {
    let t = UNIT_POS;
    let d = t & 127u32;
    let half = t >> 7u32;
    let row0 = CUBE_POS_X * rpb;
    let mut partial = f32::new(0.0f32);
    let mut r = 0u32;
    while r < rpb {
        let row = row0 + r;
        if row < seq {
            if (r & 1u32) == half {
                partial += s[(row * hd + d) as usize];
            }
        }
        r += 1u32;
    }
    let q = (partial * 4096.0f32) as i32;
    acc[d as usize].fetch_add(q);
}

#[cube(launch_unchecked)]
fn head_combine_opt(s: &[f32], acc: &[Atomic<i32>], rpb: u32, seq: u32, hd: u32) {
    let t = UNIT_POS;
    let d = t & 127u32;
    let half = t >> 7u32;
    let row0 = CUBE_POS_X * rpb;
    let mut smem = Shared::<[f32]>::new_slice(256usize);
    let mut partial = f32::new(0.0f32);
    let mut r = 0u32;
    while r < rpb {
        let row = row0 + r;
        if row < seq {
            if (r & 1u32) == half {
                partial += s[(row * hd + d) as usize];
            }
        }
        r += 1u32;
    }
    let q = (partial * 4096.0f32) as i32;
    smem[t as usize] = q as f32;
    sync_cube();
    if half == 0u32 {
        let pair = smem[t as usize] + smem[(t + 128u32) as usize];
        acc[d as usize].fetch_add(pair as i32);
    }
}

pub struct P111B {
    s: Handle,
    acc_ref: Handle,
    acc_opt: Handle,
    seq: u32,
    hd: u32,
    rpb: u32,
}

impl P111B {
    fn new(client: &Client) -> Result<Self, String> {
        let (seq, hd) = (2048usize, 128usize);
        let mut rng = Lcg::new(seed_for_pair("P111-B"));
        let s = upload_f32(client, &rng.fill(seq * hd, -0.5, 0.5));
        let acc_ref = upload_i32(client, &vec![0i32; hd]);
        let acc_opt = upload_i32(client, &vec![0i32; hd]);
        Ok(Self {
            s,
            acc_ref,
            acc_opt,
            seq: seq as u32,
            hd: hd as u32,
            rpb: 32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let blocks = self.seq / self.rpb; // 64 — exact
        unsafe {
            head_combine_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.s.clone(), (self.seq * self.hd) as usize),
                BufferArg::from_raw_parts(self.acc_ref.clone(), self.hd as usize),
                self.rpb,
                self.seq,
                self.hd,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        let blocks = self.seq / self.rpb;
        unsafe {
            head_combine_opt::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.s.clone(), (self.seq * self.hd) as usize),
                BufferArg::from_raw_parts(self.acc_opt.clone(), self.hd as usize),
                self.rpb,
                self.seq,
                self.hd,
            );
        }
    }
}

pub fn p111b_correctness(client: &Client) -> Result<(), String> {
    let s = P111B::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let a_ref = read_i32(client, &s.acc_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let a_opt = read_i32(client, &s.acc_opt)?;
    i32_exact(&a_opt, &a_ref)
}

pub fn p111b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p111b_correctness(client)?;
    let s = P111B::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p111b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P136-B quad-lane softmax ───────────────────────────────────────────
// rows=4096 cols=512. ref: thread-per-row scalar softmax over 3 global
//      passes (max, exp-sum, normalize) — 3 kernels.
// opt: ONE block per row (512 threads = cols), x staged in Shared once,
//      block max (tree), exp+sum (tree), normalize, then the "16-bit
//      store" realized as an in-kernel f16 roundtrip (f32 -> f16 RNE ->
//      f32 via u32 bit ops) cast into the f32 out buffer.
// DEVIATION: the bandwidth axis is emulated (the store stays 32-bit); the
// measured axis is the reduction (task text). Tolerance: abs L∞ 3e-3
// (f16 rounding).
// DEVIATION (pre-registration "quad-lane two-shuffle"): realized as the
// sm_89-legal block smem tree — shuffle/plane ops are a different
// intrinsic surface and the pinned mapping in the pre-registration for
// the sibling pair (P111-B) is block smem, disclosed the same way.

#[cube(launch_unchecked)]
fn softmax_max_ref(x: &[f32], mx: &mut [f32], rows: u32, cols: u32) {
    let row = ABSOLUTE_POS_X;
    if row >= rows {
        terminate!();
    }
    let mut m = f32::new(-1.0e30f32);
    let mut c = 0u32;
    while c < cols {
        let v = x[(row * cols + c) as usize];
        if v > m {
            m = v;
        }
        c += 1u32;
    }
    mx[row as usize] = m;
}

#[cube(launch_unchecked)]
fn softmax_sumexp_ref(x: &[f32], mx: &[f32], se: &mut [f32], rows: u32, cols: u32) {
    let row = ABSOLUTE_POS_X;
    if row >= rows {
        terminate!();
    }
    let m = mx[row as usize];
    let mut acc = f32::new(0.0f32);
    let mut c = 0u32;
    while c < cols {
        acc += (x[(row * cols + c) as usize] - m).exp();
        c += 1u32;
    }
    se[row as usize] = acc;
}

#[cube(launch_unchecked)]
fn softmax_norm_ref(x: &[f32], mx: &[f32], se: &[f32], out: &mut [f32], total: u32, _cols: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let r = e >> 9u32; // cols == 512 == 2^9 (pinned)
    out[e as usize] = (x[e as usize] - mx[r as usize]).exp() / se[r as usize];
}

#[cube(launch_unchecked)]
fn softmax_fused_opt(x: &[f32], out: &mut [f32], _rows: u32, cols: u32) {
    let row = CUBE_POS_X;
    let col = UNIT_POS; // cols == 512 == thread count (exact grid, no guards)
    let mut smem = Shared::<[f32]>::new_slice(512usize);
    smem[col as usize] = x[(row * cols + col) as usize];
    sync_cube();
    let x_reg = smem[col as usize];
    // block max — tree over 512
    let mut off = 256u32;
    while off >= 1u32 {
        if col < off {
            if smem[(col + off) as usize] > smem[col as usize] {
                smem[col as usize] = smem[(col + off) as usize];
            }
        }
        sync_cube();
        off = off / 2u32;
    }
    let m_reg = smem[0usize];
    sync_cube();
    let e_reg = (x_reg - m_reg).exp();
    smem[col as usize] = e_reg;
    sync_cube();
    // block exp-sum — tree over 512
    let mut off2 = 256u32;
    while off2 >= 1u32 {
        if col < off2 {
            smem[col as usize] = smem[col as usize] + smem[(col + off2) as usize];
        }
        sync_cube();
        off2 = off2 / 2u32;
    }
    let s_reg = smem[0usize];
    let v = e_reg / s_reg;
    // ── the f16 roundtrip (probs >= 0: sign lane dropped by construction) ──
    let bits = u32::reinterpret(v);
    let exp32 = (bits >> 23u32) & 0xffu32;
    let man = bits & 0x007fffffu32;
    let e32 = exp32 as i32 - 127i32;
    let hbits = if e32 >= 16i32 {
        0x7c00u32 // overflow → +inf (unreachable for probs)
    } else {
        let he = e32 + 15i32;
        if he <= 0i32 {
            // subnormal / zero half
            let shift = 9i32 - e32;
            if shift > 31i32 {
                0u32 // far underflow (abs err < 2^-22 << 3e-3)
            } else {
                let mant = man | 0x00800000u32;
                let round = 1u32 << (shift - 1i32) as u32;
                let m10 = (mant + round) >> shift as u32;
                if m10 > 1023u32 {
                    0x0400u32 // rounds up into the smallest normal
                } else {
                    m10
                }
            }
        } else {
            // normal half: round-to-nearest-even on the low 13 bits
            let rr = man + 0x0fffu32 + ((man >> 13u32) & 1u32);
            let mut he_u = he as u32;
            let mut m10 = rr >> 13u32;
            if (rr & 0x00800000u32) != 0u32 {
                m10 = 0u32;
                he_u = he_u + 1u32;
            }
            if he_u >= 31u32 {
                0x7c00u32
            } else {
                (he_u << 10u32) | m10
            }
        }
    };
    // f16 -> f32 (exact expansion)
    let hexp = (hbits >> 10u32) & 0x1fu32;
    let hman = hbits & 0x03ffu32;
    let fbits = if hexp == 0u32 {
        if hman == 0u32 {
            0u32
        } else {
            // normalize the subnormal: msb walk
            let mut m2 = hman;
            let mut eshift = 14i32;
            while (m2 & 0x400u32) == 0u32 {
                m2 = m2 << 1u32;
                eshift = eshift - 1i32;
            }
            let e2 = 0i32 - eshift;
            (((127i32 + e2) as u32) << 23u32) | ((m2 & 0x03ffu32) << 13u32)
        }
    } else {
        if hexp == 31u32 {
            0x7f800000u32
        } else {
            ((hexp + 112u32) << 23u32) | (hman << 13u32)
        }
    };
    out[(row * cols + col) as usize] = f32::reinterpret(fbits);
}

pub struct P136B {
    x: Handle,
    mx: Handle,
    se: Handle,
    out_ref: Handle,
    out_opt: Handle,
    rows: u32,
    cols: u32,
}

impl P136B {
    fn new(client: &Client) -> Result<Self, String> {
        let (rows, cols) = (4096usize, 512usize);
        let mut rng = Lcg::new(seed_for_pair("P136-B"));
        let x = upload_f32(client, &rng.fill(rows * cols, -1.0, 1.0));
        let mx = empty_f32(client, rows);
        let se = empty_f32(client, rows);
        let out_ref = empty_f32(client, rows * cols);
        let out_opt = empty_f32(client, rows * cols);
        Ok(Self {
            x,
            mx,
            se,
            out_ref,
            out_opt,
            rows: rows as u32,
            cols: cols as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let blocks_row = self.rows.div_ceil(256u32);
        let total = self.rows * self.cols;
        let blocks_el = total.div_ceil(256u32);
        unsafe {
            softmax_max_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks_row, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.x.clone(), (self.rows * self.cols) as usize),
                BufferArg::from_raw_parts(self.mx.clone(), self.rows as usize),
                self.rows,
                self.cols,
            );
            softmax_sumexp_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks_row, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.x.clone(), (self.rows * self.cols) as usize),
                BufferArg::from_raw_parts(self.mx.clone(), self.rows as usize),
                BufferArg::from_raw_parts(self.se.clone(), self.rows as usize),
                self.rows,
                self.cols,
            );
            softmax_norm_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks_el, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.x.clone(), (self.rows * self.cols) as usize),
                BufferArg::from_raw_parts(self.mx.clone(), self.rows as usize),
                BufferArg::from_raw_parts(self.se.clone(), self.rows as usize),
                BufferArg::from_raw_parts(self.out_ref.clone(), total as usize),
                total,
                self.cols,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        unsafe {
            softmax_fused_opt::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(self.rows, 1, 1),
                CubeDim::new_1d(512),
                BufferArg::from_raw_parts(self.x.clone(), (self.rows * self.cols) as usize),
                BufferArg::from_raw_parts(self.out_opt.clone(), (self.rows * self.cols) as usize),
                self.rows,
                self.cols,
            );
        }
    }
}

pub fn p136b_correctness(client: &Client) -> Result<(), String> {
    let s = P136B::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    abs_linf(&o_opt, &o_ref, 3e-3)
}

pub fn p136b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p136b_correctness(client)?;
    let s = P136B::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p136b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P138-A statistic row-split ─────────────────────────────────────────
// 4096 rows × 512 cols, per-row sums, values U(0,1).
// ref: grid (1,1,1) — ONE block, 256 threads, serial row loop: each thread
//      sums its 2 columns, thread-0 serial-walks the 256-entry smem combine
//      per row (the single-threadgroup serialization IS the naive axis).
// opt: grid 4096 blocks (one row each), smem tree reduce.
// Tolerance: rel L∞ 1e-3 (association differs: serial combine vs tree).

#[cube(launch_unchecked)]
fn rowsum_serial_ref(x: &[f32], out: &mut [f32], rows: u32, cols: u32) {
    let t = UNIT_POS; // 256 threads, one block
    let mut smem = Shared::<[f32]>::new_slice(256usize);
    let mut r = 0u32;
    while r < rows {
        let c0 = t * 2u32;
        let mut acc = f32::new(0.0f32);
        acc += x[(r * cols + c0) as usize];
        acc += x[(r * cols + c0 + 1u32) as usize];
        smem[t as usize] = acc;
        sync_cube();
        if t == 0u32 {
            let mut s = f32::new(0.0f32);
            let mut i = 0u32;
            while i < 256u32 {
                s += smem[i as usize];
                i += 1u32;
            }
            out[r as usize] = s;
        }
        sync_cube();
        r += 1u32;
    }
}

#[cube(launch_unchecked)]
fn rowsum_split_opt(x: &[f32], out: &mut [f32], cols: u32) {
    let r = CUBE_POS_X;
    let t = UNIT_POS;
    let mut smem = Shared::<[f32]>::new_slice(256usize);
    let c0 = t * 2u32;
    let mut acc = f32::new(0.0f32);
    acc += x[(r * cols + c0) as usize];
    acc += x[(r * cols + c0 + 1u32) as usize];
    smem[t as usize] = acc;
    sync_cube();
    let mut off = 128u32;
    while off >= 1u32 {
        if t < off {
            smem[t as usize] = smem[t as usize] + smem[(t + off) as usize];
        }
        sync_cube();
        off = off / 2u32;
    }
    if t == 0u32 {
        out[r as usize] = smem[0usize];
    }
}

pub struct P138A {
    x: Handle,
    out_ref: Handle,
    out_opt: Handle,
    rows: u32,
    cols: u32,
}

impl P138A {
    fn new(client: &Client) -> Result<Self, String> {
        let (rows, cols) = (4096usize, 512usize);
        let mut rng = Lcg::new(seed_for_pair("P138-A"));
        let x = upload_f32(client, &rng.fill(rows * cols, 0.0, 1.0));
        let out_ref = empty_f32(client, rows);
        let out_opt = empty_f32(client, rows);
        Ok(Self {
            x,
            out_ref,
            out_opt,
            rows: rows as u32,
            cols: cols as u32,
        })
    }
}

pub fn p138a_correctness(client: &Client) -> Result<(), String> {
    let s = P138A::new(client)?;
    unsafe {
        rowsum_serial_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), s.rows as usize),
            s.rows,
            s.cols,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    unsafe {
        rowsum_split_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.rows, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), s.rows as usize),
            s.cols,
        );
    }
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-3)
}

pub fn p138a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p138a_correctness(client)?;
    let s = P138A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                rowsum_serial_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(1, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), s.rows as usize),
                    s.rows,
                    s.cols,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                rowsum_split_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(s.rows, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), s.rows as usize),
                    s.cols,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p138a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P110-A block-local state exchange ──────────────────────────────────
// seq=2048 rows, head_dim=128; softmax over d per row (probs 2048×128).
// ref: kernel A — block per 32-row chunk (64 blocks × 128 threads), per-row
//      max+sumexp via smem trees, stats exchanged through GLOBAL m/S arrays;
//      kernel B — re-reads s, recomputes exp, normalizes from global stats.
// opt: ONE kernel — same smem trees, normalize inline (stats never leave
//      the block; exp computed once per element instead of twice).
// Tolerance: rel L∞ 1e-4 (same expf both arms, association identical trees
// → expect bit-exact).
// DEVIATION (interpretation, disclosed): the pre-registration's "running
// max/sum exchanged through global memory per chunk" is realized as the
// global stats arrays + second launch (the naive softmax shape); the opt
// is the block-local exchange "once per block" (smem only, single launch).

#[cube(launch_unchecked)]
fn softmax_stats_ref(s: &[f32], m_out: &mut [f32], s_out: &mut [f32], rpb: u32, cols: u32) {
    let d = UNIT_POS; // 128 threads = cols (exact)
    let row0 = CUBE_POS_X * rpb;
    let mut smem = Shared::<[f32]>::new_slice(128usize);
    let mut r = 0u32;
    while r < rpb {
        let row = row0 + r;
        smem[d as usize] = s[(row * cols + d) as usize];
        sync_cube();
        let x_reg = smem[d as usize]; // captured BEFORE the tree overwrites smem
        let mut off = 64u32;
        while off >= 1u32 {
            if d < off {
                if smem[(d + off) as usize] > smem[d as usize] {
                    smem[d as usize] = smem[(d + off) as usize];
                }
            }
            sync_cube();
            off = off / 2u32;
        }
        let m_reg = smem[0usize];
        sync_cube();
        let e_reg = (x_reg - m_reg).exp();
        smem[d as usize] = e_reg;
        sync_cube();
        let mut off2 = 64u32;
        while off2 >= 1u32 {
            if d < off2 {
                smem[d as usize] = smem[d as usize] + smem[(d + off2) as usize];
            }
            sync_cube();
            off2 = off2 / 2u32;
        }
        let s_reg = smem[0usize];
        if d == 0u32 {
            m_out[row as usize] = m_reg;
            s_out[row as usize] = s_reg;
        }
        sync_cube();
        r += 1u32;
    }
}

#[cube(launch_unchecked)]
fn softmax_fused_opt110(s: &[f32], out: &mut [f32], rpb: u32, cols: u32) {
    let d = UNIT_POS;
    let row0 = CUBE_POS_X * rpb;
    let mut smem = Shared::<[f32]>::new_slice(128usize);
    let mut r = 0u32;
    while r < rpb {
        let row = row0 + r;
        smem[d as usize] = s[(row * cols + d) as usize];
        sync_cube();
        let x_reg = smem[d as usize]; // captured BEFORE the tree overwrites smem
        let mut off = 64u32;
        while off >= 1u32 {
            if d < off {
                if smem[(d + off) as usize] > smem[d as usize] {
                    smem[d as usize] = smem[(d + off) as usize];
                }
            }
            sync_cube();
            off = off / 2u32;
        }
        let m_reg = smem[0usize];
        sync_cube();
        let e_reg = (x_reg - m_reg).exp();
        smem[d as usize] = e_reg;
        sync_cube();
        let mut off2 = 64u32;
        while off2 >= 1u32 {
            if d < off2 {
                smem[d as usize] = smem[d as usize] + smem[(d + off2) as usize];
            }
            sync_cube();
            off2 = off2 / 2u32;
        }
        let s_reg = smem[0usize];
        out[(row * cols + d) as usize] = e_reg / s_reg;
        sync_cube();
        r += 1u32;
    }
}

#[cube(launch_unchecked)]
fn softmax_norm_global(s: &[f32], m_arr: &[f32], s_arr: &[f32], out: &mut [f32], total: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let r = e >> 7u32; // cols == 128 == 2^7 (pinned)
    out[e as usize] = (s[e as usize] - m_arr[r as usize]).exp() / s_arr[r as usize];
}


pub struct P110A {
    s: Handle,
    m_arr: Handle,
    s_arr: Handle,
    out_ref: Handle,
    out_opt: Handle,
    rows: u32,
    cols: u32,
    rpb: u32,
}

impl P110A {
    fn new(client: &Client) -> Result<Self, String> {
        let (rows, cols) = (2048usize, 128usize);
        let mut rng = Lcg::new(seed_for_pair("P110-A"));
        let s = upload_f32(client, &rng.fill(rows * cols, -1.0, 1.0));
        let m_arr = empty_f32(client, rows);
        let s_arr = empty_f32(client, rows);
        let out_ref = empty_f32(client, rows * cols);
        let out_opt = empty_f32(client, rows * cols);
        Ok(Self {
            s,
            m_arr,
            s_arr,
            out_ref,
            out_opt,
            rows: rows as u32,
            cols: cols as u32,
            rpb: 32,
        })
    }
}

pub fn p110a_correctness(client: &Client) -> Result<(), String> {
    let s = P110A::new(client)?;
    let blocks = s.rows / s.rpb; // 64 — exact
    let total = s.rows * s.cols;
    unsafe {
        softmax_stats_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(128),
            BufferArg::from_raw_parts(s.s.clone(), total as usize),
            BufferArg::from_raw_parts(s.m_arr.clone(), s.rows as usize),
            BufferArg::from_raw_parts(s.s_arr.clone(), s.rows as usize),
            s.rpb,
            s.cols,
        );
        softmax_norm_global::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(total.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.s.clone(), total as usize),
            BufferArg::from_raw_parts(s.m_arr.clone(), s.rows as usize),
            BufferArg::from_raw_parts(s.s_arr.clone(), s.rows as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), total as usize),
            total,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    unsafe {
        softmax_fused_opt110::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(128),
            BufferArg::from_raw_parts(s.s.clone(), total as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), total as usize),
            s.rpb,
            s.cols,
        );
    }
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-4)
}

pub fn p110a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p110a_correctness(client)?;
    let s = P110A::new(client)?;
    let blocks = s.rows / s.rpb;
    let total = s.rows * s.cols;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                softmax_stats_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(128),
                    BufferArg::from_raw_parts(s.s.clone(), total as usize),
                    BufferArg::from_raw_parts(s.m_arr.clone(), s.rows as usize),
                    BufferArg::from_raw_parts(s.s_arr.clone(), s.rows as usize),
                    s.rpb,
                    s.cols,
                );
                softmax_norm_global::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(total.div_ceil(256), 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.s.clone(), total as usize),
                    BufferArg::from_raw_parts(s.m_arr.clone(), s.rows as usize),
                    BufferArg::from_raw_parts(s.s_arr.clone(), s.rows as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), total as usize),
                    total,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                softmax_fused_opt110::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(128),
                    BufferArg::from_raw_parts(s.s.clone(), total as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), total as usize),
                    s.rpb,
                    s.cols,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p110a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P133-A head-dim smem tile cap ──────────────────────────────────────
// seq=1024, hd=128. out[q] = Σ_d qbuf[q][d]·k[d].
// ref: block per 32 q's; k[128] + q tile 32×128 staged in Shared
//      (16.9 KiB/block), serial d per q.
// opt: hd capped at 64 — two passes over the d-halves, Shared k-half only
//      (256 B), register accumulator persists (higher occupancy); q read
//      from global per d.
// Tolerance: rel L∞ 1e-4 (d-ascending association both arms → expect
// bit-exact).

#[cube(launch_unchecked)]
fn qk_dot_smemtile_ref(q: &[f32], k: &[f32], out: &mut [f32], hd: u32) {
    let t = UNIT_POS; // 32 threads, 32 q's per block
    let q0 = CUBE_POS_X * 32u32;
    let mut smem_q = Shared::<[f32]>::new_slice(4096usize);
    let mut smem_k = Shared::<[f32]>::new_slice(128usize);
    let row = q0 + t;
    let mut d = 0u32;
    while d < hd {
        smem_q[(t * hd + d) as usize] = q[(row * hd + d) as usize];
        d += 1u32;
    }
    let mut e = 0u32;
    while e < 4u32 {
        smem_k[(t * 4u32 + e) as usize] = k[(t * 4u32 + e) as usize];
        e += 1u32;
    }
    sync_cube();
    let mut acc = f32::new(0.0f32);
    d = 0u32;
    while d < hd {
        acc += smem_q[(t * hd + d) as usize] * smem_k[d as usize];
        d += 1u32;
    }
    out[row as usize] = acc;
}

#[cube(launch_unchecked)]
fn qk_dot_hdcap_opt(q: &[f32], k: &[f32], out: &mut [f32], hd: u32) {
    // two passes over d-halves of 64; Shared = k half only.
    let t = UNIT_POS;
    let q0 = CUBE_POS_X * 32u32;
    let row = q0 + t;
    let mut smem_k = Shared::<[f32]>::new_slice(64usize);
    let mut acc = f32::new(0.0f32);
    let mut half = 0u32;
    while half < 2u32 {
        let d0 = half * 64u32;
        let mut e = 0u32;
        while e < 2u32 {
            smem_k[(t * 2u32 + e) as usize] = k[(d0 + t * 2u32 + e) as usize];
            e += 1u32;
        }
        sync_cube();
        let mut d = 0u32;
        while d < 64u32 {
            acc += q[(row * hd + d0 + d) as usize] * smem_k[d as usize];
            d += 1u32;
        }
        sync_cube();
        half += 1u32;
    }
    out[row as usize] = acc;
}

pub struct P133A {
    q: Handle,
    k: Handle,
    out_ref: Handle,
    out_opt: Handle,
    seq: u32,
    hd: u32,
}

impl P133A {
    fn new(client: &Client) -> Result<Self, String> {
        let (seq, hd) = (1024usize, 128usize);
        let mut rng = Lcg::new(seed_for_pair("P133-A"));
        let q = upload_f32(client, &rng.fill(seq * hd, -0.5, 0.5));
        let k = upload_f32(client, &rng.fill(hd, -0.5, 0.5));
        let out_ref = empty_f32(client, seq);
        let out_opt = empty_f32(client, seq);
        Ok(Self {
            q,
            k,
            out_ref,
            out_opt,
            seq: seq as u32,
            hd: hd as u32,
        })
    }
}

pub fn p133a_correctness(client: &Client) -> Result<(), String> {
    let s = P133A::new(client)?;
    let blocks = s.seq / 32u32; // 32 — exact
    unsafe {
        qk_dot_smemtile_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(32),
            BufferArg::from_raw_parts(s.q.clone(), (s.seq * s.hd) as usize),
            BufferArg::from_raw_parts(s.k.clone(), s.hd as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), s.seq as usize),
            s.hd,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    unsafe {
        qk_dot_hdcap_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(32),
            BufferArg::from_raw_parts(s.q.clone(), (s.seq * s.hd) as usize),
            BufferArg::from_raw_parts(s.k.clone(), s.hd as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), s.seq as usize),
            s.hd,
        );
    }
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-4)
}

pub fn p133a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p133a_correctness(client)?;
    let s = P133A::new(client)?;
    let blocks = s.seq / 32u32;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                qk_dot_smemtile_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(32),
                    BufferArg::from_raw_parts(s.q.clone(), (s.seq * s.hd) as usize),
                    BufferArg::from_raw_parts(s.k.clone(), s.hd as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), s.seq as usize),
                    s.hd,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                qk_dot_hdcap_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(32),
                    BufferArg::from_raw_parts(s.q.clone(), (s.seq * s.hd) as usize),
                    BufferArg::from_raw_parts(s.k.clone(), s.hd as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), s.seq as usize),
                    s.hd,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p133a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

`````

### File: `src/pairs_special.rs`

blake3: `d0eb21fa4ba4209c68fcbab322bebd3d82b72dc20a70aa6ae440f3e9835855c7`

`````rust
//! Special-shaped pairs: P136-A (fp32-pipe biased-split dot), P137-A
//! (coalesced tiled transpose), P137-B (branch-free codebook rank), P135-A
//! (specialized top-k with the first-index tie law), P121-A (int8 screen +
//! error-bound-certified rescore).


// The #[cube] kernel bodies deliberately use explicit index arithmetic and
// nested ifs (the house convention, e.g. riir-infer norms_cubecl.rs): clippy's
// assign_op_pattern / collapsible_if rewrites target shapes the cubecl macro
// expansion has not proven on indexed places / short-circuit chains.
#![allow(clippy::assign_op_pattern, clippy::collapsible_if)]

use crate::common::{
    bits_exact, empty_f32, empty_i32, empty_u32, interleave_bench, read_f32, read_u32,
    rel_linf, seed_for_pair, sync_ok, upload_f32, upload_i8, upload_u32, u32_exact,
    ActiveRuntime, Client, Lcg,
};
use cubecl::prelude::*;
use cubecl::server::Handle;

// ── P136-A biased split ────────────────────────────────────────────────
// rows=512 K=4096, inputs U(-0.5,0.5).
// ref: plain Σ a·b per row (8-unrolled single chain).
// opt: the biased operand split — v = hi + lo with hi the bf16-grid value
//      (f32 bits truncated (bits>>16)<<16), lo = v − hi (exact);
//      acc_hi = Σ hi_a·hi_b (the bf16-MMA pipe) and acc_x = Σ (hi_a·lo_b +
//      lo_a·hi_b + lo_a·lo_b) (the fewer-value merges riding the fp32
//      pipe); result = acc_hi + acc_x.
// DEVIATION (disclosed): the split is performed HOST-side (the task's
// "host quantizes inputs to bf16 grid"), so both arms read the same
// full-precision a, b buffers and the two results are the SAME sum
// re-associated — rel L∞ 1e-3. (Quantizing the uploaded buffers instead
// would make lo ≡ 0 and the pair degenerate.)
// k == 4096 pinned: divisible by the unroll 8, no remainder handling.

fn split_bf16(v: f32) -> (f32, f32) {
    let hi = f32::from_bits(v.to_bits() & 0xFFFF_0000);
    (hi, v - hi)
}

#[cube(launch_unchecked)]
fn biased_dot_ref(a: &[f32], b: &[f32], out: &mut [f32], rows: u32, k: u32) {
    let r = ABSOLUTE_POS_X;
    if r >= rows {
        terminate!();
    }
    let base = (r * k) as usize;
    let mut acc = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        acc += a[base + kk as usize] * b[base + kk as usize];
        acc += a[base + (kk + 1u32) as usize] * b[base + (kk + 1u32) as usize];
        acc += a[base + (kk + 2u32) as usize] * b[base + (kk + 2u32) as usize];
        acc += a[base + (kk + 3u32) as usize] * b[base + (kk + 3u32) as usize];
        acc += a[base + (kk + 4u32) as usize] * b[base + (kk + 4u32) as usize];
        acc += a[base + (kk + 5u32) as usize] * b[base + (kk + 5u32) as usize];
        acc += a[base + (kk + 6u32) as usize] * b[base + (kk + 6u32) as usize];
        acc += a[base + (kk + 7u32) as usize] * b[base + (kk + 7u32) as usize];
        kk += 8u32;
    }
    out[r as usize] = acc;
}

#[cube(launch_unchecked)]
fn biased_dot_opt(ah: &[f32], al: &[f32], bh: &[f32], bl: &[f32], out: &mut [f32], rows: u32, k: u32) {
    let r = ABSOLUTE_POS_X;
    if r >= rows {
        terminate!();
    }
    let base = (r * k) as usize;
    let mut acc_hi = f32::new(0.0f32);
    let mut acc_x = f32::new(0.0f32);
    let mut kk = 0u32;
    while kk < k {
        acc_hi += ah[base + kk as usize] * bh[base + kk as usize];
        acc_hi += ah[base + (kk + 1u32) as usize] * bh[base + (kk + 1u32) as usize];
        acc_hi += ah[base + (kk + 2u32) as usize] * bh[base + (kk + 2u32) as usize];
        acc_hi += ah[base + (kk + 3u32) as usize] * bh[base + (kk + 3u32) as usize];
        acc_hi += ah[base + (kk + 4u32) as usize] * bh[base + (kk + 4u32) as usize];
        acc_hi += ah[base + (kk + 5u32) as usize] * bh[base + (kk + 5u32) as usize];
        acc_hi += ah[base + (kk + 6u32) as usize] * bh[base + (kk + 6u32) as usize];
        acc_hi += ah[base + (kk + 7u32) as usize] * bh[base + (kk + 7u32) as usize];
        acc_x += ah[base + kk as usize] * bl[base + kk as usize];
        acc_x += al[base + kk as usize] * bh[base + kk as usize];
        acc_x += al[base + kk as usize] * bl[base + kk as usize];
        acc_x += ah[base + (kk + 1u32) as usize] * bl[base + (kk + 1u32) as usize];
        acc_x += al[base + (kk + 1u32) as usize] * bh[base + (kk + 1u32) as usize];
        acc_x += al[base + (kk + 1u32) as usize] * bl[base + (kk + 1u32) as usize];
        acc_x += ah[base + (kk + 2u32) as usize] * bl[base + (kk + 2u32) as usize];
        acc_x += al[base + (kk + 2u32) as usize] * bh[base + (kk + 2u32) as usize];
        acc_x += al[base + (kk + 2u32) as usize] * bl[base + (kk + 2u32) as usize];
        acc_x += ah[base + (kk + 3u32) as usize] * bl[base + (kk + 3u32) as usize];
        acc_x += al[base + (kk + 3u32) as usize] * bh[base + (kk + 3u32) as usize];
        acc_x += al[base + (kk + 3u32) as usize] * bl[base + (kk + 3u32) as usize];
        acc_x += ah[base + (kk + 4u32) as usize] * bl[base + (kk + 4u32) as usize];
        acc_x += al[base + (kk + 4u32) as usize] * bh[base + (kk + 4u32) as usize];
        acc_x += al[base + (kk + 4u32) as usize] * bl[base + (kk + 4u32) as usize];
        acc_x += ah[base + (kk + 5u32) as usize] * bl[base + (kk + 5u32) as usize];
        acc_x += al[base + (kk + 5u32) as usize] * bh[base + (kk + 5u32) as usize];
        acc_x += al[base + (kk + 5u32) as usize] * bl[base + (kk + 5u32) as usize];
        acc_x += ah[base + (kk + 6u32) as usize] * bl[base + (kk + 6u32) as usize];
        acc_x += al[base + (kk + 6u32) as usize] * bh[base + (kk + 6u32) as usize];
        acc_x += al[base + (kk + 6u32) as usize] * bl[base + (kk + 6u32) as usize];
        acc_x += ah[base + (kk + 7u32) as usize] * bl[base + (kk + 7u32) as usize];
        acc_x += al[base + (kk + 7u32) as usize] * bh[base + (kk + 7u32) as usize];
        acc_x += al[base + (kk + 7u32) as usize] * bl[base + (kk + 7u32) as usize];
        kk += 8u32;
    }
    out[r as usize] = acc_hi + acc_x;
}

pub struct P136A {
    a: Handle,
    b: Handle,
    ah: Handle,
    al: Handle,
    bh: Handle,
    bl: Handle,
    out_ref: Handle,
    out_opt: Handle,
    rows: u32,
    k: u32,
}

impl P136A {
    fn new(client: &Client) -> Result<Self, String> {
        let (rows, k) = (512usize, 4096usize);
        let mut rng = Lcg::new(seed_for_pair("P136-A"));
        let a = rng.fill(rows * k, -0.5, 0.5);
        let b = rng.fill(rows * k, -0.5, 0.5);
        let mut ah = vec![0.0f32; rows * k];
        let mut al = vec![0.0f32; rows * k];
        let mut bh = vec![0.0f32; rows * k];
        let mut bl = vec![0.0f32; rows * k];
        for i in 0..rows * k {
            let (h, l) = split_bf16(a[i]);
            ah[i] = h;
            al[i] = l;
            let (h, l) = split_bf16(b[i]);
            bh[i] = h;
            bl[i] = l;
        }
        let out_ref = empty_f32(client, rows);
        let out_opt = empty_f32(client, rows);
        Ok(Self {
            a: upload_f32(client, &a),
            b: upload_f32(client, &b),
            ah: upload_f32(client, &ah),
            al: upload_f32(client, &al),
            bh: upload_f32(client, &bh),
            bl: upload_f32(client, &bl),
            out_ref,
            out_opt,
            rows: rows as u32,
            k: k as u32,
        })
    }
}

pub fn p136a_correctness(client: &Client) -> Result<(), String> {
    let s = P136A::new(client)?;
    unsafe {
        biased_dot_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.rows.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.a.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.b.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), s.rows as usize),
            s.rows,
            s.k,
        );
        biased_dot_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.rows.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.ah.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.al.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.bh.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.bl.clone(), (s.rows * s.k) as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), s.rows as usize),
            s.rows,
            s.k,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-3)
}

pub fn p136a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p136a_correctness(client)?;
    let s = P136A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                biased_dot_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(s.rows.div_ceil(256), 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.a.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.b.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), s.rows as usize),
                    s.rows,
                    s.k,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                biased_dot_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(s.rows.div_ceil(256), 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.ah.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.al.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.bh.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.bl.clone(), (s.rows * s.k) as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), s.rows as usize),
                    s.rows,
                    s.k,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p136a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P137-A coalesced tile walk ─────────────────────────────────────────
// 4096×4096 transpose-read, bit-exact (pure movement).
// ref: thread per element, out[i*N+j] = in[j*M+i], threads over i fastest —
//      the global access on the write side is stride-N (the uncoalesced
//      walk; the pre-registration wording puts the strided axis on reads —
//      the pattern is symmetric, the fix below is the same).
// opt: 32×32 Shared-tiled transpose (+33 padding against bank conflicts):
//      coalesced reads AND coalesced writes.
// Tolerance: 0 (bit-exact).
// Grids are exact multiples of 32 — no guards inside the smem kernel.

#[cube(launch_unchecked)]
fn transpose_naive(inp: &[f32], out: &mut [f32], m: u32, n: u32) {
    let i = ABSOLUTE_POS_X;
    let j = ABSOLUTE_POS_Y;
    if i >= m || j >= n {
        terminate!();
    }
    out[(i * n + j) as usize] = inp[(j * m + i) as usize];
}

#[cube(launch_unchecked)]
fn transpose_tiled(inp: &[f32], out: &mut [f32], m: u32, n: u32) {
    let i0 = CUBE_POS_X * 32u32;
    let j0 = CUBE_POS_Y * 32u32;
    let tx = UNIT_POS_X;
    let ty = UNIT_POS_Y;
    let mut smem = Shared::<[f32]>::new_slice(1056usize); // 33 × 32, padded
    smem[(ty * 33u32 + tx) as usize] = inp[((j0 + ty) * m + (i0 + tx)) as usize];
    sync_cube();
    // after the barrier each thread writes a DIFFERENT element: thread
    // (tx, ty) emits out[(i0+ty)][(j0+tx)] = smem[tx][ty] (coalesced writes).
    out[((i0 + ty) * n + (j0 + tx)) as usize] = smem[(tx * 33u32 + ty) as usize];
}

pub struct P137A {
    inp: Handle,
    out_ref: Handle,
    out_opt: Handle,
    m: u32,
    n: u32,
}

impl P137A {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n) = (4096usize, 4096usize);
        let mut rng = Lcg::new(seed_for_pair("P137-A"));
        let inp = upload_f32(client, &rng.fill(m * n, -1.0, 1.0));
        let out_ref = empty_f32(client, m * n);
        let out_opt = empty_f32(client, m * n);
        Ok(Self {
            inp,
            out_ref,
            out_opt,
            m: m as u32,
            n: n as u32,
        })
    }
}

pub fn p137a_correctness(client: &Client) -> Result<(), String> {
    let s = P137A::new(client)?;
    let blocks = s.m / 32u32; // 128 — exact
    unsafe {
        transpose_naive::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, blocks, 1),
            CubeDim::new_2d(32, 32),
            BufferArg::from_raw_parts(s.inp.clone(), (s.m * s.n) as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), (s.m * s.n) as usize),
            s.m,
            s.n,
        );
        transpose_tiled::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, blocks, 1),
            CubeDim::new_2d(32, 32),
            BufferArg::from_raw_parts(s.inp.clone(), (s.m * s.n) as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), (s.m * s.n) as usize),
            s.m,
            s.n,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    bits_exact(&o_opt, &o_ref)
}

pub fn p137a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p137a_correctness(client)?;
    let s = P137A::new(client)?;
    let blocks = s.m / 32u32;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                transpose_naive::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, blocks, 1),
                    CubeDim::new_2d(32, 32),
                    BufferArg::from_raw_parts(s.inp.clone(), (s.m * s.n) as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), (s.m * s.n) as usize),
                    s.m,
                    s.n,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                transpose_tiled::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, blocks, 1),
                    CubeDim::new_2d(32, 32),
                    BufferArg::from_raw_parts(s.inp.clone(), (s.m * s.n) as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), (s.m * s.n) as usize),
                    s.m,
                    s.n,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p137a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P137-B branch-free codebook rank ───────────────────────────────────
// codes[16384][32], queries[512][32] (U(-1,1)), dist = Σ (q[d]−c[d])² fp32.
// ref: branchy nearest — `if dist < best { best = dist; idx = c; }`
//      scanning ascending (strict < keeps the lowest index on ties).
// opt: branch-free via the cubecl `select(cond, then, or_else)` intrinsic
//      (DISCLOSED spelling — frontend free function, expands to the IR
//      select op; divergence-free argmin). Ties: strict < keeps the lowest
//      index in BOTH arms.
// Seeded ties: 8 duplicate code rows per... 8 duplicate code rows total
// (bit copies) guarantee exact-distance ties; the gate compares the full
// idx arrays BIT-EXACT, which asserts the tie law matches.
// Tolerance: idx bit-exact; dist rel L∞ 1e-3.

#[cube(launch_unchecked)]
fn codebook_ref(codes: &[f32], queries: &[f32], bi: &mut [u32], bd: &mut [f32], n_codes: u32, dim: u32, nq: u32) {
    let q = ABSOLUTE_POS_X;
    if q >= nq {
        terminate!();
    }
    let qbase = (q * dim) as usize;
    let mut best = f32::new(3.0e38f32);
    let mut bidx = 0u32;
    let mut c = 0u32;
    while c < n_codes {
        let cbase = (c * dim) as usize;
        let mut dist = f32::new(0.0f32);
        let mut d = 0u32;
        while d < dim {
            let diff = queries[qbase + d as usize] - codes[cbase + d as usize];
            dist += diff * diff;
            d += 1u32;
        }
        if dist < best {
            best = dist;
            bidx = c;
        }
        c += 1u32;
    }
    bi[q as usize] = bidx;
    bd[q as usize] = best;
}

#[cube(launch_unchecked)]
fn codebook_opt(codes: &[f32], queries: &[f32], bi: &mut [u32], bd: &mut [f32], n_codes: u32, dim: u32, nq: u32) {
    let q = ABSOLUTE_POS_X;
    if q >= nq {
        terminate!();
    }
    let qbase = (q * dim) as usize;
    let mut best = f32::new(3.0e38f32);
    let mut bidx = 0u32;
    let mut c = 0u32;
    while c < n_codes {
        let cbase = (c * dim) as usize;
        let mut dist = f32::new(0.0f32);
        let mut d = 0u32;
        while d < dim {
            let diff = queries[qbase + d as usize] - codes[cbase + d as usize];
            dist += diff * diff;
            d += 1u32;
        }
        let better = dist < best;
        best = select(better, dist, best);
        bidx = select(better, c, bidx);
        c += 1u32;
    }
    bi[q as usize] = bidx;
    bd[q as usize] = best;
}

pub struct P137B {
    codes: Handle,
    queries: Handle,
    bi_ref: Handle,
    bd_ref: Handle,
    bi_opt: Handle,
    bd_opt: Handle,
    n_codes: u32,
    dim: u32,
    nq: u32,
}

impl P137B {
    fn new(client: &Client) -> Result<Self, String> {
        let (n_codes, dim, nq) = (16384usize, 32usize, 512usize);
        let mut rng = Lcg::new(seed_for_pair("P137-B"));
        let mut codes = rng.fill(n_codes * dim, -1.0, 1.0);
        let queries = rng.fill(nq * dim, -1.0, 1.0);
        // seeded exact-duplicate code rows → guaranteed exact-distance ties
        for _dup in 0..8usize {
            let src = (rng.next_u64() as usize) % n_codes;
            let mut dst = (rng.next_u64() as usize) % n_codes;
            if dst == src {
                dst = (dst + 1) % n_codes;
            }
            for d in 0..dim {
                codes[dst * dim + d] = codes[src * dim + d];
            }
        }
        let bi_ref = empty_u32(client, nq);
        let bd_ref = empty_f32(client, nq);
        let bi_opt = empty_u32(client, nq);
        let bd_opt = empty_f32(client, nq);
        Ok(Self {
            codes: upload_f32(client, &codes),
            queries: upload_f32(client, &queries),
            bi_ref,
            bd_ref,
            bi_opt,
            bd_opt,
            n_codes: n_codes as u32,
            dim: dim as u32,
            nq: nq as u32,
        })
    }
}

pub fn p137b_correctness(client: &Client) -> Result<(), String> {
    let s = P137B::new(client)?;
    let blocks = s.nq.div_ceil(256u32);
    unsafe {
        codebook_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.codes.clone(), (s.n_codes * s.dim) as usize),
            BufferArg::from_raw_parts(s.queries.clone(), (s.nq * s.dim) as usize),
            BufferArg::from_raw_parts(s.bi_ref.clone(), s.nq as usize),
            BufferArg::from_raw_parts(s.bd_ref.clone(), s.nq as usize),
            s.n_codes,
            s.dim,
            s.nq,
        );
        codebook_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.codes.clone(), (s.n_codes * s.dim) as usize),
            BufferArg::from_raw_parts(s.queries.clone(), (s.nq * s.dim) as usize),
            BufferArg::from_raw_parts(s.bi_opt.clone(), s.nq as usize),
            BufferArg::from_raw_parts(s.bd_opt.clone(), s.nq as usize),
            s.n_codes,
            s.dim,
            s.nq,
        );
    }
    sync_ok(client)?;
    let bi_ref = read_u32(client, &s.bi_ref)?;
    let bd_ref = read_f32(client, &s.bd_ref)?;
    let bi_opt = read_u32(client, &s.bi_opt)?;
    let bd_opt = read_f32(client, &s.bd_opt)?;
    u32_exact(&bi_opt, &bi_ref)?;
    rel_linf(&bd_opt, &bd_ref, 1e-3)
}

pub fn p137b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p137b_correctness(client)?;
    let s = P137B::new(client)?;
    let blocks = s.nq.div_ceil(256u32);
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                codebook_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.codes.clone(), (s.n_codes * s.dim) as usize),
                    BufferArg::from_raw_parts(s.queries.clone(), (s.nq * s.dim) as usize),
                    BufferArg::from_raw_parts(s.bi_ref.clone(), s.nq as usize),
                    BufferArg::from_raw_parts(s.bd_ref.clone(), s.nq as usize),
                    s.n_codes,
                    s.dim,
                    s.nq,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                codebook_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.codes.clone(), (s.n_codes * s.dim) as usize),
                    BufferArg::from_raw_parts(s.queries.clone(), (s.nq * s.dim) as usize),
                    BufferArg::from_raw_parts(s.bi_opt.clone(), s.nq as usize),
                    BufferArg::from_raw_parts(s.bd_opt.clone(), s.nq as usize),
                    s.n_codes,
                    s.dim,
                    s.nq,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p137b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P135-A specialized top-k ───────────────────────────────────────────
// vocab=131072, k=8, batch=64; vals U(0,1) with seeded exact duplicates.
// Tie law pinned: (value desc, index asc) — lowest index wins a tie in
// BOTH arms. Gate: the (value, index) lists compare BIT-EXACT.
// ref: 8 selection passes, each finding the max over the untaken via a
//      global mask (mask re-cleared per top-k call — the clear is part of
//      the naive cost), scanning ascending.
// opt: single pass per thread: each of 256 threads keeps a REGISTER top-8
//      (descending, admission v > tail, insert at the first strictly-
//      greater slot ⇒ per-slice lowest-index ties), stages its 8 candidates
//      to Shared (2048 total — DEVIATION: the task's "≤256 candidates"
//      note is realized as 256 threads × 8), then 8 selection passes over
//      the candidates in Shared with the SAME comparator.
// DEVIATION (bench-only, disclosed): the ref's global mask persists across
// chunk rounds, so rounds after the first scan an all-taken mask; timing is
// unaffected (the reads dominate), and the pre/post correctness gates run
// on fresh state with the mask re-cleared per call.
// NOTE: cross-thread combines use the LEXICOGRAPHIC comparator (value
// desc, index asc) — a plain strict-> tree would keep the lower THREAD's
// candidate, and thread order ≠ index order under the stride walk.

#[cube(launch_unchecked)]
fn mask_clear(taken: &mut [u32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    taken[e as usize] = 0u32;
}

#[cube(launch_unchecked)]
fn topk_ref(vals: &[f32], taken: &mut [u32], ov: &mut [f32], oi: &mut [u32], vocab: u32, k: u32) {
    // thread-per-row: batch == 64 == thread count (one block, exact), each
    // thread selection-sorts its OWN row over its OWN mask row (taken is
    // [batch][vocab]); 8 ascending strict-> passes — the naive cost.
    // DEVIATION (design, disclosed): the first cut of this kernel smeared
    // all 64 rows through one smem tree (the caught-by-the-oracle bug);
    // thread-per-row has nothing to combine across threads — the tree shape
    // belongs to the opt arm only.
    let row = UNIT_POS;
    let base = row * vocab;
    let mut p = 0u32;
    while p < k {
        let mut bv = f32::new(-1.0f32);
        let mut bi = 0u32;
        let mut i = 0u32;
        while i < vocab {
            if taken[(base + i) as usize] == 0u32 {
                let v = vals[(base + i) as usize];
                if v > bv {
                    bv = v;
                    bi = i;
                }
            }
            i += 1u32;
        }
        taken[(base + bi) as usize] = 1u32;
        ov[(row * k + p) as usize] = bv;
        oi[(row * k + p) as usize] = bi;
        p += 1u32;
    }
}

#[cube(launch_unchecked)]
fn topk_opt(vals: &[f32], ov: &mut [f32], oi: &mut [u32], vocab: u32, k: u32) {
    let row = CUBE_POS_X;
    let t = UNIT_POS; // 256 threads
    let base = row * vocab;
    let mut cv = Shared::<[f32]>::new_slice(2048usize);
    let mut ci = Shared::<[u32]>::new_slice(2048usize);
    let mut taken_s = Shared::<[u32]>::new_slice(2048usize);
    // zero the taken flags (8 per thread)
    let mut jz = 0u32;
    while jz < 8u32 {
        taken_s[(t + jz * 256u32) as usize] = 0u32;
        jz += 1u32;
    }
    sync_cube();
    // ── per-thread register top-8 over the 512-wide slice ──
    let mut s0v = f32::new(-1.0f32);
    let mut s1v = f32::new(-1.0f32);
    let mut s2v = f32::new(-1.0f32);
    let mut s3v = f32::new(-1.0f32);
    let mut s4v = f32::new(-1.0f32);
    let mut s5v = f32::new(-1.0f32);
    let mut s6v = f32::new(-1.0f32);
    let mut s7v = f32::new(-1.0f32);
    let mut s0i = 0u32;
    let mut s1i = 0u32;
    let mut s2i = 0u32;
    let mut s3i = 0u32;
    let mut s4i = 0u32;
    let mut s5i = 0u32;
    let mut s6i = 0u32;
    let mut s7i = 0u32;
    let mut i = t;
    while i < vocab {
        let v = vals[(base + i) as usize];
        if v > s7v {
            if v > s0v {
                s7v = s6v;
                s7i = s6i;
                s6v = s5v;
                s6i = s5i;
                s5v = s4v;
                s5i = s4i;
                s4v = s3v;
                s4i = s3i;
                s3v = s2v;
                s3i = s2i;
                s2v = s1v;
                s2i = s1i;
                s1v = s0v;
                s1i = s0i;
                s0v = v;
                s0i = i;
            } else {
                if v > s1v {
                    s7v = s6v;
                    s7i = s6i;
                    s6v = s5v;
                    s6i = s5i;
                    s5v = s4v;
                    s5i = s4i;
                    s4v = s3v;
                    s4i = s3i;
                    s3v = s2v;
                    s3i = s2i;
                    s2v = s1v;
                    s2i = s1i;
                    s1v = v;
                    s1i = i;
                } else {
                    if v > s2v {
                        s7v = s6v;
                        s7i = s6i;
                        s6v = s5v;
                        s6i = s5i;
                        s5v = s4v;
                        s5i = s4i;
                        s4v = s3v;
                        s4i = s3i;
                        s3v = s2v;
                        s3i = s2i;
                        s2v = v;
                        s2i = i;
                    } else {
                        if v > s3v {
                            s7v = s6v;
                            s7i = s6i;
                            s6v = s5v;
                            s6i = s5i;
                            s5v = s4v;
                            s5i = s4i;
                            s4v = s3v;
                            s4i = s3i;
                            s3v = v;
                            s3i = i;
                        } else {
                            if v > s4v {
                                s7v = s6v;
                                s7i = s6i;
                                s6v = s5v;
                                s6i = s5i;
                                s5v = s4v;
                                s5i = s4i;
                                s4v = v;
                                s4i = i;
                            } else {
                                if v > s5v {
                                    s7v = s6v;
                                    s7i = s6i;
                                    s6v = s5v;
                                    s6i = s5i;
                                    s5v = v;
                                    s5i = i;
                                } else {
                                    if v > s6v {
                                        s7v = s6v;
                                        s7i = s6i;
                                        s6v = v;
                                        s6i = i;
                                    } else {
                                        s7v = v;
                                        s7i = i;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        i += 256u32;
    }
    // ── stage the 8 candidates to Shared ──
    cv[t as usize] = s0v;
    ci[t as usize] = s0i;
    cv[(t + 256u32) as usize] = s1v;
    ci[(t + 256u32) as usize] = s1i;
    cv[(t + 512u32) as usize] = s2v;
    ci[(t + 512u32) as usize] = s2i;
    cv[(t + 768u32) as usize] = s3v;
    ci[(t + 768u32) as usize] = s3i;
    cv[(t + 1024u32) as usize] = s4v;
    ci[(t + 1024u32) as usize] = s4i;
    cv[(t + 1280u32) as usize] = s5v;
    ci[(t + 1280u32) as usize] = s5i;
    cv[(t + 1536u32) as usize] = s6v;
    ci[(t + 1536u32) as usize] = s6i;
    cv[(t + 1792u32) as usize] = s7v;
    ci[(t + 1792u32) as usize] = s7i;
    sync_cube();
    // ── 8 selection passes over the candidates in Shared ──
    let mut p = 0u32;
    while p < k {
        let mut bv = f32::new(-1.0f32);
        let mut bslot = 0u32;
        let mut j = 0u32;
        while j < 8u32 {
            let slot = t + j * 256u32;
            if taken_s[slot as usize] == 0u32 {
                let v = cv[slot as usize];
                if v > bv {
                    bv = v;
                    bslot = slot;
                }
            }
            j += 1u32;
        }
        let mut smem_v = Shared::<[f32]>::new_slice(256usize);
        let mut smem_s = Shared::<[u32]>::new_slice(256usize);
        smem_v[t as usize] = bv;
        smem_s[t as usize] = bslot;
        sync_cube();
        let mut off = 128u32;
        while off >= 1u32 {
            if t < off {
                let mv = smem_v[t as usize];
                let ms = smem_s[t as usize];
                let othv = smem_v[(t + off) as usize];
                let oths = smem_s[(t + off) as usize];
                if othv > mv {
                    smem_v[t as usize] = othv;
                    smem_s[t as usize] = oths;
                } else {
                    if othv == mv {
                        // tie on value: the candidate with the LOWER global
                        // index wins (slot order ≠ index order)
                        let mi = ci[ms as usize];
                        let othi = ci[oths as usize];
                        if othi < mi {
                            smem_v[t as usize] = othv;
                            smem_s[t as usize] = oths;
                        }
                    }
                }
            }
            sync_cube();
            off = off / 2u32;
        }
        if t == 0u32 {
            let ws = smem_s[0usize];
            taken_s[ws as usize] = 1u32;
            ov[(row * k + p) as usize] = smem_v[0usize];
            oi[(row * k + p) as usize] = ci[ws as usize];
        }
        sync_cube();
        p += 1u32;
    }
}

pub struct P135A {
    vals: Handle,
    vals_host: Vec<f32>,
    taken: Handle,
    ov_ref: Handle,
    oi_ref: Handle,
    ov_opt: Handle,
    oi_opt: Handle,
    vocab: u32,
    batch: u32,
    k: u32,
}

impl P135A {
    fn new(client: &Client) -> Result<Self, String> {
        let (vocab, batch, k) = (131072usize, 64usize, 8usize);
        let mut rng = Lcg::new(seed_for_pair("P135-A"));
        let mut vals = rng.fill(batch * vocab, 0.0, 1.0);
        // seeded exact-duplicate values to exercise ties (bit copies)
        for _ in 0..64usize {
            let row = (rng.next_u64() as usize) % batch;
            let src = (rng.next_u64() as usize) % vocab;
            let mut dst = (rng.next_u64() as usize) % vocab;
            if dst == src {
                dst = (dst + 1) % vocab;
            }
            vals[row * vocab + dst] = vals[row * vocab + src];
        }
        let taken = upload_u32(client, &vec![0u32; vocab * batch]); // per-row mask
        let ov_ref = empty_f32(client, batch * k);
        let oi_ref = empty_u32(client, batch * k);
        let ov_opt = empty_f32(client, batch * k);
        let oi_opt = empty_u32(client, batch * k);
        Ok(Self {
            vals: upload_f32(client, &vals),
            vals_host: vals,
            taken,
            ov_ref,
            oi_ref,
            ov_opt,
            oi_opt,
            vocab: vocab as u32,
            batch: batch as u32,
            k: k as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        unsafe {
            mask_clear::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static((self.vocab * self.batch).div_ceil(256), 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.taken.clone(), (self.vocab * self.batch) as usize),
                self.vocab * self.batch,
            );
            topk_ref::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(1, 1, 1),
                CubeDim::new_1d(64),
                BufferArg::from_raw_parts(self.vals.clone(), (self.batch * self.vocab) as usize),
                BufferArg::from_raw_parts(self.taken.clone(), self.vocab as usize),
                BufferArg::from_raw_parts(self.ov_ref.clone(), (self.batch * self.k) as usize),
                BufferArg::from_raw_parts(self.oi_ref.clone(), (self.batch * self.k) as usize),
                self.vocab,
                self.k,
            );
        }
    }

    fn launch_opt(&self, client: &Client) {
        unsafe {
            topk_opt::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(self.batch, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.vals.clone(), (self.batch * self.vocab) as usize),
                BufferArg::from_raw_parts(self.ov_opt.clone(), (self.batch * self.k) as usize),
                BufferArg::from_raw_parts(self.oi_opt.clone(), (self.batch * self.k) as usize),
                self.vocab,
                self.k,
            );
        }
    }
}

fn p135a_expected_topk(vals: &[f32], row: usize, vocab: usize, k: usize) -> Vec<(u32, f32)> {
    // (value desc, index asc) — the pinned tie law, host-side oracle
    let mut idx: Vec<u32> = (0..vocab as u32).collect();
    idx.sort_by(|a, b| {
        vals[row * vocab + *b as usize]
            .partial_cmp(&vals[row * vocab + *a as usize])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    idx[..k]
        .iter()
        .map(|i| (*i, vals[row * vocab + *i as usize]))
        .collect()
}

pub fn p135a_correctness(client: &Client) -> Result<(), String> {
    let s = P135A::new(client)?;
    let vocab = s.vocab as usize;
    let k = s.k as usize;
    s.launch_ref(client);
    sync_ok(client)?;
    let ov_ref = read_f32(client, &s.ov_ref)?;
    let oi_ref = read_u32(client, &s.oi_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let ov_opt = read_f32(client, &s.ov_opt)?;
    let oi_opt = read_u32(client, &s.oi_opt)?;
    // host oracle first — pins each arm against the tie law directly
    for row in 0..(s.batch as usize) {
        let want = p135a_expected_topk(&s.vals_host, row, vocab, k);
        for (slot, (wi, wv)) in want.iter().enumerate() {
            let ri = oi_ref[row * k + slot];
            let rv = ov_ref[row * k + slot];
            if ri != *wi || rv.to_bits() != wv.to_bits() {
                return Err(format!(
                    "ref disagrees with the host oracle at row {row} pass {slot}: ref=({ri},{rv}) want=({wi},{wv})"
                ));
            }
            let oi_ = oi_opt[row * k + slot];
            let ov_ = ov_opt[row * k + slot];
            if oi_ != *wi || ov_.to_bits() != wv.to_bits() {
                return Err(format!(
                    "opt disagrees with the host oracle at row {row} pass {slot}: opt=({oi_},{ov_}) want=({wi},{wv}); vals at the two idx: {} vs {}",
                    s.vals_host[row * vocab + oi_ as usize],
                    s.vals_host[row * vocab + *wi as usize]
                ));
            }
        }
    }
    Ok(())
}

pub fn p135a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p135a_correctness(client)?;
    let s = P135A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p135a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P121-A int8 screen + rescore ───────────────────────────────────────
// cands=32768 dim=256; values U(-4,4); host quantizes q and c to the i8
// grid (scale 127/8 by default — see the deterministic ladder below).
// ref: fp32 dot of ALL candidates (one kernel).
// opt: kernel 1 — i8 dot (i8 loads, i32 accumulate) over all → screen;
//      kernel 2 — top-256 of the screen by masked selection (one block,
//      256 threads, 256 passes, strict-> + lexicographic tie rule; selected
//      candidates sentinel-overwritten in the screen in place);
//      kernel 3 — fp32 rescore of ONLY the certified 256.
// Gates: (a) ref's top-8 fp32 indices ⊆ opt's certified 256 (error-bound
// property on the seeded data); (b) opt's 256 rescored values == ref's
// values at those indices (rel L∞ 1e-3).
// Scale ladder (task-mandated, deterministic + disclosed): 127/8 first; if
// gate (a) fails, retry 127/4; a second failure is a loud pair FAIL.
// (Analysis: quantization error per candidate dot ≈ 0.5–1.0 at 127/8 while
// the 8th-vs-256th score gap ≈ 50+ for 32768 draws of σ≈85 — wide margin.)

fn quant_i8(v: f32, scale: f32) -> i8 {
    let q = (v * scale).round();
    let q = q.clamp(-127.0, 127.0);
    q as i8
}

#[cube(launch_unchecked)]
fn fp32_dot_cands(q: &[f32], c: &[f32], out: &mut [f32], n_cands: u32, dim: u32) {
    let cand = ABSOLUTE_POS_X;
    if cand >= n_cands {
        terminate!();
    }
    let base = (cand * dim) as usize;
    let mut acc = f32::new(0.0f32);
    let mut d = 0u32;
    while d < dim {
        acc += q[d as usize] * c[base + d as usize];
        d += 1u32;
    }
    out[cand as usize] = acc;
}

#[cube(launch_unchecked)]
fn screen_i8_dot(q8: &[i8], c8: &[i8], out: &mut [i32], n_cands: u32, dim: u32) {
    let cand = ABSOLUTE_POS_X;
    if cand >= n_cands {
        terminate!();
    }
    let base = (cand * dim) as usize;
    let mut acc = 0i32;
    let mut d = 0u32;
    while d < dim {
        acc += (q8[d as usize] as i32) * (c8[base + d as usize] as i32);
        d += 1u32;
    }
    out[cand as usize] = acc;
}

#[cube(launch_unchecked)]
fn topk_select_sentinel(screen: &mut [i32], sel: &mut [u32], total: u32, k: u32) {
    let t = UNIT_POS; // 256 threads, 1 block
    let mut smem_v = Shared::<[f32]>::new_slice(256usize);
    let mut smem_i = Shared::<[u32]>::new_slice(256usize);
    let mut p = 0u32;
    while p < k {
        let mut bv = f32::new(-3.4e38f32);
        let mut bidx = 0u32;
        let mut i = UNIT_POS;
        while i < total {
            let v = screen[i as usize] as f32;
            if v > bv {
                bv = v;
                bidx = i;
            }
            i += 256u32;
        }
        smem_v[t as usize] = bv;
        smem_i[t as usize] = bidx;
        sync_cube();
        let mut off = 128u32;
        while off >= 1u32 {
            if t < off {
                let mv = smem_v[t as usize];
                let mi = smem_i[t as usize];
                let othv = smem_v[(t + off) as usize];
                let othi = smem_i[(t + off) as usize];
                if othv > mv {
                    smem_v[t as usize] = othv;
                    smem_i[t as usize] = othi;
                } else {
                    if othv == mv {
                        if othi < mi {
                            smem_v[t as usize] = othv;
                            smem_i[t as usize] = othi;
                        }
                    }
                }
            }
            sync_cube();
            off = off / 2u32;
        }
        if t == 0u32 {
            let w = smem_i[0usize];
            sel[p as usize] = w;
            // sentinel-overwrite the winner so the next pass skips it
            // (i32::MIN without a negative literal: 0 - 2147483647 - 1)
            screen[w as usize] = 0i32 - 2147483647i32 - 1i32;
        }
        sync_cube();
        p += 1u32;
    }
}

#[cube(launch_unchecked)]
fn rescore_selected(q: &[f32], c: &[f32], sel: &[u32], out: &mut [f32], dim: u32, _k: u32) {
    let t = UNIT_POS; // 256 threads == k (exact)
    let idx = sel[t as usize];
    let base = (idx * dim) as usize;
    let mut acc = f32::new(0.0f32);
    let mut d = 0u32;
    while d < dim {
        acc += q[d as usize] * c[base + d as usize];
        d += 1u32;
    }
    out[t as usize] = acc;
}

pub struct P121A {
    q_f32: Handle,
    c_f32: Handle,
    q8: Handle,
    c8: Handle,
    ref_vals: Handle,
    screen: Handle,
    sel: Handle,
    rescored: Handle,
    n_cands: u32,
    dim: u32,
    k_sel: u32,
}

impl P121A {
    fn new_with_scale(client: &Client, scale: f32) -> Result<Self, String> {
        let (n_cands, dim) = (32768usize, 256usize);
        let k_sel = 256usize;
        let mut rng = Lcg::new(seed_for_pair("P121-A"));
        let q_f32 = rng.fill(dim, -4.0, 4.0);
        let c_f32 = rng.fill(n_cands * dim, -4.0, 4.0);
        let q8: Vec<i8> = q_f32.iter().map(|v| quant_i8(*v, scale)).collect();
        let c8: Vec<i8> = c_f32.iter().map(|v| quant_i8(*v, scale)).collect();
        let ref_vals = empty_f32(client, n_cands);
        let screen = empty_i32(client, n_cands);
        let sel = empty_u32(client, k_sel);
        let rescored = empty_f32(client, k_sel);
        Ok(Self {
            q_f32: upload_f32(client, &q_f32),
            c_f32: upload_f32(client, &c_f32),
            q8: upload_i8(client, &q8),
            c8: upload_i8(client, &c8),
            ref_vals,
            screen,
            sel,
            rescored,
            n_cands: n_cands as u32,
            dim: dim as u32,
            k_sel: k_sel as u32,
        })
    }
}

fn p121a_run_ref(client: &Client, s: &P121A) {
    unsafe {
        fp32_dot_cands::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.n_cands.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.q_f32.clone(), s.dim as usize),
            BufferArg::from_raw_parts(s.c_f32.clone(), (s.n_cands * s.dim) as usize),
            BufferArg::from_raw_parts(s.ref_vals.clone(), s.n_cands as usize),
            s.n_cands,
            s.dim,
        );
    }
}

fn p121a_run_opt(client: &Client, s: &P121A) {
    unsafe {
        screen_i8_dot::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.n_cands.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.q8.clone(), s.dim as usize),
            BufferArg::from_raw_parts(s.c8.clone(), (s.n_cands * s.dim) as usize),
            BufferArg::from_raw_parts(s.screen.clone(), s.n_cands as usize),
            s.n_cands,
            s.dim,
        );
        topk_select_sentinel::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.screen.clone(), s.n_cands as usize),
            BufferArg::from_raw_parts(s.sel.clone(), s.k_sel as usize),
            s.n_cands,
            s.k_sel,
        );
        rescore_selected::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.q_f32.clone(), s.dim as usize),
            BufferArg::from_raw_parts(s.c_f32.clone(), (s.n_cands * s.dim) as usize),
            BufferArg::from_raw_parts(s.sel.clone(), s.k_sel as usize),
            BufferArg::from_raw_parts(s.rescored.clone(), s.k_sel as usize),
            s.dim,
            s.k_sel,
        );
    }
}

fn p121a_try(client: &Client, scale: f32) -> Result<(), String> {
    let s = P121A::new_with_scale(client, scale)?;
    p121a_run_ref(client, &s);
    sync_ok(client)?;
    let ref_vals = read_f32(client, &s.ref_vals)?;
    p121a_run_opt(client, &s);
    sync_ok(client)?;
    let sel = read_u32(client, &s.sel)?;
    let rescored = read_f32(client, &s.rescored)?;
    // gate (a): ref's top-8 (value desc, idx asc on ties) ⊆ the certified 256
    let mut order: Vec<usize> = (0..ref_vals.len()).collect();
    order.sort_by(|a, b| {
        ref_vals[*b]
            .partial_cmp(&ref_vals[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    let top8: Vec<usize> = order[..8].to_vec();
    for t in &top8 {
        if !sel.iter().any(|x| *x as usize == *t) {
            return Err(format!(
                "gate (a): ref top-8 index {t} (val {}) not in the certified 256-slice",
                ref_vals[*t]
            ));
        }
    }
    // gate (b): the rescored values equal ref's values at the selected indices
    for (t, idx) in sel.iter().enumerate() {
        let i = *idx as usize;
        if i >= ref_vals.len() {
            return Err(format!("gate (b): selected index {i} out of range"));
        }
        rel_linf(&rescored[t..t + 1], &ref_vals[i..i + 1], 1e-3)
            .map_err(|e| format!("gate (b) at sel[{t}]={i}: {e}"))?;
    }
    Ok(())
}

pub fn p121a_correctness(client: &Client) -> Result<(), String> {
    let s0 = 127.0f32 / 8.0f32;
    match p121a_try(client, s0) {
        Ok(()) => Ok(()),
        Err(e0) => {
            let s1 = 127.0f32 / 4.0f32;
            match p121a_try(client, s1) {
                Ok(()) => {
                    println!(
                        "DEVIATION P121-A: quantization scale adjusted 127/8 → 127/4 (gate (a) \
                         failed at 127/8: {e0}); deterministic retry, disclosed."
                    );
                    Ok(())
                }
                Err(e1) => Err(format!(
                    "scale ladder exhausted (127/8: {e0}; 127/4: {e1})"
                )),
            }
        }
    }
}

pub fn p121a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p121a_correctness(client)?;
    let s = P121A::new_with_scale(client, 127.0f32 / 8.0f32)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            p121a_run_ref(client, &s);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            p121a_run_opt(client, &s);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p121a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

`````

### File: `src/pairs_stream.rs`

blake3: `7dbbaf9526c90a541133a2634ddccfa1eb4da84a27c04fa869fe868c922bde5a`

`````rust
//! Stream-shaped pairs: P134-A (launch-storm consolidation), P134-B
//! (drain-time partials), P112-A (row transcendental hoist), P112-B (fused
//! chain), P138-B (dual-bank scratch), P75-B (prefetch/ILP walk), P75-C
//! (partial unroll vs the register-spill cliff).


// The #[cube] kernel bodies deliberately use explicit index arithmetic and
// nested ifs (the house convention, e.g. riir-infer norms_cubecl.rs): clippy's
// assign_op_pattern / collapsible_if rewrites target shapes the cubecl macro
// expansion has not proven on indexed places / short-circuit chains.
#![allow(clippy::assign_op_pattern, clippy::collapsible_if)]

use crate::common::{
    bits_exact, empty_f32, empty_i32, interleave_bench, i32_exact, read_f32, read_i32, rel_linf,
    seed_for_pair, sync_ok, upload_f32, upload_i32, ActiveRuntime, Client, Lcg,
};
use cubecl::prelude::*;
use cubecl::server::Handle;

// ── P134-A launch-storm consolidation ──────────────────────────────────
// 64 ops × 4096 elems. ref: 64 launches of `ew_small` (op id as scalar
//      arg; per-op a, b from the shared params buffer). opt: ONE launch
//      `ew_table` over the flattened 64×4096.
// Tolerance: rel L∞ 1e-5 (identical expressions per element).

#[cube(launch_unchecked)]
fn ew_small(params: &[f32], x: &[f32], out: &mut [f32], n_per_op: u32, op: u32) {
    let i = ABSOLUTE_POS_X;
    if i >= n_per_op {
        terminate!();
    }
    let a = params[(op * 2u32) as usize];
    let b = params[(op * 2u32 + 1u32) as usize];
    let e = op * n_per_op + i;
    out[e as usize] = x[e as usize] * a + b;
}

#[cube(launch_unchecked)]
fn ew_table(params: &[f32], x: &[f32], out: &mut [f32], _n_per_op: u32, total: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let op = e >> 12u32; // n_per_op == 4096 == 2^12 (pinned)
    let a = params[(op * 2u32) as usize];
    let b = params[(op * 2u32 + 1u32) as usize];
    out[e as usize] = x[e as usize] * a + b;
}

pub struct P134A {
    params: Handle,
    x: Handle,
    out_ref: Handle,
    out_opt: Handle,
    n_per_op: u32,
    ops: u32,
    total: u32,
}

impl P134A {
    fn new(client: &Client) -> Result<Self, String> {
        let (ops, n_per_op) = (64usize, 4096usize);
        let mut rng = Lcg::new(seed_for_pair("P134-A"));
        let mut params = Vec::with_capacity(ops * 2);
        for _ in 0..ops {
            params.push(rng.next_f32(-1.0, 1.0));
            params.push(rng.next_f32(-1.0, 1.0));
        }
        let params = upload_f32(client, &params);
        let x = upload_f32(client, &rng.fill(ops * n_per_op, -1.0, 1.0));
        let out_ref = empty_f32(client, ops * n_per_op);
        let out_opt = empty_f32(client, ops * n_per_op);
        Ok(Self {
            params,
            x,
            out_ref,
            out_opt,
            n_per_op: n_per_op as u32,
            ops: ops as u32,
            total: (ops * n_per_op) as u32,
        })
    }

    fn launch_ref(&self, client: &Client) {
        let blocks = self.n_per_op.div_ceil(256u32);
        unsafe {
            for op in 0..self.ops {
                ew_small::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(self.params.clone(), (self.ops * 2) as usize),
                    BufferArg::from_raw_parts(self.x.clone(), self.total as usize),
                    BufferArg::from_raw_parts(self.out_ref.clone(), self.total as usize),
                    self.n_per_op,
                    op,
                );
            }
        }
    }

    fn launch_opt(&self, client: &Client) {
        let blocks = self.total.div_ceil(256u32);
        unsafe {
            ew_table::launch_unchecked::<ActiveRuntime>(
                client,
                CubeCount::Static(blocks, 1, 1),
                CubeDim::new_1d(256),
                BufferArg::from_raw_parts(self.params.clone(), (self.ops * 2) as usize),
                BufferArg::from_raw_parts(self.x.clone(), self.total as usize),
                BufferArg::from_raw_parts(self.out_opt.clone(), self.total as usize),
                self.n_per_op,
                self.total,
            );
        }
    }
}

pub fn p134a_correctness(client: &Client) -> Result<(), String> {
    let s = P134A::new(client)?;
    s.launch_ref(client);
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    s.launch_opt(client);
    sync_ok(client)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-5)
}

pub fn p134a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p134a_correctness(client)?;
    let s = P134A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_ref(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            s.launch_opt(client);
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p134a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P134-B drain-time partials ─────────────────────────────────────────
// M=N=512 K=4096, 32 k-splits of 128. ref: per-block partial combined by
//      ONE atomicAdd per element per block (32 atomics/element/round)
//      through i32 fixed point at scale 2^12 (Metal has no device float
//      atomic add — the P111-B disclosure applies verbatim; the rounding
//      happens at the SAME per-block cut in BOTH arms, so the integer sums
//      compare BIT-EXACT). opt: partials into P[32][M*N] + one drain
//      kernel sums once.
// Tolerance: 0 (i32 exact).

#[cube(launch_unchecked)]
fn gemm_atomic_split(a: &[f32], b: &[f32], c_at: &[Atomic<i32>], m: u32, n: u32, kt: u32, kc: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    let z = CUBE_POS_Z;
    if row >= m || col >= n {
        terminate!();
    }
    let k0 = z * kc;
    let mut acc = f32::new(0.0f32);
    let mut e2 = 0u32;
    while e2 < kc {
        acc += a[(row * kt + k0 + e2) as usize] * b[((k0 + e2) * n + col) as usize];
        e2 += 1u32;
    }
    let q = (acc * 4096.0f32) as i32;
    c_at[(row * n + col) as usize].fetch_add(q);
}

#[cube(launch_unchecked)]
fn gemm_partial(a: &[f32], b: &[f32], p: &mut [i32], m: u32, n: u32, kt: u32, kc: u32) {
    let col = ABSOLUTE_POS_X;
    let row = ABSOLUTE_POS_Y;
    let z = CUBE_POS_Z;
    if row >= m || col >= n {
        terminate!();
    }
    let k0 = z * kc;
    let mut acc = f32::new(0.0f32);
    let mut e2 = 0u32;
    while e2 < kc {
        acc += a[(row * kt + k0 + e2) as usize] * b[((k0 + e2) * n + col) as usize];
        e2 += 1u32;
    }
    let q = (acc * 4096.0f32) as i32;
    let pi = z * m * n + row * n + col;
    p[pi as usize] = q;
}

#[cube(launch_unchecked)]
fn gemm_drain(p: &[i32], c: &mut [i32], mn: u32, parts: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= mn {
        terminate!();
    }
    let mut acc = 0i32;
    let mut z = 0u32;
    while z < parts {
        acc += p[(z * mn + e) as usize];
        z += 1u32;
    }
    c[e as usize] = acc;
}

pub struct P134B {
    a: Handle,
    b: Handle,
    c_at: Handle,
    p: Handle,
    c_opt: Handle,
    m: u32,
    n: u32,
    kt: u32,
    parts: u32,
}

impl P134B {
    fn new(client: &Client) -> Result<Self, String> {
        let (m, n, kt) = (512usize, 512usize, 4096usize);
        let parts = 32usize;
        let mut rng = Lcg::new(seed_for_pair("P134-B"));
        let a = upload_f32(client, &rng.fill(m * kt, -0.5, 0.5));
        let b = upload_f32(client, &rng.fill(kt * n, -0.5, 0.5));
        let c_at = upload_i32(client, &vec![0i32; m * n]); // atomic accumulator: zeroed
        let p = empty_i32(client, parts * m * n); // fully overwritten by the partial kernel
        let c_opt = empty_i32(client, m * n); // fully written by the drain
        Ok(Self {
            a,
            b,
            c_at,
            p,
            c_opt,
            m: m as u32,
            n: n as u32,
            kt: kt as u32,
            parts: parts as u32,
        })
    }
}

pub fn p134b_correctness(client: &Client) -> Result<(), String> {
    let s = P134B::new(client)?;
    let bn = s.n.div_ceil(32u32);
    let bm = s.m.div_ceil(32u32);
    let kc = s.kt / s.parts;
    let mn = s.m * s.n;
    unsafe {
        gemm_atomic_split::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(bn, bm, s.parts),
            CubeDim::new_2d(32, 32),
            BufferArg::from_raw_parts(s.a.clone(), (s.m * s.kt) as usize),
            BufferArg::from_raw_parts(s.b.clone(), (s.kt * s.n) as usize),
            BufferArg::from_raw_parts(s.c_at.clone(), mn as usize),
            s.m,
            s.n,
            s.kt,
            kc,
        );
        gemm_partial::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(bn, bm, s.parts),
            CubeDim::new_2d(32, 32),
            BufferArg::from_raw_parts(s.a.clone(), (s.m * s.kt) as usize),
            BufferArg::from_raw_parts(s.b.clone(), (s.kt * s.n) as usize),
            BufferArg::from_raw_parts(s.p.clone(), (s.parts * mn) as usize),
            s.m,
            s.n,
            s.kt,
            kc,
        );
        gemm_drain::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(mn.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.p.clone(), (s.parts * mn) as usize),
            BufferArg::from_raw_parts(s.c_opt.clone(), mn as usize),
            mn,
            s.parts,
        );
    }
    sync_ok(client)?;
    let c_ref = read_i32(client, &s.c_at)?;
    let c_opt = read_i32(client, &s.c_opt)?;
    i32_exact(&c_opt, &c_ref)
}

pub fn p134b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p134b_correctness(client)?;
    let s = P134B::new(client)?;
    let bn = s.n.div_ceil(32u32);
    let bm = s.m.div_ceil(32u32);
    let kc = s.kt / s.parts;
    let mn = s.m * s.n;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                gemm_atomic_split::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(bn, bm, s.parts),
                    CubeDim::new_2d(32, 32),
                    BufferArg::from_raw_parts(s.a.clone(), (s.m * s.kt) as usize),
                    BufferArg::from_raw_parts(s.b.clone(), (s.kt * s.n) as usize),
                    BufferArg::from_raw_parts(s.c_at.clone(), mn as usize),
                    s.m,
                    s.n,
                    s.kt,
                    kc,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                gemm_partial::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(bn, bm, s.parts),
                    CubeDim::new_2d(32, 32),
                    BufferArg::from_raw_parts(s.a.clone(), (s.m * s.kt) as usize),
                    BufferArg::from_raw_parts(s.b.clone(), (s.kt * s.n) as usize),
                    BufferArg::from_raw_parts(s.p.clone(), (s.parts * mn) as usize),
                    s.m,
                    s.n,
                    s.kt,
                    kc,
                );
                gemm_drain::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(mn.div_ceil(256), 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.p.clone(), (s.parts * mn) as usize),
                    BufferArg::from_raw_parts(s.c_opt.clone(), mn as usize),
                    mn,
                    s.parts,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p134b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P112-A row transcendental hoist ────────────────────────────────────
// rows=512 cols=2048. out[r][c] = x[r][c] * expf(s[r]).
// ref: thread-per-row, expf(s[r]) computed INSIDE the c-loop (per element).
// opt: block per row, expf hoisted once (smem broadcast), stride walk.
// Tolerance: rel L∞ 1e-6 (identical expf values — only loop-invariant
// motion; expect bit-exact).

#[cube(launch_unchecked)]
fn hoist_ref(x: &[f32], s: &[f32], out: &mut [f32], rows: u32, cols: u32) {
    let row = ABSOLUTE_POS_X;
    if row >= rows {
        terminate!();
    }
    let mut c = 0u32;
    while c < cols {
        out[(row * cols + c) as usize] = x[(row * cols + c) as usize] * (s[row as usize]).exp();
        c += 1u32;
    }
}

#[cube(launch_unchecked)]
fn hoist_opt(x: &[f32], s: &[f32], out: &mut [f32], cols: u32) {
    let row = CUBE_POS_X;
    let t = UNIT_POS;
    let mut smem = Shared::<[f32]>::new_slice(1usize);
    if t == 0u32 {
        smem[0usize] = (s[row as usize]).exp();
    }
    sync_cube();
    let es = smem[0usize];
    let mut c = t;
    while c < cols {
        out[(row * cols + c) as usize] = x[(row * cols + c) as usize] * es;
        c += 256u32;
    }
}

pub struct P112A {
    x: Handle,
    s: Handle,
    out_ref: Handle,
    out_opt: Handle,
    rows: u32,
    cols: u32,
}

impl P112A {
    fn new(client: &Client) -> Result<Self, String> {
        let (rows, cols) = (512usize, 2048usize);
        let mut rng = Lcg::new(seed_for_pair("P112-A"));
        let x = upload_f32(client, &rng.fill(rows * cols, -1.0, 1.0));
        let s = upload_f32(client, &rng.fill(rows, -1.0, 1.0));
        let out_ref = empty_f32(client, rows * cols);
        let out_opt = empty_f32(client, rows * cols);
        Ok(Self {
            x,
            s,
            out_ref,
            out_opt,
            rows: rows as u32,
            cols: cols as u32,
        })
    }
}

pub fn p112a_correctness(client: &Client) -> Result<(), String> {
    let s = P112A::new(client)?;
    unsafe {
        hoist_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.rows.div_ceil(256), 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
            BufferArg::from_raw_parts(s.s.clone(), s.rows as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), (s.rows * s.cols) as usize),
            s.rows,
            s.cols,
        );
        hoist_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(s.rows, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
            BufferArg::from_raw_parts(s.s.clone(), s.rows as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), (s.rows * s.cols) as usize),
            s.cols,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-6)
}

pub fn p112a_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p112a_correctness(client)?;
    let s = P112A::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                hoist_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(s.rows.div_ceil(256), 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
                    BufferArg::from_raw_parts(s.s.clone(), s.rows as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), (s.rows * s.cols) as usize),
                    s.rows,
                    s.cols,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                hoist_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(s.rows, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.rows * s.cols) as usize),
                    BufferArg::from_raw_parts(s.s.clone(), s.rows as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), (s.rows * s.cols) as usize),
                    s.cols,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p112a_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P112-B fused chain ─────────────────────────────────────────────────
// N = 2^20. ref: 3 kernels (y = x*a + 1; z = y*b + y; o = z*c - z), each
//      materialized to global. opt: ONE fused kernel, intermediates in
//      registers. Tolerance: rel L∞ 1e-5 (identical expression chain →
//      expect bit-exact).

#[cube(launch_unchecked)]
fn chain_y(x: &[f32], y: &mut [f32], total: u32, a: f32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    y[e as usize] = x[e as usize] * a + 1.0f32;
}

#[cube(launch_unchecked)]
fn chain_z(y: &[f32], z: &mut [f32], total: u32, b: f32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let yv = y[e as usize];
    z[e as usize] = yv * b + yv;
}

#[cube(launch_unchecked)]
fn chain_o(z: &[f32], o: &mut [f32], total: u32, c: f32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let zv = z[e as usize];
    o[e as usize] = zv * c - zv;
}

#[cube(launch_unchecked)]
fn chain_fused(x: &[f32], o: &mut [f32], total: u32, a: f32, b: f32, c: f32) {
    let e = ABSOLUTE_POS_X;
    if e >= total {
        terminate!();
    }
    let y = x[e as usize] * a + 1.0f32;
    let z = y * b + y;
    o[e as usize] = z * c - z;
}

pub struct P112B {
    x: Handle,
    y: Handle,
    z: Handle,
    o_ref: Handle,
    o_opt: Handle,
    total: u32,
}

impl P112B {
    fn new(client: &Client) -> Result<Self, String> {
        let n = 1usize << 20;
        let mut rng = Lcg::new(seed_for_pair("P112-B"));
        let x = upload_f32(client, &rng.fill(n, -1.0, 1.0));
        let y = empty_f32(client, n);
        let z = empty_f32(client, n);
        let o_ref = empty_f32(client, n);
        let o_opt = empty_f32(client, n);
        Ok(Self {
            x,
            y,
            z,
            o_ref,
            o_opt,
            total: n as u32,
        })
    }
}

pub fn p112b_correctness(client: &Client) -> Result<(), String> {
    let s = P112B::new(client)?;
    let (a, b, c) = (1.3f32, 0.7f32, 2.1f32);
    let blocks = s.total.div_ceil(256u32);
    unsafe {
        chain_y::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), s.total as usize),
            BufferArg::from_raw_parts(s.y.clone(), s.total as usize),
            s.total,
            a,
        );
        chain_z::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.y.clone(), s.total as usize),
            BufferArg::from_raw_parts(s.z.clone(), s.total as usize),
            s.total,
            b,
        );
        chain_o::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.z.clone(), s.total as usize),
            BufferArg::from_raw_parts(s.o_ref.clone(), s.total as usize),
            s.total,
            c,
        );
        chain_fused::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), s.total as usize),
            BufferArg::from_raw_parts(s.o_opt.clone(), s.total as usize),
            s.total,
            a,
            b,
            c,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.o_ref)?;
    let o_opt = read_f32(client, &s.o_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-5)
}

pub fn p112b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p112b_correctness(client)?;
    let s = P112B::new(client)?;
    let (a, b, c) = (1.3f32, 0.7f32, 2.1f32);
    let blocks = s.total.div_ceil(256u32);
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                chain_y::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), s.total as usize),
                    BufferArg::from_raw_parts(s.y.clone(), s.total as usize),
                    s.total,
                    a,
                );
                chain_z::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.y.clone(), s.total as usize),
                    BufferArg::from_raw_parts(s.z.clone(), s.total as usize),
                    s.total,
                    b,
                );
                chain_o::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.z.clone(), s.total as usize),
                    BufferArg::from_raw_parts(s.o_ref.clone(), s.total as usize),
                    s.total,
                    c,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                chain_fused::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), s.total as usize),
                    BufferArg::from_raw_parts(s.o_opt.clone(), s.total as usize),
                    s.total,
                    a,
                    b,
                    c,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p112b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P138-B dual-bank scratch ───────────────────────────────────────────
// rounds=256 × N=16384. ref: per round a `clear` launch + a `write` launch
//      (2 launches/round). opt: ping-pong banks, 1 write launch/round, no
//      clear. Values deterministic per round: V[256][16384] from the pair
//      LCG, same seed both arms; after 256 rounds ref's buffer and opt's
//      current bank (r=255 → bank 1) hold the identical V row.
// Tolerance: 0 (bit-exact movement).

#[cube(launch_unchecked)]
fn scratch_clear(buf: &mut [f32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    buf[e as usize] = 0.0f32;
}

#[cube(launch_unchecked)]
fn scratch_write(src: &[f32], dst: &mut [f32], n: u32, r: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    dst[e as usize] = src[(r * n + e) as usize];
}

pub struct P138B {
    v: Handle,
    buf_ref: Handle,
    bank0: Handle,
    bank1: Handle,
    n: u32,
    rounds: u32,
}

impl P138B {
    fn new(client: &Client) -> Result<Self, String> {
        let (rounds, n) = (256usize, 16384usize);
        let mut rng = Lcg::new(seed_for_pair("P138-B"));
        let v = upload_f32(client, &rng.fill(rounds * n, 0.0, 1.0));
        let buf_ref = empty_f32(client, n);
        let bank0 = empty_f32(client, n);
        let bank1 = empty_f32(client, n);
        Ok(Self {
            v,
            buf_ref,
            bank0,
            bank1,
            n: n as u32,
            rounds: rounds as u32,
        })
    }

    fn run_ref(&self, client: &Client) {
        let blocks = self.n.div_ceil(256u32);
        for r in 0..self.rounds {
            unsafe {
                scratch_clear::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(self.buf_ref.clone(), self.n as usize),
                    self.n,
                );
                scratch_write::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(self.v.clone(), (self.rounds * self.n) as usize),
                    BufferArg::from_raw_parts(self.buf_ref.clone(), self.n as usize),
                    self.n,
                    r,
                );
            }
        }
    }

    fn run_opt(&self, client: &Client) {
        let blocks = self.n.div_ceil(256u32);
        for r in 0..self.rounds {
            let bank = if (r & 1u32) == 0u32 {
                self.bank0.clone()
            } else {
                self.bank1.clone()
            };
            unsafe {
                scratch_write::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(self.v.clone(), (self.rounds * self.n) as usize),
                    BufferArg::from_raw_parts(bank, self.n as usize),
                    self.n,
                    r,
                );
            }
        }
    }
}

pub fn p138b_correctness(client: &Client) -> Result<(), String> {
    let s = P138B::new(client)?;
    s.run_ref(client);
    s.run_opt(client);
    sync_ok(client)?;
    let a = read_f32(client, &s.buf_ref)?;
    let b = read_f32(client, &s.bank1)?; // last round r=255 → bank 1
    bits_exact(&b, &a)
}

pub fn p138b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p138b_correctness(client)?;
    let s = P138B::new(client)?;
    let blocks = s.n.div_ceil(256u32);
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for r in 0..count {
            unsafe {
                scratch_clear::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.buf_ref.clone(), s.n as usize),
                    s.n,
                );
                scratch_write::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.v.clone(), (s.rounds * s.n) as usize),
                    BufferArg::from_raw_parts(s.buf_ref.clone(), s.n as usize),
                    s.n,
                    r,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for r in 0..count {
            let bank = if (r & 1u32) == 0u32 {
                s.bank0.clone()
            } else {
                s.bank1.clone()
            };
            unsafe {
                scratch_write::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.v.clone(), (s.rounds * s.n) as usize),
                    BufferArg::from_raw_parts(bank, s.n as usize),
                    s.n,
                    r,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p138b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P75-B prefetch/ILP walk ────────────────────────────────────────────
// N = 2^22, one block of 256 threads, stride 32768 (= 256 × 128).
// ref: each thread stride-walks with ONE serial accumulator.
// opt: 4 independent accumulators over interleaved sub-streams (k = 4g+j),
//      summed (a0+a1)+(a2+a3) at the end — the ILP axis.
// DEVIATION (association, disclosed): opt's final sum changes the fp
// addition order — the registration prices exactly this axis.
// Tolerance: rel L∞ 1e-3.

#[cube(launch_unchecked)]
fn strided_walk_ref(x: &[f32], out: &mut [f32], n: u32, stride: u32) {
    let t = UNIT_POS;
    let mut acc = f32::new(0.0f32);
    let mut i = t;
    while i < n {
        acc += x[i as usize];
        i += stride;
    }
    out[t as usize] = acc;
}

#[cube(launch_unchecked)]
fn strided_walk_ilp_opt(x: &[f32], out: &mut [f32], n: u32, stride: u32) {
    let t = UNIT_POS;
    let mut a0 = f32::new(0.0f32);
    let mut a1 = f32::new(0.0f32);
    let mut a2 = f32::new(0.0f32);
    let mut a3 = f32::new(0.0f32);
    let mut g = 0u32;
    while (g * 4u32) * stride + t < n {
        let b0 = (g * 4u32) * stride;
        if t + b0 < n {
            a0 += x[(t + b0) as usize];
        }
        if t + b0 + stride < n {
            a1 += x[(t + b0 + stride) as usize];
        }
        if t + b0 + 2u32 * stride < n {
            a2 += x[(t + b0 + 2u32 * stride) as usize];
        }
        if t + b0 + 3u32 * stride < n {
            a3 += x[(t + b0 + 3u32 * stride) as usize];
        }
        g += 1u32;
    }
    out[t as usize] = (a0 + a1) + (a2 + a3);
}

pub struct P75B {
    x: Handle,
    out_ref: Handle,
    out_opt: Handle,
    n: u32,
    stride: u32,
}

impl P75B {
    fn new(client: &Client) -> Result<Self, String> {
        let n = 1usize << 22;
        let mut rng = Lcg::new(seed_for_pair("P75-B"));
        let x = upload_f32(client, &rng.fill(n, -1.0, 1.0));
        let out_ref = empty_f32(client, 256);
        let out_opt = empty_f32(client, 256);
        Ok(Self {
            x,
            out_ref,
            out_opt,
            n: n as u32,
            stride: 32768,
        })
    }
}

pub fn p75b_correctness(client: &Client) -> Result<(), String> {
    let s = P75B::new(client)?;
    unsafe {
        strided_walk_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), s.n as usize),
            BufferArg::from_raw_parts(s.out_ref.clone(), 256),
            s.n,
            s.stride,
        );
        strided_walk_ilp_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), s.n as usize),
            BufferArg::from_raw_parts(s.out_opt.clone(), 256),
            s.n,
            s.stride,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-3)
}

pub fn p75b_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p75b_correctness(client)?;
    let s = P75B::new(client)?;
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                strided_walk_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(1, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), s.n as usize),
                    BufferArg::from_raw_parts(s.out_ref.clone(), 256),
                    s.n,
                    s.stride,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                strided_walk_ilp_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(1, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), s.n as usize),
                    BufferArg::from_raw_parts(s.out_opt.clone(), 256),
                    s.n,
                    s.stride,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p75b_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

// ── P75-C partial unroll (register-spill cliff) ────────────────────────
// window=64 conv scan over 2^20 elems: out[i] = Σ_{w<64} x[i+w]·kern[w].
// ref: manually fully-unrolled body — 64 live products per iteration (the
//      register-pressure cliff), kern read from GLOBAL.
// opt: outer loop over 16 groups of 4 MACs re-reading kern from Shared
//      (lower pressure). x carries +64 tail elements so i+w is in bounds.
// Tolerance: rel L∞ 1e-3 (SAME ascending-w association → expect
// bit-exact).

#[cube(launch_unchecked)]
fn conv_unroll_ref(x: &[f32], kern: &[f32], out: &mut [f32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    let p0 = x[e as usize] * kern[0usize];
    let p1 = x[(e + 1u32) as usize] * kern[1usize];
    let p2 = x[(e + 2u32) as usize] * kern[2usize];
    let p3 = x[(e + 3u32) as usize] * kern[3usize];
    let p4 = x[(e + 4u32) as usize] * kern[4usize];
    let p5 = x[(e + 5u32) as usize] * kern[5usize];
    let p6 = x[(e + 6u32) as usize] * kern[6usize];
    let p7 = x[(e + 7u32) as usize] * kern[7usize];
    let p8 = x[(e + 8u32) as usize] * kern[8usize];
    let p9 = x[(e + 9u32) as usize] * kern[9usize];
    let p10 = x[(e + 10u32) as usize] * kern[10usize];
    let p11 = x[(e + 11u32) as usize] * kern[11usize];
    let p12 = x[(e + 12u32) as usize] * kern[12usize];
    let p13 = x[(e + 13u32) as usize] * kern[13usize];
    let p14 = x[(e + 14u32) as usize] * kern[14usize];
    let p15 = x[(e + 15u32) as usize] * kern[15usize];
    let p16 = x[(e + 16u32) as usize] * kern[16usize];
    let p17 = x[(e + 17u32) as usize] * kern[17usize];
    let p18 = x[(e + 18u32) as usize] * kern[18usize];
    let p19 = x[(e + 19u32) as usize] * kern[19usize];
    let p20 = x[(e + 20u32) as usize] * kern[20usize];
    let p21 = x[(e + 21u32) as usize] * kern[21usize];
    let p22 = x[(e + 22u32) as usize] * kern[22usize];
    let p23 = x[(e + 23u32) as usize] * kern[23usize];
    let p24 = x[(e + 24u32) as usize] * kern[24usize];
    let p25 = x[(e + 25u32) as usize] * kern[25usize];
    let p26 = x[(e + 26u32) as usize] * kern[26usize];
    let p27 = x[(e + 27u32) as usize] * kern[27usize];
    let p28 = x[(e + 28u32) as usize] * kern[28usize];
    let p29 = x[(e + 29u32) as usize] * kern[29usize];
    let p30 = x[(e + 30u32) as usize] * kern[30usize];
    let p31 = x[(e + 31u32) as usize] * kern[31usize];
    let p32 = x[(e + 32u32) as usize] * kern[32usize];
    let p33 = x[(e + 33u32) as usize] * kern[33usize];
    let p34 = x[(e + 34u32) as usize] * kern[34usize];
    let p35 = x[(e + 35u32) as usize] * kern[35usize];
    let p36 = x[(e + 36u32) as usize] * kern[36usize];
    let p37 = x[(e + 37u32) as usize] * kern[37usize];
    let p38 = x[(e + 38u32) as usize] * kern[38usize];
    let p39 = x[(e + 39u32) as usize] * kern[39usize];
    let p40 = x[(e + 40u32) as usize] * kern[40usize];
    let p41 = x[(e + 41u32) as usize] * kern[41usize];
    let p42 = x[(e + 42u32) as usize] * kern[42usize];
    let p43 = x[(e + 43u32) as usize] * kern[43usize];
    let p44 = x[(e + 44u32) as usize] * kern[44usize];
    let p45 = x[(e + 45u32) as usize] * kern[45usize];
    let p46 = x[(e + 46u32) as usize] * kern[46usize];
    let p47 = x[(e + 47u32) as usize] * kern[47usize];
    let p48 = x[(e + 48u32) as usize] * kern[48usize];
    let p49 = x[(e + 49u32) as usize] * kern[49usize];
    let p50 = x[(e + 50u32) as usize] * kern[50usize];
    let p51 = x[(e + 51u32) as usize] * kern[51usize];
    let p52 = x[(e + 52u32) as usize] * kern[52usize];
    let p53 = x[(e + 53u32) as usize] * kern[53usize];
    let p54 = x[(e + 54u32) as usize] * kern[54usize];
    let p55 = x[(e + 55u32) as usize] * kern[55usize];
    let p56 = x[(e + 56u32) as usize] * kern[56usize];
    let p57 = x[(e + 57u32) as usize] * kern[57usize];
    let p58 = x[(e + 58u32) as usize] * kern[58usize];
    let p59 = x[(e + 59u32) as usize] * kern[59usize];
    let p60 = x[(e + 60u32) as usize] * kern[60usize];
    let p61 = x[(e + 61u32) as usize] * kern[61usize];
    let p62 = x[(e + 62u32) as usize] * kern[62usize];
    let p63 = x[(e + 63u32) as usize] * kern[63usize];
    let mut acc = f32::new(0.0f32);
    acc += p0;
    acc += p1;
    acc += p2;
    acc += p3;
    acc += p4;
    acc += p5;
    acc += p6;
    acc += p7;
    acc += p8;
    acc += p9;
    acc += p10;
    acc += p11;
    acc += p12;
    acc += p13;
    acc += p14;
    acc += p15;
    acc += p16;
    acc += p17;
    acc += p18;
    acc += p19;
    acc += p20;
    acc += p21;
    acc += p22;
    acc += p23;
    acc += p24;
    acc += p25;
    acc += p26;
    acc += p27;
    acc += p28;
    acc += p29;
    acc += p30;
    acc += p31;
    acc += p32;
    acc += p33;
    acc += p34;
    acc += p35;
    acc += p36;
    acc += p37;
    acc += p38;
    acc += p39;
    acc += p40;
    acc += p41;
    acc += p42;
    acc += p43;
    acc += p44;
    acc += p45;
    acc += p46;
    acc += p47;
    acc += p48;
    acc += p49;
    acc += p50;
    acc += p51;
    acc += p52;
    acc += p53;
    acc += p54;
    acc += p55;
    acc += p56;
    acc += p57;
    acc += p58;
    acc += p59;
    acc += p60;
    acc += p61;
    acc += p62;
    acc += p63;
    out[e as usize] = acc;
}

#[cube(launch_unchecked)]
fn conv_partial_opt(x: &[f32], kern: &[f32], out: &mut [f32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    let mut smem_k = Shared::<[f32]>::new_slice(64usize);
    if UNIT_POS < 64u32 {
        smem_k[UNIT_POS as usize] = kern[UNIT_POS as usize];
    }
    sync_cube();
    let mut acc = f32::new(0.0f32);
    let mut g = 0u32;
    while g < 16u32 {
        let w0 = g * 4u32;
        acc += x[(e + w0) as usize] * smem_k[w0 as usize];
        acc += x[(e + w0 + 1u32) as usize] * smem_k[(w0 + 1u32) as usize];
        acc += x[(e + w0 + 2u32) as usize] * smem_k[(w0 + 2u32) as usize];
        acc += x[(e + w0 + 3u32) as usize] * smem_k[(w0 + 3u32) as usize];
        g += 1u32;
    }
    out[e as usize] = acc;
}

pub struct P75C {
    x: Handle,
    kern: Handle,
    out_ref: Handle,
    out_opt: Handle,
    n: u32,
}

impl P75C {
    fn new(client: &Client) -> Result<Self, String> {
        let n = 1usize << 20;
        let mut rng = Lcg::new(seed_for_pair("P75-C"));
        let x = upload_f32(client, &rng.fill(n + 64, -1.0, 1.0));
        let kern = upload_f32(client, &rng.fill(64, -0.5, 0.5));
        let out_ref = empty_f32(client, n);
        let out_opt = empty_f32(client, n);
        Ok(Self {
            x,
            kern,
            out_ref,
            out_opt,
            n: n as u32,
        })
    }
}

pub fn p75c_correctness(client: &Client) -> Result<(), String> {
    let s = P75C::new(client)?;
    let blocks = s.n.div_ceil(256u32);
    unsafe {
        conv_unroll_ref::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.n + 64) as usize),
            BufferArg::from_raw_parts(s.kern.clone(), 64),
            BufferArg::from_raw_parts(s.out_ref.clone(), s.n as usize),
            s.n,
        );
        conv_partial_opt::launch_unchecked::<ActiveRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(s.x.clone(), (s.n + 64) as usize),
            BufferArg::from_raw_parts(s.kern.clone(), 64),
            BufferArg::from_raw_parts(s.out_opt.clone(), s.n as usize),
            s.n,
        );
    }
    sync_ok(client)?;
    let o_ref = read_f32(client, &s.out_ref)?;
    let o_opt = read_f32(client, &s.out_opt)?;
    rel_linf(&o_opt, &o_ref, 1e-3)
}

pub fn p75c_bench(client: &Client, iters: Option<u32>) -> Result<(f64, f64, f64, u32), String> {
    p75c_correctness(client)?;
    let s = P75C::new(client)?;
    let blocks = s.n.div_ceil(256u32);
    let ref_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                conv_unroll_ref::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.n + 64) as usize),
                    BufferArg::from_raw_parts(s.kern.clone(), 64),
                    BufferArg::from_raw_parts(s.out_ref.clone(), s.n as usize),
                    s.n,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let opt_chunk = |n: u32| {
        let count = std::hint::black_box(n);
        for _ in 0..count {
            unsafe {
                conv_partial_opt::launch_unchecked::<ActiveRuntime>(
                    client,
                    CubeCount::Static(blocks, 1, 1),
                    CubeDim::new_1d(256),
                    BufferArg::from_raw_parts(s.x.clone(), (s.n + 64) as usize),
                    BufferArg::from_raw_parts(s.kern.clone(), 64),
                    BufferArg::from_raw_parts(s.out_opt.clone(), s.n as usize),
                    s.n,
                );
            }
        }
        pollster::block_on(client.sync()).ok();
    };
    let out = interleave_bench(&ref_chunk, &opt_chunk, iters)?;
    p75c_correctness(client)?;
    Ok((out.ref_ms, out.opt_ms, out.median_speedup, out.iters))
}

`````

### File: `src/probe.rs`

blake3: `3b61321604370f3ddc34af76ea12be9c8557010648f31883e3d47689ddea1ab2`

`````rust
//! Backend API probe — one micro-kernel per risky cubecl feature, run before
//! any pair, so a pair failure is attributable to the pair's math rather than
//! a backend gap:
//!   scalar launch args + 2D dispatch · Shared + barrier tree reduce ·
//!   Atomic<i32> fetch_add · i8 buffers with i32 accumulate ·
//!   u32<->f32 reinterpret (bitcast) · select().
//! Informational: probes print PASS/FAILED and never gate the run; the pair
//! correctness gates are the real gates.


// The #[cube] kernel bodies deliberately use explicit index arithmetic and
// nested ifs (the house convention, e.g. riir-infer norms_cubecl.rs): clippy's
// assign_op_pattern / collapsible_if rewrites target shapes the cubecl macro
// expansion has not proven on indexed places / short-circuit chains.
#![allow(clippy::assign_op_pattern, clippy::collapsible_if)]

use crate::common::{read_f32, read_i32, sync_ok, upload_f32, upload_i32, upload_i8, Client};
use cubecl::prelude::*;

#[cube(launch_unchecked)]
fn probe_scalar_2d(out: &mut [f32], cols: u32, mul: f32) {
    let r = ABSOLUTE_POS_Y;
    let c = ABSOLUTE_POS_X;
    if r >= 2u32 || c >= cols {
        terminate!();
    }
    out[(r * cols + c) as usize] = (r + c) as f32 * mul;
}

#[cube(launch_unchecked)]
fn probe_smem_reduce(out: &mut [f32], n: u32) {
    let t = UNIT_POS;
    let mut smem = Shared::<[f32]>::new_slice(256usize);
    let mut v = f32::new(0.0f32);
    if t < n {
        v = t as f32;
    }
    smem[t as usize] = v;
    sync_cube();
    let mut off = 128u32;
    while off >= 1u32 {
        if t < off {
            smem[t as usize] = smem[t as usize] + smem[(t + off) as usize];
        }
        sync_cube();
        off = off / 2u32;
    }
    if t == 0u32 {
        out[0usize] = smem[0usize];
    }
}

#[cube(launch_unchecked)]
fn probe_atomic(acc: &[Atomic<i32>], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    acc[0usize].fetch_add(1i32);
}

#[cube(launch_unchecked)]
fn probe_i8_dot(a: &[i8], b: &[i8], out: &mut [i32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= 1u32 {
        terminate!();
    }
    let mut acc = 0i32;
    let mut i = 0u32;
    while i < n {
        acc += (a[i as usize] as i32) * (b[i as usize] as i32);
        i += 1u32;
    }
    out[0usize] = acc;
}

#[cube(launch_unchecked)]
fn probe_reinterpret(v_in: &[f32], out: &mut [f32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    let bits = u32::reinterpret(v_in[e as usize]);
    let bumped = bits ^ 0x00000001u32;
    out[e as usize] = f32::reinterpret(bumped);
}

#[cube(launch_unchecked)]
fn probe_select(c_in: &[f32], out: &mut [f32], n: u32) {
    let e = ABSOLUTE_POS_X;
    if e >= n {
        terminate!();
    }
    let v = c_in[e as usize];
    let better = v > 0.5f32;
    out[e as usize] = select(better, v, 0.0f32);
}

fn check(name: &str, r: Result<(), String>) {
    match r {
        Ok(()) => println!("PROBE {name}: ok"),
        Err(e) => println!("PROBE {name}: FAILED :: {e}"),
    }
}

pub fn run_all(client: &Client) {
    // scalar args + 2D dispatch
    let out = crate::common::empty_f32(client, 8);
    unsafe {
        probe_scalar_2d::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(2, 2, 1),
            CubeDim::new_2d(4, 2),
            BufferArg::from_raw_parts(out.clone(), 8),
            4u32,
            2.0f32,
        );
    }
    let _ = sync_ok(client);
    let got = read_f32(client, &out).unwrap_or_default();
    let want: Vec<f32> = (0..2usize)
        .flat_map(|r| (0..4usize).map(move |c| (r + c) as f32 * 2.0))
        .collect();
    check(
        "scalar_2d",
        crate::common::rel_linf(&got, &want, 1e-6).map_err(|e| format!("scalar args / 2D dispatch: {e}")),
    );

    // shared memory + barrier tree reduce (256 threads, one block)
    let out = crate::common::empty_f32(client, 1);
    unsafe {
        probe_smem_reduce::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(out.clone(), 1),
            256u32,
        );
    }
    let _ = sync_ok(client);
    let got = read_f32(client, &out).unwrap_or_default();
    check(
        "smem_reduce",
        if got.len() == 1 && (got[0] - 32640.0).abs() < 1e-2 {
            Ok(())
        } else {
            Err(format!("smem tree reduce: got {got:?} want [32640]"))
        },
    );

    // i32 device atomics
    let acc = upload_i32(client, &[0i32]);
    unsafe {
        probe_atomic::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(2, 1, 1),
            CubeDim::new_1d(256),
            BufferArg::from_raw_parts(acc.clone(), 1),
            512u32,
        );
    }
    let _ = sync_ok(client);
    let got = read_i32(client, &acc).unwrap_or_default();
    check(
        "atomic_i32",
        if got.len() == 1 && got[0] == 512 {
            Ok(())
        } else {
            Err(format!("atomic fetch_add: got {got:?} want [512]"))
        },
    );

    // i8 buffers + i32 accumulate
    let a = upload_i8(client, &[2i8, -3, 4]);
    let b = upload_i8(client, &[5i8, 6, -7]);
    let out = crate::common::empty_i32(client, 1);
    unsafe {
        probe_i8_dot::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(a, 3),
            BufferArg::from_raw_parts(b, 3),
            BufferArg::from_raw_parts(out.clone(), 1),
            3u32,
        );
    }
    let _ = sync_ok(client);
    let got = read_i32(client, &out).unwrap_or_default();
    check(
        "i8_dot",
        if got.len() == 1 && got[0] == -36 {
            Ok(())
        } else {
            Err(format!("i8 dot: got {got:?} want [-36]"))
        },
    );

    // u32 <-> f32 reinterpret bitcast
    let v = upload_f32(client, &[1.0f32]);
    let out = crate::common::empty_f32(client, 1);
    unsafe {
        probe_reinterpret::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(v, 1),
            BufferArg::from_raw_parts(out.clone(), 1),
            1u32,
        );
    }
    let _ = sync_ok(client);
    let got = read_f32(client, &out).unwrap_or_default();
    check(
        "reinterpret_bits",
        if got.len() == 1 && got.first().is_some_and(|g| g.to_bits() == 1.0f32.to_bits() ^ 1) {
            Ok(())
        } else {
            Err(format!("reinterpret roundtrip: got {got:?}"))
        },
    );

    // select()
    let v = upload_f32(client, &[0.9f32, 0.1]);
    let out = crate::common::empty_f32(client, 2);
    unsafe {
        probe_select::launch_unchecked::<crate::common::ActiveRuntime>(
            client,
            CubeCount::Static(1, 1, 1),
            CubeDim::new_1d(2),
            BufferArg::from_raw_parts(v, 2),
            BufferArg::from_raw_parts(out.clone(), 2),
            2u32,
        );
    }
    let _ = sync_ok(client);
    let got = read_f32(client, &out).unwrap_or_default();
    check(
        "select",
        crate::common::rel_linf(&got, &[0.9, 0.0], 1e-6)
            .map_err(|e| format!("select(): {e}")),
    );
}

`````
