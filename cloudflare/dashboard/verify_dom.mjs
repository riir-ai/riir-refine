import { readFileSync } from 'node:fs';
const path = process.argv[2] || 'cloudflare/dashboard/public/index.html';
const html = readFileSync(path, 'utf8');
const ds = html.match(/<script id="ds" type="application\/json">([\s\S]*?)<\/script>/)[1];
const body = html.match(/<script>([\s\S]*?)<\/script>/)[1];

const captured = {};
const els = {};
function el(id) {
  if (els[id]) return els[id];
  const e = { id, _h: '', textContent: '' };
  Object.defineProperty(e, 'innerHTML', {
    get() { return this._h; },
    set(v) { this._h = v; if (v) captured[id] = v; },
  });
  els[id] = e; return e;
}
globalThis.document = { getElementById: (id) => { const e = el(id); if (id === "ds") e.textContent = ds; return e; } };
globalThis.window = globalThis;
globalThis.fetch = () => Promise.resolve({ ok: false, status: 503, json: async () => ({}) });

let threw = null;
try {
  new Function('document', 'fetch', body)(globalThis.document, globalThis.fetch);
} catch (e1) { threw = e1; }
if (threw) { console.error('SCRIPT THREW:', threw); process.exit(1); }

const fails = [];
const ok = (cond, msg) => { if (!cond) fails.push(msg); };

if (!captured['corpusBar']) {
  console.error('corpusBar not captured — script did not reach corpus render');
  process.exit(1);
}

// 1. corpus bar: one rect per DOMAIN, on the correct rows.
// The count is DERIVED from the page's own DOMAINS list, not pinned. A
// hardcoded `=== 4` turns legitimate growth (adding a domain) into a gate
// FAILURE, and since this gate hard-blocks the unattended nightly deploy,
// that would have read as "the dashboard is broken" when it was merely
// bigger. The invariant that actually catches defects — rows on the 11+26k
// lattice, one per series, inside the viewBox — is preserved exactly.
const bar = captured['corpusBar'];
const domMatch = body.match(/const DOMAINS = (\[[^\]]*\]);/);
ok(domMatch !== null, 'could not read DOMAINS from the page script');
const N_DOM = domMatch ? JSON.parse(domMatch[1].replace(/'/g, '"')).length : 0;
ok(N_DOM > 0, 'DOMAINS list is empty');
const ys = [...bar.matchAll(/<rect[^>]*? y="(-?[\d.]+)"/g)].map(m => +m[1]);
const expectedYs = Array.from({ length: N_DOM }, (_, k) => 11 + 26 * k);
ok(ys.length === N_DOM, 'corpusBar rect count ' + ys.length + ' != ' + N_DOM + ' domains');
ok(JSON.stringify(ys) === JSON.stringify(expectedYs), 'corpusBar rect ys = ' + ys.join(','));
ok(bar.includes('x="88"'), 'corpusBar label anchor x=88 (lw=94)');
ok(bar.includes('clippy_lints'), 'clippy_lints label present untruncated source');
// every bar inside its viewBox
for (const svg of bar.split('<svg').slice(1)) {
  const H = +svg.match(/viewBox="0 0 [\d.]+ ([\d.]+)"/)[1];
  for (const m of svg.matchAll(/ y="(-?[\d.]+)"/g)) {
    ok(+m[1] + 17 <= H, 'bar y ' + m[1] + ' inside viewBox H ' + H);
  }
}

// 2. sparks: fixed px size + nowrap labels — ONE PER DOMAIN.
// Derived from N_DOM for the same reason as the corpus bar: these are
// rendered by `DOMAINS.forEach`, so a hardcoded 4 fails the moment a domain
// is added. (Caught by running the gate against a simulated 5-domain page —
// fixing only the bar left these three pinned, and the bar alone would have
// reported a clean pass.)
const sp = captured['corpusSparks'];
const nDom = String(N_DOM);
ok((sp.match(/width:150px;height:26px/g) || []).length === N_DOM, 'spark svg fixed px size x' + nDom);
ok((sp.match(/white-space:nowrap/g) || []).length === N_DOM, 'spark range label nowrap x' + nDom);
ok((sp.match(/→/g) || []).length === N_DOM, 'range labels content (' + nDom + ' arrows)');

// 3. global: every rect y in EVERY chart is on the 11+26k lattice (catches the "y=83" concat class)
const VALID = new Set(Array.from({ length: 40 }, (_, k) => 11 + 26 * k));
let n = 0;
for (const [id, h] of Object.entries(captured)) {
  for (const m of h.matchAll(/<rect[^>]*? y="(-?[\d.]+)"/g)) {
    n++;
    ok(VALID.has(+m[1]), 'chart ' + id + ': rect y ' + m[1] + ' off-lattice');
  }
}
ok(n >= 8, 'total rects scanned ' + n + ' (expected >=8 across all charts)');

if (fails.length) { console.error('FAIL:\n - ' + fails.join('\n - ')); process.exit(1); }
console.log('PASS: ' + n + ' rects across ' + Object.keys(captured).length + ' charts; corpus ' + N_DOM + ' domains ys [' + ys.join(',') + ']; sparks fixed-size + nowrap');
