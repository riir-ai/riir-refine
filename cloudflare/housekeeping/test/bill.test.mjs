// Unit tests for the housekeeping Worker's bill model (Issue 065 T7).
//
// `src/bill.js` is a port of `scripts/gen_dashboard.py`'s BILL_LINES /
// bill_line() / bill_reconciliation(), which Bench 067 verified against the
// REAL Cloudflare bill. These tests reproduce that verification locally so
// the worker copy cannot drift from the bill (or from the dashboard copy)
// without going red:
//
//   node --test cloudflare/housekeeping/test/bill.test.mjs

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  BILL_LINES, KV_ACTION_TO_LINE,
  billLine, billReconciliation, billingPeriodStart,
} from "../src/bill.js";

// .benchmarks/cf_bill_pin.json — the usage Cloudflare actually charged on
// 2026-09-03, with its own `usd` figures. `billable`/`usd` are CF's numbers,
// never recomputed; `used` for kv_list is the reconstructed exact value.
const PIN_USED = {
  kv_list: 1301970, kv_read: 15310000, kv_write: 419160,
  container_mem: 122940, container_disk: 246330, container_cpu: 1020,
};
const PIN_USD = {
  kv_list: 5.00, kv_read: 3.00, kv_write: 0.00,
  container_mem: 0.08, container_disk: 0.00, container_cpu: 0.00,
};

test("billLine reproduces every pinned line of the 2026-09-03 CF bill", () => {
  for (const [key, used] of Object.entries(PIN_USED)) {
    const l = billLine(key, used);
    assert.equal(l.key, key);
    assert.equal(l.used, used);
    assert.equal(l.billable, Math.max(0, used - BILL_LINES[key].included));
    assert.ok(Math.abs(l.usd - PIN_USD[key]) < 0.005,
      `${key}: model $${l.usd} vs CF's $${PIN_USD[key]}`);
  }
});

test("the reconciliation totals the pin to the cent ($8.08)", () => {
  const bill = billReconciliation({ ...PIN_USED }, "2026-08-16");
  assert.ok(Math.abs(bill.usd - 8.08) < 0.005, `total $${bill.usd} vs $8.08`);
  // kv_delete was not observed on the bill → genuinely uncollected here,
  // not a zero.
  assert.deepEqual(bill.uncollected, ["kv_delete"]);
});

test("KV rounds UP to the whole million — the one line a linear model reads too LOW", () => {
  const l = billLine("kv_list", PIN_USED.kv_list);
  assert.equal(l.billable, 301970);       // 30% of a block
  assert.equal(l.billed_units, 1);        // ...still a whole block
  assert.equal(l.usd, 5.00);
  // The linear misread, pinned so the gap is explicit: $1.51 vs $5.00.
  assert.ok(Math.abs((l.billable / 1e6) * 5 - 1.50985) < 1e-9);
});

test("marginal_usd is the from-unit-zero number the old alert wrongly summed", () => {
  const l = billLine("container_mem", PIN_USED.container_mem);
  assert.ok(Math.abs(l.marginal_usd - 122940 * 0.0000025) < 1e-12);
  // $0.307 marginal vs $0.08 billed — the 4x-class distortion Issue 065
  // measured on the live dashboard.
  assert.ok(l.marginal_usd > l.usd * 3);
});

test("an uncollected line is reported, never silently zeroed", () => {
  const bill = billReconciliation({ container_mem: 100 }, "2026-08-16");
  assert.equal(bill.lines.length, 1);
  assert.equal(bill.uncollected.length, 6);
  assert.deepEqual(bill.uncollected.slice().sort(),
    ["container_cpu", "container_disk", "kv_delete", "kv_list", "kv_read", "kv_write"]);
});

test("KV_ACTION_TO_LINE covers exactly the four billable KV actions", () => {
  assert.deepEqual(Object.values(KV_ACTION_TO_LINE).sort(),
    ["kv_delete", "kv_list", "kv_read", "kv_write"]);
  for (const line of Object.values(KV_ACTION_TO_LINE)) {
    assert.ok(BILL_LINES[line], `${line} missing from BILL_LINES`);
  }
});

test("billingPeriodStart pins the anniversary day off UTC fields", () => {
  assert.equal(billingPeriodStart(new Date("2026-08-28T10:00:00Z"), 16), "2026-08-16");
  assert.equal(billingPeriodStart(new Date("2026-08-10T10:00:00Z"), 16), "2026-07-16");
  // Year rollover through the UTC-field arithmetic, never Date-parsed.
  assert.equal(billingPeriodStart(new Date("2026-01-05T10:00:00Z"), 16), "2025-12-16");
  // On the anniversary day itself, the period starts today.
  assert.equal(billingPeriodStart(new Date("2026-08-16T00:00:00Z"), 16), "2026-08-16");
  // Clamp to 1..28; non-numeric falls back to the dashboard's pin (16).
  assert.equal(billingPeriodStart(new Date("2026-08-28T10:00:00Z"), 40), "2026-08-28");
  assert.equal(billingPeriodStart(new Date("2026-08-28T10:00:00Z"), 0), "2026-08-01");
  assert.equal(billingPeriodStart(new Date("2026-08-28T10:00:00Z"), NaN), "2026-08-16");
  assert.equal(billingPeriodStart(new Date("2026-08-28T10:00:00Z")), "2026-08-16");
});

test("the wrangler [vars] marginal rates ARE the BILL_LINES prices (no drift)", () => {
  // Two sources for the container prices exist on purpose: the [vars] rates
  // feed marginalUsd(), the BILL_LINES prices feed the bill. This pin makes
  // silent drift between them a red test instead of a wrong budget.
  const toml = readFileSync(new URL("../wrangler.toml", import.meta.url), "utf8");
  const vars = Object.fromEntries(
    [...toml.matchAll(/^\s*([A-Z_]+)\s*=\s*"([^"]*)"/gm)].map((m) => [m[1], m[2]]));
  assert.equal(Number(vars.MEM_USD_PER_GIB_S), BILL_LINES.container_mem.price);
  assert.equal(Number(vars.DISK_USD_PER_GB_S), BILL_LINES.container_disk.price);
  assert.equal(Number(vars.CPU_USD_PER_S), BILL_LINES.container_cpu.price);
});
