#!/usr/bin/env python3
"""Issue 213 G3 + the citation-removal disclosure — the shader-gate retention diff.

Compares two kernel_goat_g5 tables over the SAME tree: PRE (the pre-gate
fan-out, built by reverting only the Issue-213 T1/T2 files) vs POST (the
gated fan-out). Same features on both arms (`latent_retrieval,shader_perf`,
no kernel_opt — the kernel slot is absent from both, symmetrically).

Reads the ADDITIVE `rank1_all` key: per-span, per-domain rank-1 ids
(`label:rule_id`), walk order, last pass. Every existing json key keeps its
shape — the cost/utility instruments are untouched.

Adjudicates (verdict round 1, D2, session 4785a7a0):
  - G3 retention: every per-span change is the ShaderPerf entry's REMOVAL or
    its absence on both arms; ZERO changes to any other domain's rank-1.
    Any non-shader change exits 1.
  - Disclosure: the count of shader rank-1 citations removed on the walk
    (a disclosure, never a bar) + the per-domain rank-1 census both arms.

Usage: g5_shader_g3_diff.py PRE_JSON POST_JSON [--shader-domain ShaderPerf]
Exits 1 on any walk drift or retention violation; 0 on PASS.
"""
import argparse
import json
import sys


def load(path: str) -> dict:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("pre_json")
    ap.add_argument("post_json")
    ap.add_argument("--shader-domain", default="ShaderPerf",
                    help="the DomainId::label() spelling of the gated domain")
    args = ap.parse_args()

    pre = load(args.pre_json)
    post = load(args.post_json)

    # The corpus walk must be identical across arms — any drift REFUSES
    # (the same-tree revert guarantees it; a violation means the arms were
    # built from different trees and the comparison is void).
    if pre.get("files") != post.get("files") or pre.get("corpus_root") != post.get("corpus_root"):
        print(
            f"REFUSE: corpus walk drifted (pre files={pre.get('files')} "
            f"post files={post.get('files')})",
            file=sys.stderr,
        )
        return 1

    pre_all = pre.get("rank1_all")
    post_all = post.get("rank1_all")
    if not isinstance(pre_all, list) or not isinstance(post_all, list) or len(pre_all) != len(post_all):
        print("REFUSE: rank1_all missing or span-count drift", file=sys.stderr)
        return 1

    dom = args.shader_domain
    prefix = dom + ":"
    removed = 0
    gained = 0
    violations: list[str] = []

    for idx, (a_set, b_set) in enumerate(zip(pre_all, post_all)):
        if a_set == b_set:
            continue
        a_only = [x for x in a_set if x not in b_set]
        b_only = [x for x in b_set if x not in a_set]
        # The ONLY lawful change: ShaderPerf entries leaving the span.
        if all(x.startswith(prefix) for x in a_only) and not b_only:
            removed += len(a_only)
            continue
        gained += sum(1 for x in b_only if x.startswith(prefix))
        violations.append(f"span {idx}: {a_set} -> {b_set}")

    print(f"spans compared: {len(pre_all)} (files {pre.get('files')}, walk {pre.get('corpus_root')})")
    print(f"G3 retention: non-shader rank-1 changes = {len(violations)} "
          f"({'PASS' if not violations else 'FAIL'})")
    for row in violations[:20]:
        print(f"  violation: {row}")
    if len(violations) > 20:
        print(f"  ... and {len(violations) - 20} more")
    print(f"disclosure: {dom} rank-1 citations removed pre-gate -> gated: {removed} "
          f"(gained: {gained})")

    def census(tables: list[list[list[str]]]) -> dict[str, int]:
        # tables = rank1_all: one entry per span, each a list of
        # `Domain:rule_id` strings — exactly two iteration layers.
        counts: dict[str, int] = {}
        for span in tables:
            for e in span:
                dom_label = e.rsplit(":", 1)[0]
                counts[dom_label] = counts.get(dom_label, 0) + 1
        return counts

    pre_census = census(pre_all)
    post_census = census(post_all)
    print(f"per-domain rank-1 census PRE:  {pre_census}")
    print(f"per-domain rank-1 census POST: {post_census}")
    verdict = "PASS" if not violations else "FAIL"
    print(f"verdict: {verdict}")
    return 0 if not violations else 1


if __name__ == "__main__":
    sys.exit(main())
