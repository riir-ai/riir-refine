import { readFileSync } from 'node:fs';
const path = process.argv[2] || 'cloudflare/dashboard/public/index.html';
const html = readFileSync(path, 'utf8');
const ds = html.match(/<script id="ds" type="application\/json">([\s\S]*?)<\/script>/)[1];
const body = html.match(/<script>([\s\S]*?)<\/script>/)[1];

const captured = {};
const els = {};
function el(id) {
  if (els[id]) return els[id];
  const e = { id, _h: '', textContent: '', style: {} };
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

// 4. bill reconciliation (Issue 065). Two things are gated and one is not,
// and the split is the point.
//
// GATED: that the cards RENDERED, and that the bill ARITHMETIC holds. A
// payload-key rename blanks a table silently — this file captures innerHTML
// per id and previously only ever asserted on <rect>, so every table card on
// the page could have gone empty and the gate would have said PASS. And the
// arithmetic is exactly what was broken for a week: dollars computed from unit
// zero while the page claimed the allowance was subtracted.
//
// NOT GATED: the per-line delta against `.benchmarks/cf_bill_pin.json`. Usage
// moves between the bill snapshot and any given run, so a dollar delta reds on
// legitimate growth — the same false-red class this file already documents for
// the corpus bar's rect count. It is REPORTED, so drift is visible, and the
// rules above are what actually fail.
const D = JSON.parse(ds);
ok(!!captured['billTable'] && captured['billTable'].length > 200,
   'billTable did not render (payload key drift?) — got ' + (captured['billTable'] || '').length + ' chars');
ok(!!captured['kvTable'] && captured['kvTable'].length > 100,
   'kvTable did not render — got ' + (captured['kvTable'] || '').length + ' chars');
ok(!!captured['contTable'] && captured['contTable'].length > 100,
   'contTable did not render — got ' + (captured['contTable'] || '').length + ' chars');
ok(!!captured['kpis'] && captured['kpis'].includes('CF billed'),
   'KPI row is missing the billed-cost tile');

let pinDeltas = 'no bill in payload';
if (D.bill && D.bill.lines) {
  const B = D.bill;
  ok(B.lines.length > 0, 'bill has zero lines');
  let sum = 0;
  for (const L of B.lines) {
    // billable = max(0, used - included). This is the subtraction whose
    // absence made $0.08 read as $0.3780.
    const wantBillable = Math.max(0, L.used - L.included);
    ok(Math.abs(L.billable - wantBillable) < 1e-6,
       L.key + ': billable ' + L.billable + ' != max(0, ' + L.used + ' - ' + L.included + ')');
    // usd = ceil(billable / block) * price. The ceil is why 301.97k list ops
    // cost a whole $5.00 block; pricing it linearly reads too LOW.
    const wantUnits = L.gran > 1 ? Math.ceil(L.billable / L.gran) : L.billable;
    ok(Math.abs(L.billed_units - wantUnits) < 1e-6,
       L.key + ': billed_units ' + L.billed_units + ' != ' + wantUnits + ' (gran ' + L.gran + ')');
    ok(Math.abs(L.usd - wantUnits * L.price) < 1e-9,
       L.key + ': usd ' + L.usd + ' != ' + (wantUnits * L.price));
    // The allowance can only ever reduce a line (every included allowance
    // here is >= its block size), so a billed figure ABOVE the from-unit-zero
    // figure means the model regressed to double-charging.
    ok(L.usd <= L.marginal_usd + 1e-9,
       L.key + ': billed ' + L.usd + ' exceeds marginal ' + L.marginal_usd);
    sum += L.usd;
  }
  ok(Math.abs(B.usd - sum) < 1e-9, 'bill total ' + B.usd + ' != sum of lines ' + sum);
  // The pin must be PRESENT — removing it would silently disarm the only
  // cross-check against a number this repo did not compute itself.
  ok(B.pin && B.pin.lines && Object.keys(B.pin.lines).length > 0,
     'cf_bill_pin.json absent or empty from the payload — the bill cross-check is disarmed');
  if (B.pin && B.pin.lines) {
    const ds2 = B.lines.map(L => {
      const pin = B.pin.lines[L.key];
      return pin === undefined ? L.key + '=unpinned'
        : L.key + '=' + (L.usd - pin.usd >= 0 ? '+' : '') + (L.usd - pin.usd).toFixed(4);
    });
    pinDeltas = 'vs pin(' + B.pin.snapshot_utc + ' $' + B.pin.usage_cost_usd + '): $'
      + B.usd.toFixed(2) + ' [' + ds2.join(' ') + ']'
      + (B.uncollected && B.uncollected.length ? ' uncollected:' + B.uncollected.join(',') : '');
  }
}

if (fails.length) { console.error('FAIL:\n - ' + fails.join('\n - ')); process.exit(1); }
console.log('PASS: ' + n + ' rects across ' + Object.keys(captured).length + ' charts; corpus ' + N_DOM + ' domains ys [' + ys.join(',') + ']; sparks fixed-size + nowrap');
console.log('PASS: bill rules hold (billable/ceil-block/total) — ' + pinDeltas);
