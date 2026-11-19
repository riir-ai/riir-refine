"""riir-refine Issue 250 T1a: the per-rule historical revert prior as a
decline-guard floor, time-split held-out (pre-registered in .issues/250)."""
import json, sys, statistics as st
rows, seen = [], set()
for path in sys.argv[1:]:
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)["_row"]
        key = (r.get("digest"), r.get("ts"))
        if key not in seen:
            seen.add(key); rows.append(r)
rows = [r for r in rows if r.get("mode") == "fix"]
med = st.median(r["ts"] for r in rows)
fit = [r for r in rows if r["ts"] < med]
ev = [r for r in rows if r["ts"] >= med and r["outcomes"].get("verify_ran")]
WHOLE = "(whole file)"
prior = {}
for r in fit:
    for k, v in r.get("strata", {}).items():
        a = prior.setdefault(k, [0, 0]); a[0] += v.get("written", 0); a[1] += v.get("reverted", 0)
p = {k: a[1] / a[0] for k, a in prior.items() if a[0] > 0}
outlier = max(ev, key=lambda r: r["outcomes"]["checks"]) if ev else None
def simulate(tau, pop):
    actual = sim = 0; declined_kept = kept_total = declined_rev = rev_total = 0
    for r in pop:
        o = r["outcomes"]; actual += o["checks"]
        rem_written = rem_rev = 0
        for k, v in r.get("strata", {}).items():
            w, rv = v.get("written", 0), v.get("reverted", 0)
            if k == WHOLE:
                rem_rev += rv; rev_total += rv; continue
            kept = max(0, w - rv); kept_total += kept; rev_total += rv
            if k in p and p[k] >= tau:
                declined_kept += kept; declined_rev += rv
            else:
                rem_written += w; rem_rev += rv
        if rem_written == 0 and rem_rev == 0:
            sim += 0
        elif rem_rev == 0:
            sim += 2
        else:
            sim += o["checks"]
    return {"actual": actual, "simulated": sim, "reduction": round(1 - sim / max(1, actual), 4),
            "false_decline_rate": round(declined_kept / max(1, kept_total), 4),
            "reverts_caught": f"{declined_rev}/{rev_total}"}
print(json.dumps({"fit_runs": len(fit), "eval_verify_runs": len(ev), "median_ts": med,
                  "fit_rules_with_p_ge_0.25": {k: round(v, 3) for k, v in p.items() if v >= 0.25}}, indent=1))
for tau in (0.25, 0.5, 0.75):
    allp = simulate(tau, ev); ex = simulate(tau, [r for r in ev if r is not outlier])
    print(f"tau={tau}: all={allp}  ex-outlier={ex}  PASS={ex['reduction'] >= 0.10 and ex['false_decline_rate'] <= 0.01}")
print("outlier in eval:", outlier is not None and outlier["outcomes"]["checks"])
ideal = sum(2 if r["outcomes"].get("kept_edits", 0) > 0 else 0 for r in ev)
act = sum(r["outcomes"]["checks"] for r in ev)
print(f"held-out perfect-guard ceiling: actual {act} -> ideal {ideal} = {1 - ideal / max(1, act):.4f} reduction")
