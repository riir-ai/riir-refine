# The pure-Rust msgSend bridge negative result + the working ObjC substrate

> Recorded 2026-08-19 (Issue 726 T0). The most mysterious finding of the
> spike — read this before attempting any Rust-side ANE work, and read it
> again before concluding the runtime is "broken" on a new macOS.

## The result in one paragraph

A pure-Rust bridge (`riir-poc/src/ane_t0.rs`, `AneLinear`) — dlopen +
`objc_msgSend` transmutes, same classes, same selectors, same sequence —
reaches `ANECCompile` with **byte-identical MIL text and weight blobs**
(verified end-to-end: the runtime's own hash-triple compile dir, its
serialized `net.plist` containing our exact MIL, its `data` file exactly
our blob) and is **rejected every time** with
`Error: Could not convert input MIL program`. The identical artifacts
through a 200-line compiled ObjC bridge compile and evaluate first try
(cosine 1.000016). Four different MIL variants, NSData vs NSString
carriers, QoS 0 vs 21, empty vs hint options, tmp-dir name with/without
prefix, chunk-header bit-width field — none of it mattered. **Root cause
never isolated.**

## What was ruled out

- **MIL text**: byte-diffed identical to the working reference (`diff`
  clean after marker-line stripping).
- **Weight blob**: same bytes (the runtime's serialized `data` file
  matched our blob length-for-length).
- **Selector drift**: `class_copyMethodList` dump confirmed every selector
  exists with expected names on this build.
- **Carrier type**: NSData (rane-style) gets furthest; NSString as the
  MILText argument **throws an ObjC exception** that unwinds into Rust as
  `fatal runtime error: Rust cannot catch foreign exceptions` (verified
  twice, with clean surrounding code — the first observation was
  invalidated by a since-fixed bug in the weights-dict construction, the
  retest was clean).
- **QoS, options dict, tmp-dir naming, hexStringIdentifier ordering**:
  all A/B'd, no effect.

## The fingerprint of the failure

The runtime serializes a WORKING descriptor as
`{hash-dir}/model.mil + weights/ + net.plist`, but the Rust-built one as
`{hash-dir}/net.plist + data` only — i.e. **the runtime treats the
Rust-built descriptor as a different kind of thing** despite hashing the
same bytes. Something in the object graph (the msgSend-constructed
NSDictionary/NSData graph passed to `modelWithMILText:weights:optionsPlist:`)
differs from what the ObjC compiler emits for the literal equivalents.
Candidates never isolated: autorelease-pool interactions, NSMutable vs
literal NSDictionary internals, NSData subclass differences
(`dataWithBytes:` vs `dataWithBytesNoCopy:`/`dataUsingEncoding:`),
or something subtler in how the descriptor validates its inputs.

## The working substrate pattern (use this)

**Don't fight it — wrap ObjC.** The spike ships the pattern:

```
crates/riir-poc/
  objc/ane_t0_bridge.m     # ObjC: the proven flow verbatim behind a C ABI
  build.rs                 # macOS-gated: xcrun clang -O2 -fobjc-arc -dynamiclib
                           #   -framework Foundation -framework IOSurface -ldl
                           #   → $OUT_DIR/libane_t0_bridge.dylib
  src/ane_t0.rs            # Rust: dlopen(OUT_DIR dylib) + dlsym the 3 fns
```

Rust side keeps: MIL generation, blob packing, fp16 conversion, timing,
threading, all probe logic. ObjC side is a thin ~200-line translation
layer (compile/eval/free + per-procedure requests + hint options) that
mimics the reference implementation exactly. The dylib path is known at
compile time via `env!("OUT_DIR")` — no runtime search.

Cost of the pattern: an ObjC compile step in build.rs (macOS-only; the
module is `#[cfg(target_os = "macos")]`-gated so cross-platform builds
never see it). Benefit: the exact object graph the runtime accepts.

## If the pure-Rust path is ever retried

Highest-value next probes, in order:

1. Build the weights dict via `CFDictionaryCreate` (CoreFoundation, not
   msgSend-setObject) — toll-free-bridged, different construction path.
2. Try `dataWithBytesNoCopy:freeWhenDone:NO` for both mil + blob.
3. Dump `networkTextHash`/`weightsHash`/`optionsPlistHash` on both the
   Rust- and ObjC-built descriptors and compare — if the hashes differ,
   the descriptor saw different bytes and the divergence is upstream of
   serialization; if identical, the graph shape is the difference.
4. Interpose on the XPC message (the runtime→ANECCompile handoff) to diff
   what actually crosses — heavy, definitive.

The ObjC bridge is good enough for every planned use (Issue 726 T1+); the
pure-Rust retry is a curiosity unless a no-clang build environment becomes
a requirement.
