import re, glob, os

os.chdir(r"C:\Users\katop\AppData\Local\Temp\walk17")
seed = set(open(r"E:\git\riir-refine\.distill\004_walk17_dedup_seed.txt", encoding="utf-8").read().split())
dbg = open("debug.txt", "w", encoding="utf-8")
dbg.write(f"seed size: {len(seed)}\n")
seen = {}
for f in sorted(glob.glob("*_w.xml")):
    xml = open(f, encoding="utf-8").read()
    for m in re.finditer(r"<entry>(.*?)</entry>", xml, re.S):
        e = m.group(1)
        ma = re.search(r"<id>http://arxiv.org/abs/([^<]+?)</id>", e)
        if not ma:
            dbg.write(f"[{f}] NO-ID entry head: {e[:200]!r}\n\n")
            continue
        aid = re.sub(r"v\d+$", "", ma.group(1))
        if aid in seed:
            continue
        mt = re.search(r"<title>(.*?)</title>", e, re.S)
        mp = re.search(r"<published>(.*?)</published>", e)
        mc = re.search(r'category="([^"]+)"', e)
        if not (mt and mp):
            dbg.write(f"[{f}] {aid} MISSING t/p; entry len {len(e)}\n")
            continue
        cat = mc.group(1) if mc else "?"
        t = " ".join(mt.group(1).split())
        seen[aid] = (mp.group(1)[:16], cat, t[:140])

lines = []
for aid in sorted(seen, key=lambda k: seen[k][0], reverse=True):
    pub, cat, t = seen[aid]
    lines.append(f"{pub} [{cat}] {aid} :: {t}")
open("candidates.txt", "w", encoding="utf-8").write("\n".join(lines) + "\n")
dbg.write(f"seen={len(seen)}\n")
dbg.close()
print("done", len(seen))
