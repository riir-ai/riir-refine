//! Bench 004 — Plan 199 T3.4: the WASM validator lane's THREE-arm PoC gate.
//!
//! The lane (T3.2/T3.3) ships a committed, BLAKE3-verified validator artifact
//! executed in-process by the fuel-bounded interpreter. This bench prices the
//! lane honestly, against the two baselines the plan names:
//!
//! 1. **G-safety (hard floor, posture-independent)** — the planted-violation
//!    fixture family: every violation class the validator claims to catch,
//!    caught 100%. ONE false pass fails this gate permanently. Runs on any
//!    box: it is a correctness claim, not a timing claim.
//! 2. **G-cost (timing, provenance-gated)** — verdict cost over a
//!    deterministic 10⁴-span fixture set, WASM interpreter vs the same
//!    predicate compiled NATIVELY into the healer (the honest baseline —
//!    likely faster, zero boundary cost). Every published number quotes the
//!    `bench_preflight.sh` PROVENANCE line; a refused preflight labels every
//!    G-cost row PROVISIONAL (never skipped — a missing script is
//!    UNAVAILABLE, never a green pass).
//! 3. **G-value (hot-swap quantification)** — the bytes→serving cost of the
//!    committed-artifact store: install (admission + BLAKE3 + write) + load
//!    (re-verify + re-admit). The native equivalent is a host rebuild —
//!    measured separately for the T3.5 verdict, not here.
//!
//! The 10⁴-span set is generated in-code (seeded xorshift64*, 10 archetypes)
//! so the set is identical on every host without committing 10⁴ files. The
//! timing is correctness-coupled: the verdict/score sinks MUST equal the
//! generator's own truth (a triple agreement — generator vs native vs wasm),
//! so an optimiser deleting the measured loop fails the bench instead of
//! reporting a fast zero.
//!
//! The `cargo clippy` re-run baseline (arm 3 of G-cost in the plan) is
//! env-gated — it spawns a real clippy over a scratch crate (seconds class)
//! and is only meaningful with preflight provenance:
//!
//! ```bash
//! # the whole gate (safety floor + provisional-labeled timing):
//! cargo bench --bench bench_004_wasm_validator_goat --features wasm_validator
//! # + the clippy re-run baseline arm:
//! WASM_VALIDATOR_BENCH_CLIPPY=1 cargo bench --bench bench_004_wasm_validator_goat --features wasm_validator
//! # the GATE posture — exit 2 unless preflight PASSED (for the citable run):
//! WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_004_wasm_validator_goat --features wasm_validator
//! ```

#![allow(clippy::pedantic)]

use riir_refine::validators::{
    Outcome, StoreError, Trap, ValidatorStore, Vm, ABI_INPUT_ADDR, DEFAULT_FUEL, MEM_LEN,
};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

/// The committed validator this bench prices (Plan 199 T3.2's artifact).
const VALIDATOR_NAME: &str = "useless_format_brace";

/// The 10⁴-span fixture set the plan's G-cost arm names.
const SPAN_COUNT: usize = 10_000;
/// Best-of-N total-wall passes (the contention-resistant headline statistic —
/// the same reasoning as preflight's best-of-N canary: contention adds time
/// to SOME runs, so the minimum separates it from a slow kernel).
const MEASURED_PASSES: usize = 5;
/// Spans per instrumented chunk (per-span Instant overhead ~25 ns would
/// poison the native arm's ~tens-of-ns span cost; chunk means do not).
const CHUNK: usize = 100;
/// Fixed generator seed — the fixture set is identical on every host.
const SPAN_SEED: u64 = 0x1990_0040_0002_0001;

// ── the fixture generator ────────────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// xorshift64* — deterministic, no deps, identical on every host.
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const WORDS: [&str; 10] = [
    "cargo", "format", "span", "literal", "refactor", "inline", "healer", "lint", "wasm", "bounded",
];

/// One generated span. `braces` is the brace count the archetype INJECTED —
/// generator truth, asserted against both arms before any number is printed.
struct SpanFixture {
    text: Vec<u8>,
    braces: usize,
}

impl SpanFixture {
    fn violating(&self) -> bool {
        self.braces > 0
    }
}

fn push_words(s: &mut String, rng: &mut Rng, n: usize) {
    for _ in 0..n {
        s.push_str(WORDS[rng.below(WORDS.len())]);
        s.push(' ');
    }
}

/// 10 archetypes, cycled by index: 4 always-violating (interpolation,
/// escapes, mid-span brace, brace-heavy), 4 always-clean (plain literal,
/// path-ish, long clean, punctuation bait), 2 half-and-half (short,
/// unicode). The truth rides `braces`, never the archetype label.
fn gen_spans(n: usize, seed: u64) -> Vec<SpanFixture> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let len: usize = 8 + rng.below(504); // 8..=512 bytes
        let mut s = String::with_capacity(len + 16);
        let mut braces = 0usize;
        match i % 10 {
            0 => {
                s.push_str("format!(\"");
                push_words(&mut s, &mut rng, len / 12);
                s.push_str("\")");
            }
            1 => {
                s.push_str("format!(\"");
                push_words(&mut s, &mut rng, len / 16);
                s.push_str("{name} more text\")");
                braces += 2;
            }
            2 => {
                s.push_str("format!(\"");
                push_words(&mut s, &mut rng, len / 16);
                s.push_str("a {{literal}} tail\")");
                braces += 4;
            }
            3 => {
                s.push_str("src/");
                push_words(&mut s, &mut rng, 2);
                s.push_str(".rs:12:5 render_span");
            }
            4 => {
                s.push_str("format!(\"");
                push_words(&mut s, &mut rng, len / 12);
                s.push_str("{x}\")");
                braces += 2;
            }
            5 => {
                push_words(&mut s, &mut rng, len / 6);
            }
            6 => {
                if i & 1 == 0 {
                    s.push_str("format!(\"{v}\")");
                    braces += 2;
                } else {
                    s.push_str("plain str");
                }
            }
            7 => {
                s.push_str("héllo wörld — naive 措辞 ");
                if i & 1 == 0 {
                    s.push_str("{flag}");
                    braces += 2;
                }
                push_words(&mut s, &mut rng, 2);
            }
            8 => {
                s.push_str("[vec](tuple)<angle>|pipe:colon;comma,stop. ");
                push_words(&mut s, &mut rng, 1);
            }
            _ => {
                let pairs = (len / 8).max(2);
                for p in 0..pairs {
                    s.push_str(&format!("{{k{p}}}"));
                    braces += 2;
                }
            }
        }
        out.push(SpanFixture {
            text: s.into_bytes(),
            braces,
        });
    }
    out
}

// ── the native arm — the honest baseline ─────────────────────────────────

/// The same brace-escape predicate the committed wasm module encodes,
/// compiled natively into the healer. Zero boundary cost: no ABI write, no
/// reset, no interpreter. This is what shipping the predicate natively
/// costs per verdict — the number the wasm lane must justify itself against.
fn useless_format_brace_native(span: &[u8]) -> Outcome {
    let mut braces: u32 = 0;
    for &b in span {
        if b == b'{' || b == b'}' {
            braces += 1;
        }
    }
    Outcome {
        verdict: if braces > 0 { 1 } else { 0 },
        score: braces,
    }
}

// ── percentiles with disclosed tail support ──────────────────────────────

/// Nearest-rank percentile + tail support (observations the rank actually
/// rests on — a p99 with support 1 is the max wearing a percentile's name).
fn pct(sorted: &[u128], p: usize) -> (u128, usize) {
    let n = sorted.len();
    let idx = (p * n).div_ceil(100); // ceil(p·n), 1-based rank
    let idx = idx.clamp(1, n);
    (sorted[idx - 1], n - idx + 1)
}

// ── the timing harness ───────────────────────────────────────────────────

struct ArmTiming {
    /// Best-of-N wall for the whole 10⁴-span set (the headline; per-verdict
    /// mean = this / SPAN_COUNT, no per-span timer overhead anywhere near it).
    best_total: std::time::Duration,
    /// Per-span ns estimates: chunk-wall / CHUNK over one instrumented pass.
    chunk_ns: Vec<u128>,
    sink_verdicts: u64,
    sink_braces: u64,
}

fn time_arm<F>(spans: &[SpanFixture], mut verdict: F) -> ArmTiming
where
    F: FnMut(&[u8]) -> Outcome,
{
    let mut sink_verdicts: u64 = 0;

    let mut sink_braces: u64 = 0;

    // Every pass counts into the sinks — the correctness coupling is the
    // loud-zero defense, so it rides the measured passes themselves, not a
    // separate counting pass. (The adds are identical across arms.)
    let mut run_set = || {
        for s in spans {
            let o = verdict(&s.text);

            sink_verdicts += u64::from(o.verdict as u32);

            sink_braces += u64::from(o.score);
        }
    };

    run_set(); // warmup + correctness-coupled pass

    let mut totals = Vec::with_capacity(MEASURED_PASSES);

    for _ in 0..MEASURED_PASSES {
        let t0 = Instant::now();

        run_set();

        totals.push(t0.elapsed());
    }

    totals.sort();

    let mut chunks: Vec<u128> = Vec::with_capacity(SPAN_COUNT / CHUNK + 1);
    for chunk in spans.chunks(CHUNK) {
        let t0 = Instant::now();

        for s in chunk {
            let o = verdict(&s.text);

            sink_verdicts += u64::from(o.verdict as u32);

            sink_braces += u64::from(o.score);
        }

        chunks.push(t0.elapsed().as_nanos() / CHUNK as u128);
    }

    chunks.sort();

    ArmTiming {
        best_total: totals[0],
        chunk_ns: chunks,
        sink_verdicts,
        sink_braces,
    }
}

// ── provenance (never skipped) ───────────────────────────────────────────

fn preflight() -> (String, String) {
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../riir-reflex/scripts/bench_preflight.sh");
    if !script.exists() {
        return (
            "UNAVAILABLE".to_string(),
            format!(
                "preflight script absent at {} — provenance never skipped",
                script.display()
            ),
        );
    }
    let out = match Command::new("sh")
        .arg(&script)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return (
                "UNAVAILABLE".to_string(),
                format!("preflight spawn failed: {e}"),
            )
        }
    };
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let prov = text
        .lines()
        .find(|l| l.starts_with("PROVENANCE:"))
        .unwrap_or("")
        .to_string();
    match out.status.code() {
        Some(0) => ("PASSED".to_string(), prov),
        Some(1) => {
            let reason = text
                .lines()
                .rev()
                .find(|l| l.starts_with("REFUSE"))
                .unwrap_or("preflight refused (no reason line parsed)")
                .to_string();
            ("REFUSED".to_string(), reason)
        }
        _ => (
            "UNAVAILABLE".to_string(),
            format!(
                "preflight exit {:?} — the instrument could not read the box",
                out.status.code()
            ),
        ),
    }
}

// ── G-safety: the planted-violation family ───────────────────────────────

struct SafetyCase {
    name: &'static str,
    bytes: Vec<u8>,
    want_verdict: i32,
    want_braces: u32,
}

fn safety_family() -> Vec<SafetyCase> {
    let mut v: Vec<SafetyCase> = Vec::new();
    let mut case = |name: &'static str, text: &str, braces: u32| {
        v.push(SafetyCase {
            name,
            bytes: text.as_bytes().to_vec(),
            want_verdict: if braces > 0 { 1 } else { 0 },
            want_braces: braces,
        });
    };
    case("empty", "", 0);
    case("brace_at_byte_0", "{name} prefix", 2);
    case("brace_at_last_byte", "abc{", 1);
    case("escaped_open_pair", "a {{ b", 2);
    case("escaped_close_pair", "a }} b", 2);
    case("anonymous_positional", "fmt {}", 2);
    case("unicode_adjacent", "héllo {name}", 2);
    case("cjk_emoji_adjacent", "措辞 🦀 {flag}", 2);
    case(
        "lookalikes_only",
        "[vec](tuple)<angle>|pipe:colon;comma,stop.",
        0,
    );
    case("utf8_continuation_bytes_only", "\u{80}\u{bf}\u{7f}tail", 0);
    case("brace_heavy", &"{k}".repeat(512), 1024);
    // The closed form over the whole byte space: exactly 0x7B and 0x7D are
    // braces — a UTF-8 continuation byte (0x80-0xBF) must never read as one.
    v.push(SafetyCase {
        name: "all_256_byte_values",
        bytes: (0u8..=255).collect(),
        want_verdict: 1,
        want_braces: 2,
    });
    // Capacity boundary: exactly the ABI input region is legal, one byte
    // over is a named trap (a refusal, not a verdict).
    let cap = MEM_LEN - ABI_INPUT_ADDR as usize;
    let mut at_cap_clean = vec![b'a'; cap];
    at_cap_clean[cap - 1] = b'z';
    v.push(SafetyCase {
        name: "capacity_exact_clean",
        bytes: at_cap_clean,
        want_verdict: 0,
        want_braces: 0,
    });
    let mut at_cap_violating = vec![b'a'; cap];
    at_cap_violating[cap - 1] = b'{';
    v.push(SafetyCase {
        name: "capacity_exact_violating",
        bytes: at_cap_violating,
        want_verdict: 1,
        want_braces: 1,
    });
    // A long span with ONE brace buried mid-way (the realistic shape).
    let mut long = Vec::with_capacity(4096);
    long.extend_from_slice(b"format!(\"");
    while long.len() < 2000 {
        long.extend_from_slice(b"literal words ");
    }
    long.extend_from_slice(b"{x}");
    while long.len() < 4090 {
        long.extend_from_slice(b" tail words ");
    }
    long.push(b'"');
    long.push(b')');
    v.push(SafetyCase {
        name: "one_brace_buried_in_4k",
        bytes: long,
        want_verdict: 1,
        want_braces: 2,
    });
    v
}

// ── the clippy re-run baseline (env-gated) ───────────────────────────────

const CLIPPY_PROBE_FNS: usize = 64;

/// Spawns the real oracle — `cargo clippy --message-format=json --quiet`
/// over a pid-scoped zero-dep scratch crate carrying CLIPPY_PROBE_FNS
/// useless_format findings — and reports the per-invocation wall cost plus
/// the finding count the oracle actually emits (the baseline's HONESTY
/// check: a clippy that finds nothing is not a baseline).
fn clippy_baseline() -> Result<(std::time::Duration, usize), String> {
    let dir = std::env::temp_dir().join(format!(
        "wasm_validator_bench_clippy_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let r = (|| {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"wvb_clippy_probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[lib]\npath = \"lib.rs\"\n",
        )
        .map_err(|e| e.to_string())?;
        let mut lib = String::with_capacity(CLIPPY_PROBE_FNS * 96);
        for i in 0..CLIPPY_PROBE_FNS {
            lib.push_str(&format!(
                "pub fn f{i}() -> String {{ format!(\"static text only\") }}\n"
            ));
        }
        std::fs::write(dir.join("lib.rs"), lib).map_err(|e| e.to_string())?;

        let t0 = Instant::now();
        let out = Command::new("cargo")
            .args(["clippy", "--message-format=json", "--quiet"])
            .current_dir(&dir)
            .env("CARGO_TERM_COLOR", "never")
            .output()
            .map_err(|e| e.to_string())?;
        let wall = t0.elapsed();
        if !out.status.success() {
            return Err(format!(
                "clippy exited {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);

        // One diagnostic per JSON line; the code field names the lint once
        // per finding — substring counting over the whole stream would
        // overcount (the name also rides message text and children).
        let findings = stdout
            .lines()
            .filter(|l| l.contains("clippy::useless_format"))
            .count();
        Ok((wall, findings))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    r
}

// ── main ─────────────────────────────────────────────────────────────────

fn main() {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut say = |line: &str| {
        let _ = writeln!(out, "{line}");
    };

    say("== bench_004 — Plan 199 T3.4: wasm validator three-arm gate ==");
    say(&format!(
        "artifact: {VALIDATOR_NAME} via the committed store (blake3-verified load path)"
    ));

    // The production read path: manifest row → blake3 verify → admission.
    let committed =
        ValidatorStore::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/validators"));
    let admitted = committed
        .load(VALIDATOR_NAME)
        .unwrap_or_else(|e| panic!("committed artifact failed the store load: {e}"));
    say(&format!(
        "store load ok: v{} blake3 {}",
        admitted.record.version,
        &admitted.record.blake3[..12.min(admitted.record.blake3.len())]
    ));

    // ── G-safety (hard floor — any box, any load) ──
    say("== G-SAFETY (hard floor: planted violations caught 100%) ==");
    let module = &*admitted.module;
    let mut vm = Vm::default();
    let family = safety_family();
    let mut planted = 0usize;
    let mut clean = 0usize;
    let mut caught = 0usize;
    let mut passed_clean = 0usize;
    let mut score_ok = 0usize;
    for c in &family {
        let o = module
            .run(&c.bytes, DEFAULT_FUEL, &mut vm)
            .unwrap_or_else(|t| panic!("fixture `{}` trapped: {t:?}", c.name));
        let verdict_ok = o.verdict == c.want_verdict;
        let braces_ok = o.score == c.want_braces;
        if braces_ok {
            score_ok += 1;
        }
        if c.want_verdict == 1 {
            planted += 1;
            if verdict_ok {
                caught += 1;
            } else {
                panic!(
                    "FALSE PASS — fixture `{}` carries {} brace(s) and the validator read it clean",
                    c.name, c.want_braces
                );
            }
        } else {
            clean += 1;
            if verdict_ok {
                passed_clean += 1;
            } else {
                panic!(
                    "FALSE FLAG — fixture `{}` is clean and the validator refused it (score {})",
                    c.name, o.score
                );
            }
        }
    }
    // Determinism: every fixture twice, byte-identical outcomes.
    for c in &family {
        let a = module
            .run(&c.bytes, DEFAULT_FUEL, &mut vm)
            .unwrap_or_else(|t| panic!("determinism rerun `{}` trapped: {t:?}", c.name));
        let b = module
            .run(&c.bytes, DEFAULT_FUEL, &mut vm)
            .unwrap_or_else(|t| panic!("determinism rerun `{}` trapped: {t:?}", c.name));
        assert_eq!(a, b, "fixture `{}` is not deterministic", c.name);
    }
    // The over-capacity refusal is a named trap, not a verdict.
    let over = vec![b'a'; MEM_LEN - ABI_INPUT_ADDR as usize + 1];
    match module.run(&over, DEFAULT_FUEL, &mut vm) {
        Err(Trap::InputTooLarge { .. }) => {}
        other => panic!("over-capacity input must trap InputTooLarge, got {other:?}"),
    }
    say(&format!(
        "planted violations: {caught}/{planted} caught · clean: {passed_clean}/{clean} passed · score cells: {score_ok}/{} exact · determinism: {} fixtures ×2 identical · over-capacity: InputTooLarge ✓",
        family.len(),
        family.len()
    ));
    if caught != planted {
        panic!(
            "G-SAFETY FLOOR FAILED: {}/{planted} — one false pass fails this gate permanently",
            caught
        );
    }
    say("G-SAFETY FLOOR: PASS");

    // ── G-value: the hot-swap cost (install + load), pid-scoped store ──
    say("== G-VALUE (committed-artifact hot-swap quantification) ==");
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/validators/useless_format_brace.wasm"),
    )
    .unwrap_or_else(|e| panic!("committed artifact unreadable: {e}"));
    let root =
        std::env::temp_dir().join(format!("wasm_validator_bench_store_{}", std::process::id()));

    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap_or_else(|e| panic!("temp store root: {e}"));
    let store = ValidatorStore::new(&root);
    let t0 = Instant::now();
    store
        .install(VALIDATOR_NAME, &bytes)
        .unwrap_or_else(|e| panic!("temp-store install failed: {e}"));
    let install = t0.elapsed();
    let t0 = Instant::now();
    store
        .load(VALIDATOR_NAME)
        .unwrap_or_else(|e| panic!("temp-store load failed: {e}"));
    let load = t0.elapsed();
    let t0 = Instant::now();
    match store.install(VALIDATOR_NAME, &bytes) {
        Err(StoreError::AlreadyCurrent) => {}
        other => panic!("byte-identical reinstall must be a no-op, got {other:?}"),
    }
    let noop = t0.elapsed();
    let _ = std::fs::remove_dir_all(&root);
    say(&format!(
        "install (admit+blake3+write): {:?} · load (blake3+admit): {:?} · byte-identical reinstall: {:?}",
        install, load, noop
    ));
    say(&format!(
        "bytes→serving (install+load): {:?} — the native equivalent is a host rebuild (measured separately for the T3.5 verdict)",
        install + load
    ));

    // ── G-cost: the 10⁴-span set, wasm vs native, provenance-gated ──
    say("== G-COST (10⁴-span fixture set) ==");
    let (prov_state, prov_line) = preflight();
    say(&format!("provenance: {prov_state} — {prov_line}"));

    let spans = gen_spans(SPAN_COUNT, SPAN_SEED);
    // Generator truth: the injected brace counts must equal an independent
    // scan, and violating ⇔ braces>0 — a generator bug would poison the mix.
    let mut truth_violating = 0usize;
    let mut truth_verdict_sum = 0u64;
    let mut truth_brace_sum = 0u64;
    for s in &spans {
        let counted = s.text.iter().filter(|&&b| b == b'{' || b == b'}').count();
        assert_eq!(counted, s.braces, "generator truth drifted on its own set");
        if s.violating() {
            truth_violating += 1;
        }
        truth_verdict_sum += u64::from(s.braces > 0);
        truth_brace_sum += s.braces as u64;
    }
    say(&format!(
        "fixture set: {} spans · {truth_violating} violating (seed {SPAN_SEED:#x}) — generator truth verified",
        spans.len()
    ));

    let passes = 1 + MEASURED_PASSES + 1; // warmup + measured + instrumented
    let want_verdicts = truth_verdict_sum * passes as u64;
    let want_braces = truth_brace_sum * passes as u64;

    let mut wasm_vm = Vm::default();
    let module = &*admitted.module;
    let wasm = time_arm(&spans, |bytes| {
        module
            .run(bytes, DEFAULT_FUEL, &mut wasm_vm)
            .unwrap_or_else(|t| panic!("wasm arm trapped: {t:?}"))
    });
    let native = time_arm(&spans, useless_format_brace_native);

    // The loud-zero defense: the sinks MUST equal generator truth × passes —
    // a deleted measured loop cannot guess the sums.
    assert_eq!(
        wasm.sink_verdicts, want_verdicts,
        "wasm arm verdict sink mismatch — the measured loop is not running the set"
    );
    assert_eq!(
        wasm.sink_braces, want_braces,
        "wasm arm score sink mismatch"
    );
    assert_eq!(
        native.sink_verdicts, want_verdicts,
        "native arm verdict sink mismatch"
    );
    assert_eq!(
        native.sink_braces, want_braces,
        "native arm score sink mismatch — wasm and native disagree with generator truth"
    );

    for (name, arm) in [("wasm interpreter", &wasm), ("native predicate", &native)] {
        let (p50, s50) = pct(&arm.chunk_ns, 50);
        let (p99, s99) = pct(&arm.chunk_ns, 99);
        say(&format!(
            "{name}: best-of-{MEASURED_PASSES} total {:?} → {:?}/verdict mean · chunk p50 {:.0} ns (support {s50}) · p99 {:.0} ns (support {s99}) · sink Σverdicts {} Σbraces {} ✓ (×{passes} passes)",
            arm.best_total,
            arm.best_total / SPAN_COUNT as u32,
            p50,
            p99,
            arm.sink_verdicts,
            arm.sink_braces,
        ));
        // The interpreter includes vm.reset per verdict (the production
        // posture: state cleared + input written per call).
        if name == "wasm interpreter" {
            say("  (wasm arm includes vm.reset per verdict — the production per-call posture)");
        }
    }
    let wasm_mean = wasm.best_total.as_nanos() / SPAN_COUNT as u128;
    let native_mean = native.best_total.as_nanos() / SPAN_COUNT as u128;
    if native_mean > 0 {
        say(&format!(
            "boundary cost: wasm/native = {:.1}× — the native baseline is what the lane must justify",
            wasm_mean as f64 / native_mean as f64
        ));
    }

    if prov_state == "PASSED" {
        say("G-COST rows: CITABLE (quote the PROVENANCE line)");
    } else {
        say(&format!(
            "G-COST rows: PROVISIONAL — do not cite (provenance {prov_state})"
        ));
    }

    // ── the clippy re-run baseline (env-gated) ──
    say("== CLIPPY RE-RUN BASELINE (arm 3) ==");
    if std::env::var("WASM_VALIDATOR_BENCH_CLIPPY").is_ok_and(|v| v == "1") {
        match clippy_baseline() {
            Ok((wall, findings)) => {
                say(&format!(
                    "cargo clippy (auto-oracle shape) over a {CLIPPY_PROBE_FNS}-finding scratch crate: {:?} wall · {findings} useless_format findings {}",
                    wall,
                    if findings >= CLIPPY_PROBE_FNS { "✓" } else { "⚠ UNDER-COUNT — not a baseline" },
                ));
                say(&format!(
                    "per-run oracle cost vs the per-span pre-gate: the pre-gate filters at edit time ({wasm_mean} ns/span), the oracle at batch time"
                ));
            }
            Err(e) => say(&format!("clippy baseline SKIPPED loud: {e}")),
        }
    } else {
        say("skipped (set WASM_VALIDATOR_BENCH_CLIPPY=1)");
    }

    say("== done ==");
    if prov_state != "PASSED"
        && std::env::var("WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE").is_ok_and(|v| v == "1")
    {
        say("GATE POSTURE: provenance not PASSED and WASM_VALIDATOR_BENCH_REQUIRE_PROVENANCE=1 — exit 2");
        std::process::exit(2);
    }
}
