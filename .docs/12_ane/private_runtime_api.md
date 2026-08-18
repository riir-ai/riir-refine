# The private ANE runtime API contract

> Verified on M3 Max / macOS 26.5.2 (25F84) on 2026-08-19 by direct
> `objc_msgSend` + a compiled ObjC bridge. The API is UNDOCUMENTED and can
> change on any macOS update — the maderix issue tracker shows it breaking
> across 15.x → 26.x at least twice (their #27, our multi-procedure bank
> rejection).

## Frameworks

Three private frameworks, loaded via `dlopen` (no on-disk binaries since
macOS 11 — they live in the dyld shared cache; dlopen by install-name path
still resolves):

```
/System/Library/PrivateFrameworks/AppleNeuralEngine.framework/AppleNeuralEngine
/System/Library/PrivateFrameworks/ANECompiler.framework/ANECompiler
/System/Library/PrivateFrameworks/ANEServices.framework/ANEServices
```

`RTLD_NOW | RTLD_LOCAL`. Loading all three is what every working
implementation does (maderix loads only AppleNeuralEngine; rane loads all
three; either worked for class resolution on 26.5).

Related frameworks seen on-disk (not required for the in-memory path):
`ANEClientSignals`, `DANECPWrapper`, `MilAneflow`, `ParavirtualizedANE`.

## The four ObjC classes

Resolve with `objc_getClass` (or `NSClassFromString`) AFTER dlopen. If any
is null, the runtime is unavailable:

| Class | Role | Key methods |
|---|---|---|
| `_ANEInMemoryModelDescriptor` | holds MIL text + weights | `+modelWithMILText:weights:optionsPlist:` (class), `-weights`, `-networkText`, `-hexStringIdentifier`, `-isMILModel` |
| `_ANEInMemoryModel` | the loadable/evaluable model | `+inMemoryModelWithDescriptor:`, `-compileWithQoS:options:error:`, `-loadWithQoS:options:error:`, `-evaluateWithQoS:options:request:error:`, `-unloadWithQoS:error:`, `-hexStringIdentifier`, `-model` (inner), `-inputSymbolIndicesForProcedureIndex:`, `-outputSymbolIndicesForProcedureIndex:` |
| `_ANERequest` | I/O binding for one eval | `+requestWithInputs:inputIndices:outputs:outputIndices:weightsBuffer:perfStats:procedureIndex:` |
| `_ANEIOSurfaceObject` | wraps an IOSurface | `+objectWithIOSurface:` |

Full instance+class method lists can be dumped at runtime via
`class_copyMethodList` / `method_getName` / `sel_getName` (+ metaclass via
`object_getClass(cls)`) — the ane_t0 spike ships `dump_class_methods()`
for exactly this; use it to settle selector drift after any macOS update.
Notable extra `_ANEInMemoryModel` methods seen on 26.5 (not yet exercised):
`compilerOptionsWithOptions:isCompiledModelCached:`, `compilerOptionsFileName`,
`purgeCompiledModel`, `compiledModelExists`, `saveModelFiles`,
`mapIOSurfacesWithRequest:cacheInference:error:`, `unmapIOSurfacesWithRequest:`,
`programHandle`, `intermediateBufferHandle`, `modelURL`/`setModelURL:`,
`localModelPath`, `perfStatsMask`, `queueDepth`.

There is also a `_ANEDeviceInfo` class (maderix uses it:
`+aneSubType` → e.g. "h15" on this box).

## Lifecycle (the working sequence)

```text
dlopen frameworks
  → mil NSData + weights NSDictionary{path: {offset: @0, data: NSData}}
  → descriptor = [Descriptor modelWithMILText:weights:optionsPlist: mil, wdict, nil]
  → model = [Model inMemoryModelWithDescriptor: descriptor]
  → hex = [model hexStringIdentifier]              // 64-char hash
  → write {tmp}/{hex}/model.mil + {tmp}/{hex}/weights/weight.bin
  → [model compileWithQoS:options:error: 0/21, opts, &err]
  → [model loadWithQoS:options:error:    0/21, opts, &err]
  → in/out IOSurfaceCreate (byte-sized, 1×bytes layout)
  → wrap via [_ANEIOSurfaceObject objectWithIOSurface:]
  → request = [_ANERequest requestWithInputs:… procedureIndex:@0]
  → [model evaluateWithQoS:options:request:error: 0/21, opts, req, &err]  // BLOCKING
  → read output surface
  → [model unloadWithQoS:error:] + remove tmp dir
```

**QoS:** both `0` and `21` compile+load+eval fine on 26.5 (maderix uses 0,
rane/omlx use 21 — measured identical acceptance). Use 21.

**The tmp-dir write is required for compilation** (BLOBFILE paths in the
MIL resolve against `{tmp}/{hex}` — `@model_path` maps to that dir). The
call-provided dir name itself is NOT what the compiler consumes — see
"XPC internals" below — but write it anyway; every working impl does.

## Execution options (instance pinning)

`compileWithQoS` / `loadWithQoS` / `evaluateWithQoS` all take an options
NSDictionary. Empty dict `{}` = default scheduling. omlx's pinning form:

```objc
@{ @"kANEFProcedureVariantHint": @1, @"kANEFAneInstanceHint": @(1..4) }
```

All hint values 0-4 are accepted (compile+load+eval) on M3 Max. On M3
Ultra the two dies surface as instances 1 and 2 (omlx comment); on M3 Max
the hint is accepted but **does not create hardware parallelism** — see
`m3_max_hardware.md`. The empty-options runs hash to SHA256("") which shows
up as `E3B0C442…855` in the compiler dir name.

## What evaluate actually does

`evaluateWithQoS:options:request:error:` is **synchronous** — it returns
after the output IOSurface is fully written (verified by reading correct
numerics immediately after return). Two models evaluated concurrently from
two threads is legal and parallelizes (measured 2.4-2.9× on M3 Max).
omlx's production path additionally uses an `MTLSharedEvent` pair
(`encodeSignalEvent` on the GPU command buffer → spin on `signaledValue` →
evaluate → CPU-side `setSignaledValue` → `encodeWaitForEvent` in the merge
buffer) to chain ANE work between GPU passes without ANE-side fences —
their measured note: ANE's private request-wide shared-event fence
*serializes concurrent GPU work on current macOS drivers*, so they signal
from the CPU side instead.

## XPC internals (what the runtime does under you)

The client framework XPCs to two system services: `aned` (the daemon
holding the IOKit entitlement) and `ANECompilerService` (via aned). On
compile, the runtime serializes the in-memory descriptor into its OWN
directory:

```
{TMPDIR}/{sha256(mil)}_{sha256(weights)}_{sha256(options)}/net.plist
```

plus the weight payload as `data` (failed/pure-Rust path) or
`model.mil` + `weights/` + `net.plist` (working ObjC path). ANECompilerService
parses `net.plist` (which despite the name contains the RAW MIL TEXT on
this build) and emits the real error behind any NSError:

```
log show --predicate 'process == "ANECompilerService"' --style compact
  → "(ANECompiler) Parsing error: <private>"
  → "(ANECompiler) Error: Could not convert input MIL program."
```

Unified-log redaction (`<private>`) hides the detail; `--private` needs
sudo. The hash-triple dir name is a useful fingerprint — stable across
runs for identical (mil, weights, options), so you can identify WHICH
program failed from the error message alone.

## Error taxonomy seen

| NSError surface | Meaning | Where it came from |
|---|---|---|
| `ANECCompile() FAILED … InvalidMILProgram` | MIL text parse rejection | the compiler service |
| ObjC exception → Rust foreign-exception abort | wrong carrier type / bad argument object at descriptor build | descriptor factory |
| `load failed` at `loadWithQoS` | program compiled but residency refused (budget/shape) | not yet hit on this box |
