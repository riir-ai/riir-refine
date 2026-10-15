# `sec` — security-pattern healing (4th retrieval domain)

> Phase 0+1 shipped 2026-08-22 (Plan 018 / Proposal 004 / Research 078-079 /
> Bench 046). Opt-in feature `sec`, default-off — promotion needs measured
> consumer pull (the `rust_perf` precedent). **Phase 2 (Batch 88) landed
> 2026-08-29** — see below. **The secret-blind scan surface (Issue 151)
> landed 2026-10-06 — see the section at the bottom.**

## What it is

Security-relevant **source-code patterns with bounded fix-spaces**: FFI
panic containment, Drop-glue panic safety, raw-pointer null hygiene, the
`from_raw_parts` len-zero contract, Rc backedge leaks — and since Phase 2:
unsound thread-safety impls, panicking FFI exports, weak-hash / password-
KDF hygiene, and constant-time secret comparison. NOT dependency auditing
(cargo-audit/cargo-deny), NOT a lint engine (dylint), NOT judgment-bearing
detection — every rule is parse-only provable with a bounded fix, or
suggest-only.

## The Phase-1 corpus (6 rules, 5 of 11 candidates killed)

| Rule | Family | `--fix` | The bounded fix |
|---|---|---|---|
| `ffi-extern-fn-catch-unwind` | `Ffi` | ✓ | wrap unit-return extern fn bodies in `std::panic::catch_unwind` (the in-house `#[riir_ffi]` rule generalized) |
| `ptr-deref-null-check-first` | `UnsafeHygiene` | ✓ | `let Some(p) = (unsafe { p.as_ref() }) else { return None };` prologue in Option-returning unsafe fns |
| `drop-impl-no-panic` | `DropLeak` | ✓ | `.unwrap()`/`.expect()` → `.unwrap_or_default()` inside drop glue (double-panic = abort) |
| `slice-from-raw-parts-len-guard` | `UnsafeHygiene` | ✓ | `if len == 0 { &[] } else { from_raw_parts(..) }` (ptr must be non-null even at len 0) |
| `extern-fn-reference-param-to-pointer` | `Ffi` | suggest-only | `&T` param → `*const T` + explicit deref (signature-changing — the C-side caller ripples) |
| `rc-backedge-field-prefer-weak` | `DropLeak` | suggest-only | direct `Rc<Self>` backedge field → `Weak<Self>` (constructor sites change too) |

## The Phase-2 corpus (Batch 88 — 5 rules, probe vs clippy 1.97)

All 5 candidates probed OPEN against real clippy 1.97.0 (pedantic +
restriction + nursery; zero equivalent-fix firings). The crypto trio is
**dep-gated** (the `fxhashmap` pattern: fires only when the file already
imports the replacement crate — the healer never edits `Cargo.toml`).

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `unsafe-impl-send-sync-unsound-field` | `UnsafeHygiene` | suggest-only | `unsafe impl Sync/Send` on a same-file type with `Rc`/`RefCell`/`Cell` fields — the Rudra unsound-impl class clippy cannot see; fix = Arc/Mutex redesign (type-level, never mechanical) |
| `ffi-export-panicking-body` | `Ffi` | suggest-only | panicking body (unwrap/expect/panic/indexing/assert) in a `#[no_mangle] extern` fn — the VALUE-return complement of Phase-1's catch-unwind (unit-return); fix needs a caller-chosen sentinel |
| `weak-hash-to-blake3` | `CryptoHygiene` | suggest-only, dep-gated | SHA1/MD5 (broken) + SHA256 (house loser) digest calls → blake3; fires only when the file imports blake3 |
| `password-hash-argon2` | `CryptoHygiene` | suggest-only, dep-gated | fn named `*password*` calling a bare digest → argon2 (memory-hard KDF); name-based trigger — false-NEGATIVES honest and expected |
| `constant-time-secret-compare` | `CryptoHygiene` | yes (strict-gated) | `a == b` → `a.ct_eq(b).into()` via `subtle::ConstantTimeEq`; gates: file imports subtle + (enclosing-fn name OR operand name hits the secret vocabulary) + both operands plain idents + `#[cfg(test)]` mods skipped |

Honest scope notes: the R5 proposer is shape-based, so a `String`-typed
secret (subtle has no `ConstantTimeEq` for `String`) FIRES and then the
`--fix --verify` compile gate reverts it — the type backstop, recorded as
a fixture. The `!=` form stays out of Phase-2 scope.

## P8 intake (2026-09-01) — the 12th rule, new `PanicSafety` family

| Rule | Family | `--fix` | The bounded fix / gate |
|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | yes (strict-gated) | `partial_cmp(..).unwrap_or(Equal)` (or `.unwrap()`) inside a sort comparator → `total_cmp(..)` — NaN becomes deterministically ordered instead of equal-to-everything (an inconsistent order the std sorts MAY PANIC on since 1.81). Gates: enclosing fn contains a `sort_by`/`sort_unstable_by`/`select_nth_unstable_by` call + exact chain shape + `Equal`-only default + comment guard + macro args refuse. The arg carries over VERBATIM (`PartialOrd::partial_cmp` pins `(&self, other: &Self)` — the same shape `total_cmp` takes), so the output is the idiomatic human form; non-float receivers (`Option<f32>`, other Ord primitives) fail E0599 → the `--verify` gate auto-reverts. Probe 2026-09-01: zero clippy lints fire on the shape (real clippy 1.97, pedantic+nursery+perf+suspicious+correctness+style) — the boundary holds with margin. Source: riir-ai Issue 832 T5's P8 intake (≈300 sibling library sites); the intake's type-blindness concern dissolved by `total_cmp` (direction-preserving recv/arg order + name-identical on f32/f64) — recorded in the intake close-out. Honest recall limit: a comparator defined in a helper fn and passed to `sort_by(helper)` by name is not detected — the sort call and the chain must share the enclosing fn. Property: KANI-VERIFIED (harnesses g1a/g1b/g1c, Issue 127 — see the "Property evidence" section below)

## Batch 131 (2026-09-08) — the `InputValidation` family opens, suggest-only

Two rules from the dhishwasher/Girder distill (BSL 1.1 — ORIGINAL fixtures
below; the evidence shapes are cited at the pin in `.research/153`), both
SUGGEST-ONLY: parse-only detectors with zero-FP negative suites ship in
`proposers::{length_prefix_alloc, secret_file_create}`; `FIXABLE_SEC` and
the `collect_edits` arms are deliberately untouched — these rules emit
suggestions, never auto-fixes (the `ffi-export-panicking-body` shape).

| Rule | Family | `--fix` | The detection + the bounded form |
|---|---|---|---|
| `length-prefix-validate-before-alloc` | `InputValidation` | suggest-only | a length parsed in-fn (`from_be_bytes`-class byte-swap or a header `.parse()`) reaching `vec![0u8; len]` with NO intervening bound comparison = allocation-size DoS (one peer-controlled header demands a multi-GB buffer). The bounded form: `if len == 0 \|\| len > MAX { return Err(..) }` BEFORE the alloc. Gates: same-fn chain only (cross-fn origins undetected — the helper-fn recall limit), identity tracking through casts/`?`/`ok_or`/`Some`/arithmetic, any comparison/match/assert on the tracked ident suppresses (validating a derived length bounds its source), `vec!`-repeat form only (`with_capacity` out of scope), `#[cfg(test)]` mods skipped. The evidence was BOTH WAYS in one codebase: an unbounded transport beside its own bounded twin (same author) — the class recurs even in careful code |
| `secret-file-create-without-0600` | `InputValidation` | suggest-only | a credential-named file (`secret`/`password`/`passwd`/`credential`/`token`/`hmac`, word-boundary, plural stems count — `secrets.json`) created by `File::create` / `fs::write` / an OpenOptions create-chain WITHOUT `.mode(..)` is 0644-by-umask, world-readable. The bounded forms: `#[cfg(unix)] OpenOptions::mode(0o600)` at create, or portably after create `#[cfg(unix)] set_permissions(Permissions::from_mode(0o600))` (the house `kat_account::write_private` form). Gates: narrow vocabulary (the ambiguous words are EXCLUDED — `key`/`identity`/`private`/`pem`, so `api_key` writes are missed, honest), create-class writes only (reads out), ANY hardening marker anywhere in the fn suppresses the whole fn (one hardened file hides a sibling — conservative). Suggest-only because a portable mechanical rewrite is not parse-only-provable (Windows `from_mode` maps coarsely) |

Gates at landing: sec lib 1261 passed / 0 failed (count pins: corpus 12→14,
`rule_ids_unique_and_kebab`, FIXABLE_SEC unchanged), boundary gate 2/2 (real
clippy over the grown probe crate — the new fixtures are compile-clean, the
E0425 class the parse gate cannot see), clippy 0 default + sec, score-bench
#80 FLAT 96.2/1.9 (suggest-only adds no fixer), retrieval: kernel+latent
byte-identical (94.7/96.9/97.7 blended, kernel 100/100/100, router 93/131),
sec self-variant top-1 holds for both new rules, ROUTER ANNOTATION re-pinned
per the Issue 020 contract — the two sec arms re-measured: 41/101 → 35/101
(floor 36→30/101) and 82/131 → 79/131 (floor 77→74/131); the new wire/fs
vocabulary ATTRACTS generic Rust spans to the sec centroid — first measured
sec-arm decay, recorded not weakened. The no-docker sec arms (Plan-018 pins)
are no longer measurable build states (the tree grew default-feature hard
deps) — noted in the floor fn.

## Batch 140 (2026-09-09) — the advisory-db distill, the family's 3rd suggest-only rule

One rule from the rustsec/advisory-db seed (CC0 @ `b50980aa`; Research 161 /
Plan 120 — the DB is a class TAXONOMY with real-world frequency, cited as
class provenance only; fixtures are ORIGINAL syntheses):

| Rule | Family | `--fix` | The detection + the bounded form |
|---|---|---|---|
| `unbounded-read-into-vec` | `InputValidation` | suggest-only | an unbounded stream read into a growable Vec from a network reader (`read_to_end`/`read_to_string`, or a `loop { read?; extend }` drain, no `Read::take` cap) grows the buffer without limit on peer-controlled bytes — the RUSTSEC-2024-0003 / CVE-2019-9514 (h2) resource-exhaustion class. The bounded form: `Read::take(reader, MAX_BYTES)` + `Vec::with_capacity(..)`. Gates: untrusted-source vocabulary is parse-provable only (TcpStream-typed params/bindings, listener `accept()`/`incoming()` children, `connect()`/`reqwest::get()` results — a bare `R: Read` is an honest NO); File readers are the trusted regime and never track; `.take(` anywhere in the receiver chain suppresses; the loop-drain shape needs BOTH halves (read on a tracked reader + extend on a local Vec) and caps suppress (`resize` fn-wide, `buf.len()` in a loop comparison, the read-count binding vs a non-zero operand — `n == 0` is the EOF sentinel, not a cap); `#[cfg(test)]` mods skipped. Suggest-only: choosing the limit is a protocol decision |

Gates at landing: default lib 1188/0 + sec lib 1290/0 (batch140 detector suite
18/18); clippy 0 default + sec; boundary gate 2/2 (real clippy over the grown
probe crate — and the E0425-class lesson fired again as designed: the first
buggy fixture was E0596-broken (`stream` not `mut`) and the gate red'd via the
`unwrap_used` pin going vacuous BEFORE anything could claim fixture readiness);
score-bench #87 FLAT 96.2/1.9 (suggest-only adds no fixer, same as B131);
corpus-stats sec 14 → 15; retrieval: blended 93.1/96.0/97.0 of 101 (floors
93/95/97 held), sec self-variant top-1 holds for the new rule, ROUTER
ANNOTATION re-measured per the Issue-020 contract — sec+docker 34/101 (33.7%,
was 35/101; the read_to_end/TcpStream vocabulary keeps the B131 sec-centroid
drift ticking −1 span; floor re-pinned 30 → 29/101) and full state 83/131
(63.4%, was 79/131 — +4 UP, cross-arm divergence consistent with the B130
kernel-vocabulary raise; floor re-pinned 74 → 78/131); both deltas recorded,
neither weakened silently. En-route fix on develop: `ce74bac`'s healer
`redundant_closure` rewrite of the `--score-bench` filter was load-bearing
(`&String` → `&str` deref-coercion — the documented healer escape class) and
compiled to nothing under default features; restored + verified via the
score-bench run itself.

## Batch 165 (2026-09-20) — the overflow sink, the family's 4th suggest-only rule

One rule from Research 193's staged lead (Plan 156; the Issue-127 Kani
pre-validation carried it through the probe gate): unchecked arithmetic on
wire-byte origins — the overflow SINK beside the alloc sink, on the SHARED
walker. `proposers::wire_taint` extracts the Batch-131 taint walk as a
`WireSink`-parameterized substrate (`length_prefix_alloc` consumes the same
walker — its 14-test suite pins the extraction behavior-identical);
`unchecked_wire_arithmetic` adds the arithmetic sink: `+ - * << >>` and the
syn-2 assign spellings (`len += 8` parses as `Expr::Binary` with an
assign-suffixed `BinOp`) where either operand is a tracked, unvalidated
bare ident. The repair is the checked op + an INVENTED error path
(`len.checked_add(FRAME_OVERHEAD)` — the Option/Result type the fn may not
carry), so suggest-only like the rest of the family.

| Rule | Family | `--fix` | The detection + the bounded form |
|---|---|---|---|
| `unchecked-wire-arithmetic` | `InputValidation` | suggest-only | a wire-byte-origin integer (`from_{be,le,ne}_bytes` / header `.parse()`) through a BARE overflow-prone operator with no intervening bound check — panic in debug, silent WRAP in release, and the wrapped length corrupts the frame parse ("check the multiplication, not just the result": the derived-validated shape's post-hoc range check validates the WRAPPED value). Bounded form: `checked_add`/`checked_sub`/`checked_mul`/`checked_shl` + an error path. Sink-distinct from `length-prefix-validate-before-alloc` (Plan 156 T2, recorded): distinct defect classes, distinct repairs, no composition gate — both sinks fire on the unvalidated composition shape with disjoint spans, and in the derived-validated shape the alloc rule is correctly silent while this one correctly still fires. Boundary (measured, Plan 156 T1 + T7 refinement): `arithmetic_side_effects` is SILENT on literal/const operands (`len + 8`, `len + CONST`) and where it fires (variable operands) it is blanket/unscoped/no-suggestion, restriction/allow-by-default; `manual_checked_ops` is Div-only at this pin; Div/Rem are NOT this sink. Honest recall limits: same-fn bare-ident origins only; `vec![0u8; len + 8]` (arithmetic INLINE in the repeat) is unreached — the shared walk never descends into macro tokens |

Gates at landing: sec lib 2156/0 (batch165 suite 15/15; the walker-extraction
pinned by the unchanged 131 suite); clippy 0 default + sec (--all-targets sec);
boundary gate 2/2 (real clippy over the grown probe crate — our fixture adds
NO banned-lint firing at all: the restriction is silent on const operands);
score-bench FLAT 96.36 (suggest-only adds no fixer, 15th consecutive FLAT);
corpus-stats sec 15 → 16; retrieval floors HELD zero re-pins (blended
93.1/95.0/96.0 of 101 vs floors 93/95/97), sec self-variant top-1 holds for
the new rule, ROUTER ANNOTATION re-measured per the Issue-020 contract —
sec+docker 31/101 (30.7%, was 34/101 at B140; the sec-centroid drift ticks
−3 spans; floor 29/101 STANDS, not weakened) and full state 78/131 (59.5%,
was 77/131 — +1, the kernel-vocabulary raise dominating; floor 72/131
stands). Property evidence: the landed rule's Kani row pair (g2c/g2d)
passed both ways on the WSL2 lane — see §Property evidence; the
pre-validation block returns to EMPTY.

## Property evidence — machine-checked (Kani, Issue 127)

A second evidence axis beside probe evidence: Kani (model checking over
CBMC) decides the property BOTH ways on real MIR — the buggy form must
VIOLATE it (existence witness), the fixed form must SATISFY it. The
harness crate is deliberately DETACHED:
[`.proofs/kani_sec_poc/`](../../.proofs/kani_sec_poc/README.md)
(`[workspace]`-marked, zero deps, Kani 0.68.0 / CBMC 6.11.0 / its own
pinned `nightly-2026-08-21` — never the workspace's stable
`rust-toolchain.toml`), run on the 4090 box's WSL2 lane only
(Windows-native Kani is unsupported), **never in CI, never a per-push
gate** — a dev-time oracle. Measured 2026-09-20 (5 harnesses at the POC
landing; 7 after the Batch 165 g2c/g2d extension); witnesses captured via
`-Z concrete-playback --concrete-playback print`.

| Rule | Family | Harness | Property | Verdict | Solver |
|---|---|---|---|---|---|
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1a_buggy_cmp_violates_transitivity_of_equal` | the `partial_cmp(..).unwrap_or(Equal)` comparator is non-total | **VIOLATED + witness** — x=`-inf`, y=`+NaN`, z=`+inf`: cmp(x,y)=Equal, cmp(y,z)=Equal, cmp(x,z)=Less | 0.046 s |
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1b_total_cmp_equal_is_transitive` | `total_cmp` transitive on Equal | **VERIFIED** | 0.060 s |
| `nan-unsafe-sort-comparator` | `PanicSafety` | `g1c_total_cmp_is_total_antisymmetric_nan_self_equal` | `total_cmp` total + antisymmetric + NaN-self-Equal | **VERIFIED** | 0.074 s |
| `unchecked-wire-arithmetic` | `InputValidation` | `g2c_buggy_framed_len_reaches_overflow` | the corpus fixture's bare `parse + FRAME_OVERHEAD` reaches overflow (its u32-domain twin — the fixture's `as usize` widening absorbs the overflow on 64-bit hosts, so the machine-checkable property lives in the wire-width domain the rule's advice targets: checked_add BEFORE any widening) | **VIOLATED + witness** — `"attempt to add with overflow"` inside the buggy callee (any header above `u32::MAX - 8`) | 0.023 s |
| `unchecked-wire-arithmetic` | `InputValidation` | `g2d_checked_framed_len_never_silently_wraps` | the bounded repair is exact two-sided: Some ⟺ u64-exact length, None ⟺ true length > u32::MAX | **VERIFIED** (every check SUCCESS, 0 failures) | 0.027 s |

### Pre-validation (staged leads, NOT shipped rules)

This block is deliberately separate: a machine-checked property on a rule
that does not exist yet is the strongest-looking row in the section and
the weakest claim in it — it must never read as corpus coverage.

**EMPTY — as designed.** The `unchecked-wire-arithmetic` rows moved UP
into the shipped table at the Batch 165 landing (Plan 156 T6): the
g2a/g2b probe-shape harnesses (two parses summed) pre-validated the
Research-193 staged lead; the g2c/g2d fixture-shape harnesses
post-validate the shipped rule. The block refills when the next staged
lead earns machine-checked pre-validation before its batch.

**Scope caveat (verbatim from the POC record):** these numbers are the
FLOOR, not the ceiling. The harnesses are loop-free, dependency-free,
≤6 symbolic variables, and needed no unwinding config; solver times of
0.03–0.07 s say nothing about loop/unwind bounds, heap structures, or
workspace-real-code harnesses — the "minutes per fixture" ceiling was
never approached and remains UNMEASURED. Every cost figure in this
section is a loop-free-fixture figure.

## The boundary discipline (non-negotiable)

`tests/sec_boundary_gate.rs` runs REAL clippy over the corpus fixtures with
the banned list enabled (`mem_forget`, `indexing_slicing`,
`arithmetic_side_effects`, `unwrap_used`/`expect_used`, `improper_ctypes*`,
`partialeq_ne_impl`, `undocumented_unsafe_blocks`, `disallowed_methods`):
no banned lint may fire MachineApplicable on a sec fixture — a future
clippy that auto-fixes one of our territories FAILS the gate and that rule
retires. The pinned allowed overlaps (all fire with NO suggestion — our
fixes are additive): `unwrap_used` on the drop fixture, `indexing_slicing`
on the panicking-FFI fixture, `undocumented_unsafe_blocks` on the
unsound-impl fixture.

## The unsafe counter (geiger taxonomy, zero deps)

`count_unsafe` / `UnsafeCounts` / `file_forbids_unsafe`: unsafe fns +
methods, attribute-unsafety (`#[no_mangle]`/`#[export_name]` on otherwise
safe fns), unsafe impls/traits, unsafe exprs. Feeds
`SecVerdict.unsafe_count_delta` — a review SIGNAL, never a gate.

## Negative control

riir-viewbridge IS the negative control: its whole FFI surface is
`riir_ffi!`-wrapped (catch_unwind inside), and the dry `--fix --domain sec`
run over its 24 files proposes 0 edits. A nonzero there falsifies the
proposer.

## Status

- Phase 0 (scaffold + wiring + boundary gate + unsafe counter): DONE
- Phase 1 (corpus + 4 proposers + GOAT): DONE (Bench 046)
- Phase 2 (unsafe + crypto hygiene): UNBLOCKED (≥6-rule gate met) but
  deferred per Plan 018 — a future batch
- Promotion to default: needs ≥1 workspace repo adopting sec in CI

## See also

- [Proposal 004](../../.proposals/004_sec_domain_fourth_heal_domain.md) — the design argument
- [Plan 018](../../.plans/018_sec_domain_phase0_batch1.md) — execution plan
- [Research 078](../../.research/078_sec_domain_seed_verdict_rust_secure_code_index.md) — the seed survey
- [Research 079](../../.research/079_sec_domain_phase0_1.md) — the Phase-0+1 record
- [Bench 046](../../.benchmarks/046_sec_domain_phase0_1_goat.md) — the GOAT numbers

## The secret-blind surface (Issue 151, 2026-10-06)

Detection of secrets in code is **scan-based, not corpus-match** — the
verdict record: secret formats are exact enumerable facts (fixed prefix +
body charset + length), so detection is deterministic and versioned
(`redact v4`), while corpus-match (latent retrieval over buggy fixtures)
stays the FIX-suggestion machinery. A "hardcoded secret" corpus entry would
carry a fake key in its fixture (tripping our own scanners), match fuzzily
where exactness matters, and its fix is a process change (env var / secret
store) — never a mechanical rewrite.

One canonical scanner, three consumers:

| Consumer | Boundary | Posture |
|---|---|---|
| `kat_redact::RedactionGate` (riir-kat v4) | **egress** — mining sync, the LLM harness lanes | fail-closed deny-by-default; any finding drops the span |
| `structural_admission::scan_secret_shapes` (reef P2) | **persistence** — structural proposals | the SAME table, post-filtered: prefix kinds ∪ private-key blocks only + the word-boundary check (entropy rules would block every payload carrying hashes) |
| `secret_blind` (`--secret-scan`) | **developer surface** | suggest-only warn + report + blind transform; values and fingerprints never shown |

The v4 pattern set consolidates the industry sources: gitleaks `b58d3f1`
(MIT — the transcription source, parity-fixture-pinned),
secrets-patterns-db (CC BY-SA 4.0 — coverage cross-check ONLY; ShareAlike
forbids table copying), secret-regex-list (no license — reference only).
Provider prefixes are functional facts, not creative expression; the
scanner's transcription is prefix + charset predicate + length, not a regex
copy (no ReDoS surface by construction).

**The shareable-output rule:** a finding's fingerprint is an unkeyed
`blake3(bytes)[..12]` — safe in the gate's LOCAL JSONL log, a
dictionary-reversal leak on stdout. Every stdout surface (report, warn
lines, `--blind`) carries kind + location only. Recorded upgrade path if
cross-run correlation is ever needed: a per-install `blake3::keyed_hash`
key.

**Digest-pin adjudication (P63, 2026-10-15):** the developer surface
splits from the gate on ONE class — contextually-evidenced digest pins.
A high-entropy-hex run carrying a digest-scheme tag (blake3:/sha256:/…,
glued or across whitespace + one opening quote — the blake3: "<hex>"
field shape) or opening the fresh-literal value of a pin-named
declaration (_SHA/_DIGEST/_BASELINE/_COMMITMENT + algo spellings,
rustfmt-wrap carried) is adjudicated benign in the REPORT count only,
with the suppressed count DISCLOSED (never a silent drop). Exact
provider formats are never adjudicated (aws/sk-/AQ. keep firing under
both guards; probe-pinned by test), and the egress gate + --blind keep
the RAW canonical verdicts — a report FP costs noise, a gate FP never
ships a secret, and only the former is a defect (REDACT_VERSION
untouched). Live basis: 65 suppressions workspace-wide at first run,
100% digest-pin class (instinct 4, refine 53 incl. the 49-row corruption
corpus, reflex 3 wrapped BLAKE3 fixture pins, katgpt-rs 5 _BLAKE3/_DIGEST
fixture pins); zero non-digest findings moved.

**Egress census (the trust claim's basis):** every span-text egress passes
the gate — `push_mining_batch`/`load_mining_rows` (heal.rs), the
`fixer_regate` harness (`gate_samples` before any arm), the `kat_drain`
daily loop (same gate; all-blocked = exit 4). Transports that carry no code
text: burn/lease/fixstat/replay (counts + rule ids only). Honest limits:
unknown formats are caught only by the entropy rules (the Bench-052 hex
story — 37% of hex-wrapped AWS-shaped keys pre-v2, unconditional since);
arbitrary-transform classes (byte arrays, xor, split literals) are uncaught
BY DISCLOSURE; PII is out of scope (the span itself is the asset — no
pattern set closes that); a redacted payload is terminal and is never
re-gated.
