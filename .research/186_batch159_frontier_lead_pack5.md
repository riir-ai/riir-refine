# Research 186 — Batch 159: the frontier-lead mining pack V

**Status:** RECORD (batch landed 2026-09-18, Plan 150; closed same-session)

## Source

The Batch 158 re-arm's committed leads (frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan149_frontier_probe.py`): `single_char_add_str` +
`manual_clamp`. The third consecutive all-FIXABLE pack (every lead joins
the atom set, every lead is loop-verifiable — zero retirees).

## The rules

1. **`single_char_add_str`** (style, default-on — group pinned by the
   plan150 probe's default-groups arm) — `s.push_str("!")` → `s.push('!')`.
   FIXABLE by construction on a signature-proven mutable-String receiver:
   the NEW third derivation twin `helpers::mut_string_param_heads_over`
   (`String` by value + `&mut String`, the `body_rebinds` soundness leg) —
   on such a receiver the ONLY in-scope `push_str`/`push` pair is
   `String`'s own inherent `&str`/`char` methods, and `push_str(c)`
   appends exactly the UTF-8 bytes `push(c)` does for any single-`char`
   string VALUE. The arg resolves by VALUE (`LitStr::value()` — `"\n"`,
   `"\u{41}"`→`'A'`, multi-byte all qualify); the emit re-escapes via
   `char::escape_default` (`'`/`\`/controls escaped, ASCII printables
   verbatim, non-ASCII as `\u{..}` — always VALUE-identical, never
   textual). Byte/C strings, empty, multi-char, paren receivers, unproven
   receivers (locals, fields, custom types), UFCS, and glued comments all
   refuse. Documented divergences: local `String::new()` bindings +
   non-param receivers pending a local-binding derivation.
2. **`manual_clamp`** (style, default-on — group pinned by the plan150
   probe's default-groups arm, the arm plan149 never ran) —
   `x.max(3).min(5)` → `x.clamp(3, 5)` + the mirrored `x.min(9).max(7)`
   order twin. FIXABLE by the lattice absorption law on TOTAL orders
   (ints + `char`: `min(max(x,lo),hi)` ≡ `max(min(x,hi),lo)` ≡
   `clamp(x,lo,hi)`, all through the same `partial_cmp`). TWO real
   divergences REFUSE by design, both carried in-rule: (a) FLOATS —
   `NaN.max(3).min(5) == 3.0` (max/min ignore NaN) vs `NaN.clamp == NaN`
   (clamp propagates) — the anti-regime witness; (b) INVERTED bounds —
   clamp asserts `min <= max` (release too) while the manual chain never
   panics; closed by literal-bounds-only with a decidable `lo <= hi`
   compare (int digits as i128, char as u32, mixed kinds refuse).
   Variable bounds refuse (the `manual_range_contains` precedent).

## The graduation + re-arm

- BOTH families flip `covered: true` → the HeldOut per-family loop (10th +
  11th; heldout 9→11 families / 20→24 cases; frontier stays 4).
- The 2 consumed frontier rows ARCHIVED out of the active store (git
  history preserves them) — the KnownSet contract: a landed rule re-opens
  the family by archiving the row. Without this the e2e reads
  `heldout_ok=false` (the family-done guard sees the stale frontier rows'
  families as committed while their `solving_rule: null` never counts as
  holding — measured, the first run of this batch).
- Re-arm probe (`scripts/plan150_frontier_probe.py`, 5 candidates):
  ACCEPTS `from_str_radix_10` (the plan149-recorded spare, the `Vec::push`
  const-dodge added) + `assign_op_pattern` (`y = y + 1` ↔ `y += 1`).
  REJECTS recorded: `string_extend` (fires NOTHING on the shape under the
  full oracle — the lint has moved/renamed on this toolchain),
  `double_neg` (`!!x` fires `nonminimal_bool` — the crossfire),
  `unit_arg` (VERIFIED this round with the side-effect-block dodge — kept
  as the recorded spare for the next re-arm).

## GOAT

- **G1**: module tests (13 + 12 arms incl. escape/NaN/panic-guard refusal
  matrices) + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` (76 rows) + corpus `all_fixed_code_parses`.
  En-route: 5 lint findings in the new code healed/fixed (the healer
  caught 4 — question_mark, doc_markdown ×2, manual_let_else — one
  hand-fixed: a redundant closure outside the Batch-157 rule's whitelist,
  correctly refused by our own shipped rule).
- **G2**: score-bench row #135 — heal_rate **96.4% FLAT** vs the 96.36%
  baseline (vocab 55→57), parse_safety 100.0%, strata repo 75.0% /
  authored 98.0%; frontier snapshot appended (generation 7); the 2 known
  pre-existing v1-frozen drift warnings stand (unrelated lane).
- **G3**: freeze v2/v3 REGENERATED (the corpus moved → the Batch-151 law)
  + verified against real clippy: v2 e2e 35.2 s — **78 verified / 0 newly
  excluded / 15 pre-excluded**, solve 100.0% at every k (51/51, 26/26,
  27/27, 20/20), M4 PASS; v3 e2e 40.9 s — **80 verified / 0 newly
  excluded / 17 pre-excluded**, solve 94.4/92.3/95.8/100.0% (the
  pre-registered reported posture), stratification PASS. Bad-atom sets
  UNCHANGED at 6/7 — neither new atom interferes. The exclusion-count DROP
  (17→15 v2, 18→17 v3 vs Batch 158) is the benign window shuffle: two
  straddling composites no longer straddle at the 57-atom stride.
  Frontier e2e: **VERDICT ✓** (graduation 2 fresh heldout admissions + the
  re-arm's 2 leads; 28 seeds clean; 11 committed rows verified 0 drift
  post-archive; band fold population 11).
- **G4**: no new deps; all edits span-preserving. Corpus 74→76, FIXABLE
  55→57, atoms 55→57.

## Fallout repairs (pre-existing gaps this batch's gates caught)

1. **`RERANK_POOL` 20 → 24 + `HEADROOM_DEPTHS` +28 rung** (the P10/P11
   precedent, second re-fire): the +2 corpus entries crowded the
   Issue-024 casualty (`needless_return`×`vec-with-capacity`) to
   first-found at depth **21** — one past the pool floor of 20 — firing
   `pool_depth_rescue_guard` under `--features score_bench` (the designed
   canary; the ladder: depth 15→pool 16, 17→20, 21→24, slack 3 each
   time). `HEADROOM_DEPTHS` gained the 28 rung (a rung must stay above
   production — zeroing the recoverable band reads as "no headroom" and
   is actually "no deeper probe"); `PROD_DEPTH_IDX` 3→4.
2. **Batch 158's freeze record paragraph was MISSING** from
   `generations.rs` (the landing commit carried the 77/17 + 78/18
   numbers; the in-file paragraph was missed) — repaired en route.
3. **`.research/.highwater` was STALE at 184** (Batch 158 used 185
   without bumping) — this note is 186 and the highwater now reads 186.

## Gates

lib default **1892/0** · lib `latent_retrieval` **2045/0** (floors held) ·
lib `frontier_miner` **2209/0** · lib `score_bench` **2196/0** · clippy
`-D` clean in 4 states (default `--all-targets` / `latent_retrieval --lib`
/ `frontier_miner` example build / `score_bench --lib`).

## NEXT POOL

The Batch 159 re-arm's leads: `from_str_radix_10` +
`assign_op_pattern` (probe-verified this batch; spare: `unit_arg` with
the side-effect-block dodge). Beyond: arxiv walk #10 (Mon 09-21 window),
the nnethercote blog after his next post.
