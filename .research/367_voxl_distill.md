# Research 367 — tembo/voxl distill verdict

**Status:** DISTILLED — LANDED (Batch 329, plan 337, 2026-11-04: rust_perf +2, sec +2, 2 faces; see the addendum)

- **Source:** https://github.com/tembo/voxl — "extremely fast web remote desktop
  streaming": low-latency desktop streaming for the browser, in Rust. Streams an X11
  display, a Mac screen or a Windows desktop as H.264 over one WebSocket to a web client
  decoding with WebCodecs; input, clipboard, audio, microphone, file transfer, dynamic
  resolution; agent APIs (`/screenshot`, `/record`, `/wait`). Production-operated (built
  for software factories / headless VMs / computer-use agent sandboxes; crates.io
  `voxl-server` 0.3.2).
- **Pin:** `eba3b7ced9f60e885bb7b3a96a12e3ccb347fdcd` (shallow clone, tip 2026-11-03
  "fix(mac): capture in sRGB… (#64)"; **inspected 2026-11-04**; clone removed at the
  verdict commit — re-clone at this pin for the batch; the repo is FAST-MOVING (a tip
  commit the day before inspection), moved HEAD = re-pin + re-verify every quote).
- **License:** MIT (code). The BINARY is GPL-2.0 via x264 linkage — irrelevant to rule
  mining (fixtures are always original per the constraints law).
- **Verdict:** **YES B− corpus** [3-4 rules: 1 rust_perf LEAD (B) + 1 rust_perf
  secondary (B−) + 1 sec LEAD (B) + 1-2 sec lower-tier (C+/B−, expected-drop risk on
  one)] / **lane-intel LOW — STATED, not skipped** (zero mechanism overlap with the
  league lanes — no GDN/prefill/decode/quant/ANE surface anywhere; the only workspace
  adjacency is riir-net's LANDED relay design independently corroborated — recorded
  below, nothing filed) / **product coverage MODERATE** (sec web-trust gains the
  local-server handshake-trust member — every localhost dev/dashboard/API server is the
  population; rust_perf gains the change-map hint-forwarding family — the population is
  every change-driven pipeline: editors, image/video tools, build caches,
  incremental compilers).
- **Queue slot:** `.plans/337_batch329_voxl_distill.md` (nominal Batch 329, after 328
  oss-fuzz), STAGED behind the open queue.
- Session: zed-m3-voxl-distill, epoch 1793774991

## Source shape (what this repo actually is)

~12.2K lines of Rust across 37 `src/*.rs` files. Three surfaces, graded at module level
per the thermite precedent:

1. **The video pipeline** (`video.rs` 1214 + `capture.rs` 477 + `encoder.rs` 299 +
   `convert.rs` + `wait.rs` 260 + `hub.rs` 361) — the mineable hot path: XDamage-driven
   capture → damage map → band-limited yuv conversion → per-stripe x264 → per-client
   ack-based flow control. **Read in full this verdict.**
2. **The web/agent surface** (`server.rs`, `access.rs` 255, `files.rs` 527, `login.rs`,
   `wait.rs`) — auth, host/origin gating, file transfer, the settle/change API.
   **access.rs/files.rs/login.rs read (login.rs partial).**
3. **Platform plumbing** (`win/*` 2.2K, `mac/*` 1.8K, `desktop.rs`, `input.rs`,
   `mic.rs`, `audio.rs`, `corners.rs`, `mp4.rs`, `record.rs`, `screen.rs`) — DXGI /
   ScreenCaptureKit / CGEvent bindings; **UNREAD this verdict — carried for T0**
   (module-level grading: nothing in the graded surface depends on them).

Quality signals, cited: `[profile.release] opt-level = 3, lto = "thin", codegen-units
= 1, panic = "abort"` (they benchmark their code); an in-repo encode microbench
(`encoder.rs #[cfg(test)] mod bench::encode_cost`, hints-vs-no-hints arms, rusage CPU
accounting); a README comparison table with named opponents (selkies, KasmVNC, Neko,
noVNC, Daytona), methodology disclosed (pixel-marker decoding client, worst-second FPS,
p50/p99 content-to-decode latency) and a server-CPU table. Zero `todo!()` /
`unimplemented!()` / `#![allow(unused)]` in `src/` (the encoder's allow list is FFI
naming hygiene only). Provenance: no port chain — original work; the x264 flag lore
(UMH motion estimation over ultrafast's diamond search, zerolatency, mb_info) is
community knowledge — cite x264 upstream for the flags, the repo for the Rust shape.

## Ranked candidates

### R1 — B — `event-damage-map-selective-partition-skip-with-consumer-hints` (rust_perf, dispatch) — THE GOAT LEAD

The whole pipeline is ONE law with three firings: an event-sourced change map (raw
damage rectangles drained during event pump — no second pass, no server round trip)
selects, budgets, and hints:

1. **Partition selection** — only stripes whose row band intersects the damage band
   get a job; untouched stripes are skipped whole (`video.rs::encode_loop`
   `selected` = `job.band.is_some_and(|b| (b.y0 as usize) < y + h && (b.y1 as usize) > y)`),
   with the measured anchor, verbatim (`video.rs::stripe_layout` doc): "x264 pays ~9 ms
   of CPU per 2560x1600 frame even when nothing changed, and sliced threading triples
   that; independent single-threaded stripes are encoded in parallel (low latency) and
   only when they changed (low CPU)."
2. **Work budget from map density** — the per-stripe changed-macroblock count feeds the
   governor (R2) and the per-stripe rate share.
3. **Hint forwarding to the downstream consumer** — the distinctive atom: the map is
   FORWARDED into the black-box heavy consumer as per-cell skip hints, so the
   consumer's own internal analysis elides static areas (`encoder.rs`, verbatim):
   "Per-macroblock \"unchanged\" hints from X damage let x264 skip analysis of static
   areas." / encode() doc: "`mb_info` (one byte per macroblock, X264_MBINFO_CONSTANT
   for unchanged ones) lets x264 skip analysis of static areas". Plus the same map
   band-limits the yuv conversion (`cap.convert(&mut buf, t.y0, t.y1, …)` — only dirty
   rows are converted) — the map drives EVERY stage, not just one.

Also in the family: `Refiner::mark/due` reuses the same map to drive a second quality
pass over blocks that went still.

**Dedup (both vocabularies, run 2026-11-04 at rust_perf 294 / kernel_opt 1051, live
counts):** rust_perf rule ids: `skip-*` = prefilter-skip-density-demotion (demotes a
filter by density — not event-sourced, no forwarding), checkpoint-reads-skip-page-cache
(io) — ZERO owner. kernel_opt skip family is rich but GPU-kernel-scoped:
`skip-unchanged-writes-via-dirty-encoding` (gpu_07) owns per-ELEMENT STORE elision
inside a kernel via dirty-bits-encoded-in-values — DISTINCT (this: host pipeline,
whole-partition selection, HINT FORWARDING to an external consumer's analyzer);
`predicate-gated-skip-empty-workset-step` owns a zero-count whole-step guard (no
persistent map, no consumer hints); `cost-model-counts-work-the-kernel-skips` (gpu_129)
owns cost-model FIDELITY for autotune ceilings (this elides the work itself);
`mask-block-classification-skip-placement` / `swa-evicted-prefix-tile-analytic-skip`
own GPU tile placement. No rule owns "forward the change map to the consumer as skip
hints" — the corpus cannot currently tell ANY downstream stage what to skip.

**GOAT:** criterion 1 benchable — rust_perf_bench fixture: per-cell work model
(dirty cells pay the full transform, clean cells pay ~0 with the hint vs full
without), a 10-15% dirty band, compare full-frame vs map-driven; axes = total work +
copy bytes + allocs. Criterion 2 exact — integer cell counts, bit-identical trivially.
Criterion 3 magnitude — ~1/dirty-fraction (their own anchors: ~9 ms/frame idle-frame
cost; README 1.39 cores busy desktop). Criterion 4 deterministic. PASSES ALL FOUR.

### R2 — B− — `online-cost-fit-thread-ladder-reopen-hysteresis` (rust_perf, concurrency)

`video.rs::ThreadGovernor` + `CostFit`: per-partition thread-count selection from an
ONLINE learned cost model, not a static knob. The fit, verbatim: "Exponentially
weighted least squares fit of encode ms = a + b * changed MBs." (decay 0.995, min 30
samples before predicting, degenerate-variance fallback to the mean). The switch
decision, verbatim (doc): "x264's sliced threading costs ~3x the CPU of one thread on
desktop content (and on small updates is even slower), so a stripe encodes
single-threaded and only fans out while sustained heavy motion would otherwise blow
the frame budget." Asymmetric hysteresis both directions: UP after 20 consecutive
frames ≥ 65% of frame budget (EWMA 0.8/0.2); DOWN only after the fit predicts one
thread ≤ 50% of budget SUSTAINED 2 s — with the switch cost disclosed in-source:
"(switching reopens the encoder, which costs a keyframe, so not on a brief lull)".
The mode-switch itself is a full consumer rebuild (encoder reopen), accepted and
logged ("A fresh encoder opens with an IDR on its next frame").

**Dedup:** zero corpus owner for online thread-count adaptation — rule-id greps
governor/adaptive-thread/least-squares clean; kinships to cite in-rule:
`fastest-of-n-dip-drift-retime-cost-table` + `pava-monotone-cost-pooling-modeled-cost-not-clock`
(entries_19 — captured-table integrity + decision-input determinism; THIS is online
MODE SELECTION from a live fit), `dynamic-work-parallelism-ladder` (entries_06 — the
STRUCTURE choice for unknown-amount work, not thread-count adaptation),
`cost-model-counts-work-the-kernel-skips` (the fit's x-input is the changed-count
FROM the damage map — consistent with the fidelity law),
`thermal-hysteresis-band-pacing-chunk-placement` (GPU thermal banding — hysteresis
family vocabulary, different axis).

**GOAT:** criterion 1 — benchable with a trace fixture (feed a repeating
mixed-workload; governor vs always-1-thread vs always-max under a fixed budget;
measure total ms + budget misses), but the win is REGIME-dependent (time-varying
load) — likely lands NotBenchable-annotated like the B271 serving pool; criterion 2
exact at the decision layer (the fit is deterministic given the trace); criterion 3 —
their own 3x sliced-threading anchor; criterion 4 deterministic. Graded B− as
decision content (rich constants, two-regime shape, honest switch cost).

### R3 — B — `browser-reachable-service-host-origin-rebinding-gate` (sec, web-trust) — THE SEC LEAD

`access.rs`: a browser-reachable local server's handshake gate against DNS rebinding
and cross-origin WebSocket drive-by. The module doc is the class description, verbatim:
"Browsers let any web page open a WebSocket to voxl or send it requests, and a page
whose own domain is made to resolve to voxl's address (DNS rebinding) even counts as
voxl's own page. So voxl only answers a `Host` that can't be rebound (an IP address or
localhost) or that `--allowed-hosts` names, and turns away what a browser sends from
another site's page (its `Origin`) unless `--allowed-origins` names that site." The
missing-Origin allowance is fail-open BY DESIGN with the reason in-source: "Browsers
always send a Host, so one without it comes from another kind of client." Bounded fix
space: axum middleware — refuse the Host unless it parses as `IpAddr` || localhost
family || explicit allowlist (`.example.com` covers subdomains, `*`); refuse the
Origin unless its canonical key equals the requested host:port || allowlist; 403
naming the flag (their refusal strings TEACH the fix — "voxl doesn't answer for the
host {name:?}. To serve it under that name, start voxl with --allowed-hosts={name}").

Count-neutral rider faces: (a) origin default-port canonicalization before comparison,
verbatim: "An origin with its port written out, so `https://a.example` and
`https://a.example:443` compare equal" (`origin_key` — compare CANONICAL keys, never
raw header strings); (b) non-UTF-8 header values read as `""` → fail-closed refusal
("A value that isn't text can't name an allowed host or origin").

**Dedup:** zero sec owner — `cors-any-origin-with-credentials` (B258/penthera) is
RESPONSE-header issuance policy (tower-http co-occurrence); THIS is the REQUEST-side
handshake gate, and the mechanism reason is distinct: WebSocket handshakes are NOT
CORS-gated (no preflight) — a WS server that sets no CORS headers is still reachable
from any page, so the server itself must gate. `client-ip-from-untrusted-forwarded-header`
is spoofed-proxy-header trust — different header, different axis.
`gateway-type-forces-security-check` is constructor-reachability. Host-header
rebinding/allowlist vocabulary: ZERO corpus hits (rebind|host-allowlist|origin-gate).
Regime: any localhost-listening service with a browser-facing API or WS endpoint (dev
tools, dashboards, debuggers, agent APIs) — the drive-by/rebinding class. population
is enormous and the class is RUST-SHAPED (axum/tower/hyper middleware).

**GOAT:** criterion 1 fails for a DOMAIN reason (sec has no perf validator;
probe-parity is the substitute) → per-rule NotBenchable annotation, the dial9 split —
never a demotion. Decision content × novelty × dedup clearance carry the grade.

### R4 — C+/B− — `custom-header-mutation-gate-unanswered-preflight` (sec) — B258-drop adjudication owed

`files.rs`: `pub const UPLOAD_HEADER: &str = "x-voxl-upload"` with the law, verbatim:
"Uploads must carry this header. Browsers only send custom headers cross-origin after
a CORS preflight, which we never answer, so other sites can't upload." — CSRF defense
by MECHANISM: a state-changing endpoint requires a custom header that cross-site
form/img/fetch-simple requests cannot attach, and the server never answers the
preflight that would allow it. Bounded fix: header-presence check on mutating routes.
⚠ The batch must adjudicate against B258's recorded drop ("the JWT/CSRF/session/OAuth
bulk — architectural — feeds the SKILL checklist, not the corpus"): this is the
specific mechanical form (header gate + unanswered preflight), not session
architecture — expected to survive as suggest-only, honest drop allowed. Dedup: zero
csrf/preflight/custom-header owner in sec.

### R5 — C+/B− — `untrusted-shared-memory-attach-pid-and-content-verify` (sec, input validation)

`capture.rs::find_xvfb_framebuffer`: attaching to ANOTHER PROCESS's SysV shared memory
discovered by enumerating `/proc/sysvipc/shm`, with TWO trust gates before any pixel
is trusted, verbatim: "Xvfb started with `-shmem` keeps each screen as an XWD image in
a SysV segment. Find it, check the header and verify pixels against the server before
trusting it." — (a) creator-pid scoping from the X server's lock file ("Several Xvfbs
can run side by side with similar (or blank) screens, so only look at segments our X
server created when we know its pid"), (b) a content ORACLE — a pixel strip is
compared against a ground-truth server read, near-total match (≥98%) accepted across
10 retries @ 30 ms (animation between reads), plus header-shape checks (XWD type,
bpp, stride, offset bounds). Class: enumerated cross-process resources must bind to
the owning process AND be validated against an oracle before trust. Population is
niche (who else attaches to enumerated shm?) — honest drop allowed at T0. Dedup: zero
shared-memory/attach owner in sec; kinship vocabulary: untrusted-keyed-state-unbounded
(cardinality) is unrelated.

### Lower-tier / faces

- **C+ face** — `fixed-wire-header-reserve-in-place-not-prepend` (`video.rs::encode_loop`):
  the 16-byte stripe wire header is pre-reserved in the encode output
  (`out.resize(HDR, 0)` before encode) and written IN PLACE after
  (`out[2..6].copy_from_slice(&frame_id.to_be_bytes())…`), then handed to the client
  zero-copy (`Bytes::from(out)`) — a prepend/concat copy per stripe-frame avoided.
  Likely a count-neutral face on the capacity-reservation family
  (`reserve-then-commit-capacity-accounting`); adjudicate at authoring.
- **C+ face** — symlink listing re-check (`files.rs::list`): follows symlinks for
  type/size "as a file manager would" but HIDES links whose canonical target escapes
  the root — a count-neutral face on `untrusted-path-join-escape` (the listing must
  re-check each entry's resolved target, not just the base path).
- **C+ candidate, drop-expected** — `burst-settle-coalesce-before-consume`
  (`video.rs::capture_loop`, the `cfg.coalesce` window): "Apps present a frame as a
  burst of puts; give the burst a moment to land so the capture holds one whole frame
  instead of its first tiles." — consumer-side temporal settle before consuming, vs
  the corpus's spatial `coalesce-nearby-reads-bounded-gap-cacheability-gated` and the
  crowded batch family; distinct axis but expect the kinship line to be contested.
- **C+ candidate, drop-expected** — the `/wait` observe-at-source pattern
  (`wait.rs`): the capture thread records per-16×16-block last-change marks
  whether-or-not-anyone-watches ("The capture thread hears about every change anyway
  … whether or not anyone is watching, so it notes when each 16x16 block last changed
  and a wait needs no screenshots and no polling"), waiters subscribe to a
  `tokio::watch` seq with a computed deadline (`done_at = min(deadline, at.max(start) +
  idle)`, `select!` on changed vs sleep_until) and the seq witness invariant ("a `seq`
  handed out never covers a change the answer didn't see" — `latest()` reads map lock
  and seq together). Crisp API pattern; thin corpus decision content; drop allowed.

## Kills (positive instances — the gate working, recorded never graded)

- `login.rs::same()` — hand-rolled constant-time compare, verbatim doc: "Compares in
  time that depends only on the lengths, not on where the bytes differ." → POSITIVE
  INSTANCE of sec `constant-time-secret-compare`.
- `files.rs::resolve()` — components() walk refusing RootDir/ParentDir/Prefix +
  canonicalize().starts_with(root), doc verbatim: "Absolute paths and `..` are
  rejected; symlinks are followed only if they stay inside the root." → POSITIVE
  INSTANCE of sec `untrusted-path-join-escape` (the riir-deployer 48d63f7 in-house
  shape B258 cited — third independent implementation of the same fix).
- Stalled-consumer quarantine (`video.rs::flow_ok`, age > 4000 ms → clear inflight +
  needs_keyframe, "Stalled client (hidden tab, dead socket): stop letting it gate
  everyone") → adjacent-owned by rust_perf `bounded-channel-backpressure` AND a
  second independent implementation of riir-net's LANDED slow-client design
  (CLIENT_QUEUE_FRAMES / RIIR_RELAY_SLOW_WINDOW_SECS) — recorded as corroboration on
  the lane line below, never a rule.
- SendPtr raw-pointer Send/Sync wrappers (`convert.rs`, `video.rs`) → the pattern sec
  `unsafe-impl-send-sync-unsound-field` guards; voxl's scoped usage (all workers reply
  before the sender proceeds) is the sound shape — vocabulary corroboration only.

## Drops (graded and dropped at verdict)

- `wrapping_le` sequence compare (`hub.rs`: `b.wrapping_sub(a) < 0x8000_0000`) — crisp
  and correct, but the win is CORRECTNESS at wrap, not perf: no magnitude story (the
  naive comparison is equally fast when correct) — the "no perf axis" drop precedent
  (the B251 M-rule). Zero corpus owner; if the corpus ever opens a correctness
  micro-family this is the seed.
- RateCtl atoms (`video.rs::RateCtl`) — TCP-like slow start + capacity-ceiling memory
  with 15 s expiry + "only probe upward once the content actually presses against the
  cap" — control-loop policy / cache-freshness family flavor, video-domain shaped.
- Keyframe budget boost (force_idr: cap×4 / min 16 Mbps, 8-frame VBV — "A keyframe is
  what a new viewer sees first; don't starve it to the steady-state frame budget") —
  domain flavor (transient-work headroom), no bounded Rust fix-space.
- Refiner piggyback (sharpening still blocks ON frames already being encoded) —
  amortization-family flavor.
- UMH motion estimation over ultrafast's diamond search ("a third of the bits for a
  window drag for about the same CPU") — encoder config lore; x264 upstream owns it.
- Server-grab screenshot consistency (grab_server + pump-before-copy so "a repaint
  can't land in the pixels without its damage, or after them with it"; the seq
  covering exactly the shown pixels) — beautiful domain-deep correctness pattern, no
  corpus home.

## Negative observations (honesty notes, possible upstream-PR leads — outbound lane owner-gated, recorded only)

1. `encoder.rs::encode()` reads `std::env::var("VOXL_DEBUG_MARKER")` on EVERY encoded
   frame (a getenv per frame in the hot path) — while `video.rs::capture_loop` hoists
   exactly this read before its loop (`let debug_marker =
   std::env::var("VOXL_DEBUG_MARKER").is_ok();`). The codebase knows the law; the
   encoder site missed it.
2. `capture.rs::take_mb_map()` allocates a fresh zeroed `vec![0; n]` per frame take
   (~16 KB/frame at 2560×1600 @ 60 fps ≈ 1 MB/s churn) where a double-buffer swap or
   pool slot would be alloc-free.
3. `files.rs::download` stream allocates a fresh 256 KiB `Vec` per chunk inside the
   `unfold` state (a carried buffer in the state tuple would be reuse).

## Lane-intel axis (MANDATORY output)

- League scoreboard check: voxl's mechanisms (x264 encode, X11/DXGI/ScreenCaptureKit
  capture, WS flow control, yuv conversion) share NO mechanism with any open
  Metal/CUDA league lane (GDN prefill/decode/quant/tensor) — **lane-intel LOW, no
  mover, nothing filed.**
- The one adjacency, recorded as CORROBORATION (not filed): voxl's ack-based inflight
  accounting + RTT-scaled in-flight cap (`flow_ok`'s `max_n` from measured RTT) +
  stalled-client resync is a second independent production implementation of the
  design riir-net's Plan-494 substrate already ships (bounded fan-out
  `CLIENT_QUEUE_FRAMES`, `RIIR_RELAY_SLOW_WINDOW_SECS` slow-client eviction). A
  workspace grep of `../riir-ai/.issues/` found no open issue voxl's design moves
  (the one keyword hit, Issue 1163 relay-intent sender authentication, is a different
  concern voxl does not address — its frames carry no authenticated sender either,
  which is the same gap shape, noted here only).

## Product-coverage axis (MANDATORY output)

- **sec web-trust family** gains its first local-server handshake-trust member (R3).
  Population: every localhost dev/dashboard/API server in the Rust ecosystem (axum
  actix warp hyper tide + every `--port` flag that prints a URL). Follow-up pool
  (accumulation clause applies): the localhost drive-by/rebinding literature (the
  MetaMask-local-RPC hijack lineage, Chrome DevTools port security docs, Jupyter /
  webpack-dev-server rebinding advisories), OWASP DNS-rebinding guidance, and the
  axum/tower middleware surface (a `Host`-gate tower layer as the canonical Rust
  fix).
- **rust_perf** gains the change-map hint-forwarding family (R1). Population:
  change-driven pipelines everywhere — editors, image/video tools, build caches,
  incremental compilers, diff engines. Follow-up pool: incremental-computation
  sources (salsa, zed's incremental computes, turborepo-style caches) as future
  distill candidates.

## Caveats

- Fast-moving tip (a commit the day before inspection) — the pin is load-bearing;
  T0 re-clones at `eba3b7ce…` and re-verifies every quote; moved HEAD = re-pin.
- Unread files carried to T0: `server.rs` (only grepped: routes, ws_handler, pinger,
  read-only command allowlist `READ_ONLY_COMMANDS` — a positive instance of command
  allowlisting, recorded), `desktop.rs`, `input.rs`, `mic.rs`, `audio.rs`,
  `corners.rs`, `mp4.rs`, `record.rs`, `screen.rs`, `login.rs` (partial), `mac/*`,
  `win/*`. Module-level grading stands per the thermite precedent; the graded surface
  is platform-neutral.
- The in-repo `encode_cost` bench asserts the mb_info win but was NOT run this
  verdict (needs the x264 system lib at build time) — mine-time option on a box with
  x264 installed (`cargo test --release --ignored encode_cost`), macOS/brew fine.
- Platform share: ~40% of lines are mac/win plumbing — the verdict prices the
  platform-neutral pipeline only.
- No upstream provenance chain (original work) — quote authority is the pin itself.

## Yield estimate

3-4 rules land after ~50% coordinator falsification (R1 + R2 + R3 nearly certain;
one of R4/R5 + 1-2 faces; honest drops allowed on the two C+ candidates). Corpus
targets: rust_perf 294→296, sec 38→39-40.

## Addendum — LANDED 2026-11-04 (Batch 329, plan 337, the claim-lease lane)

**4 rules + 2 count-neutral faces land: rust_perf 294→296 (R1 `event-damage-map-
selective-partition-skip-with-consumer-hints`, the GOAT-eligible lead BENCHABLE —
ParetoDominant 2.14×/3.20×/3.14× at 256/4096/65536, allocs 0/0, dirty band ~12%,
both arms checksum-identical asserted in-harness; R2 `online-cost-fit-thread-
ladder-reopen-hysteresis`, NotBenchable-by-regime, the decayed-weight gate
(w<30.0 → the 33rd sample) verified against the clone and mirrored faithfully) +
sec 39→41 (R3 `browser-reachable-service-host-origin-rebinding-gate` + R4
`custom-header-mutation-gate-unanswered-preflight` — the B258 CSRF-bulk drop
ADJUDICATED IN-RULE: the declined bulk is session ARCHITECTURE; the mechanical
header-gate form with the in-source unanswered-preflight mechanism reason survives
as suggest-only).** Faces: the symlink-listing re-check rider on
`untrusted-path-join-escape` (files.rs::list L94-105 — the listing re-checks each
entry's canonical target; two sinks, one boundary) + the wire-header reserve-in-
place rider on rust_perf `reserve-then-commit-capacity-accounting` (entries_58;
video.rs out.resize(HDR,0) L911 + in-place writes L1107-1114 + Bytes::from zero-
copy). Gates ALL GREEN: default lib 2819/0, sec+latent 3361/0, rust_perf_bench
2850/0, clippy −D ×4 postures + all-targets zero, boundary gate 2/2 (its probe
build-failure witness added en-route — the gate's green-zero blindness repaired:
a probe whose fixtures fail to COMPILE produced zero diagnostics and read vacuous-
green; the failure is now loud; the R4 fixture's axum API was repaired to real
axum-0.8 `to_bytes` — `Body::into_inner` does not exist at 0.8 — and the probe
crate gained the axum+tokio deps per the B88/B258/B291/B328 convention),
retrieval re-measured (default+latent router 86/142 UNCHANGED, no re-pin; sec-on:
sec self-variants 37/37 top-1 — 35→37, both new spans route + retrieve top-1),
score-bench run #224 heal 0.9636 FLAT parse 1.0, freeze NOT owed (zero clippy_lints
delta, P30), release-set binary rebuilt + installed at the landing HEAD.

**R5 DROPPED honestly at T0** (the verdict's own drop-expected): the enumerated-
SysV-shm attach population is niche (who else attaches to /proc/sysvipc/shm
segments?). The creator-pid scoping + content-oracle law (≥98% pixel match over
10 retries @ 30 ms) stays recorded here as the seed if the corpus ever opens an
enumerated-cross-process-resource family.

**T0's unread-file read (all 22 files, ~8.3k lines) — the value was KILLS, not
candidates**: ~30 line-verified positive instances across the sec families
(Drop/leak pairing on every platform, null-check-before-FFI everywhere,
bound-before-alloc ×4, strike-counts ×3 platforms, constant-time ×4 sites,
length-prefix ×2, keyed-state cleanup ×3, the mdat overflow check L111-113 a
STRONG wire-arithmetic instance). The strong face candidate (guard the panicking
conversion/clamp BEFORE applying client numbers — record.rs record_len L207-214 +
screen.rs fit L21-28, both panic=abort-commented, both test-pinned) has NO corpus
host rule, so it is a 5th-rule candidate, not a face — recorded as the TOP
follow-up-pool item. Weaker faces likewise pooled: the DXGI rect-count>32
whole-vs-incremental copy crossover (win/capture.rs L196-211, cost-based dispatch
corollary) and the mp4 avc1 `as u16` vs tkhd full-width sibling-field truncation
(mp4.rs L130-131 vs L185). Follow-up pools unchanged from the verdict: the
incremental-computation sources (salsa, zed, turborepo-style) for the damage-map
family; the localhost drive-by/rebinding literature for the web-trust family.

Session: zed-m3-refine-b329, epoch 1793800137 (claim 817e8567 discharged by the
landing; `.raw/voxl` removed at close).
