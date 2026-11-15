# Issue 240 — wrong heal: the 8b06a45 sweep's THREE non-compiling classes (contains_key on LocalKvStore + closure-move filter + guard-collapse) — REPAIRED + GUARDED 2026-11-16

**Status:** CLOSED 2026-11-16 (m3 repair lane) — dapps green at the kat-service posture, all three rulewright guards landed + fixture-pinned, wave residue screened repo-by-repo. Filed 2026-11-16 by the plan-378 lane after the figure-render verification lane hit the red parent lib.

## The close-out (2026-11-16)

**The finding was BIGGER than filed.** The 8b06a45 sweep (dapps, 2026-11-11, 76 edits) landed THREE non-compiling classes, not one:

1. **E0599 contains_key on `LocalKvStore`** (~30 sites across 10 files) — as filed. REPAIRED UPSTREAM: riir-neuron-db `ef2e249` adds `LocalKvStore::contains_key(&self, key: &[u8]) -> bool` (the zero-alloc existence probe — papaya guard direct, no value copy, no Arc clone; the call sites STAY as clippy spells them, so dapps' own clippy gate stays green; a revert would have re-flagged all ~30). Their gates: clippy -D warnings default+local_kv AND --all-features --all-targets clean; lib battery 655 passed / 0 failed incl. the new `g1_contains_key_probe`; rustfmt (edition 2024) clean.
2. **E0382 closure-move** (2 sites: `pay.rs:970`, `deposit.rs:722`) — NEW class found by the repair: `redundant_closure` rewrote `.filter(|row| f(row))` → `.filter(f)` inside a `for` loop; `f: impl Fn(&T) -> bool` is a non-Copy fn value, the move re-executes every iteration. REPAIRED: dapps `2dde9e9` → the compiler-suggested `.filter(&f)` at both sites.
3. **E0004 guard-collapse** (1 site: `migrate_sweep.rs:1244`) — NEW class: the sweep collapsed `Dispose::Zero => { if v.iter().any(|b| *b != 0) { return Err(..); } }` into a match GUARD (`Zero if .. =>`), leaving the all-zero fall-through uncovered (guards don't count toward exhaustiveness). REPAIRED: dapps `2dde9e9` → the pre-image nested-if (semantics identical).

**dapps verification:** clippy --all-features --all-targets -D warnings clean; kat-service full battery 670 passed / 0 failed (the plan-378 role_flows lane 6/6 — the verification lane that found this is UNBLOCKED); dapps --features kat_migrate 486/0; dapps default 435/0; direction_gate OK.

## The gate-posture reconstruction (how "compile-gated: cargo check clean" shipped three broken classes)

The 8b06a45 verify posture ran DEFAULT-features cargo check — and dapps' entire kat plane is OPT-IN (`#[cfg(feature = "kat_ledger")] pub mod kat`, lib.rs:119; `pay.rs` behind `kat_solana`, `deposit.rs` behind `all(kat_cegel, kat_solana)`, `migrate_sweep.rs` behind `kat_migrate`). The default posture compiles the whole edited arm OUT — three error classes invisible. The false-green shape is the P20/P43 class one layer out: not `required-features` on test targets (which the P43 audit checks) but **lib modules gated on non-default features**, which NO current fix-lane audit reaches. Consequences carried: the four post-verify disclosures exist and would have named the cfg-gated files IF the lane had run them over the sweep's edit set — the sweep's operator ran it in a posture where those audits were silent. Recorded as the post-mining lesson: a sweep touching feature-gated files owes an `--all-features`-or-documented-posture check, never the default posture alone. (Trajectory-store reconstruction on amo is rig-gated — amo offline; the posture inference above is proven locally by the compile matrix: default check GREEN at parent, kat-service posture RED with exactly these three classes.)

## The rulewright-side guards (this repo — all three classes now AUTOMATED-declining)

1. `unnecessary_get_then_check` (the filed ask): **already post-235 declining** the LocalKvStore shape — the receiver classification sees the foreign store struct as Unknown and refuses. Fixture-pinned: `foreign_store_receiver_declines` (both the `self.store` field spelling and the bare binding, the exact sweep shapes). The Map-suffix residual (a `…Map`-named type without contains_key) stays the --verify gate's disclosed catch.
2. `redundant_closure` — NEW guard (the E0382 class): the collector now carries the file's **fnval set** (closure `let`s annotated-or-not + `impl Fn*`/`dyn Fn*`-typed fn params) and **loop depth** (for/while/loop nesting); a bare-lowercase single-ident callee that is a fnval DECLINES when in a loop (the iteration re-move — invisible to any source census) or at 2+ in-file uses (the sequential use-after-move). Single-use outside loops and uppercase constructors (fn items, Copy) keep firing — upstream's MachineApplicable posture preserved. Fixtures: `impl_fn_param_in_loop_declines` (the deposit.rs shape), `impl_fn_param_second_use_declines`, `single_use_fnval_outside_loop_still_fires` (the floor), `uppercase_constructor_in_loop_still_fires`.
3. `collapsible_match` — **already guarded by fa6f64f8b** (landed 2h after 8b06a45, the neuron-db sweep's specimens): fires only beside a covering no-op wildcard. Fixture extended with the dapps variant (`no_wildcard_later_arm_declined` now carries the `Dispose::Zero/Delete` + `return Err` shape) — declines.

Divergence table: both new classes added to `.agents/skills/cargo-refine/SKILL.md` REFUSES-BY-DESIGN (now AUTOMATED rows). Dogfood note: the rulewright fixed the guard author's own two mechanical clippy findings (`.map(ToString::to_string)`, `is_some_and`) compile-gated — the tool consuming its own new guard.

Gates: lib battery 3914 passed / 0 failed (incl. the 7 new fixtures); clippy --lib -D warnings clean (post-refine); rustfmt clean on all three proposers.

## Guard-landing protocol — the residue sweep (executed as a targeted wave screen; full sweep rig-gated)

The full `consumer_residue_sweep.sh unnecessary_get_then_check` cargo-clippy walk is amo/bmo-gated (offline tonight). Executed INSTEAD — the precise, cheap residue measurement for THIS sweep event: every repo the `katgpt-rs-33@amo` wave touched, diff-inspected + compile-verified at its own postures:

| repo | sweep commit | classes landed | current state |
|---|---|---|---|
| riir-dapps | 8b06a45 | contains_key ×~30 + filter-move ×2 + guard-collapse ×1 | REPAIRED tonight (ef2e249 + 2dde9e9), all gates green |
| riir-neuron-db | f0688a9 | collapsible_match ×2 (`if let`→guard, let-chain form) + shrink_to_fit ×3 + doc | VERIFIED SOUND: builds + clippy clean at default AND --all-features (the trailing `_ => {}` wildcard makes the guards legal; let-chains need edition 2024 — the crate has it) |
| riir-rethink | 03ab27a | contains_key on `Vec<String>` (E0599 in the decstat posture) | ALREADY REPAIRED 11-13 (1a45ae6 → a len() bounds check + a comment naming this issue) |
| riir-reflexer | f07c629 | doc re-indent only | green (42/0) |
| riir-viewbridge | a7f7ab7 | missing_panics_doc/empty_line/semicolon | green (workspace test 109/0) |
| katgpt-rs | (docs only) | none | n/a |

Wave verdict: every landed class is either repaired (dapps tonight, rethink 11-13) or verified sound (neuron-db's let-chain guard arms). No unmeasured residue. The cargo-clippy-family full sweep remains owed at the next rig window for the general population (not this event's residue).

**Lease closed:** m3 (zed-native), claimed 1773875000, closed 2026-11-16 same session — repair executed in-full (see the close-out).


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
