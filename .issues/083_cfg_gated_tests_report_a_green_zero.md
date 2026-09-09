# Issue 083 — 16 `#![cfg]`-gated test targets have no `[[test]]` row, so naming them prints a green zero

**Status:** RESOLVED 2026-09-09 (same day). All 16 rows added and verified in
BOTH directions; `SILENT-NOW` is **0** with the walk size unchanged.

## The finding

```
repo          targets  #![cfg]  w/ req-f  SILENT-NOW  load-bear  latent
riir-clippy       102       67        35          16          0      16
```

Sixteen `tests/*.rs` files open with a whole-file `#![cfg(feature = …)]` and
have **no `[[test]]` row at all** in `Cargo.toml`. When the gating feature is
off, such a file compiles to an **empty binary**: cargo prints
`test result: ok. 0 passed` and **exits 0** — byte-for-byte a real pass.

The `#![cfg]` protects the **count** (an ungated body would fail to compile);
`required-features` protects the **reader**, and only the second is visible in
the output. Both are needed. The 35 targets that already have rows are the
pattern to follow — `kat_fast_settle_g2` in riir-dapps even documents why
("the row mirrors the file's `#![cfg(all(...))]` header VERBATIM").

Every one of the 16 is an AND-only feature conjunction, which is exactly what
`required-features` can express — none of them is in the platform-predicate or
`any(...)` class the audit reports separately as unexpressible.

| target | its own `#![cfg]` |
|---|---|
| `bonsai_retrieval_probe` | `bonsai_retrieval` |
| `cross_domain_oracle_truth` | `clippy_verify`, `latent_retrieval` |
| `docker_class_a_agreement` | `docker_verify` |
| `docker_tier2_buildcheck` | `docker_buildcheck` |
| `jury_disagreement_poc` | `clippy_verify`, `latent_retrieval` |
| `kat_redact_t32_external` | `kat_service` |
| `kat_wetrun_gwet` | `kat_transport` |
| `p12_suppress_reverted_e2e` | `fix_verify`, `latent_retrieval` |
| `store_backend_e2e` | `clippy_verify`, `fix_verify`, `latent_retrieval` |
| `t062_withheld_pair_ood` | `clippy_lints`, `latent_retrieval` |
| `t44_end_to_end_l4` | `l4_lora` |
| `t51_l4_vs_decline` | `clippy_verify`, `l4_lora` |
| `t71_evolved_directions` | `latent_retrieval` |
| `t73_memory_axis_headroom` | `clippy_verify`, `latent_retrieval` |
| `wgsl_mine_pairs` | `kernel_heal` |
| `wgsl_t5_control` | `kernel_heal` |

## Severity — read it honestly

`load-bear = 0`: no script or workflow names any of these 16 by target name,
so no *gate's* verdict is currently laundered by one. `SILENT-NOW` means a
plain `cargo test --test <name>` — the thing a human types — green-zeros. That
is the real exposure: an agent asked to "run the G-WET gate" types
`cargo test --test kat_wetrun_gwet`, reads `ok. 0 passed`, and reports a pass.
`.issues/082`'s verification had to pass `--features kat_transport` explicitly
for the 11 tests to exist at all.

`ci_feature_guard.sh` layer 5c/5d runs `cargo test --all-features --tests`, so
CI does execute all 16 today, and layer 5d would red on a `running 0 tests`
line. **Adding the rows cannot shrink that** — under `--all-features` every
row is satisfied — which is what makes this fix safe against the
"arming a target reds a binary-counting floor" trap. Layer 5d's instrument
check (`Running tests/` lines == `find tests -maxdepth 1 -name '*.rs' | wc -l`)
is also unaffected: an explicit `[[test]]` row with the target's own name
still resolves `tests/<name>.rs`, and cargo keeps auto-discovering the rest.

## Fix

- [x] Added the 16 rows, each mirroring its file's `#![cfg]` header verbatim
- [x] **Verified each row is SUFFICIENT, not merely necessary** — 13 distinct
      exact feature sets, one `cargo test --features <EXACT set> --test <name>
      --no-run` per set, **13/13 BUILDS, 0 FAILS**. Never a superset: a
      superset build may supply the very import the row forgot. Full log:
      `bonsai_retrieval` · `clippy_verify,latent_retrieval` (3 targets) ·
      `docker_verify` · `docker_buildcheck` · `kat_service` ·
      `kat_transport` · `fix_verify,latent_retrieval` ·
      `clippy_verify,fix_verify,latent_retrieval` ·
      `clippy_lints,latent_retrieval` · `l4_lora` · `clippy_verify,l4_lora` ·
      `latent_retrieval` · `kernel_heal` (2 targets).
      Why this half matters — a row that
      exists and is too narrow is strictly worse than a missing one:
      `cargo test --workspace` silently SKIPS the target, `--all-features`
      BUILDS it (the union supplies whatever the row forgot — the one
      configuration anyone runs it in passes), and every audit counts it as
      protected. The row is wrong relative to what the file *imports*, and
      imports resolve through cfg-gated re-exports that defeat grep — so ask
      the compiler, once per row:
      `katgpt-rs/scripts/required_features_build_audit.py ../riir-clippy --batch`
      (one cargo run per EXACT feature set, never a superset). **Neither an
      error nor an artifact = UNSEEN, never BUILDS**
- [x] Re-ran the audit and the population is UNCHANGED while the findings
      are gone — a falling finding count is otherwise ambiguous between
      "repaired" and "the instrument went blind", and the walk size is what
      disambiguates:

      | | targets | `#![cfg]` | w/ req-f | SILENT-NOW | latent |
      |---|---|---|---|---|---|
      | before | 102 | 67 | 35 | **16** | 16 |
      | after | 102 | 67 | **51** | **0** | 16 |

      `targets` and `#![cfg]` identical, `w/ req-f` +16, `SILENT-NOW` → 0.
      `latent` unchanged at 16 — a different population (gated by a
      default-ON feature), untouched by this fix and not a defect

## Workspace context, measured the same day

`cfg_gated_target_audit.py` with no arguments, all 17 contract repos:
riir-clippy and riir-dao are now **0**, and riir-dapps, seal-remake,
riir-viewbridge were already 0. The remaining class is **208 SILENT-NOW**
targets over SEVEN repos — riir-ai 76, katgpt-rs 42 (a **pinned ratchet**,
`scripts/cfg_gated_floors.txt` `max_silent_now 42`, with `max_load_bearing 0`
as the wall), riir-game-sdk 31, riir-chain 28, riir-train 19,
riir-neuron-db 8, riir-mmorpg-examples 4 (= 208). **`load-bear` is 0 in every
one**, so no gate's verdict is laundered today. riir-game-sdk is the
worst-covered by proportion — 31 of its 50 gated targets unarmed, 18 of them
`prod_e2e`, i.e. production end-to-end suites that green-zero when named —
filed there as riir-game-sdk `.issues/028`. The rest belong to their owning
repos; katgpt-rs's is already pinned and does not need one

## Cross-refs

`.issues/082` (found this while verifying the G-WET suite — the explicit
`--features kat_transport` was load-bearing), katgpt-rs `AGENTS.md`
§"cfg-gated targets — the green-zero rule" (the doctrine + both scripts),
riir-dao `.issues/003` (the same class, one row, fixed 2026-09-09 in
`f3a0578` with both-directions verification).
