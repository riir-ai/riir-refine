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

// 1. corpus bar: exactly 4 rects at the correct rows
const bar = captured['corpusBar'];
const ys = [...bar.matchAll(/<rect[^>]*? y="(-?[\d.]+)"/g)].map(m => +m[1]);
ok(ys.length === 4, 'corpusBar rect count ' + ys.length);
ok(JSON.stringify(ys) === JSON.stringify([11, 37, 63, 89]), 'corpusBar rect ys = ' + ys.join(','));
ok(bar.includes('x="88"'), 'corpusBar label anchor x=88 (lw=94)');
ok(bar.includes('clippy_lints'), 'clippy_lints label present untruncated source');
// every bar inside its viewBox
for (const svg of bar.split('<svg').slice(1)) {
  const H = +svg.match(/viewBox="0 0 [\d.]+ ([\d.]+)"/)[1];
  for (const m of svg.matchAll(/ y="(-?[\d.]+)"/g)) {
    ok(+m[1] + 17 <= H, 'bar y ' + m[1] + ' inside viewBox H ' + H);
  }
}

// 2. sparks: fixed px size + nowrap labels
const sp = captured['corpusSparks'];
ok((sp.match(/width:150px;height:26px/g) || []).length === 4, 'spark svg fixed px size x4');
ok((sp.match(/white-space:nowrap/g) || []).length === 4, 'spark range label nowrap x4');
ok((sp.match(/→/g) || []).length === 4, 'range labels content (4 arrows)');

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
console.log('PASS: ' + n + ' rects across ' + Object.keys(captured).length + ' charts; corpus ys [11,37,63,89]; sparks fixed-size + nowrap');
