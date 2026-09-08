# Plan 112 — Batch 131: the Girder sec-domain distill (length-prefix-validate-before-alloc)

**Status:** IN PROGRESS (2026-09-08 — plan-first per the 2026-08-18 owner protocol; verdict YES B−, owner go "do distill"; corpus edit not started)

Queue lead: dhishwasher/Girder → **route `sec`** (opt-in, 12 rules, 5 families, Phase 0+1 GOAT
Bench 046). NOT kernel_opt (zero GPU kernels — wgpu is egui rendering only), NOT rust_perf
(application code — a rule CONSUMER, not a technique library; hot paths are a petgraph reverse
BFS and TF-IDF/Jaccard, both standard shapes). Verdict + dedup evidence: Research 153.

**Pin:** `dhishwasher/Girder` @ `3b29453f1dec39ef24f975d76e25ab2600a38b8c` (2026-09-07 commit,
fresh at inspection 2026-09-08). **License: BSL 1.1** (source-AVAILABLE; Apache-2.0 conversion
2030-09-04) — the standing fixture constraint applies with force: **never paste Girder bytes
into `buggy_code`/`fixed_code`; every fixture is an ORIGINAL minimal reproduction** — cite the
shape and the file/line at the pin, nothing more.

**The evidence (both ways in one codebase — the strongest kind):**

- UNBOUNDED hazard: `crates/aether-dap/src/transport.rs` `read_frame` (~L24–55) parses
  `Content-Length: N` from a peer and does `vec![0u8; len]` with **no upper bound** — an
  adapter-controlled `usize` straight into an allocation (abort/OOM at multi-GB, immediate
  abort at huge N).
- BOUNDED twin: `crates/aether-app/src/project/collaboration_transport.rs` `read_frame`
  (~L1209–1221) reads the 4-byte BE length prefix, validates `0 < n ≤ limit` **before**
  `vec![0; n]`, with a negative test writing `limit + 1` as the prefix (~L2314).
- Same author, two transports, one missed the gate → the hazard class recurs even in careful
  code. That recurrence is what makes it a corpus rule rather than a one-off patch.

**Boundary fit (from `src/domains/sec/mod.rs`):** parse-only provable, bounded fix OR
suggest-only, strict-gated, never judgment-bearing. The mechanical fix for the frame rule
would need to INVENT a limit constant and an error-return path — not mechanical, therefore
**suggest-only**, per the shipped `ffi-export-panicking-body` precedent (suggest-only rules
are admissible; they emit, they never auto-fix).

## Tasks

- [ ] T0: provenance re-clone — `.raw/girder` @ `3b29453f`, verify `HEAD == pin` + LICENSE
      (BSL 1.1); re-verify both evidence sites at the pin (transport.rs unbounded,
      collaboration_transport.rs bounded twin); re-verify the secondary-candidate sites
      (collaboration_identity.rs L821 / collaboration_discovery.rs L379 mode(0o600) writes).
      4090 idle-check before any gate run (standing rule).
- [ ] T1: dedup re-verification at mine time — BOTH vocabularies vs `src/domains/sec/`
      (all 12 rules), `rust_perf`, `clippy_lints`: source vocabulary
      (`length|frame|read_exact|Content-Length|content_length|from_be_bytes`) × operator
      vocabulary (`alloc|bound|limit|untrusted`); and
      (`symlink|0o600|0o700|permission|secret-file`). Known nearest hits (from the verdict,
      re-confirm at mine time): kernel_opt `length-prefix-shape-faithful-hash` (B32c — the
      HASHING class, not alloc-bound: zero overlap), `constant-time-secret-compare`'s
      vocabulary-gating discipline (the gate PATTERN to reuse, not a duplicate),
      `kat_account.rs` 0600 write-half (our own code = precedent, not corpus coverage).
- [ ] T2: `SecFamily` extension — additive variant `InputValidation` (wire/length-prefix/
      input-bound class; no existing family fits: Ffi/DropLeak/UnsafeHygiene are
      unsoundness/panic classes, CryptoHygiene is crypto-API, PanicSafety is sort comparators).
      Walk every family-exhaustive surface: the enum + corpus doc table + family-keyed tests +
      any match sites. Additive only — no variant renumbering.
- [ ] T3: author rule `length-prefix-validate-before-alloc` (suggest-only,
      `SecFamily::InputValidation`) — corpus entry with ORIGINAL fixture code; concept carries
      the both-ways rider (one codebase holding the hazard AND the fix = the recurrence
      evidence) + the DoS magnitude story (peer-controlled usize → allocation; the cost axis
      is security, not cycles). Strict gates: same-fn chain from a length PARSE
      (`u32::from_be_bytes` / header string `parse::<usize>()`) to a `vec![..; len]`-class
      allocation, NO intervening comparison against a constant or limit, comment guard per the
      house discipline.
- [ ] T4: suggest-only proposer — parse-only syn walk, NO `FIXABLE_SEC` membership, NO
      `collect_edits` arm (suggest emission only, `ffi-export-panicking-body` shape); suggest
      text states the bounded form (validate `0 < n ≤ limit` before the alloc) + the failure
      class. Negative suite ZERO false positives: bounded sites (validated n), trusted-length
      sites (`vec![0; buf.len()]`, const-size arrays), empty vecs, `with_capacity` from
      trusted metadata. Honest recall limit stated in-rule (cross-fn length origin undetected
      — the `nan_safe_cmp` helper-fn precedent).
- [ ] T5: secondary candidate admission gate — `secret-file-create-without-0600` (C+ at
      verdict) enters THIS batch only if its own negative suite stays zero-FP under the same
      strict-gate bar (secret-vocabulary path + `File::create`-class write + no
      mode/permissions hardening; decide fix-vs-suggest per the unix-only `mode()` issue —
      `set_permissions` after create is the portable bounded form our own `kat_account.rs`
      uses). If it fails the bar: queue lead line, NOT a rule — recorded honestly.
- [ ] T6: standing gates — corpus parse gate; `cargo test -p riir-clippy --features sec
      --lib` count pins (sec corpus 12→13[/14], family table, `rule_ids_unique_and_kebab`,
      FIXABLE_SEC UNCHANGED for suggest-only, mod.rs lists); retrieval floors re-run + ROUTER
      ANNOTATION RE-PIN per the Issue 020 contract (new corpus entries shift TF-cosine
      centroids — the B129/B130 lesson, expect a re-measure); clippy 0 default + sec;
      score-bench re-run (FLAT expected — suggest-only adds no fixer).
- [ ] T7: honesty floors — AGENTS.md sec row + .docs + README count rows if pinned (12 →
      landed count); suggest-only status labeled everywhere the fixable count is stated.
- [ ] T8: records — Research 153 status → RECORD; queue snapshot Last-landed line; this
      plan's checkboxes `[x]`; `.raw/girder` removed at landing (the pin lives in this plan
      + Research 153).
- [ ] T9: commit + push on develop — `feat:` prefix (corpus change), named-file staging only
      (sibling agents in this worktree), `scripts/staged_set_audit.py` before the commit.

## Honest caveats

- Suggest-only batch: the value axis is detection PRECISION (zero-FP negative suite), not
  applied fixes. Expected yield 1–2 rules before the ~50% coordinator falsification; ~1
  landing is a success. Do not inflate.
- BSL 1.1: fixtures original, quotes limited to the research note's evidence citations.
- Unread at verdict (not load-bearing for the two candidates, re-check only if a third
  candidate emerges): `aether-debugger/*`, `aether-extensions/src/marketplace.rs`.
