#!/usr/bin/env python3
"""Issue 213 G3 + the citation-removal disclosure — the shader-gate retention diff.

Compares two kernel_goat_g5 tables over the SAME tree: PRE (the pre-gate
fan-out, built by reverting only the Issue-213 T1/T2 files) vs POST (the
gated fan-out). Same features on both arms (`latent_retrieval,shader_perf`);
the walk skips the gate's own three source files (--skip, both arms) because
their chunking drifts with their own content — the self-referential surface
is excluded BY CONSTRUCTION (the Issue-214 self-match precedent).

Reads the ADDITIVE `rank1_all` key: per-span, per-domain rank-1 ids
(`label:rule_id`), walk order, last pass. Every existing json key keeps its
shape — the cost/utility instruments are untouched.

Adjudicates (verdict round 1, D2, session 4785a7a0; predicate completed
2026-11-07 zed-2f171db9@m3 after the first real run):
  - G3 retention: on the `.rs` walk the LAWFUL per-span change is the removal
    of the NON-APPLYING domains' entries — the Issue-145 path contract
    (`DomainId::applies_to_path`): Docker (Dockerfiles), Dist (manifests),
    ShaderPerf (`.wgsl`) cannot touch `.rs`, and the amended design pins them
    ABSENT post-gate ("applying domains byte-identical to the ungated fan-out
    and non-applying ones absent"). ZERO gained entries; ZERO changes to any
    APPLYING domain's rank-1 (clippy/perf/kernel on this walk). Any other
    change exits 1.
  - Completeness: a non-applying entry surviving in POST is a violation —
    removal is not enough, absence is the pinned shape.
  - Disclosure: the per-non-applying-domain removal counts (a disclosure,
    never a bar) + the per-domain rank-1 census both arms.

Usage: g5_shader_g3_diff.py PRE_JSON POST_JSON
       [--non-applying Docker,Dist,ShaderPerf]
Exits 1 on any walk drift, skip mismatch, or retention violation; 0 on PASS.
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
    ap.add_argument("--non-applying", default="docker,dist,shader",
                    help="comma list of DomainId::label() spellings (LOWERCASE, "
                         "as emitted in rank1_all) whose rules cannot touch the "
                         "walk's path kind (the Issue-145 contract); their "
                         "removal is lawful, their survival in POST is a "
                         "violation")
    args = ap.parse_args()

    pre = load(args.pre_json)
    post = load(args.post_json)

    # The corpus walk must be identical across arms — any drift REFUSES
    # (the same-tree revert guarantees it; a violation means the arms were
    # built from different trees and the comparison is void). The skip list
    # must match too — differently-skipped walks are different populations.
    if pre.get("files") != post.get("files") or pre.get("corpus_root") != post.get("corpus_root"):
        print(
            f"REFUSE: corpus walk drifted (pre files={pre.get('files')} "
            f"post files={post.get('files')})",
            file=sys.stderr,
        )
        return 1
    if pre.get("skipped") != post.get("skipped"):
        print(
            f"REFUSE: skip lists differ (pre={pre.get('skipped')} "
            f"post={post.get('skipped')})",
            file=sys.stderr,
        )
        return 1

    pre_all = pre.get("rank1_all")
    post_all = post.get("rank1_all")
    if not isinstance(pre_all, list) or not isinstance(post_all, list) or len(pre_all) != len(post_all):
        print("REFUSE: rank1_all missing or span-count drift", file=sys.stderr)
        return 1

    prefixes = tuple(d + ":" for d in args.non_applying.split(",") if d)
    removed_by_dom: dict[str, int] = {}
    gained = 0
    violations: list[str] = []

    for idx, (a_set, b_set) in enumerate(zip(pre_all, post_all)):
        if a_set == b_set:
            continue
        a_only = [x for x in a_set if x not in b_set]
        b_only = [x for x in b_set if x not in a_set]
        # The ONLY lawful change: a NON-APPLYING domain's entry leaving the
        # span (the gate skipping its index entirely). Everything else — any
        # gained entry, any applying-domain change — is a violation.
        if all(x.startswith(prefixes) for x in a_only) and not b_only:
            for x in a_only:
                dom_label = x.rsplit(":", 1)[0]
                removed_by_dom[dom_label] = removed_by_dom.get(dom_label, 0) + 1
            continue
        gained += sum(1 for x in b_only if x.startswith(prefixes))
        violations.append(f"span {idx}: {a_set} -> {b_set}")

    # Completeness (the amended design's pinned shape): a non-applying entry
    # surviving anywhere in POST is a violation, even where PRE also had it.
    survived = sum(
        1 for span in post_all for x in span if x.startswith(prefixes)
    )
    if survived:
        violations.append(
            f"COMPLETENESS: {survived} non-applying entries survive in POST"
        )

    print(f"spans compared: {len(pre_all)} (files {pre.get('files')}, walk {pre.get('corpus_root')})")
    print(f"G3 retention: applying-domain rank-1 changes = {len(violations)} "
          f"({'PASS' if not violations else 'FAIL'})")
    for row in violations[:20]:
        print(f"  violation: {row}")
    if len(violations) > 20:
        print(f"  ... and {len(violations) - 20} more")
    print(f"disclosure: non-applying rank-1 citations removed pre-gate -> gated: "
          f"{removed_by_dom} (gained: {gained}, survived: {survived})")

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
