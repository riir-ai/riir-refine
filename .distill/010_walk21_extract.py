#!/usr/bin/env python3
"""Walk #21 fetch — arxiv pool refill (amo/macOS adaptation of .distill/005_walk17_extract.py).

Window: [202610271800 TO <arg end>] — the Thu-cycle batch + post-stall tail (the 10-28+ opening
balance). Fetches the BARE submittedDate range (all categories), paginates, dedups vs the tracked
seed, writes /tmp/walk21/candidates.txt + abstracts.txt. Spacing >= 12 s between API calls (the
429 law — the 03:55Z lane-loop burst earned a per-IP block; never fetch faster than ~1 req/10 s).
"""
import re, sys, time, urllib.request, os

END = sys.argv[1] if len(sys.argv) > 1 else "202610290400"
SEED_PATH = os.path.join(os.path.dirname(__file__), "009_walk21_dedup_seed.txt")
OUT_DIR = "/tmp/walk21"

os.makedirs(OUT_DIR, exist_ok=True)
seed = set(open(SEED_PATH, encoding="utf-8").read().split())
dbg = open(os.path.join(OUT_DIR, "debug.txt"), "w", encoding="utf-8")
dbg.write(f"seed size: {len(seed)}\n")

BASE = "https://export.arxiv.org/api/query"
entries = {}
start = 0
total = None
while True:
    q = (f"search_query=submittedDate:%5B202610271800+TO+{END}%5D"
         f"&start={start}&max_results=200&sortBy=submittedDate&sortOrder=descending")
    xml = None
    for attempt in range(5):
        try:
            with urllib.request.urlopen(f"{BASE}?{q}", timeout=40) as r:
                xml = r.read().decode("utf-8", "replace")
            if "Rate exceeded" not in xml and "<feed" in xml:
                break
        except Exception as e:
            dbg.write(f"[start={start}] attempt {attempt}: {e}\n")
        time.sleep(30 * (attempt + 1))
    if xml is None:
        dbg.write(f"[start={start}] GIVING UP after retries\n")
        break
    m = re.search(r"totalResults>(\d+)<", xml)
    total = int(m.group(1)) if m else 0
    got = 0
    for em in re.finditer(r"<entry>(.*?)</entry>", xml, re.S):
        e = em.group(1)
        got += 1
        ma = re.search(r"<id>http://arxiv.org/abs/([^<]+?)</id>", e)
        if not ma:
            dbg.write(f"[start={start}] NO-ID entry head: {e[:200]!r}\n\n")
            continue
        aid = re.sub(r"v\d+$", "", ma.group(1))
        if aid in seed:
            continue
        mt = re.search(r"<title>(.*?)</title>", e, re.S)
        mp = re.search(r"<published>(.*?)</published>", e)
        mc = re.search(r'category="([^"]+)"', e)
        ms = re.search(r"<summary>(.*?)</summary>", e, re.S)
        if not (mt and mp):
            dbg.write(f"[{aid}] MISSING t/p; entry len {len(e)}\n")
            continue
        cat = mc.group(1) if mc else "?"
        t = " ".join(mt.group(1).split())
        s = " ".join(ms.group(1).split()) if ms else ""
        entries[aid] = (mp.group(1)[:16], cat, t, s)
    dbg.write(f"[start={start}] got={got} cumulative={len(entries)} total={total}\n")
    if got == 0 or start + 200 >= total:
        break
    start += 200
    time.sleep(12)

lines = []
for aid in sorted(entries, key=lambda k: entries[k][0], reverse=True):
    pub, cat, t, s = entries[aid]
    lines.append(f"{pub} [{cat}] {aid} :: {t}")
open(os.path.join(OUT_DIR, "candidates.txt"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
with open(os.path.join(OUT_DIR, "abstracts.txt"), "w", encoding="utf-8") as f:
    for aid in sorted(entries, key=lambda k: entries[k][0], reverse=True):
        pub, cat, t, s = entries[aid]
        f.write(f"### {aid} [{cat}] {pub}\n{t}\n{s}\n\n")
dbg.write(f"FINAL seen={len(entries)} total={total}\n")
dbg.close()
print(f"done seen={len(entries)} total={total} seed={len(seed)}")
