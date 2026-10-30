#!/usr/bin/env python3
"""Walk #22 fetch — arxiv pool refill (copy of .distill/010_walk21_extract.py with the TWO
walk-21 standing corrections applied; both recorded in the walk-21 landing row, top of
.distill/001_mining_queue_snapshot.md).

Corrections vs 010:
1. Category regex fixed — the export API's Atom emits `<category term="cs.LG" .../>`; the
   attribute is `term=`, never `category=`. 010's regex matched nothing and graded every row
   `[?]`; this copy reads the real primary category.
2. The window FROM is a REQUIRED argument. 010 hard-coded `202610271800` (walk-21's opening
   balance) inside the query string; that number is per-walk state, not a law.

Usage:
    python3 012_walk22_extract.py <END yyyymmddHHMM> <FROM yyyymmddHHMM> [seed_file] [out_dir]

Standing window law (walk-17): nightly window = [prev-announce-cutoff TO this-announce-cutoff].
First useful poll ~02:15Z (walk-18 datum); probes before ~01:45Z are dead reads.

Unchanged from 010: bare submittedDate range (all categories), 200/page pagination,
>= 12 s spacing between API calls (the 429 law), 30 s*(n+1) backoff ladder, seed dedup,
candidates.txt + abstracts.txt + debug.txt under OUT_DIR.
"""
import os
import re
import sys
import time
import urllib.request

if len(sys.argv) < 3:
    print("usage: 012_walk22_extract.py <END yyyymmddHHMM> <FROM yyyymmddHHMM>"
          " [seed_file] [out_dir]", file=sys.stderr)
    sys.exit(2)
END = sys.argv[1]
FROM = sys.argv[2]
SEED_PATH = (sys.argv[3] if len(sys.argv) > 3
             else os.path.join(os.path.dirname(__file__), "011_walk22_dedup_seed.txt"))
OUT_DIR = sys.argv[4] if len(sys.argv) > 4 else "/tmp/walk22"

if not os.path.exists(SEED_PATH):
    # A silent empty seed is a dedup hole (the walk-18 seed incident class) — refuse loud.
    print(f"seed file missing: {SEED_PATH}", file=sys.stderr)
    sys.exit(2)

os.makedirs(OUT_DIR, exist_ok=True)
seed = set(open(SEED_PATH, encoding="utf-8").read().split())
dbg = open(os.path.join(OUT_DIR, "debug.txt"), "w", encoding="utf-8")
dbg.write(f"seed size: {len(seed)}\n")
dbg.write(f"window: [{FROM} TO {END}]\n")

BASE = "https://export.arxiv.org/api/query"
entries = {}
start = 0
total = None
while True:
    q = (f"search_query=submittedDate:%5B{FROM}+TO+{END}%5D"
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
        mc = re.search(r'<category term="([^"]+)"', e)
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
open(os.path.join(OUT_DIR, "candidates.txt"), "w", encoding="utf-8").write(
    "\n".join(lines) + "\n")
with open(os.path.join(OUT_DIR, "abstracts.txt"), "w", encoding="utf-8") as f:
    for aid in sorted(entries, key=lambda k: entries[k][0], reverse=True):
        pub, cat, t, s = entries[aid]
        f.write(f"### {aid} [{cat}] {pub}\n{t}\n{s}\n\n")
dbg.write(f"FINAL seen={len(entries)} total={total}\n")
dbg.close()
print(f"done seen={len(entries)} total={total} seed={len(seed)}")
