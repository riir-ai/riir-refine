# ANE ecosystem map + the debugging methodology that worked

> Where the external knowledge lives, what each source is good for, and
> the bisection playbook that cracked a fully-redacted private-API failure
> in one session. All links resolved 2026-08-19.

## The ecosystem

### maderix/ANE — the load-bearing reference

**github.com/maderix/ANE** — ObjC training framework ("Training neural
networks directly on Apple's Neural Engine"). The ONLY codebase verified
working on THIS box (compiled `ane_int8_bench.m` natively → 18.45 TFLOPS
fp16 on M3 Max/macOS 26.5.2). Their docs (full index at
`mintlify.wiki/maderix/ANE/llms.txt`, also maderix-ane.mintlify.app) cover
MIL generation, weight blobs, IOSurface I/O, runtime API, and training
loops. Key files: `ane_int8_bench.m` (self-contained benchmark — the
compile-it-first canonical test), `training/ane_runtime.h` +
`ane_mil_gen.h` (the wrappers), `bridge/ane_bridge.*`.

**Their issue tracker is a goldmine — read it before debugging:**

| Issue | What it teaches |
|---|---|
| #42 | M3 Ultra (macOS 15.7.4): their bench accepted ONLY exactly 512 channels (-4/-3 otherwise) — per-arch channel quirks are real; did NOT reproduce on M3 Max/26.5 |
| #27 | macOS 26 broke `MLModel compileModelAtURL:`; the in-memory `_ANEInMemoryModel` MIL path is the fix (i.e. the in-memory path DID work on early 26.x) |
| #40 | The runtime-generation map: `_ANEChainingRequest` (legacy, dead on 15+), **E5 runtime (`MLE5Engine`/`MLE5ProgramLibraryOnDeviceAOTCompilationImpl`)** — the modern CoreML-internal path, custom-MIL-capable — and where each breaks |
| #50 | M5 Max = first h17 ANE on record (arch generations: h11 M1, h13 M2, h15 M3/M4-era, h17 M5) |
| #44/#26 | ane-infer + full-transformer-on-ANE were built ON this framework |

Substack write-ups (`maderix.substack.com`) include "Inside the M4 Apple
Neural Engine" reverse-engineering series.

### rane (crates.io) — the pure-Rust precedent, with caveats

`rane` 0.2.0 (Apr 2026) — pure-Rust dlopen+msgSend driver; the Rust
precedent for Research 155's "Path B". Caveats learned the hard way:
README says zero-dep but 0.2.0 imports `acpu` + `unimem`; their hardware
tests ran on M1 Pro (macOS 15-era); **their BLOBFILE weight path was never
hardware-exercised** (the example passes `&[]` weights — the dynamic-matmul
MIL carries weights on the input surface). Fine as a source of FFI
patterns; not as a source of verified MIL text.

### omlx PR #2756 (+ #2760/#2781/#2803) — the production hybrid

github.com/jundot/omlx — merged 2026-08-17. The dual-ANE/GPU hybrid
prefill implementation this whole line descends from (distilled in
katgpt-rs `.research/489`): instance pinning, procedure banks, fused
SwiGLU merge, INT8 requant, fail-open eligibility. **Their macOS/boxes:
M3 Ultra (merged, macOS unstated — plausibly 15.7) and M3 Max (#2781,
open).** Their MIL forms did NOT compile here as-written — treat their
format strings as needing the brace/space audit (see
`mil_and_blob_formats.md`) per macOS version.

### Papers + background

- **Orion: Characterizing and Programming Apple's Neural Engine**
  (arXiv:2603.06728, 2026) — the academic characterization; "MIL codegen
  emits text-format MIL programs with BLOBFILE weight references"; cites
  the maderix-adjacent ANEgpt work.
- hollance/neural-engine (GitHub) — the long-running ANE background repo
  (what the ANE is, prior reverse-engineering history).
- katgpt-rs `.research/377` — our own 302-page ANE architecture
  distillation (dispatch floor ~0.23-0.28 ms, 2 MB SRAM, int8 formats,
  h11-era layout details).

## The debugging playbook (what cracked it)

The failure: every NSError said only `InvalidMILProgram`; the unified log
redacted everything (`<private>`); four external "known-good" MILs all
failed. Ordered by payoff:

1. **Runtime introspection over guessing**: dump the private classes'
   method lists (`class_copyMethodList`) — settles selector-drift
   questions in one run ("is it the API or the arguments?").
2. **Read the system log even when redacted**:
   `log show --predicate 'process == "ANECompilerService"'` — the
   category line ("Parsing error … Could not convert input MIL program")
   named the STAGE even when the detail was private. `--private` needs
   sudo (not available) — the category alone was half the diagnosis.
3. **Find the runtime's own artifacts**: the compiler materializes
   `{TMPDIR}/{sha256(mil)}_{sha256(weights)}_{sha256(options)}/net.plist`
   — reading OUR net.plist proved the bytes arrived intact and moved the
   suspicion from "content" to "object graph".
4. **Race a watcher to capture SUCCESS artifacts**: successful compiles
   clean up their dirs; a 50 ms poll-loop `cp -R` around a reference
   binary's run captured working `net.plist`/`model.mil`/`weights/` for
   byte-diff against ours.
5. **Compile the reference implementation natively and run it on the same
   box** — the single highest-value move. `xcrun clang` + their
   self-contained bench answered "system-broken vs us-broken" in one
   command (18 TFLOPS = system fine).
6. **Feed their exact bytes through your stack** (mil+blob from files)
   — splits "my generator differs" from "my plumbing differs".
7. **Stop and pivot when the bisection converges on an unfalsifiable
   residue**: identical bytes + divergent outcomes = object-graph
   territory; wrap the proven path (ObjC bridge) instead of burning the
   session on objc internals.

Also: NSString-vs-NSData carrier bugs masquerade as crashes (foreign
exception → Rust abort, no message) — checkpoint-`eprintln!` bisection is
the only way through when you can't catch the exception
(`objc_setExceptionHandler` is NOT linkable on this toolchain).
