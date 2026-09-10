#!/usr/bin/env python3
"""Split rust_perf/corpus.rs into a kernel_opt-style corpus/ folder (verbatim).

Mirrors the 2026-08-15 kernel_opt split (its slice headers say: "Pure data
moved verbatim from the former corpus.rs monolith"): entries move verbatim
into numbered slice files, the module doc + builder live in corpus/mod.rs,
tests move to corpus/tests.rs. Entry order is preserved EXACTLY — the corpus
is BLAKE3-committed via Corpus::new and order is part of the commitment.

Why a script: the file interleaves real Rust with col-0 raw-string bench
fixtures (r#"..."#), so naive line/regex splitting is wrong — a `},` or a
`PerfRule {` inside a fixture string must not terminate an entry. This
scanner is quote-aware (line/block comments, normal + raw + byte strings,
char literals vs lifetimes) and tracks brace depth the way rustc tokenizes.

Byte guarantees (asserted, both modes):
  1. every top-level item classifies (builder / *_bench_spec helper / tests)
     and the item spans tile the file exactly — nothing silently dropped;
  2. entry spans tile the builder's vec![...] region exactly (head blank,
     tail comments-only, re-emitted in the last shard);
  3. entry count matches --expect-entries (126 as of Batch 143).

Usage:
  scripts/split_rust_perf_corpus.py --self-test        # fixture round-trip, no repo files
  scripts/split_rust_perf_corpus.py                    # dry run: parse + report
  scripts/split_rust_perf_corpus.py --write            # write corpus/ + rm corpus.rs
  scripts/split_rust_perf_corpus.py --expect-entries 128  # after a landed batch
"""

from __future__ import annotations

import argparse
import bisect
import re
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_SRC = REPO / "src/domains/rust_perf/corpus.rs"
DEFAULT_OUT = REPO / "src/domains/rust_perf/corpus"

CHAR_LIT = re.compile(r"'(\\u\{[0-9a-fA-F_]+\}|\\.|[^'\\])'")
RAW_OPEN = re.compile(r"(?:br|r)(#*)\"")
HELPER_CALL = re.compile(r"([a-z0-9_]+_bench_spec)\s*\(")
HELPER_DEF = re.compile(r"\bfn\s+([a-z0-9_]+_bench_spec)\s*\(")
PERF_RULE = re.compile(r"PerfRule\s*\{")
SHARD_LINES_HARD_CAP = 3200  # repo file-size rule; packing target stays at --shard-lines


def _skip(src: str, i: int) -> int | None:
    """If src[i] starts a comment/string/char-literal, return its end index; else None."""
    c = src[i]
    if c == "/" and src[i + 1:i + 2] == "/":
        j = src.find("\n", i)
        return len(src) if j < 0 else j + 1
    if c == "/" and src[i + 1:i + 2] == "*":
        j, level = i + 2, 1
        while j < len(src) and level:
            if src.startswith("/*", j):
                level, j = level + 1, j + 2
            elif src.startswith("*/", j):
                level, j = level - 1, j + 2
            else:
                j += 1
        return j
    if c in "rb":
        m = RAW_OPEN.match(src, i)
        if m:
            closer = '"' + m.group(1)
            j = src.find(closer, m.end())
            if j < 0:
                raise SyntaxError(f"unterminated raw string at byte {i}")
            return j + len(closer)
    if c == '"':
        j = i + 1
        while j < len(src):
            if src[j] == "\\":
                j += 2
                continue
            if src[j] == '"':
                break
            j += 1
        if j >= len(src):
            raise SyntaxError(f"unterminated string at byte {i}")
        return j + 1
    if c == "'":
        m = CHAR_LIT.match(src, i)
        return m.end() if m else None
    return None


def scan(src: str) -> list[tuple[int, int]]:
    """Brace events as (pos, delta). Quote/comment/string-aware.

    Skips line/block comments, normal + raw + byte strings, char literals,
    and `::{...}` use-path groups (whose braces are not structural). Treats
    an unmatched `'` as a lifetime tick. Raises on unbalanced braces.
    """
    i, n, depth = 0, len(src), 0
    events: list[tuple[int, int]] = []
    while i < n:
        end = _skip(src, i)
        if end is not None:
            i = end
            continue
        c = src[i]
        if c == "{":
            if i >= 2 and src[i - 1] == ":" and src[i - 2] == ":":
                # use-path group (use a::{B, C};) — consume whole group, no events
                local, i = 1, i + 1
                while i < n and local:
                    end2 = _skip(src, i)
                    if end2 is not None:
                        i = end2
                        continue
                    if src[i] == "{":
                        local += 1
                    elif src[i] == "}":
                        local -= 1
                    i += 1
                if local:
                    raise SyntaxError(f"unterminated use-path group at byte {i}")
                continue
            depth += 1
            events.append((i, 1))
        elif c == "}":
            events.append((i, -1))
            depth -= 1
        i += 1
    if depth != 0:
        raise SyntaxError(f"unbalanced braces: depth {depth} at EOF")
    return events


def top_level_items(src: str, events: list[tuple[int, int]]) -> list[tuple[int, int, int]]:
    """[(head_start, open_brace_pos, end_excl)] for every top-level {..} item.

    head_start = end of the previous item, so headers (comments, attrs,
    `use` lines, doc comments) ride with the item that follows them.
    """
    items: list[tuple[int, int, int]] = []
    prev_end = 0
    depth = 0
    open_pos = None
    for pos, delta in events:
        depth += delta
        if delta == 1:
            if depth == 1:
                open_pos = pos
        else:
            if depth == 0 and open_pos is not None:
                items.append((prev_end, open_pos, pos + 1))
                prev_end = pos + 1
                open_pos = None
    trailing = src[prev_end:]
    if trailing.strip():
        raise SystemExit(
            f"FAIL: unclassified trailing bytes after last item: {trailing[:120]!r}"
        )
    return items


def comment_block_start(src: str, pos: int) -> int:
    """Start of the contiguous blank-line + // comment block ending just before pos."""
    lines = src[:pos].split("\n")
    k = len(lines) - 1
    while k >= 0 and (lines[k].strip() == "" or lines[k].strip().startswith("//")):
        k -= 1
    return sum(len(l) + 1 for l in lines[: k + 1]) if k >= 0 else 0


def split(src_path: Path, out_dir: Path, expect_entries: int, shard_lines: int,
          write: bool, label: str) -> dict:
    """Parse src_path and (optionally) write the corpus/ folder. Returns a report."""
    src = src_path.read_text()
    n = len(src)
    events = scan(src)
    positions = [p for p, _ in events]

    def depth_before(pos: int) -> int:
        k = bisect.bisect_left(positions, pos)
        d = 0
        for _, delta in events[:k]:
            d += delta
        return d

    # ── top-level items ────────────────────────────────────────────────────
    builder = tests = None
    helpers: list[dict] = []
    for head_start, brace, end in top_level_items(src, events):
        header = src[head_start:brace]
        if "fn build_perf_seed_corpus" in header:
            builder = (head_start, brace, end)
        elif (m := HELPER_DEF.search(header)) and "#[cfg(test" not in header:
            helpers.append({"head_start": head_start, "brace": brace, "end": end,
                            "name": m.group(1)})
        elif re.search(r"^#\s*\[\s*cfg\s*\(\s*test\s*\)", header, re.M) and "mod tests" in header:
            tests = (head_start, brace, end)
        else:
            raise SystemExit(
                f"FAIL: unclassified top-level item at byte {head_start}: {header[-200:]!r}"
            )
    if builder is None or tests is None:
        raise SystemExit("FAIL: builder or tests module not found")
    b_head, b_brace, b_end = builder
    t_head, t_brace, t_end = tests

    # ── entries inside the builder's vec![...] ─────────────────────────────
    vec_open = src.index("vec![", b_brace, b_end) + len("vec![")
    vec_close = src.rindex("])", b_brace, b_end)

    hits = [(m.end() - 1, m.start(), depth_before(m.end() - 1))  # '{' pos, text pos, depth
            for m in PERF_RULE.finditer(src, vec_open, vec_close)]
    if not hits:
        raise SystemExit("FAIL: no PerfRule entries found in vec![...]")
    body_depth = hits[0][2]
    entries_at = [(bp, tp) for bp, tp, d in hits if d == body_depth]
    skipped = len(hits) - len(entries_at)
    if skipped:
        print(f"note: {skipped} 'PerfRule {{' occurrence(s) at depth != {body_depth} "
              f"ignored (string/field decoys — expected)")

    entries: list[tuple[int, int]] = []
    for bp, tp in entries_at:
        k0 = bisect.bisect_left(positions, bp)
        if k0 >= len(events) or events[k0] != (bp, 1):
            raise SystemExit(f"FAIL: no opening-brace event at PerfRule '{{' pos {bp}")
        d, close_pos = body_depth + 1, None  # depth just after the entry's '{'
        k = k0
        while k + 1 < len(events):
            k += 1
            d += events[k][1]
            if d == body_depth:
                close_pos = events[k][0]
                break
        if close_pos is None:
            raise SystemExit(f"FAIL: entry at {bp} never closes")
        eend = close_pos + 1
        if eend < len(src) and src[eend] == ",":
            eend += 1
        cstart = max(comment_block_start(src, tp), entries[-1][1] if entries else vec_open)
        entries.append((cstart, eend))

    if len(entries) != expect_entries:
        print(f"FAIL: found {len(entries)} entries, expected {expect_entries} "
              f"(pass --expect-entries if a batch just landed)")
        raise SystemExit(1)

    head = src[vec_open:entries[0][0]]
    if head.strip():
        raise SystemExit(f"FAIL: unaccounted bytes between vec![ and first entry: {head[:120]!r}")
    tail_in_vec = src[entries[-1][1]:vec_close]
    for line in tail_in_vec.split("\n"):
        s = line.strip()
        if s and not s.startswith("//"):
            raise SystemExit(f"FAIL: non-comment bytes between last entry and ]): {s[:80]!r}")
    if entries[-1][1] > vec_close or entries[0][0] < vec_open:
        raise SystemExit("FAIL: entry spans escape the vec![...] region")

    # ── entry → helper mapping ─────────────────────────────────────────────
    helper_names = {h["name"] for h in helpers}
    entry_refs = [set(HELPER_CALL.findall(src[a:b])) for a, b in entries]
    dangling = (set().union(*entry_refs) - helper_names) if entries else set()
    if dangling:
        raise SystemExit(f"FAIL: entries reference unknown helpers: {sorted(dangling)}")
    unused = helper_names - set().union(*entry_refs)
    if unused:
        raise SystemExit(f"FAIL: helpers never referenced by any entry: {sorted(unused)}")
    helper_gated = {h["name"]: "rust_perf_bench" in src[h["head_start"]:h["brace"]]
                    for h in helpers}

    # ── tests: production-helper references ────────────────────────────────
    tests_text = src[t_head:t_end]
    tests_defined = set(HELPER_DEF.findall(tests_text))
    tests_ext = (set(HELPER_CALL.findall(tests_text)) - tests_defined) & helper_names

    # ── shard packing (line-balanced, order-preserving, helper-aware) ──────
    def span_lines(a: int, b: int) -> int:
        return src[a:b].count("\n") + 1

    helper_lines = {h["name"]: span_lines(h["head_start"], h["end"]) for h in helpers}
    entry_cost: list[int] = []
    assigned: set[str] = set()
    for k, (a, b) in enumerate(entries):
        cost = span_lines(a, b)
        for name in entry_refs[k]:
            if name not in assigned:  # helper lands with its FIRST referencing entry
                cost += helper_lines[name]
                assigned.add(name)
        entry_cost.append(cost)

    shards: list[list[int]] = []
    cur: list[int] = []
    cur_lines = 0
    for k, cost in enumerate(entry_cost):
        if cur and cur_lines + cost > shard_lines:
            shards.append(cur)
            cur, cur_lines = [], 0
        cur.append(k)
        cur_lines += cost
    if cur:
        shards.append(cur)

    shard_of_entry = {k: si for si, ks in enumerate(shards) for k in ks}
    helper_shard = {}
    for name in helper_names:
        refs = [k for k, refs in enumerate(entry_refs) if name in refs]
        helper_shard[name] = min(shard_of_entry[k] for k in refs)
    shared = {name for name in helper_names
              if len({shard_of_entry[k] for k, refs in enumerate(entry_refs)
                      if name in refs}) > 1}
    pub_names = shared | tests_ext

    # ── report ─────────────────────────────────────────────────────────────
    print(f"[{label}] source: {src_path} ({n} bytes, {src.count(chr(10)) + 1} lines)")
    print(f"[{label}] items: builder + {len(helpers)} bench-spec helpers + tests "
          f"({span_lines(t_head, t_end)} lines) | entries: {len(entries)}")
    print(f"[{label}] shards: {len(shards)} (target ~{shard_lines} lines incl. helpers, "
          f"hard cap {SHARD_LINES_HARD_CAP})")
    shard_ids = []
    for si, ks in enumerate(shards, 1):
        lines = sum(entry_cost[k] for k in ks)
        first = re.search(r'rule_id:\s*"([^"]+)"', src[entries[ks[0]][0]:entries[ks[0]][1]])
        last = re.search(r'rule_id:\s*"([^"]+)"', src[entries[ks[-1]][0]:entries[ks[-1]][1]])
        print(f"[{label}]   entries_{si:02d}.rs: {len(ks)} rules, ~{lines} lines  "
              f"({first.group(1)} .. {last.group(1)})")
        shard_ids.append((first.group(1), last.group(1)))
        if lines > SHARD_LINES_HARD_CAP:
            raise SystemExit(
                f"FAIL: entries_{si:02d}.rs would be ~{lines} lines > {SHARD_LINES_HARD_CAP} "
                f"cap — lower --shard-lines")
    for nm in sorted(shared):
        print(f"[{label}]   shared helper (pub(super)): {nm} in shard {helper_shard[nm] + 1:02d}")
    if tests_ext:
        print(f"[{label}] tests reference production helpers: {sorted(tests_ext)}")

    if not write:
        return {"entries": len(entries), "helpers": len(helpers), "shards": len(shards),
                "shard_ids": shard_ids, "files": {}}

    # ── coverage: the three groups must exactly tile the classified items ──
    # (top_level_items already asserts items are contiguous with a ws-only
    # tail; this catches overlap/gap between the groups themselves. Helpers
    # MAY legitimately sit after the tests module — later batches appended
    # them there — so no ordering assumption is made.)
    last_end = max([b_end] + [h["end"] for h in helpers] + [t_end])
    accounted = (b_end - b_head) + sum(h["end"] - h["head_start"] for h in helpers) \
        + (t_end - t_head)
    if accounted != last_end:
        raise SystemExit(
            f"FAIL: group spans ({accounted}) != item tile ({last_end}) — overlap or gap")

    # ── generate ───────────────────────────────────────────────────────────
    GATE = '#[cfg(feature = "rust_perf_bench")]'
    BENCH_USE = "use crate::domains::rust_perf::bench::BenchSpec;"
    files: dict[str, str] = {}

    shard_uses: dict[int, list[str]] = {}
    for si, ks in enumerate(shards):
        local_refs = set().union(*[entry_refs[k] for k in ks]) if ks else set()
        uses = []
        for ref in sorted(local_refs):
            tgt = helper_shard[ref]
            if tgt != si:
                g = f"{GATE}\n" if helper_gated[ref] else ""
                uses.append(f"{g}use super::entries_{tgt + 1:02d}::{ref};")
        shard_uses[si] = uses

    for si, ks in enumerate(shards):
        my_helpers = [h for h in helpers if helper_shard[h["name"]] == si]
        parts = [
            f"//! Entries slice {si + 1:02d} of {len(shards):02d} — rules "
            f"{ks[0] + 1}–{ks[-1] + 1} of {len(entries)} — moved VERBATIM from the\n"
            f"//! former corpus.rs monolith (2026-09-10 split, scripted by\n"
            f"//! scripts/split_rust_perf_corpus.py). Append new batches to the\n"
            f"//! LAST slice or open a new one past ~{shard_lines} lines.\n"
            "\n"
            "use crate::domains::rust_perf::PerfRule;\n"
        ]
        if my_helpers:
            parts.append(f"\n{GATE}\n{BENCH_USE}\n")
        for u in shard_uses[si]:
            parts.append("\n" + u + "\n")
        parts.append(
            "\n/// Data table, not logic — grows with distillation batches.\n"
            "#[allow(clippy::too_many_lines)]\n"
            "pub(super) fn entries() -> Vec<PerfRule> {\n    vec![\n"
        )
        for k in ks:
            parts.append(src[entries[k][0]:entries[k][1]] + "\n")
        if si == len(shards) - 1:
            parts.append(tail_in_vec)
        parts.append("    ]\n}\n")
        for h in my_helpers:
            htext = src[h["head_start"]:h["end"]]
            # the monolith carried ONE gated `use ...bench::BenchSpec;` that rode
            # into the first helper's header — drop it (the shard-level generated
            # use at the top of the file already covers the whole module)
            htext = re.sub(
                r'#\[cfg\(feature = "rust_perf_bench"\)\]\s*\n'
                r'use crate::domains::rust_perf::bench::BenchSpec;\s*\n',
                '', htext, count=1)
            if h["name"] in pub_names:
                htext = re.sub(r"\bfn\s+" + h["name"] + r"\s*\(",
                               "pub(super) fn " + h["name"] + "(", htext, count=1)
            parts.append("\n" + htext)
        files[f"entries_{si + 1:02d}.rs"] = "".join(parts)

    # mod.rs — b_head carries doc + use lines + builder doc/attrs/sig verbatim
    bhead = re.sub(r"#\[allow\(clippy::too_many_lines\)\][^\n]*\n", "",
                   src[b_head:b_brace])
    # PerfRule is consumed by the slice files + tests.rs directly, not by mod.rs
    # (a plain import here is unused in non-test builds)
    bhead = bhead.replace(
        "use crate::domains::rust_perf::{PerfRule, RustPerfDomain};",
        "use crate::domains::rust_perf::RustPerfDomain;")
    use_iter = list(re.finditer(r"(?m)^use [^\n]*;\n", bhead))
    ins = use_iter[-1].end() if use_iter else 0
    mods_block = ("\n" + "\n".join(f"mod entries_{si + 1:02d};" for si in range(len(shards)))
                  + "\n\n#[cfg(test)]\nmod tests;\n")
    body = ["    let mut entries = entries_01::entries();"]
    body += [f"    entries.extend(entries_{si + 1:02d}::entries());"
             for si in range(1, len(shards))]
    files["mod.rs"] = (bhead[:ins] + mods_block + bhead[ins:]
                       + "{\n" + "\n".join(body) + "\n    Corpus::new(entries)\n}\n")

    # tests.rs — body verbatim + gated use lines for production helpers
    pre = []
    for ref in sorted(tests_ext):
        g = f"{GATE}\n" if helper_gated[ref] else ""
        pre.append(f"{g}use super::entries_{helper_shard[ref] + 1:02d}::{ref};\n")
    pre.append("use crate::domains::rust_perf::PerfRule;\n")
    files["tests.rs"] = "".join(pre) + src[t_brace + 1:t_end - 1].lstrip("\n")

    out_dir.mkdir(parents=True, exist_ok=True)
    for name, content in files.items():
        (out_dir / name).write_text(content)
        scan(content)  # every generated file must itself scan clean
    src_path.unlink()
    print(f"[{label}] wrote {out_dir}/ (mod.rs + {len(shards)} slices + tests.rs), "
          f"removed {src_path}")
    return {"entries": len(entries), "helpers": len(helpers), "shards": len(shards),
            "shard_ids": shard_ids, "files": files}


# ─────────────────────────── self-test fixture ───────────────────────────────
FIXTURE = r'''//! Seed corpus fixture — mentions build_perf_seed_corpus() in prose only.
//! Decoy prose: PerfRule { and fn fake_bench_spec( and #[cfg(test)] mod tests
//! all appear in this doc comment and must NOT classify anything.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

#[must_use]
#[allow(clippy::too_many_lines)] // data table
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    Corpus::new(vec![
        // ── entry one — references alpha (shared across shards) ──
        PerfRule {
            rule_id: "alpha-user".to_string(),
            name: "A".to_string(),
            category: "complexity".to_string(),
            buggy_code: "decoy }; } and PerfRule { inside a normal string".to_string(),
            fixed_code: "x".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(alpha_bench_spec()),
        },
        // ── entry two — beta only ──
        PerfRule {
            rule_id: "beta-user".to_string(),
            name: "B".to_string(),
            category: "allocation".to_string(),
            buggy_code: "y".to_string(),
            fixed_code: "z".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(beta_bench_spec()),
        },
        // ── entry three — alpha again (different shard) ──
        PerfRule {
            rule_id: "alpha-user-2".to_string(),
            name: "C".to_string(),
            category: "io".to_string(),
            buggy_code: "w".to_string(),
            fixed_code: "v".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(alpha_bench_spec()),
        },
        // ── entry four — gamma with col-0 raw-string fixture content ──
        PerfRule {
            rule_id: "gamma-user".to_string(),
            name: "D".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "u".to_string(),
            fixed_code: "t".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(gamma_bench_spec()),
        },
    ])
}

use crate::domains::rust_perf::bench::BenchSpec;

// alpha is referenced by entries in two different shards → pub(super)
#[cfg(feature = "rust_perf_bench")]
fn alpha_bench_spec() -> BenchSpec {
    BenchSpec {
        name: "alpha",
        buggy_source: r#"
use std::collections::HashMap;
pub fn make_input(n: usize) -> Vec<u32> { vec![n] } }, )
mod buggy { }
"#.to_string(),
        fixed_source: r#"mod fixed { }"#.to_string(),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn beta_bench_spec() -> BenchSpec {
    BenchSpec { name: "beta", buggy_source: String::new(), fixed_source: String::new() }
}

#[cfg(feature = "rust_perf_bench")]
fn gamma_bench_spec() -> BenchSpec {
    BenchSpec { name: "gamma", buggy_source: String::new(), fixed_source: String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_production_helper() {
        let s = alpha_bench_spec();
        assert_eq!(s.name, "alpha");
    }

    #[cfg(feature = "rust_perf_bench")]
    fn local_test_fixture() -> BenchSpec {
        BenchSpec { name: "local", buggy_source: String::new(), fixed_source: String::new() }
    }

    #[test]
    fn count() {
        assert_eq!(build_perf_seed_corpus().entries.len(), 4);
    }
}
'''


def self_test() -> int:
    failures: list[str] = []

    def check(cond: bool, msg: str) -> None:
        print(f"  {'ok:' if cond else 'FAIL:'} {msg}")
        if not cond:
            failures.append(msg)

    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        fixture = tmp / "corpus.rs"
        fixture.write_text(FIXTURE)
        rep = split(fixture, tmp / "corpus", expect_entries=4, shard_lines=10,
                    write=True, label="self-test")
        files = rep["files"]

        check(rep["entries"] == 4, "entry count 4 (decoy PerfRule { in string NOT counted)")
        check(rep["helpers"] == 3, "helper count 3 (doc-comment fn fake_bench_spec( NOT counted)")
        check(rep["shards"] >= 2, f"sharded ({rep['shards']} slices at 10-line target)")
        check(set(files) == {f"entries_{i + 1:02d}.rs" for i in range(rep["shards"])}
              | {"mod.rs", "tests.rs"}, "file set complete")
        check(not fixture.exists(), "source removed after write")

        mod = files["mod.rs"]
        check("use crate::corpus::Corpus;" in mod, "mod.rs keeps the use lines")
        check("mod entries_01;" in mod and "#[cfg(test)]\nmod tests;" in mod,
              "mod.rs declares slices + tests")
        check("too_many_lines" not in mod, "mod.rs drops the monolith allow attr")
        check("let mut entries = entries_01::entries();" in mod
              and "Corpus::new(entries)" in mod, "mod.rs builder rewritten, extend-ordered")
        check(mod.index("mod entries_01;") < mod.index("pub fn build_perf_seed_corpus"),
              "mod decls precede the builder")
        check(mod.count("pub fn build_perf_seed_corpus") == 1, "no duplicated builder signature")

        # alpha is shared: defined pub(super) in exactly one shard, used from another
        owners = [n for n, c in files.items() if "pub(super) fn alpha_bench_spec" in c]
        check(len(owners) == 1, f"alpha defined pub(super) exactly once ({owners})")
        owner_shard = int(owners[0][8:10]) if owners else -1
        users = [n for n, c in files.items()
                 if f"use super::entries_{owner_shard:02d}::alpha_bench_spec;" in c]
        check(len(users) >= 1, f"cross-shard use lines emitted for alpha ({users})")
        check(all("#[cfg(feature = \"rust_perf_bench\")]\nuse super::" in files[u]
                  for u in users), "cross-shard uses carry the gate")

        # raw-string fixture content survives verbatim in alpha's shard
        check("pub fn make_input(n: usize) -> Vec<u32> { vec![n] } }, )"
              in files[owners[0]],
              "col-0 raw-string fixture content verbatim (}, did not truncate)")
        check("decoy }; } and PerfRule { inside a normal string"
              in next(c for c in files.values() if 'rule_id: "alpha-user"' in c),
              "normal-string decoy content verbatim")

        tests = files["tests.rs"]
        check("#[cfg(feature = \"rust_perf_bench\")]\nuse super::entries_"
              f"{owner_shard:02d}::alpha_bench_spec;" in tests,
              "tests.rs gates the production-helper use line")
        check("fn local_test_fixture" in tests, "tests-local helper stays in tests.rs")
        check("assert_eq!(build_perf_seed_corpus().entries.len(), 4);" in tests,
              "tests body verbatim")

        # every generated file re-scans clean
        ok = True
        for name, content in files.items():
            try:
                scan(content)
            except SyntaxError as e:
                ok = False
                print(f"  FAIL: {name} does not scan clean: {e}")
        check(ok, "all generated files scan clean (balanced braces)")

    if failures:
        print(f"self-test: {len(failures)} FAILURE(S)")
        return 1
    print("self-test: PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--src", type=Path, default=DEFAULT_SRC)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--expect-entries", type=int, default=126)
    ap.add_argument("--shard-lines", type=int, default=1800)
    ap.add_argument("--write", action="store_true", help="write corpus/ and remove corpus.rs")
    ap.add_argument("--self-test", action="store_true",
                    help="round-trip a synthetic fixture in a tempdir; touches no repo file")
    args = ap.parse_args()

    if args.self_test:
        return self_test()
    rep = split(args.src, args.out, args.expect_entries, args.shard_lines,
                args.write, "dry-run" if not args.write else "write")
    if not args.write:
        print("dry run only — rerun with --write")
    return 0


if __name__ == "__main__":
    sys.exit(main())
