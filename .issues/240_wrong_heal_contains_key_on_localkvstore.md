# Issue 240 — wrong heal: `unnecessary_get_then_check` emitted `contains_key` on receivers that never carried it (riir-dapps 8b06a45 landed ~13 non-compiling rewrites; dapps develop red since 2026-11-11)

**Status:** OPEN — CLAIMED 2026-11-16 (m3 repair lane) — filed 2026-11-16 by the plan-378 lane after the figure-render verification lane hit the red parent lib
**Claimed:** m3 (zed-native), 1773875000, expires 2026-11-18T18:00:30Z — repair lane: upstream `contains_key` on LocalKvStore (keeps clippy green, avoids the value-copy at every probe) + the rulewright receiver-capability guard + divergence row + targeted residue screen; full consumer_residue_sweep owed at the next rig window (amo/bmo/4090 offline tonight).


## The finding

riir-dapps commit `8b06a45` ("fix: mechanical clippy fixes via cargo refine — 76 verified
edits across 21 files", 2026-11-11) carries, among its 76 edits, rewrites of the shape

```rust
// before (compiles, correct)
if self.store.get(&key).is_some() { ... }
// after (does NOT compile on this receiver)
if self.store.contains_key(&key) { ... }
```

where `self.store` is `riir_neuron_db::local_kv::LocalKvStore`. **`LocalKvStore` has no
`contains_key` method — and never had one** (riir-neuron-db all-history `-S "fn contains_key"`
is empty; current `src/local_kv/mod.rs` surface: `get`, `scan_prefix`, `scan_prefix_bytes`,
`scan_prefix_visit`, …). The rulewright's `unnecessary_get_then_check` family rewrote the
`get(..).is_some()` guard into the HashMap-shaped spelling without a receiver-capability
check, and the fix lane's compile gate did not stop the sweep from landing.

Result: `cargo build` of the riir-dapps lib at develop HEAD `fce4690` fails with **40
errors** (E0599 `no method named contains_key found for struct LocalKvStore`), blocking
every kat-service / kat-service-adjacent cargo lane in that repo since 2026-11-11. Found
2026-11-16 while verifying the plan-378 figure re-render (the kat-service `role_flows` test
lane cannot run; the assets themselves are `include_str!` bytes and landed regardless —
riir-dapps `046c29b`).

## Affected call sites (riir-dapps HEAD `046c29b`, grep `contains_key` on a `LocalKvStore` receiver)

`src/item_sync.rs` (×2) · `src/gallery.rs` · `src/kat/statehash.rs` ·
`src/kat/telemetry.rs` · `src/kat/ledger/mining_rewards.rs` · `src/kat/migrate_sweep.rs` ·
`src/kat/healqual.rs` · `src/kat/decstat.rs` (×3) · `src/kat/mining.rs` ·
`src/kat/deposit.rs` · `src/kat/staking.rs` — every site's pre-image was the correct
`get(&key).is_some()` form (verified on `8b06a45`'s diff for telemetry.rs; the other sites
carry the same rewrite shape from the same sweep).

## The repair (dapps-side, mechanical)

Revert each rewritten guard to `self.store.get(&key).is_some()` (the pre-image form —
`.get` returns `Option<Vec<u8>>`; the intent "write-once / already-settled" is unchanged),
or add a `contains_key(&self, key: &[u8]) -> bool` to `LocalKvStore` in riir-neuron-db and
keep the call sites. The revert is the smaller blast radius; the upstream method is the
better substrate if the fleet wants the spelling (one method, zero deps) — EITHER is fine,
never both half-done. Until it lands, riir-dapps develop stays red and kat-service cargo
lanes stay blocked.

## The rulewright-side defect (this repo — the actionable half)

1. **Receiver-capability check missing in the `unnecessary_get_then_check` family**: the
   rewrite must verify the receiver's type actually exposes `contains_key` (method
   resolution through the crate's own API surface, not a vocabulary assumption). A
   `get(..).is_some()` on a NON-std receiver (any crate type) should decline, not rewrite —
   the same posture as the P98/P104 crate-root-path guards (the fix lane declines what it
   cannot prove resolvable).
2. **How did the compile gate pass?** `8b06a45` landed 2026-11-11 through the fix lane;
   either the verify posture compiled a feature set that compiled the affected modules OUT
   (the cfg-swapped-decl class — the P20 audit exists for exactly this) or the sweep ran
   `--no-verify`. Reconstruct the run posture from the refine trajectories store
   (`8b06a45`'s epoch window) and close the gap: a 76-edit sweep whose final check compiles
   less than the edit surface is the exact false-green the fix_verify gate exists to
   prevent. Record the reconstruction here when done.
3. **Post-mining intake row**: add the class (`get-is-some` → `contains_key` on
   non-HashMap receivers) to the divergence table if it is not already there — the
   rulewright must be SILENT on it until it can prove method resolution.

## Guard-landing protocol (residue, owed on the fix)

When the family gains the receiver-capability guard, run
`scripts/consumer_residue_sweep.sh unnecessary_get_then_check` across
`scripts/swept_repos.txt` and paste the table into this issue's close-out — `8b06a45`-class
residue may exist in OTHER consumer repos (any repo whose stores are crate types, not
std maps: katgpt-rs consumers, seal lanes). Fix or file every row; never leave the residue
unmeasured.
