"""riir-refine Issue 250 T0: the fix-lane cost-model census over the fleet's
blueprint_history.jsonl rows (the persisted VerifyCapture per run)."""
import json, sys, statistics as st
rows, seen = [], set()
for path in sys.argv[1:]:
    for line in open(path, encoding="utf-8"):
        w = json.loads(line); r = w["_row"]
        key = (r.get("digest"), r.get("ts"))
        if key in seen:
            continue
        seen.add(key); r["_rig"] = w["_rig"]; r["_repo"] = w["_repo"]; rows.append(r)
fix = [r for r in rows if r.get("mode") == "fix"]
ver = [r for r in fix if r["outcomes"].get("verify_ran")]
o = lambda r, k: r["outcomes"].get(k, 0)
wasted = [r for r in ver if o(r, "kept_edits") == 0 and o(r, "reverted") > 0]
partial = [r for r in ver if o(r, "kept_edits") > 0 and o(r, "reverted") > 0]
clean = [r for r in ver if o(r, "reverted") == 0]
checks = [o(r, "checks") for r in ver]
extra = [max(0, o(r, "checks") - 2) for r in ver]  # beyond baseline + one clean round
w_checks = sum(o(r, "checks") for r in wasted)
rev_edits = sum(o(r, "reverted") for r in ver); wr_edits = sum(o(r, "edits_written") for r in ver)
print(json.dumps({
  "rows_total": len(rows), "rigs": sorted({r["_rig"] for r in rows}), "repos": len({r["_repo"] for r in rows}),
  "modes": {m: sum(1 for r in rows if r.get("mode") == m) for m in sorted({r.get("mode") for r in rows})},
  "fix_runs": len(fix), "verify_ran": len(ver),
  "checks_per_verify_run": {"min": min(checks), "median": st.median(checks), "mean": round(st.mean(checks), 2), "max": max(checks)} if checks else None,
  "runs_with_checks_gt2": sum(1 for x in checks if x > 2), "extra_checks_total": sum(extra), "checks_total": sum(checks),
  "wasted_verify_runs": len(wasted), "wasted_rate_of_verify_runs": round(len(wasted) / max(1, len(ver)), 4),
  "wasted_rate_of_fix_runs": round(len(wasted) / max(1, len(fix)), 4),
  "checks_spent_on_wasted": w_checks, "wasted_check_share": round(w_checks / max(1, sum(checks)), 4),
  "partial_revert_runs": len(partial), "clean_runs": len(clean),
  "reverted_edits": rev_edits, "written_edits": wr_edits, "reverted_edit_share": round(rev_edits / max(1, wr_edits), 4),
  "skipped_checks_total": sum(o(r, "skipped_checks") for r in ver),
  "declined_total": sum(o(r, "declined") for r in fix),
  "pool_size_recorded": any("pool" in json.dumps(r) for r in fix),
}, indent=1))
for r in wasted + partial:
    print("  ", r["_rig"], r["_repo"], "kept", o(r, "kept_edits"), "rev", o(r, "reverted"), "checks", o(r, "checks"),
          "strata_rev", {k: v["reverted"] for k, v in r.get("strata", {}).items() if v.get("reverted")})

def ceiling(vs):
    actual = sum(o(r, "checks") for r in vs)
    # A perfect decline-guard declines exactly the edits that would revert:
    # a run with surviving edits costs baseline + one clean round (2 checks
    # — the measured floor, min = 2); a run whose whole batch reverts never
    # verifies (0 checks).
    ideal = sum(2 if o(r, "kept_edits") > 0 else 0 for r in vs)
    return actual, ideal, round(1 - ideal / max(1, actual), 4)
out = max(ver, key=lambda r: o(r, "checks"))
rest = [r for r in ver if r is not out]
print("CEILING all:", ceiling(ver), " excluding the 206-check outlier:", ceiling(rest))
by_rule = {}
for r in ver:
    for k, v in r.get("strata", {}).items():
        a = by_rule.setdefault(k, [0, 0]); a[0] += v.get("written", 0); a[1] += v.get("reverted", 0)
print("rule revert rates (written>=5):", sorted(((k, a[0], a[1], round(a[1]/a[0], 3)) for k, a in by_rule.items() if a[0] >= 5), key=lambda x: -x[3])[:12])
print("verify posture off runs:", sum(1 for r in fix if not r["posture"].get("verify")))
