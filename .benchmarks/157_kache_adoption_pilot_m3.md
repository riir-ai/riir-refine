# Bench 157 — the kache adoption pilot, m3 (Research 414 §lane-intel; owner "go" 2026-11-16)

**Status:** PASS — pilot executed 2026-11-16 (zed-m3-kache-pilot@m3, epoch 1794811370); T0 + bars (a)(b)(c)(g) PASS, (d)(e)(f) PASS with provisional (shared-box) walls; skill/AGENTS lines landed same session per pre-reg item 6.

## Posture under test

`RUSTC_WRAPPER=kache` (v2.1.1 @ pin `da717cb9`, built release from the pinned clone), env-only config — `KACHE_CACHE_DIR`/`KACHE_RUNTIME_DIR` pointed at the pilot's isolated store/runtime; **no `kache init` anywhere** (the per-lane env law). Pilot workspace `/tmp/kache-pilot`: probe crate (6 named neutrality tests) + deps crate (serde/serde_json/anyhow/thiserror/tokio-full, ~26-unit tree) in three worktrees — wt-a (armed miss), wt-b (armed hit), wt-c (incumbent, no wrapper). Armed and disabled lanes never shared a target dir (bar g, structural).

## T0 — passthrough argv-neutrality of `KACHE_DISABLED=1` (the blocking confirm)

**PASS with one disclosed delta class.** Method: an argv-logging rustc shim as `RUSTC`; baseline cargo build (no wrapper) vs the identical build under `RUSTC_WRAPPER=kache KACHE_DISABLED=1`; diff the logged rustc argv.

- All non-rustc invocations (`rustc -vV`, `--print` probes): **byte-for-byte identical argv** (the `!rustc_invocation` branch execs `args[0] args[1..]` directly).
- The rustc compile invocation: identical EXCEPT `-C incremental=<dir>` stripped (2 argv entries) and the child env sees `CARGO_INCREMENTAL=0` — the in-source design (`strip_incremental_flags`, `preserve_incremental` defaults false; `config.rs` `KACHE_PRESERVE_INCREMENTAL`).
- With `KACHE_PRESERVE_INCREMENTAL=1`: **fully identical argv** modulo the target-dir paths themselves (kache renames the incremental dir to `<dir>.kache-preserved` — an isolation detail, disclosed).
- Both builds succeeded; outputs present.

**Load-bearing disclosure for the gates-disabled discipline:** `KACHE_DISABLED=1` is NOT identical to "no wrapper" on the incremental axis — a gate running with the wrapper installed (even disabled) loses rustc incremental unless `KACHE_PRESERVE_INCREMENTAL=1`. Gates don't want incremental anyway (determinism), but the provenance line should name the state; the wrapper must be REMOVED (env unset), not disabled, when a lane wants byte-incumbent rustc.

**The remap injection measured on a real miss** (bar-b's blast radius, made concrete): kache prepends 9 `--remap-path-prefix` rules to the rustc argv mapping machine-local roots → sentinels (`/Users/kat`→`/kache/home`, `~/.rustup`→`/kache/rustup`, `~/.cargo`→`/kache/.cargo`, target→`/kache/target`, workspace→`/kache/workspace`, TMPDIR→`/kache/tmp`, debug→`/kache/profile` — both /tmp and /private/tmp spellings). The source path itself stays cargo-RELATIVE (`src/lib.rs`), which is why `file!()`/`Location::file()` stay workspace-relative (bar b below) — the remap axis is the absolute paths embedded in debuginfo. Cache-hit diagnostics are un-remapped for display (`diagnostics_for_display`).

## Bar (a) — cache identity: PASS

armed-miss (wt-a) vs armed-hit (wt-b, same content, same store, different dir): **26/26 release rlibs blake3-identical** (whole-tree hash comparison, zero differ/missing).

## Bar (b) — behavioral neutrality armed vs disabled: PASS

`cargo test` on the probe: **6/6 passed in BOTH postures**, including the four named probes — `file!()` == "src/lib.rs" (workspace-relative), `Location::caller().file()` workspace-relative, panic message + location intact, backtrace symbolizes into the crate. Plus the release binaries' runtime outputs byte-identical (armed vs incumbent, same stdout, exit 0). Identical pass counts: yes.

## Bar (c) — disk: PASS (the headline)

| lane | df delta | meaning |
|---|---|---|
| wt-a armed miss | +90 MB | target + store blobs (store amortizes forward) |
| wt-b armed hit | **±0 MB** (−1.0 MB noise) | CoW-cloned into target (`cloned_into_targets_bytes` 86.6 MB shares blocks; inodes differ — APFS clonefile, not hardlinks) |
| wt-c incumbent | +92 MB | full private target |

Per added worktree: **incumbent 92 MB vs armed ≈ 0**. Apparent du stays 111 MB/target (du lies across clones, exactly as the pre-reg housekeeping amendment says — account by df). Store after the pilot: 92.9 MB, limit ~93 GB default, GC daemon active.

## Bar (d) — wall: PASS, provisional walls (shared-box disclosure)

| cell | armed | incumbent | delta |
|---|---|---|---|
| cold build (dep tree ~26 units) | 7.6 s | 6.3 s | +1.3 s (first-build hash cost, one-time) |
| **new worktree, warm store** | **1.7 s** | 6.3 s | **~3.7x** |
| clean rebuild, warm store | 1.2 s | 5.1 s | **~4.3x** |
| clippy run 1 / run 2 | 3.4 s / **0.02 s** | — | warm clippy ~170x |

**PROVENANCE (the box-state law):** `power=AC Power powermode=2(high) swap=0.00M`; load **6.07–13.15 — ABOVE the preflight MAX_LOAD=6.0, preflight REFUSED** (a sibling session — the Plan-382 kache mining lane's clippy run on riir-refine — was compiling on m3 during the wall cells). Both arms of every wall pair ran in the same window, so the RELATIVE deltas are load-robust, but the absolute walls are **provisional by the house law**; the fleet walls come from the amo/bmo rollout lanes (m3 takes no jobs). The equality/count bars above are wall-independent and stand.

## Bar (e) — dev-loop falsifier: PASS (overhead quantified), disposition holds

5 edit→build iterations on the leaf crate: armed median ~547 ms vs incumbent median ~500 ms → **~9% per-edit overhead** (the wrapper's dep-info/hash pre-pass on the one recompiled unit; incremental is stripped while armed per the T0 axis). 47 ms on a 0.5 s loop is not material HERE, but the same fraction on `just dev` (seal-remake ~4.9 s edit→viewer) would be ~440 ms — perceptible. **Disposition: job lanes armed, interactive dev loops stay incumbent byte-identically** (exactly the pre-registered split; nothing new to decide).

## Bar (f) — clippy-lane hit-rate: PASS (measured, not assumed)

`cargo clippy --release` armed on the deps tree: run 1 = 23 misses (compile, 3.4 s); run 2 = **27 hits, cargo-internal 0.02 s**. Clippy units are cache-keyable (the round-2 `compiler/mod.rs:891` correction, now measured). Cumulative store at pilot end: 128 hits / 76 misses, count hit rate 61.2%, weighted 64.5%, time saved 50.8 s.

## Bar (g) — mixed arming: PASS (both halves)

- Structural: every armed and unarmed lane in this pilot had its own target dir — the rule holds by construction; it is now recorded doctrine (the skill line).
- Armed build over an unarmed-warm target: cargo's fingerprints are wrapper-independent → **0 forced rebuilds** (cargo no-op, 0.19 s; the 48 fingerprint-level hits still counted in the store).
- Disabled passthrough over an armed-restored (read-only CoW) target: **exit 0**, including a forced leaf recompile over read-only restored artifacts — the #238 pre-clean unlink path verified working on this box.

## Operational gotchas (for the rollout lanes)

1. **The kache CLI needs the lane's env**: `kache stats` without `KACHE_CACHE_DIR`/`KACHE_RUNTIME_DIR` reads the DEFAULT store (`~/Library/Caches/kache`) and a default-socket daemon — silent wrong-store reads. Always export the lane's env for CLI calls (or init per project — which we avoid on dev boxes; job lanes export env anyway).
2. Daemon-per-runtime-dir: each `KACHE_RUNTIME_DIR` spawns its own daemon (`kache-engine daemon run`); stop it with the lane's env (`kache daemon stop`) at lane teardown, else daemons linger (two were found at pilot close — both stopped).
3. First build after arming is slower (hash cost, +1.3 s on 26 units — amortizes across every later worktree).
4. Binary install needs BOTH `kache` and `kache-engine` (the v2 companion — README's explicit warning; bare-copying `kache` alone refuses at first run).

## Verdict

**ADOPT per the pre-registered posture**: per-lane armed via `RUSTC_WRAPPER=kache` env in JOB lanes; gates stay `KACHE_DISABLED=1` (with the T0 incremental disclosure named in their provenance); dev loops incumbent; remotes deferred (owner credential gate). The disk axis — the pain this pilot was approved for — is decisively solved (92 MB/worktree → ~0). Skill + AGENTS lines landed this session per pre-reg item 6.

Rollout: m3 binary installed (`~/.cargo/bin/kache{,-engine}` v2.1.1 @ `da717cb9`); amo/bmo job lanes next; 4090 last.

NOTHING INVENTED — every number above is from this pilot's logged cells (`/tmp/kache-pilot/logs/` at run time, transcribed here); equality bars are blake3/test-count facts. Session: zed-m3-kache-pilot@m3, epoch 1794811370.
