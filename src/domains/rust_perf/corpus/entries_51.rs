//! Entries slice — Batch 288 (2026-10-26): ONE `rust_perf` rule from the
//! upstream llama.cpp grammar distill (Research 327; ggml-org/llama.cpp
//! #31153 @ `48c9f08b7b25766324282b7605177e2f2be755bb` "common, grammar:
//! add recursive depth limit, make it a requirement", MIT, merged
//! 2026-10-25; every quote re-verified verbatim at the fetched patch this
//! batch — plan 293 T0, the B96-98 precedent).
//!
//! - `recursive-input-walk-explicit-stack-indices-not-copies` (complexity)
//!   — a recursion depth limit must count the RECURSING axis (stack
//!   frames), never a per-input quantity that travels with the data; and
//!   a walk that recurses once per input position must not recurse at
//!   all: carry an explicit stack of levels over candidate INDICES.
//!
//! ANNOTATED `NotBenchable` `bench_spec: None` — the win is stack-safety +
//! length-generality + sampling latency inside a serving grammar engine;
//! the faster-than-master claim is source-reported provenance and a
//! faithful microbench needs a full grammar matcher (the Batch-136
//! decision-content class). Fixtures are ORIGINAL machinery (never the
//! llama.cpp source pasted — the rejection walk is re-authored as a
//! plain-Rust stand-in with neutral names), each compiled and executed
//! standalone (`rustc --edition 2021 --test`) before this table was
//! assembled; the buggy/fixed arm pair shares the surrounding harness
//! shape and differs only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── complexity: the rejection walk recurses per input position,
        // and its depth limit counts the input, not the recursion ──────
        PerfRule {
            rule_id: "recursive-input-walk-explicit-stack-indices-not-copies"
                .to_string(),
            name: "when a recursive rejection/validation walk over input positions carries a recursion depth limit, the limit must count the RECURSING axis (stack frames), never a per-input quantity that travels with the data — a limit fed candidate/token length silently rejects every input past the bound (any vocab token of 256+ code points was ALWAYS rejected under a grammar); and a walk that can recurse once per input position must not recurse at all: carry an explicit stack of levels (each level = the rule arms still in play at one code point + the candidate indices it rejects), pop when a level's arms exhaust or its survivors empty, splicing child rejects into the parent — no limit, no stack risk, whatever the input length — and carry candidate INDICES into the input through the levels instead of copies: at a given position a candidate differs from its input only by its position pointer, so indices make the rejects map back to the input directly and the first level needs no special case"
                .to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `recursive-input-walk-explicit-stack-indices-not-copies`
// (riir-refine rust_perf corpus, Batch 288) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's grammar rejection
// (llama-grammar.cpp's reject_candidates_for_stack recursion is cited in
// context_notes): a rule-driven matcher rejects candidate tokens whose
// code points the rule stack cannot consume. The recursion carries a
// depth limit intended to guard the CALL STACK -- but the value it tests
// is the CODE-POINT POSITION of the walk, which is the candidate's own
// length: any candidate of 256+ code points is ALWAYS rejected (the
// guard silently became an input-length cutoff), and legitimate deep
// inputs ride real recursion the whole way.

const RECURSION_LIMIT: usize = 256;

#[derive(Clone)]
struct Candidate {
    id: u32,
    code_points: Vec<u32>, // the token's code points, 0-terminated
}

fn rule_allows(rules: &[fn(u32) -> bool], pos: usize, cp: u32) -> bool {
    pos < rules.len() && cp != 0 && rules[pos](cp)
}

fn reject_walk(
    rules: &[fn(u32) -> bool],
    cand: &Candidate,
    depth: usize,
    rejects: &mut Vec<u32>,
) {
    // THE BUG: `depth` is the code-point POSITION, so the "recursion
    // limit" counts the candidate's own length. A long token (the
    // 512-space token class) hits the limit and is rejected ALWAYS.
    if depth >= RECURSION_LIMIT {
        rejects.push(cand.id);
        return;
    }
    let cp = cand.code_points[depth];
    if cp == 0 {
        if depth < rules.len() {
            rejects.push(cand.id); // ended before the required prefix
        }
        return; // consumed the whole token through every rule: accepted
    }
    if !rule_allows(rules, depth, cp) {
        rejects.push(cand.id);
        return;
    }
    reject_walk(rules, cand, depth + 1, rejects); // the real recursion
}

pub fn reject_candidates(rules: &[fn(u32) -> bool], cands: &[Candidate]) -> Vec<u32> {
    let mut rejects = Vec::new();
    for c in cands {
        reject_walk(rules, c, 0, &mut rejects);
    }
    rejects
}

#[test]
fn the_limit_rejects_a_long_token_silently() {
    fn spaces(cp: u32) -> bool {
        cp == b' ' as u32
    }
    let rules: Vec<fn(u32) -> bool> = std::iter::repeat(spaces as fn(u32) -> bool)
        .take(300)
        .collect();
    let long_token = Candidate {
        id: 7,
        code_points: std::iter::repeat(b' ' as u32)
            .take(300)
            .chain([0])
            .collect(),
    };
    // the matcher wanted to ACCEPT it (300 spaces through 300 space
    // rules) -- the misapplied limit rejected it instead, silently.
    assert_eq!(reject_candidates(&rules, &[long_token]), vec![7]);
}
"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `recursive-input-walk-explicit-stack-indices-not-copies`
// (riir-refine rust_perf corpus, Batch 288) -- the FIXED arm.
//
// The rejection walks the code points with its OWN STACK of levels (the
// source's `level { stacks, i, cur, rejects }` shape, simplified to one
// rule arm per position): each level owns one code-point position,
// carries the INDICES of the candidates still alive, and when a deeper
// level pops it splices its rejects into the parent. The "stack" lives
// in a heap Vec, not the call stack -- no recursion, no depth limit, no
// stack risk, whatever the token length; the rejects map back to the
// input by index directly.

struct Candidate {
    id: u32,
    code_points: Vec<u32>, // 0-terminated
}

struct Level {
    pos: usize,            // the code point this level checks
    survivors: Vec<usize>, // candidate INDICES this level is walking
    rejects: Vec<usize>,   // indices this level rejected
}

/// rules[p] must allow code_points[p] for every position p of the
/// required prefix; a candidate that survives every level AND ends on
/// its terminator exactly there is accepted.
pub fn reject_candidates(rules: &[fn(u32) -> bool], cands: &[Candidate]) -> Vec<u32> {
    let mut levels = vec![Level {
        pos: 0,
        survivors: (0..cands.len()).collect(),
        rejects: Vec::new(),
    }];
    loop {
        let top = levels.len() - 1;
        if levels[top].pos >= rules.len() || levels[top].survivors.is_empty() {
            if levels.len() == 1 {
                break;
            }
            // pop: the deeper walk finished. Its REJECTS join the
            // parent's ledger; its survivors are the candidates the
            // deeper levels accepted -- each survives only if it ends on
            // its terminator exactly at this level's position.
            let done = levels.pop().unwrap();
            let parent = levels.last_mut().unwrap();
            parent.rejects.extend(done.rejects.iter().copied());
            for &i in &done.survivors {
                if cands[i].code_points.get(done.pos).copied().unwrap_or(1) != 0 {
                    parent.rejects.push(i); // leftover input past the prefix
                }
            }
            continue;
        }
        let pos = levels[top].pos;
        let rule = rules[pos];
        let walking = std::mem::take(&mut levels[top].survivors);
        let mut next = Level { pos: pos + 1, survivors: Vec::new(), rejects: Vec::new() };
        for i in walking {
            let cp = cands[i].code_points[pos];
            if cp != 0 && rule(cp) {
                next.survivors.push(i);
            } else {
                levels[top].rejects.push(i);
            }
        }
        levels.push(next);
    }
    // each candidate was rejected exactly once, at the level where its
    // walk died; the base level's emptied survivor set is the accepted
    // set, already accounted for.
    let mut rejected: Vec<u32> = levels
        .iter()
        .flat_map(|lv| lv.rejects.iter().copied())
        .map(|i| cands[i].id)
        .collect();
    rejected.sort_unstable();
    rejected
}

#[test]
fn the_stack_walk_accepts_what_the_recursion_rejected() {
    fn spaces(cp: u32) -> bool {
        cp == b' ' as u32
    }
    let rules: Vec<fn(u32) -> bool> = std::iter::repeat(spaces as fn(u32) -> bool)
        .take(300)
        .collect();
    let long_token = Candidate {
        id: 7,
        code_points: std::iter::repeat(b' ' as u32)
            .take(300)
            .chain([0])
            .collect(),
    };
    let short_wrong = Candidate {
        id: 9,
        code_points: vec![b' ' as u32, b'x' as u32, 0],
    };
    let rejects = reject_candidates(&rules, &[long_token, short_wrong]);
    // the 300-space token ACCEPTS now (no limit); the wrong token rejects.
    assert_eq!(rejects, vec![9]);
}
"#.to_string(),
            context_notes: Some(
                "SOURCE (ggml-org/llama.cpp @ 48c9f08b7 #31153 'common, grammar: add recursive depth limit, make it a requirement', MIT, merged 2026-10-25T23:53+0100, Xuan-Son Nguyen; every quote verbatim at the fetched patch this batch): the commit message states the defect -- 'The depth limit in llama_grammar_reject_candidates_for_stack counted the code points of a vocab token, so any token of 256 chars or more was always rejected under a grammar, such as the 512-space token of GLM-4.7-Flash or the long space tokens of Laguna-S-2.1.' -- and the repair -- 'The rejection now walks the code points with its own stack of levels instead of recursing: there is no limit and no stack risk, whatever the length of a token from the GGUF, and the masks match master.' The INDICES half, verbatim: 'At a given code point a candidate only differs from its input by the position of its code point pointer, so the levels now carry indices into the input candidates instead of copies. The first level needs no special case and the rejects map back to the input directly.' In-code shape: the level struct '{ stacks, i, cur, rejects }' over 'std::vector<level> levels', seeded with 'std::iota(all.begin(), all.end(), 0u)' (ALL candidates as indices), the loop popping when 'lv.i == lv.stacks.size() || lv.cur.empty()' and splicing 'parent.rejects ... parent.cur = std::move(parent.rejects)'; reject_step threads 'levels.size() - 1' as the CODE-POINT POSITION -- the old depth variable became the input position, never a recursion counter. MEASURED (commit message, source-reported -- provenance, never bars): 'Grammar sampling gets faster than master on GLM-4.7, Laguna, Qwen3.6 and gemma-4 with the json, list and spaces grammars, same masks.' THE AXIS LAW IS THE CORRECTNESS CONTENT: a limit that guards the CALL STACK must count RECURSION DEPTH, never a per-input quantity that merely travels with the data -- the misapplied limit converted a stack-overflow guard into a silent input-length REJECTION (a correctness hole wearing a perf guard's clothing; real model vocabs hit it). The same commit's correctly-guarded kin (depth_guard for schema walks, COMMON_RECURSIVE_LIMIT for glob_match) shows the guard axis is fine WHEN it counts recursion -- this rule fires when it does not, or when the recursion is unbounded per input position. DISTINCT-FROM: grammar-window-draft-prune-path-local-mask (entries_11, Batch 182 -- that prunes a verify TREE by grammar admissibility BEFORE a forward, row-locality exactness; THIS restructures the rejection walk itself, one layer down); dead-result-timing-loop-folded-under-lto (measurement -- compiler deletion of a timed body, unrelated); revalidation-skipped-by-certifying-identity-token (caching -- skip-on-unchanged, unrelated). NotBenchable ON THIS BOX (the win is stack-safety + length-generality + sampling latency inside a serving grammar engine; a faithful microbench needs a full grammar matcher and masks). Grade B+. Batch 288 / Plan 293 / Research 327."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
