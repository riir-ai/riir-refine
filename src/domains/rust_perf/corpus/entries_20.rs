//! Entries slice 20 — Batch 219 (2026-10-10, Plan 215 slot 2 / Research 251):
//! the `TensorFold` native-engine serving/timing pack's prompt-persistence pair.
//!
//! Three `rust_perf` rules distilled from `ashhart/TensorFold` @ `f1aef9c3f2ab85b2a68f2eed817e45eecbd04781`
//! (Apache-2.0): the R1 staging row landed SPLIT — the canonical-interval-keys
//! atom (cache-KEY design) and the rewind-lifecycle atom (`min_gap` spacing,
//! kept-under-min_gap copy-skip reuse, history-first budget fit, pre-pass
//! refusal, last-evicted protection) each carry distinct verbatim law +
//! distinct regime, so they land as two rules that cross-cite in KINSHIP.
//! Quotes re-verified VERBATIM at the pin via git-show blob reads (the tree
//! is not checkout-able on this box: an NTFS-invalid filename; commit
//! subjects + bodies read at the pin too). All `NotBenchable` on this box:
//! decision-content rules whose measured figures are foreign-box serving
//! numbers — provenance, never bars. Upstream note: the tip has moved past
//! the pin (a 6-commit learned-disk-admission series touches both source
//! files) — the pin is the provenance; the series is a future lead, not mined.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching (prompt-cache KEY design): states are keyed by tokens
        // AND canonical decoded intervals — a token prefix alone is half a
        // key when row arithmetic varies at a position ────────────────────
        // Source: zig/src/core/prompt_cache.zig:1+40-54+56-58+137-160+354+408+429,
        // zig/src/core/cache_modes.zig (whole file), CHANGELOG.md:36,
        // commits 71dff68 + cca0fb7 + e936c98.
        PerfRule {
            rule_id: "prompt-cache-canonical-interval-keys".to_string(),
            name: "exact prompt reuse keys a stored state by its token prefix AND the canonical decoded intervals through the cut — ordered, nonempty, strictly separated, compared clipped to consumed rows — because two requests can stand at the same token position with different row arithmetic (sliding window, attention sink, planned chunk rows) and resuming a token-matching but row-divergent state produces rows a full prefill never would".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: a prompt-state cache keyed by the TOKEN PREFIX alone. Two stored
// states can agree on tokens at a cut yet keep different decoded-row
// arithmetic (sliding-window rows, attention-sink rows, planned-chunk row
// sets differ per request), but the lookup sees only tokens: it hands
// back a token-matching state whose rows a full prefill of the same
// prompt would never produce, and the resumed reply drifts.
type Span = (u32, u32); // half-open decoded-row interval [start, end)

struct CachedState {
    tokens: Vec<u32>, // the prompt through the cut
    rows: Vec<Span>,  // the decoded rows the state kept at the cut
    bits: u64,        // the state itself (stand-in)
}

struct NaiveCache {
    entries: Vec<CachedState>,
}

impl NaiveCache {
    // longest token prefix wins — the row layout is never consulted
    fn find(&self, prompt: &[u32]) -> Option<&CachedState> {
        self.entries
            .iter()
            .filter(|e| e.tokens.len() < prompt.len() && prompt.starts_with(&e.tokens))
            .max_by_key(|e| e.tokens.len())
    }
}
"
            .to_string(),
            fixed_code: "// OPT: the key is the token prefix AND the canonical decoded intervals
// through the cut. Canonical: ordered, nonempty, strictly separated —
// enforced at INSERT, so an alternate spelling of the same layout can
// never enter the store. Equality is CLIPPED to consumed rows: an
// interval that only differs past the cut cannot invalidate a resume
// at the cut.
type Span = (u32, u32); // half-open decoded-row interval [start, end)

// ordered, nonempty, strictly separated; the empty list is valid
fn validate(spans: &[Span]) -> Result<(), &'static str> {
    let mut previous = 0u32;
    for (i, span) in spans.iter().enumerate() {
        if span.0 >= span.1 || (i > 0 && span.0 <= previous) {
            return Err(\"spans must be ordered, nonempty and strictly separated\");
        }
        previous = span.1;
    }
    Ok(())
}

// equality clipped to consumed rows before `at`
fn spans_equal(a: &[Span], b: &[Span], at: u32) -> bool {
    let mut i = 0;
    loop {
        let left = i < a.len() && a[i].0 < at;
        let right = i < b.len() && b[i].0 < at;
        if !left || !right {
            return left == right;
        }
        if a[i].0 != b[i].0 || a[i].1.min(at) != b[i].1.min(at) {
            return false;
        }
        i += 1;
    }
}

struct CachedState {
    at: u32, // the cut: tokens through it (a resume leaves a row)
    tokens: Vec<u32>,
    rows: Vec<Span>, // canonical: validated at insert
    bits: u64,
}

struct Cache {
    entries: Vec<CachedState>,
}

impl Cache {
    fn insert(&mut self, s: CachedState) -> Result<(), &'static str> {
        validate(&s.rows)?;
        self.entries.push(s);
        Ok(())
    }

    // longest token prefix with matching canonical spans through the cut
    fn find(&self, prompt: &[u32], spans: &[Span]) -> Option<&CachedState> {
        self.entries
            .iter()
            .filter(|e| {
                (e.at as usize) < prompt.len()
                    && prompt.starts_with(&e.tokens)
                    && spans_equal(&e.rows, spans, e.at)
            })
            .max_by_key(|e| e.at)
    }
}

#[test]
fn tokens_alone_are_not_the_key() {
    let prompt = [1u32, 2, 3, 4, 5];
    let request_spans = [(0u32, 2u32)]; // this request keeps window rows 0..2

    let mut cache = Cache { entries: Vec::new() };
    cache
        .insert(CachedState { at: 4, tokens: vec![1, 2, 3, 4], rows: vec![(0, 1), (2, 3)], bits: 0xB })
        .unwrap();
    // token prefix matches, rows differ THROUGH the cut: no hit — the
    // naive cache returned 0xB and served the wrong row arithmetic
    assert!(cache.find(&prompt, &request_spans).is_none());

    cache
        .insert(CachedState { at: 4, tokens: vec![1, 2, 3, 4], rows: vec![(0, 2)], bits: 0xA })
        .unwrap();
    let hit = cache.find(&prompt, &request_spans).unwrap();
    assert_eq!(hit.bits, 0xA);

    // a difference only PAST the cut never invalidates the resume
    let later = [(0u32, 2u32), (4u32, 9u32)];
    assert_eq!(cache.find(&prompt, &later).unwrap().bits, 0xA);

    // canonicality at insert: empty, reversed, overlapping, adjacent and
    // unsorted are the same refusal
    assert!(validate(&[]).is_ok());
    assert!(validate(&[(2u32, 2u32)]).is_err());
    assert!(validate(&[(3u32, 2u32)]).is_err());
    assert!(validate(&[(1u32, 4u32), (3u32, 5u32)]).is_err());
    assert!(validate(&[(1u32, 4u32), (4u32, 5u32)]).is_err());
    assert!(validate(&[(7u32, 8u32), (1u32, 3u32)]).is_err());
}
"
            .to_string(),
            context_notes: Some("Grade B+ — NotBenchable decision-content ON THIS BOX (the 1.7-vs-10.9 s figures are SOURCE-REPORTED serving numbers on an M3 Ultra — provenance, never bars; the decision content is the CACHE-KEY law — what makes two stored states the same state — which a Rust microbench of the stand-in cannot measure). SOURCE (TensorFold @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781, Apache-2.0). THE LAW IS THE MODULE HEADER (zig/src/core/prompt_cache.zig:1, verbatim): 'Exact prompt reuse: states are keyed by tokens and canonical decoded intervals through their position.' — tokens are HALF the key: an Entry carries its row arithmetic beside its tokens (prompt_cache.zig:57-58, verbatim): 'tokens: []u32, // the state's tokens and `lookahead` more' + 'decode_spans: []modes.Span = &.{}, // canonical row arithmetic through at, excluding lookahead'. CANONICALITY IS DEFINED AND ENFORCED (zig/src/core/cache_modes.zig:1, verbatim): 'Canonical decoded intervals are ordered, nonempty and strictly separated.' with the type (cache_modes.zig:3-4): '/// A half-open decoded-row interval.' / 'pub const Span = [2]u32;' and the validator (cache_modes.zig:6): 'Reject empty, reversed, overlapping, adjacent or unsorted intervals; the empty list is valid.' — the in-source test names state the two invariants in prose: 'canonical spans reject alternate spellings before storage' (cache_modes.zig:49) and 'canonical equality and owned prefixes ignore future rows and lookahead' (cache_modes.zig:56). THE COMPARISON IS CLIPPED TO CONSUMED ROWS (cache_modes.zig:15-16, verbatim): 'Compare canonical interval lists in linear time, clipped to consumed rows before at.' / 'pub fn equal(a: []const Span, b: []const Span, at: u32) bool' — an interval that starts at or after `at` never invalidates a resume at `at`, and a longer final interval compares through @min(end, at) (cache_modes.zig:22): 'if (a[i][0] != b[i][0] or @min(a[i][1], at) != @min(b[i][1], at)) return false;'. THE LOOKUP JOINS BOTH HALVES (prompt_cache.zig:148-149, verbatim): 'Find a kept token prefix with matching canonical spans through its position, leaving at least one row.' / 'pub fn find(s: *Store, prompt: []const u32, starts: []const u32, spans: []const modes.Span) ?*Entry {' — token equality first (a last-token prefilter then the full prefix, prompt_cache.zig:155): 'if (prompt[n - 1] != e.tokens[n - 1] or !std.mem.eql(u32, prompt[0..n], e.tokens)) continue;' then the span half (prompt_cache.zig:156): 'if (!modes.equal(e.decode_spans, spans, e.at)) continue;'. THE SAME JOIN ON EVERY CONSUMER: keptNear (the rewind-side near hit of sibling rule prompt-cache-rewind-states-min-gap-eviction) requires both too (prompt_cache.zig:141-142): 'if (e.at > at or at - e.at >= s.rules.min_gap or e.tokens.len > prompt.len or !s.usable(e.at, starts)) continue;' + 'if (!std.mem.eql(u32, prompt[0..e.tokens.len], e.tokens) or !modes.equal(e.decode_spans, spans, e.at)) continue;' — reserve's dedup does (prompt_cache.zig:218), forget does (prompt_cache.zig:239), and keep() stores the owned canonical prefix at insert (prompt_cache.zig:354): 'const decoded = modes.prefix(s.gpa, spans, at) catch |err| return s.fail(at, err);' — the owned copy CLIPS future rows (cache_modes.zig:26): 'Copy a canonical map through at, without retaining later rows or borrowed storage.'. WHY TOKENS ALONE FAIL: two requests can stand at the same token position with DIFFERENT kept-row sets — sliding-window rows, attention-sink rows, a planned family's chunk-row arithmetic (Rules.lookahead, prompt_cache.zig:42: 'Tokens past the position a state read (Flash Next's MTP head keys row at-1 with token at: 1).'; Rules.planned, prompt_cache.zig:44: 'Prompt kernels that change arithmetic with a chunk's rows: resume and keep only at the request's chunk starts.') — so a token-equal state can be arithmetically WRONG to resume, and the span map is what makes the resume exact. THE DISK LANE RUNS THE SAME LAW TWICE: learn refuses states whose key would need a span map (prompt_cache.zig:408): 'if (e.decode_spans.len != 0) return; // learned states are prompt arithmetic only: their key holds no span map' — and recall refuses a learned hit when the request DOES decode rows (prompt_cache.zig:429): 'if (!modes.equal(&.{}, spans, m.at)) return have; // the request decodes rows the learned prompt pass prefilled'. EXACTNESS IS TESTED AS EQUALITY, NOT TOLERANCE: the CHANGELOG binds every cache feature to it (CHANGELOG.md:36, verbatim): '...and the reply equals the cache-off reply.'. MEASURED (source-reported, attributed): the cache family this key serves — M3 Ultra, Qwen3.8-27B, 9,000-token chat, deleted/edited/regenerated/re-edited last turns: 1.7 s prompt processing in all vs 10.9 s, and on a GB10 replacing a pasted last message resumed at 8,192 tokens instead of 6,144 (both CHANGELOG.md:36, in-source); the behavioral-identity probe for the disk half (learned_prompts.zig:65): 'This build's prompt-pass bits: a fixed prompt's state at a mark, hashed (a kernel or compiler change moves it).' — that probe is Batch 219's sibling rule prompt-state-fingerprint-probe-bits-keyed-persistence. REGIME: serving caches over models whose per-position row arithmetic varies across requests (sliding window, attention sink, planned chunk grids, speculative lookahead); any store that resumes mid-sequence state and claims resumed-equals-fresh bits. ANTI-REGIME: arithmetic that is a pure function of position (dense causal attention, no sink, no window) — the token prefix IS the layout and the span map is overhead; stores that only checkpoint whole requests (no mid-sequence resume, nothing to mis-join). KINSHIP (do not double-count): prefix-cache-entry-one-token-early (entries_13, B200, same source repo at v0.6.0) owns the ENTRY-BOUNDARY axis — where a cache entry ENDS (one token early) so a strict-extension resume can hit at all; THIS owns WHAT MAKES TWO STATES THE SAME STATE (the key's second half: canonical decoded intervals) — a token-prefix hit at the right end is necessary but NOT sufficient; the two compose. zero-anchored-prompt-grid-chunk-identity (kernel_opt entries_gpu_82, Batch 215, same repo at this pin) owns the CUT-PLACEMENT axis (chunk boundaries on the zero-anchored grid make resumed == cache-off bits CONSTRUCTIVE; Rules.grid, prompt_cache.zig:46-47: 'A planned family's own zero-anchored chunk rows, its starts when a request names none (0: none).' is how the cache learns that grid, floored through startAt, prompt_cache.zig:131-134: 'The chunk start at or before `at`: the request's, else the family's grid (0 when none).'); THIS owns the KEY-DESIGN axis — the spans equality that decides whether a stored state may serve at all. concurrent-round-parity-solo-equal (entries_14, B200) owns the CONCURRENCY parity axis (concurrent == solo == serial); THIS owns the RESUME identity axis — the cache-off == resumed half of the same exactness family. prompt-state-fingerprint-probe-bits-keyed-persistence (this batch) extends the same identity law across restarts for the disk lane. NOVEL: grep 'canonical', 'decode-span', 'interval', 'prompt-cache', 'cache-key' over rust_perf + kernel_opt corpora: zero cache-key rules (the hits are prose). STAGED-VS-PIN DRIFTS: (1) the R1 row landed SPLIT (the rewind/eviction atom is the sibling rule) — the staged single-row framing mixed a key-design law with a lifecycle law. (2) staged 'Rules block (:44-62)' — the Rules struct is prompt_cache.zig:40-54 (min_gap doc+field :48-49; :62 sits inside Entry); staged 'lookupRewind (:148-162)' — :148-160 is find(), lookupRewind is :186-210; the body is now read in full (the file is 525 lines at the pin, not '~600'). (3) staged '1.7 s vs 10.9 s (9k chat, 4 edits)' — in-source at CHANGELOG.md:36 with the edits enumerated ('deleted, edited, regenerated and re-edited') plus the GB10 8,192-vs-6,144 figure the staging missed. Cite: @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781 — zig/src/core/prompt_cache.zig:1+40-54+56-58+126-134+137-160+354+408+429; zig/src/core/cache_modes.zig:1-33+49+56; CHANGELOG.md:36; commits 71dff68 + cca0fb7 + e936c98 (messages read at the pin); Batch 219 / Plan 215 (slot 2) / Research 251.".to_string()),
            // Not benchable: the win is a cache-key CORRECTNESS law — the
            // resumed-equals-fresh tests ARE the verification; no Rust A/B.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching (prompt-cache LIFECYCLE): the edit-resume rewind state —
        // min_gap spacing, kept-under-min_gap reuse (no new copy),
        // history-first budget fit with pre-pass refusal, last-evicted
        // while its turn lasts ────────────────────────────────────────────
        // Source: zig/src/core/prompt_cache.zig:48-53+137-146+186-210+212-226
        // +256-291+313-341+343-362+474-503, CHANGELOG.md:36, commits
        // 71dff68 + cca0fb7 + e936c98.
        PerfRule {
            rule_id: "prompt-cache-rewind-states-min-gap-eviction".to_string(),
            name: "edit-and-regenerate serving plans a second state just before the latest user turn under the same spacing law as every mark — min_gap from the resume point and each other, an already-kept state under min_gap before the rewind position IS the rewind state (no new copy), fitted against the budget history-first with refusals counted before the pass — and while its turn lasts it is the last one evicted".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: edit-resume serving without the lifecycle law — a checkpoint
// at EVERY turn boundary (a mark five tokens from the resume point
// copies a near-identical state), no protection at eviction (the
// just-kept checkpoint this turn's edit still needs goes out with the
// ordinary oldest-first sweep), and marks that cannot fit are only
// discovered MID-PASS — after the prompt work is spent.
const BUDGET: u64 = 4096;

struct NaiveStore {
    kept: Vec<(u32, u64)>, // (position, bytes)
    held: u64,
}

impl NaiveStore {
    // every turn boundary becomes a copy, however close together
    fn keep(&mut self, at: u32, bytes: u64) {
        self.kept.push((at, bytes));
        self.held += bytes;
    }

    // plain oldest-first eviction: position order, no liveness
    fn evict_for(&mut self, need: u64) {
        while self.held + need > BUDGET {
            let (_, bytes) = self.kept.remove(0);
            self.held -= bytes;
        }
    }

    // the pass starts, THEN discovers a mark cannot fit
    fn plan_marks(&self, turns: &[u32]) -> Vec<u32> {
        turns.to_vec() // no budget, no spacing
    }
}
"
            .to_string(),
            fixed_code: "// OPT: the edit-resume checkpoint is PLANNED, SPACED, FITTED and
// PROTECTED — all before the pass runs:
// (1) spacing: a mark other than the history's is kept only MIN_GAP
//     from every other mark and the resume point;
// (2) reuse: a state already kept UNDER MIN_GAP before the rewind
//     position IS the rewind state — no new copy;
// (3) refusal: marks whose state does not fit the budget are counted
//     and dropped BEFORE the pass (so no pass, nor a peer, copies
//     them), fitted history-first, then the rewind, then the rest;
// (4) eviction: while its turn lasts the rewind state is the LAST one
//     evicted, and the room for every keep is made before the pass.
const MIN_GAP: u32 = 256;
const BUDGET: u64 = 4096;

#[derive(Clone, Copy)]
struct Entry {
    at: u32,
    bytes: u64,
    used: u64, // the store's clock at its last keep or resume
}

struct Store {
    kept: Vec<Entry>,           // sorted by `at`
    held: u64,
    clock: u64,
    active_rewind: Option<u32>, // this turn's rewind state, while live
}

impl Store {
    fn is_active_rewind(&self, e: &Entry, prompt_len: usize) -> bool {
        self.active_rewind == Some(e.at) && (e.at as usize) <= prompt_len
    }

    // (2): the latest kept state at `at` or under MIN_GAP before it
    fn kept_near(&self, at: u32) -> Option<u32> {
        self.kept
            .iter()
            .filter(|e| e.at <= at && at - e.at < MIN_GAP)
            .map(|e| e.at)
            .max()
    }

    // (3): history first, then the rewind, then the rest in order;
    // anything that does not fit is refused NOW, before the pass
    fn fit(&self, priority: &[(u32, u64)]) -> (Vec<u32>, u64) {
        let mut out = Vec::new();
        let mut used = 0u64;
        let mut refused = 0u64;
        for &(at, bytes) in priority {
            if bytes <= BUDGET.saturating_sub(used) {
                out.push(at);
                used += bytes;
            } else {
                refused += 1; // \"kept nothing at N tokens: ... the budget\"
            }
        }
        (out, refused)
    }

    // (4): least recently used first, the turn's rewind state last
    fn victim(&self, prompt_len: usize) -> Option<usize> {
        let mut best: Option<usize> = None;
        for (i, e) in self.kept.iter().enumerate() {
            if self.is_active_rewind(e, prompt_len) {
                continue; // the live rewind goes last — never, while its turn lasts
            }
            match best {
                Some(b) if self.kept[b].used <= e.used => {}
                _ => best = Some(i),
            }
        }
        best
    }

    fn evict_for(&mut self, need: u64, prompt_len: usize) {
        while self.held + need > BUDGET {
            let Some(i) = self.victim(prompt_len) else { return };
            let e = self.kept.remove(i);
            self.held -= e.bytes;
        }
    }
}

#[test]
fn rewind_is_reused_refused_and_evicted_last() {
    let mut s = Store { kept: Vec::new(), held: 0, clock: 0, active_rewind: None };

    // an earlier turn kept a state 100 rows before this turn's rewind
    // position: that state IS the rewind state — no new copy
    s.clock += 1;
    s.kept.push(Entry { at: 3_000, bytes: 512, used: s.clock });
    s.held = 512;
    let rewind_pos: u32 = 3_100;
    assert_eq!(s.kept_near(rewind_pos), Some(3_000));
    s.active_rewind = s.kept_near(rewind_pos);

    // (1) spacing: a fresh mark 5 rows from the resume point is skipped
    let from: u32 = 3_095;
    assert!((from + 5) - from < MIN_GAP);

    // (3) refusal before the pass: history (2,048 B) fits, the rewind
    // (512 B) fits, the next mark does not — refused NOW, before any
    // pass runs, and nothing was copied
    let priority = [(4_096u32, 2_048u64), (3_000u32, 512u64), (3_500u32, 2_048u64)];
    let (plan, refused) = s.fit(&priority);
    assert_eq!(plan, vec![4_096, 3_000]);
    assert_eq!(refused, 1);

    // (4) eviction protects the live rewind: the LRU non-rewind state
    // goes first; the rewind goes only when its turn ends
    s.clock += 2;
    s.kept.push(Entry { at: 4_096, bytes: 2_048, used: s.clock });
    s.held += 2_048;
    s.clock += 1;
    s.kept.push(Entry { at: 3_500, bytes: 2_048, used: s.clock });
    s.held += 2_048;
    let victim = s.victim(9_000).map(|i| s.kept[i].at);
    assert_eq!(victim, Some(4_096)); // LRU non-rewind first (rewind protected)
    s.active_rewind = None; // the turn ended
    let victim = s.victim(9_000).map(|i| s.kept[i].at);
    assert_eq!(victim, Some(3_000)); // plain LRU again
}
"
            .to_string(),
            context_notes: Some("Grade B+ — NotBenchable decision-content ON THIS BOX (the 1.7-vs-10.9 s CHANGELOG figure and the commit's ~60 ms are SOURCE-REPORTED serving numbers — provenance, never bars; the decision content is a CHECKPOINT-LIFECYCLE law: which states a serving pass keeps, when a copy is skipped, what is refused, and what evicts last). SOURCE (TensorFold @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781, Apache-2.0). THE FEATURE IN ONE SENTENCE (CHANGELOG.md:36, verbatim): 'Editing, deleting or regenerating a conversation's last turn resumes from a prompt state kept just before the latest user message, on every native engine with a prompt cache, and the reply equals the cache-off reply.' THE SPACING LAW (prompt_cache.zig:48-53, verbatim): '/// A mark other than the history's is kept only this far from every other mark and the resume point.' / 'min_gap: u32 = 256,' + '/// Shorter prompts keep nothing: below it a mark's extra prompt call costs more than a later turn's reuse saves.' / 'min_prompt: u32 = 4096,' + '/// Replies are prefilled in background passes kept at their end: a pass resumed near its history keeps no mark.' / 'warm: bool = false,' — marksRewind enforces it (prompt_cache.zig:313, verbatim): '`marks` with the active rewind second: kept however near the other marks, never at or before the resume point.' with the two skips in-source (prompt_cache.zig:330): 'if (!is_rewind and at - from < s.rules.min_gap and (k > 0 or (from > 0 and s.rules.warm))) continue; // near the resume point' and (prompt_cache.zig:331-334) the near-another-mark skip: 'if (k > 0 and !is_rewind) { // the history's mark always (warm families: away from the resume point); the others away from it and each other' + 'if (@max(o, at) - @min(o, at) < s.rules.min_gap) break true;'. THE REWIND STATE IS PLANNED, NOT DISCOVERED (prompt_cache.zig:186, verbatim): '`lookup` plus a rewind state on the starts or grid; a state kept under min_gap before it serves instead.' — gated (prompt_cache.zig:193): 's.active_rewind_at = if (prompt.len >= s.rules.min_prompt and rewind_bytes > 0 and rewind != endpoint and rewind_bytes <= s.budget - priority_bytes) rewind else 0;' — and the COPY-SKIP IS THE POINT (prompt_cache.zig:194-195, verbatim): 'if (s.active_rewind_at > 0) if (s.keptNear(prompt, starts, spans, s.active_rewind_at)) |at| {' / 's.active_rewind_at = at; // a state kept under min_gap before it is the rewind state: no new copy' with keptNear's doc (prompt_cache.zig:137): 'The latest kept state this prompt can resume at `at` or under min_gap before it (null: none).'. The commit carries the law and its number (e936c98 message, verbatim): 'When a state this prompt can resume is kept less than min_gap before the latest user message, that state is the turn's rewind state: it goes last when room is made and serves an edit, and no pass stops a few tokens later to copy another. The turn, its reply's keep and its warm pass all find the same state. The 27B resumes most turns from its reply state four tokens before the boundary, so it copied a whole state each turn; on an M3 Ultra that added about 60 ms to a resumed turn against main. Further from any kept state the rewind state is kept as before.'. BUDGET FIT BEFORE THE PASS (prompt_cache.zig:256, verbatim): 'The marks whose state the budget can hold (a new slice in `a`); the rest are refused now, so no pass, nor a peer, copies them.' with the refusal counted (prompt_cache.zig:263-264): 's.counts.refused += 1;' + 'note(\"kept nothing at {d} tokens: {d} MiB passes the {d} MiB budget\", .{ at, bytes >> 20, s.budget >> 20 });' and the priority order (prompt_cache.zig:270, verbatim): 'Fitted marks that the budget holds together: the history's first, then the rewind, then the rest in order.' (endpoint appended first :275-278, the rewind second :279-280, the remainder in order under 'bytes <= s.budget -| used' :281-288). EVICTION PROTECTS THE LIVE TURN (prompt_cache.zig:493-497, verbatim): 'Eviction order past the superseded states: least recently used first, the turn's rewind state last.' / 'return if (e_rewind != o_rewind) o_rewind else e.used < o.used;' with the liveness test (prompt_cache.zig:500-502): 'The last lookup's rewind state, while the prompt still extends it.' + 'return e.at == s.active_rewind_at and e.tokens.len <= prompt.len and std.mem.eql(u32, e.tokens, prompt[0..e.tokens.len]);' and victimBut's doc (prompt_cache.zig:474): 'The entry to free first, never `skip`: a superseded one, oldest first; else the oldest, the turn's rewind last.' — the CHANGELOG's own phrasing (CHANGELOG.md:36): 'The state is kept only when it fits beside the history's, and while its turn lasts it is the last one evicted.'. EVICTION IS PRE-ANNOUNCED (prompt_cache.zig:212, verbatim): 'Room for every state the pass keeps, made before it: a peer that mirrors the keeps hears of these evictions with the request.'. GRID AWARENESS (cca0fb7 message, verbatim): 'The rewind and the history mark floor through startAt, so a family with its own grid (CUDA Nemotron) keeps its rewind on the 2,048-row grid. It kept none before.' plus the same commit's robustness rows: 'keep() unwrapped a null victim when the turn's rewind state was the only one left, as when the serial host keeps a reply's state after decode. The rewind state now goes last instead.' and 'Marks past the budget are counted refused again, and a request whose rewind is not kept plans its marks as before.'. MEASURED (source-reported, attributed): M3 Ultra, Qwen3.8-27B, 9,000-token chat, deleted/edited/regenerated/re-edited last turns: 1.7 s prompt processing in all vs 10.9 s, and GB10 replacing a pasted last message resumed at 8,192 tokens instead of 6,144 (both CHANGELOG.md:36, in-source); the ~60 ms/turn over-copy the keptNear reuse removed (e936c98 message, commit-only). REGIME: chat serving where the LAST TURN is edited, deleted or regenerated (any stateful prefix cache over growing conversations); background/warm passes that resume near the history end; byte-budgeted state stores sized in GiB. ANTI-REGIME: single-shot prompts (min_prompt keeps nothing below 4,096 — the rewind state is dead weight); a workload that never revises a turn (the history mark alone is right); stores without an eviction policy a pass can pre-announce (the reserve half assumes a mirroring peer); memory-unbounded caches (nothing to fit, nothing to refuse). KINSHIP (do not double-count): prompt-cache-canonical-interval-keys (this batch, split sibling) owns the KEY-DESIGN half — keptNear's near-hit is a KEYS-equal hit first (tokens + canonical spans) before it is a distance hit; the reuse is exact BECAUSE the key is exact; the two cross-cite and compose. prefix-cache-entry-one-token-early (entries_13, B200) owns the entry-END axis (a resume must strictly extend, so entries end one token early) — the rewind state is the OTHER boundary (a SECOND state before the latest user turn), not a moved end. append-cache-rewind-overwrite-in-place (kernel_opt entries_gpu_05, riir-gpu source) owns the POINTER-ROLLBACK mechanism for re-execution (rewind_to overwrites appended cache slots); THIS owns the SERVING-LIFECYCLE policy (which checkpoint exists, when it is copied, what it evicts last) — different plane, compose. zero-anchored-prompt-grid-chunk-identity (kernel_opt entries_gpu_82, Batch 215) owns the grid the rewind floors through (Rules.grid) — cut placement, not lifecycle. bounded-batch-buffer-pool + partial-reset-watermark-pool (entries_06) are retention-CAP rules (bounded pool sizes); THIS is an eviction-ORDER + refusal-TIMING law over a byte-budgeted store. resume-revalidates-provenance-before-reuse (entries_05, B121) is build-artifact resume identity — a different resume. dataflow-fixpoint-convergence-traversal-order (entries_16, B202) matches the word 'rewind' on a worklist cursor — unrelated. NOVEL: grep 'rewind', 'min-gap', 'min_gap', 'evict-last', 'checkpoint' over rust_perf + kernel_opt: no serving-lifecycle rule (the 'rewind' hits are the worklist cursor and the GPU pointer-rollback/KV-ring families; 'min-gap' zero hits). STAGED-VS-PIN DRIFTS: none material beyond the split itself — every staged atom verified verbatim at the lines above; staged '1.7 s vs 10.9 s (9k chat, 4 edits)' is in-source (CHANGELOG.md:36, edits enumerated). Cite: @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781 — zig/src/core/prompt_cache.zig:48-53+137-146+186-210+212-226+256-291+313-341+343-362+474-503; CHANGELOG.md:36; commits 71dff68 (subject) + cca0fb7 (body) + e936c98 (body), read at the pin; Batch 219 / Plan 215 (slot 2) / Research 251.".to_string()),
            // Not benchable: the win is a checkpoint-lifecycle decision — the
            // reuse/refusal/eviction-order tests ARE the verification.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching (persistence identity): learned prompt states on disk
        // keyed by a startup fingerprint — metadata PLUS this build's
        // behavioral bits on a fixed prompt; drift = unreachable, never
        // guessed ─────────────────────────────────────────────────────────
        // Source: zig/src/native/metal.zig:161-234 (learnedStates +
        // shardStamps + the --learn call site), zig/src/families/nemotron/
        // learned_prompts.zig (whole file), zig/src/core/prompt_imprint.zig,
        // zig/src/families/nemotron/kernels.zig:86-93,
        // docs/recipes/nemotron-3.5.md:110-137, CHANGELOG.md:26,
        // commit c6e050a6.
        PerfRule {
            rule_id: "prompt-state-fingerprint-probe-bits-keyed-persistence".to_string(),
            name: "learned prompt states persist on disk under an identity fingerprint of the whole serving build — config, tensor index, shard size/inode/mtimes, prefill step, head, every kernel source hash plus the prebuilt library, a startup probe's output bits on a fixed prompt, OS build and chip — so a drifted term lands in a different identity directory and stale states are unreachable by construction, never read and never guessed".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: learned prompt states persisted under a FIXED path — or
// keyed by the checkpoint name alone. Since they were written, the
// kernel sources were recompiled, the OS was updated, a shard was
// rewritten in place — but the server cannot see any of it: it reads
// the old states and serves rows produced by arithmetic this build no
// longer runs.
fn learned_dir(cache_root: &str, _checkpoint: &str) -> String {
    format!(\"{cache_root}/learned\") // one directory for every build ever run here
}

fn read_state(dir: &str, key: u64) -> Option<Vec<u8>> {
    std::fs::read(format!(\"{dir}/{key:016x}.bin\")).ok() // presence = trust
}
"
            .to_string(),
            fixed_code: "// OPT: the identity is a FINGERPRINT over every term the state's
// bits depend on — config, tensor index, shard identity (size, inode,
// mtimes), prefill step, head, every kernel source + the prebuilt
// library, the PROBE BITS (behavioral identity: this build's output
// bytes for a fixed prompt's state at a mark, hashed at startup),
// OS build and chip — and the identity IS the directory name. A
// drifted term lands in a different directory: stale states are
// UNREACHABLE, never guessed at; a file whose own header disagrees
// is refused too.
struct Fnv(u64);

impl Fnv {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x1_0000_0001_b3);
        }
    }

    fn update_u64(&mut self, v: u64) {
        self.update(&v.to_le_bytes());
    }

    fn finalize(self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
struct ShardStamp {
    name: String,
    size: u64,
    inode: u64,
    mtime_ns: i64,
    ctime_ns: i64,
}

#[derive(Clone)]
struct Build {
    config_json: Vec<u8>,
    tensor_index: Vec<u8>,
    shards: Vec<ShardStamp>,
    prefill_step: u64,
    head: bool,
    kernel_source_hash: u64, // every kernel source + the prebuilt library
    probe_bits: u64,         // a fixed prompt's state bytes, hashed
    os_build: String,
    chip: String,
}

impl Build {
    fn identity(&self) -> u64 {
        let mut h = Fnv::new(0x6e65);
        h.update(&self.config_json);
        h.update(&self.tensor_index);
        for s in &self.shards {
            h.update(s.name.as_bytes());
            h.update_u64(s.size);
            h.update_u64(s.inode);
            h.update(&s.mtime_ns.to_le_bytes());
            h.update(&s.ctime_ns.to_le_bytes());
        }
        h.update_u64(self.prefill_step);
        h.update(&[u8::from(self.head)]);
        h.update_u64(self.kernel_source_hash);
        h.update_u64(self.probe_bits);
        h.update(self.os_build.as_bytes());
        h.update(self.chip.as_bytes());
        h.finalize()
    }

    fn learned_dir(&self, root: &str) -> String {
        format!(\"{root}/{:016x}\", self.identity())
    }
}

#[test]
fn every_term_moves_the_identity() {
    let base = Build {
        config_json: b\"hidden=4096\".to_vec(),
        tensor_index: b\"weight_map=1\".to_vec(),
        shards: vec![ShardStamp {
            name: \"model.safetensors\".into(),
            size: 1 << 30,
            inode: 424_242,
            mtime_ns: 1_000,
            ctime_ns: 1_000,
        }],
        prefill_step: 4_096,
        head: true,
        kernel_source_hash: 0xabc,
        probe_bits: 0xdef,
        os_build: \"27.0.1\".into(),
        chip: \"M3 Ultra\".into(),
    };
    let id0 = base.identity();

    // metadata terms: any one moving is a different identity
    let mut b = base.clone();
    b.prefill_step = 2_048;
    assert_ne!(b.identity(), id0);
    let mut b = base.clone();
    b.shards[0].mtime_ns += 1; // a rewrite in place moves the identity
    assert_ne!(b.identity(), id0);
    let mut b = base.clone();
    b.kernel_source_hash ^= 1;
    assert_ne!(b.identity(), id0);
    let mut b = base.clone();
    b.os_build = \"27.1\".into();
    assert_ne!(b.identity(), id0);

    // the sharp one: every METADATA field identical, BEHAVIOR different —
    // the probe's output bits still split the identity
    let mut b = base.clone();
    b.probe_bits ^= 1;
    assert_ne!(b.identity(), id0);
    assert_ne!(b.learned_dir(\"/cache\"), base.learned_dir(\"/cache\"));
}
"
            .to_string(),
            context_notes: Some("Grade B — NotBenchable decision-content ON THIS BOX (the 0.27-vs-1.30 s and 213 MB are SOURCE-REPORTED M5 Ultra figures — provenance, never bars; the decision content is a PERSISTENCE-IDENTITY law: what may key on-disk derived state across process restarts). SOURCE (TensorFold @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781, Apache-2.0). THE LAW IS THE FUNCTION DOC (zig/src/native/metal.zig:197, verbatim): 'Learned states under `root` by checkpoint, shards, prefill step, head, kernels, probe bits, OS build and chip.' — the fingerprint IS the directory name (zig/src/core/prompt_imprint.zig:15, verbatim): 'dir: [:0]u8, // root/<identity>: the index, the last-open stamp and each state's files (the family's)' with open() (prompt_imprint.zig:21): 'The states learned under `identity` in `root` (made when missing); other identities past `cap` go, oldest first.'. THE TERMS, in-source: config + tensor index hashed whole (native/metal.zig:200-202, verbatim): 'for ([_][]const u8{ \"config.json\", \"model.safetensors.index.json\" }) |name| {' + 'h.update(try std.Io.Dir.cwd().readFileAlloc(io, path, a, .limited(1 << 26)));'; the MTP head's safetensors header when present (native/metal.zig:205): 'if (b.head != null) h.update(try safetensorsHeader(a, dir, \"mtp-4bit.safetensors\"));'; shard IDENTITY, never contents — name, size, inode, mtime AND ctime (shardStamps, native/metal.zig:216, verbatim): 'Each safetensors file's name, size, inode and change times: a weight written in place (--slide) moves the identity.' with the stamp (native/metal.zig:230): 'const stamp = [4]i128{ info.size, info.inode, info.mtime.nanoseconds, info.ctime.nanoseconds };'; then one u64 batch (native/metal.zig:207, verbatim): 'for ([_]u64{ b.o.chunk, @intFromBool(b.head != null), nemotron.kernels.sourceHash(), probe }) |x| h.update(std.mem.asBytes(&x));' — prefill step, head flag, the kernel-source hash (zig/src/families/nemotron/kernels.zig:86, verbatim): 'Every source the pipelines compile from and the prebuilt library, hashed: a learned prompt state's identity.' (the prebuilt-library term is sources.packed_metallib, kernels.zig:90), and the probe bits (native/metal.zig:206): 'const probe = try nemotron.learned_prompts.probe(b);'; the OS build (native/metal.zig:209, fed by prompt_imprint.zig:302, verbatim): 'This Mac's OS build (sysctl kern.osversion): its Metal compiler can change a kernel's bits.' and the chip (native/metal.zig:210): 'h.update(std.mem.span(b.m.device.name()));'. THE SHARP TERM IS BEHAVIORAL, NOT METADATA (zig/src/families/nemotron/learned_prompts.zig:65, verbatim): 'This build's prompt-pass bits: a fixed prompt's state at a mark, hashed (a kernel or compiler change moves it).' — the probe runs ONE fixed prompt through THIS build's kernels at startup and hashes the STATE BYTES (learned_prompts.zig:67): 'var toks: [4096 + 40 + 7]u32 = undefined; // a whole chunk, a middle one, then a short one on the decode kernels' — 4,143 tokens; mark 4136 (learned_prompts.zig:86): 'const marks = [_]u32{4136};'; chunks { 4096, 4136 } (learned_prompts.zig:87); the hash (learned_prompts.zig:95): 'return std.hash.Wyhash.hash(0x4e45, snap.buf.contents()[0..snap.bytes]);' — metadata CAN lie (every input hash equal, the compiled pipeline still different: a toolchain change the inputs cannot see); the probe's OUTPUT BITS cannot. ~1 s cost (docs/recipes/nemotron-3.5.md:133-134, verbatim): 'The probe runs a fixed 4,143-token prompt at startup, about a second.'. DRIFT IS REFUSAL BY UNREACHABILITY, NEVER A GUESS: a moved term means a different identity means a different root/<identity> directory, so zero states are found and the server learns fresh — it never reads the old ones; a file whose header disagrees with the read is refused too (learned_prompts.zig:35, verbatim): '`file`'s state into `snap`, whose buffer holds its bytes; a file of another position, layout or size is refused.' (learned_prompts.zig:43): 'if (head[0] != FILE_MAGIC or head[1] != snap.at or head[2] != @intFromBool(snap.head) or bytes != snap.bytes) return error.SnapshotRead;'; and a learned hit is refused when the request decodes rows the learned pass prefilled (prompt_cache.zig:429): 'if (!modes.equal(&.{}, spans, m.at)) return have; // the request decodes rows the learned prompt pass prefilled'. LIVE WEIGHTS STOP READS UNTIL RESTART (docs/recipes/nemotron-3.5.md:132-133, verbatim): 'Weights that `--slide` writes into the shards start a new identity, and a live weight change stops the server reading or writing learned states until it restarts.' — wired in-source: the weight-change path clears the store and drops the imprint (prompt_cache.zig:229-233, verbatim): 'Every kept state goes after a weight change, and learned states on disk stop being read: old arithmetic.' + 's.imprint = null;' at the call site (native/metal.zig:165): 'h.host.cache = &h.cache.?; // the lane host's store: a Sliding Weights step that moves weights clears it'. THE WRITE SIDE IS ATOMIC (learned_prompts.zig:18, verbatim): '`snap` to `file` (its position, layout and size, then its bytes), through a temporary file renamed into place.'. MEASURED (source-reported, attributed): fresh server vs resume on a 9.2k-token system prompt — 0.27 s vs 1.30 s, 9,213 tokens resumed from disk, every reply equal to the cache-off reply, and 'The two learned states took 213 MB on disk.' — IN-SOURCE at docs/recipes/nemotron-3.5.md:135-137 (verbatim): 'with the 9.2k-token system prompt above, a new server's first turn took 0.27 s instead of 1.30 s, resuming 9,213 tokens from disk, and every reply equalled the cache-off reply. The two learned states took 213 MB on disk.' — also commit-message (c6e050a6): 'On an M5 Ultra a fresh server resumed a new conversation on a 9.2k-token system prompt in 0.27 s instead of 1.30 s, reading 9,213 tokens from disk, with every reply equal to the cache-off reply.'; box state (docs/recipes/nemotron-3.5.md:117-118): 'Measured on an M5 Ultra (256 GB), macOS 27.0.1, Zig 0.17.0, checkpoint revision `d9d758fb`'. The CHANGELOG entry #499 (CHANGELOG.md:26) carries NO figures — figure-free one-liner: '--learn' keeps Nemotron's shared prompt prefixes, such as a system prompt and its tools, on disk, so a new server resumes them without reading them again. Weights that --slide write into the model's files start a new set (#499).'. REGIME: on-disk DERIVED state (learned prefixes, compiled-pipeline caches, tuned overlays) whose bits depend on the whole serving build; shared dev boxes where several builds alternate against one cache root. ANTI-REGIME: state that is already CONTENT-addressed (the stored bytes carry their own identity — key by that hash instead); builds whose only variation is content-addressable input (a source hash suffices and the probe is a second of startup you need not pay); single-tenant frozen deployments (one build, one identity — the fingerprint degenerates to a constant). KINSHIP (do not double-count): resume-revalidates-provenance-before-reuse (entries_05, B121) revalidates an identity TUPLE at resume time and refuses on mismatch — THIS computes the identity UP FRONT and makes stale state UNREACHABLE (a different directory) rather than discovered-and-refused; they compose (the fingerprint is the identity tuple that rule wants recorded). write-verify-readback-before-publish (entries_05, B121) owns the publish-side integrity gate (independent read-back + atomic rename) — the tmp-then-rename write here is that shape, but this rule's atom is the IDENTITY, not the publish protocol. The corpus's other 'fingerprint' (entries_08, FingerprintGate) is a hash-TABLE membership gate (SIMD layout axis) — a name collision, not kinship. zero-anchored-prompt-grid-chunk-identity (kernel_opt entries_gpu_82, Batch 215, same repo + pin) makes resumed == cache-off bits constructive WITHIN a process; THIS extends the identity question ACROSS restarts (which states may survive one). NOVEL: grep 'fingerprint', 'identity', 'probe', 'stale' over rust_perf + kernel_opt: no on-disk-derived-state keying rule (entries_08's fingerprint is a SIMD layout; entries_05's identity is revalidated at resume, never used to KEY storage; 'probe' hits are measurement/hotpath probes, not behavioral identity). STAGED-VS-PIN DRIFTS: (1) staged cite 'metal.zig:170-175' — the file is zig/src/native/metal.zig (families/nemotron/metal.zig does NOT exist at the pin); :170-175 IS the --learn call site and :172 carries the 4,143-token startup error: '--learn keys its states with a 4,143-token prompt at startup: serve with --context 4096 or more, or without --learn'. (2) staged 'Find where 0.27-vs-1.30 s and 213 MB live (CHANGELOG #499? commit message? source comment)' — ANSWERED: in-source recipe (docs/recipes/nemotron-3.5.md:135-137) + commit message (c6e050a6); NOT CHANGELOG #499 (figure-free). (3) staged 'lane-B body NEVER read' — now read in full (learned_prompts.zig is 136 lines; the fingerprint lives in native/metal.zig:197-234 + prompt_imprint.zig + kernels.zig:86-93, all read this session). (4) the identity list's canonical in-source order is 'checkpoint, shards, prefill step, head, kernels, probe bits, OS build and chip' (native/metal.zig:197) — the staging row's order is faithful. Cite: @ f1aef9c3f2ab85b2a68f2eed817e45eecbd04781 — zig/src/native/metal.zig:161-234; zig/src/families/nemotron/learned_prompts.zig:1-136; zig/src/core/prompt_imprint.zig:1-33+302-313; zig/src/families/nemotron/kernels.zig:86-93; zig/src/core/prompt_cache.zig:229-235+429; docs/recipes/nemotron-3.5.md:110-137; CHANGELOG.md:26; commit c6e050a6 (message, read at the pin); Batch 219 / Plan 215 (slot 2) / Research 251. RIDER (Batch 249 slice 3 / Plan 253 — ashhart/TensorFold @ `807c6d42`, read at origin/main `9ccf4727`; the REPRODUCIBILITY face — the inverse hazard): the host law keys states on probe bits so a drifted identity strands states unreachable-by-construction; this commit adds the WITHIN-BUILD variant — an identity term that varies inside one build (the probe state's last-window row count, set by whatever ran first; cuda_disk.zig:143, verbatim): '// the row count: the last window's, from whatever ran first (a new build measures its windows)' — makes the identity NON-REPRODUCIBLE (commit-message, verbatim): 'A new build's first server measures its draft windows before the probe, so its states named an identity no later server computed, and a restart did not read them.' Law extension: every probe-bits term is a function of the build plus FIXED probe inputs only — run-order-dependent slices are zeroed before the hash ('@memset(bytes[8..words], 0);'); companion `4af04851` adds the cost face — the probe runs at every startup, so it must also be minimal. Count-neutral rider, Batch 249 slice 3.".to_string()),
            // Not benchable: the win is a persistence-identity decision — the
            // any-term-drifts test IS the verification; the latency figures
            // are foreign-box serving numbers (provenance, never bars).
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
