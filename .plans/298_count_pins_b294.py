# Batch 294 (Plan 298 T1.4) — ONE shape-asserted script pass over the count pins.
# Asserts the pre-state (26 rules / 20 suggest-only / 14 detectors), then applies
# the 28 / 22 / 16 rewrite. Exits non-zero if any shape is missing.
import io
import sys

ROOT = "E:/git/riir-refine"


def load(p):
    return io.open(f"{ROOT}/{p}", encoding="utf-8", newline="").read()


def save(p, s):
    io.open(f"{ROOT}/{p}", "w", encoding="utf-8", newline="").write(s)


def must(s, needle, where):
    if needle not in s:
        print(f"SHAPE MISS [{where}]: {needle!r} not found")
        sys.exit(1)


edits = []

# --- AGENTS.md (2 sites) ---
s = load("AGENTS.md")
must(s, "security-pattern bounded fixes (26 rules; 6 fixable + 20 suggest-only)", "AGENTS:18")
s = s.replace(
    "security-pattern bounded fixes (26 rules; 6 fixable + 20 suggest-only).",
    "security-pattern bounded fixes (28 rules; 6 fixable + 22 suggest-only).",
)
must(
    s,
    "security-pattern healing domain (26 rules — 6 fixable + 20 suggest-only;",
    "AGENTS:134",
)
s = s.replace(
    "security-pattern healing domain (26 rules — 6 fixable + 20 suggest-only;",
    "security-pattern healing domain (28 rules — 6 fixable + 22 suggest-only;",
)
must(
    s,
    "the client-IP/shell-string/mapped-IP/strike-count std classes)",
    "AGENTS:134-tail",
)
s = s.replace(
    "the client-IP/shell-string/mapped-IP/strike-count std classes)",
    "the client-IP/shell-string/mapped-IP/strike-count std classes; "
    "Batch 294 RustSec transcription (RUSTSEC-2026-0380/0379, CC0 — the "
    "masked-key packing + silent-cap degrade classes)",
)
edits.append("AGENTS.md")
save("AGENTS.md", s)

# --- README.md (2 sites) ---
s = load("README.md")
must(s, "Batch 291 + Batch 292 (Bench 046;", "README:15-head")
s = s.replace(
    "Batch 291 + Batch 292 (Bench 046;",
    "Batch 291 + Batch 292 + Batch 294 (Bench 046;",
)
must(s, "penthera distill Plan 295; CVEngine distill Plan 296) | 26 rules —", "README:15-plans")
s = s.replace(
    "penthera distill Plan 295; CVEngine distill Plan 296) | 26 rules —",
    "penthera distill Plan 295; CVEngine distill Plan 296; RustSec transcription Plan 298) | 28 rules —",
)
must(s, "the std `IpAddr`/`Command` + identity/strike classes) — ALL", "README:15-tail")
s = s.replace(
    "the std `IpAddr`/`Command` + identity/strike classes) — ALL",
    "the std `IpAddr`/`Command` + identity/strike classes) + the Batch-294 "
    "RustSec transcription (`packed-field-silent-overflow-wrong-key` masked-key "
    "packing + `capacity-exceeded-silent-fallback-not-refusal` silent-cap degrade "
    "— the RUSTSEC-2026-0380/0379 classes) — ALL",
)
must(s, "6 fixable proposers + 14 suggest-only detectors + the real-clippy boundary gate", "README:15-detectors")
s = s.replace(
    "6 fixable proposers + 14 suggest-only detectors + the real-clippy boundary gate",
    "6 fixable proposers + 16 suggest-only detectors + the real-clippy boundary gate",
)
must(
    s,
    "the Batch-292 CVEngine client-IP/shell-string/mapped-IP/strike-count quartet, 26 rules + 6 fixable proposers + 14 suggest-only detectors",
    "README:134",
)
s = s.replace(
    "the Batch-292 CVEngine client-IP/shell-string/mapped-IP/strike-count quartet, 26 rules + 6 fixable proposers + 14 suggest-only detectors",
    "the Batch-292 CVEngine client-IP/shell-string/mapped-IP/strike-count quartet "
    "+ the Batch-294 RustSec masked-key/silent-cap pair, 28 rules + 6 fixable proposers + 16 suggest-only detectors",
)
edits.append("README.md")
save("README.md", s)

# --- .docs/11_domains/sec.md — B294 section + Status row ---
s = load(".docs/11_domains/sec.md")
must(s, "## Property evidence — machine-checked (Kani, Issue 127)", "secmd-anchor")
b294 = """## Batch 294 (2026-10-27) — the RustSec transcription, the JIT classes userland-shaped

Two suggest-only rules from the wasmtime RUSTSEC-2026-0380/0379 distill
(Research 332, CC0 — the two CVSS-9.9 sandbox-escape advisories behind
riir-ai's pinned 28.0.1; the JIT-internal failures generalized to their
Rust-shaped userland classes). Both `InputValidation`.

| Rule | Family | Shape |
|---|---|---|
| `packed-field-silent-overflow-wrong-key` | `InputValidation` | a count/ID masked to a 2^k−1 literal (`& 0xFFF`) and used as a LOOKUP KEY silently loses the high bits — values past the field width alias onto wrong keys: wrong-item access, no panic (CVE-2026-115889: the 24-bit packed value-number class). Bounded form: range assert at the packing site / checked newtype / widen the field. Gates: lookup-key scoping (index form only) + origin gate (same-fn parse origin or counter/ID vocabulary segments) + 2^k−1 mask literals; `try_from`/`checked_` anywhere in the fn suppresses, any comparison/match/assert naming the origin suppresses. Distinct-from: `unchecked-wire-arithmetic` owns the bare-op sink (BitAnd is not in its operator set); `cast_possible_truncation` is blanket as-cast pedantic with no key scoping; the B258 recorded drop adjudicated in-rule — this rule owns neither half. Honest limits: designed aliases (ring buffers, hash buckets) fire and review decides; type-blind (`u8 & 0xFF` no-ops fire); named-const masks and method lookups out; detector `packed_field_wrong_key::detect` |
| `capacity-exceeded-silent-fallback-not-refusal` | `InputValidation` | the cap-exceeded branch TRUNCATES and falls through — the encode/build continues with silently-dropped entries: wrong encoding, wrong dispatch, no refusal (CVE-2026-115893: the br_table class; the fix's >65,520-targets REFUSAL half is the lesson). Bounded form: `if items.len() > CAP { return Err(..) }` — a cap that lies is worse than no cap. Gates: the same-cap shape (`.len() > CAP` + the branch truncates the SAME receiver to the SAME cap text) + encode/build vocabulary (fn-name segments or `to_{be,le,ne}_bytes` in the body); the refusal twin (any return/?/panic-family in the branch) suppresses. Distinct-from `length-prefix-validate-before-alloc`: missing-check (alloc-DoS sink) vs lying-cap (wrong-dispatch sink). Honest limits: benign truncation inside encode-vocabulary fns fires and review decides; slice form + `take(n)` out; cross-fn builds unconnected; detector `capacity_silent_truncate::detect` |

Gates at landing: sec module 215/0 (19 new detector tests); sec lib 2964/0;
default lib 2749/0; clippy `-D` ×4 postures (default / sec / all-features /
no-default) + all-targets+sec; T7 wide probe (real clippy,
pedantic+nursery+suspicious+correctness+perf+style+complexity: ZERO
default/suspicious/correctness safety lints on either buggy shape —
`cast_possible_truncation` fires on r2's unrelated `as u16` length-encode
(the B258-owned axis, not the rule's site, no fix) + two nursery
fixture-noise candidates; boundary gate 2/2 — `indexing_slicing` fires on
the packed-key fixture with no machine fix (the ffi-export additive
precedent), the silent-cap fixture fully silent); retrieval floors
re-measured at the sec posture: sec self-variants 28/28 top-1, fixture
labels green, router annotation (sec arm) re-pinned 60/167 → 62/169
(measured 67/169 = 39.6%; both new spans routed correctly — delta +2
correct +0.7pp); score-bench run 204 appended (sec spans ride the
demonstrability column at n=28, constant 1.000); freeze NOT owed
(`generations.rs::atoms()` builds from the clippy seed corpus only —
verified live; the P30 confirm, sec-only delta); release-set binary
rebuild + install executed (sec compiles into the default build).

## Property evidence — machine-checked (Kani, Issue 127)"""
s = s.replace("## Property evidence — machine-checked (Kani, Issue 127)", b294, 1)
must(s, "- Batch 292 (2026-10-27): corpus 22 → 26 (6 fixable + 20 suggest-only;", "secmd-status-292")
anchor = "- Promotion to default: needs ≥1 workspace repo adopting sec in CI"
must(s, anchor, "secmd-status-anchor")
s = s.replace(
    anchor,
    """- Batch 294 (2026-10-27): corpus 26 → 28 (6 fixable + 22 suggest-only;
  the RustSec transcription, Plan 298) — sec self-variants 28/28 top-1,
  router annotation (sec arm) re-pinned 60/167 → 62/169 (measured 67/169
  = 39.6%; both new spans routed correctly, delta +2 correct +0.7pp);
  boundary gate 2/2 (indexing_slicing on the packed-key fixture, no
  machine fix — additive; silent-cap fully silent); wide probe clean
  (zero default/suspicious/correctness safety lints); freeze NOT owed
  (clippy-seeded corpus unmoved); binary rebuilt + installed
- Promotion to default: needs ≥1 workspace repo adopting sec in CI""",
)
edits.append(".docs/11_domains/sec.md")
save(".docs/11_domains/sec.md", s)

# --- post-assertions ---
s = load("AGENTS.md")
assert "28 rules; 6 fixable + 22 suggest-only" in s, "AGENTS:18 post"
assert "28 rules — 6 fixable + 22 suggest-only" in s, "AGENTS:134 post"
assert "Batch 294 RustSec transcription" in s, "AGENTS:134 tail post"
s = load("README.md")
assert "Batch 291 + Batch 292 + Batch 294" in s, "README:15 head post"
assert "28 rules —" in s, "README:15 rules post"
assert "16 suggest-only detectors" in s, "README detectors post"
assert "Batch-294 RustSec masked-key/silent-cap pair, 28 rules" in s, "README:134 post"
s = load(".docs/11_domains/sec.md")
assert "## Batch 294 (2026-10-27)" in s, "sec.md section post"
assert "corpus 26 → 28" in s, "sec.md status post"

print("count pins applied:", ", ".join(edits))
