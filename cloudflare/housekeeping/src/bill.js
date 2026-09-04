// The Cloudflare bill model, inside the housekeeping Worker (Issue 065 T7).
//
// Ported from `scripts/gen_dashboard.py` (BILL_LINES / bill_line() /
// bill_reconciliation() / billing_period_start(), Issue 065 T1, verified
// against the real bill in Bench 067 and pinned in
// `.benchmarks/cf_bill_pin.json`). The worker copy exists because the
// MONTHLY_BUDGET_USD alert used to sum per-hour/per-day MARGINAL est_usd rows
// — a quantity whose monthly allowance cannot be apportioned to a day even in
// principle, and one that covered only the container family (1.0% of the real
// bill; KV was $8.00 of $8.08). A budget set from the real bill would have
// alarmed at ~1/4.7 of the intended spend. The alert now reconciles the
// account-wide, period-scoped bill instead.
//
// Keep this file and gen_dashboard.py in lockstep: same lines, same included
// allowances, same prices, same block granularity. `test/bill.test.mjs`
// reproduces the 2026-09-03 bill to the cent and pins the [vars] marginal
// rates to these numbers, so the two sources cannot drift silently.

// ── the CF bill model (Issue 065) ──────────────────────────────────────────
//
//   billable = max(0, PERIOD_ACCOUNT_total - included)
//   usd      = ceil(billable / gran) * price          # gran > 1
//   usd      = billable * price                       # gran == 1 (continuous)
//
// Three properties of that formula are load-bearing (full narrative in
// gen_dashboard.py):
//  1. `included` is SUBTRACTED. `usage * rate` from unit zero is the MARGINAL
//     number — measured $0.3780 where Cloudflare billed $0.08 (4.7x).
//  2. The allowance is MONTHLY and ACCOUNT-WIDE. It is not per app, per
//     namespace, per hour or per day, so only a period-scoped account total is
//     a billing quantity at all.
//  3. `gran` rounds the remainder UP. KV is sold by the million: 301,970
//     billable list ops cost the full $5.00, not $1.51 — the one line where a
//     naive linear model reads too LOW.
export const BILL_LINES = {
  container_mem:  { label: "container memory", unit: "GiB-s",
                    included: 25 * 3600, price: 0.0000025,
                    gran: 1, included_label: "25 GiB-h" },
  container_disk: { label: "container disk", unit: "GB-s",
                    included: 200 * 3600, price: 0.00000007,
                    gran: 1, included_label: "200 GB-h" },
  container_cpu:  { label: "container vCPU", unit: "vCPU-s",
                    included: 375 * 60, price: 0.00002,
                    gran: 1, included_label: "375 vCPU-min" },
  kv_read:        { label: "KV read ops", unit: "ops",
                    included: 10_000_000, price: 0.50,
                    gran: 1_000_000, included_label: "10M" },
  kv_list:        { label: "KV list ops", unit: "ops",
                    included: 1_000_000, price: 5.00,
                    gran: 1_000_000, included_label: "1M" },
  kv_write:       { label: "KV write ops", unit: "ops",
                    included: 1_000_000, price: 5.00,
                    gran: 1_000_000, included_label: "1M" },
  kv_delete:      { label: "KV delete ops", unit: "ops",
                    included: 1_000_000, price: 5.00,
                    gran: 1_000_000, included_label: "1M" },
};

// kvOperationsAdaptiveGroups actionType -> bill line (mirrors
// KV_ACTION_TO_LINE in gen_dashboard.py).
export const KV_ACTION_TO_LINE = { read: "kv_read", list: "kv_list",
                                   write: "kv_write", delete: "kv_delete" };

// One line of the CF bill from an account-wide, period-scoped total.
// `marginal_usd` is the from-unit-zero number kept alongside for contrast —
// the exact figure the old alert wrongly summed.
export function billLine(key, used) {
  const spec = BILL_LINES[key];
  const u = Number(used) || 0;
  const billable = Math.max(0, u - spec.included);
  const gran = spec.gran;
  // gran == 1 keeps the line continuous; gran > 1 is sold in whole blocks and
  // the remainder rounds UP, so a 30% block still costs a whole one.
  const units = gran > 1 ? Math.ceil(billable / gran) : billable;
  return {
    key, label: spec.label, unit: spec.unit,
    used: u, included: spec.included, included_label: spec.included_label,
    billable, gran, billed_units: units, price: spec.price,
    usd: units * spec.price,
    marginal_usd: (u * spec.price) / (gran || 1),
  };
}

// Account-wide, period-scoped bill. `usage` maps BILL_LINES keys to totals.
//
// Lines with no collected total are reported in `uncollected` rather than as
// zeros — an uncollected line and an unused one are different facts, and
// conflating them is exactly how this account reported $0.08 of container
// spend as the whole cloud bill while KV was charging $8.00 (Issue 065). A
// successful collector whose response carries no rows presents a genuine zero
// for its lines (CF recorded no usage); only a FAILED or degraded collector
// leaves its lines out.
//
// Unlike the dashboard's bill_reconciliation() there is no cf_bill_pin.json
// cross-check here: the worker cannot read the repo. The pin reconciliation
// stays a nightly-dashboard duty; this copy feeds the budget alert.
export function billReconciliation(usage, periodStart) {
  const keys = Object.keys(BILL_LINES);
  const lines = keys.filter((k) => k in usage).map((k) => billLine(k, usage[k]));
  const uncollected = keys.filter((k) => !(k in usage));
  return {
    period_start: periodStart,
    lines,
    usd: lines.reduce((a, x) => a + x.usd, 0),
    marginal_usd: lines.reduce((a, x) => a + x.marginal_usd, 0),
    uncollected,
  };
}

// Most recent CF_BILLING_ANNIVERSARY_DAY at or before `now` (UTC).
//
// The billing period start is NOT derivable with this worker's credential
// (analytics GraphQL only — the same 10000-authentication-error wall the
// dashboard hit), so the anniversary day is a PIN, defaulting to 16 to match
// gen_dashboard.py, bounded by arithmetic to 2026-08-15..2026-08-26 and
// currently unfalsifiable. Override with the CF_BILLING_ANNIVERSARY_DAY var
// once known.
export function billingPeriodStart(now, anniversaryDay = 16) {
  let day = parseInt(anniversaryDay, 10);
  if (!Number.isFinite(day)) day = 16;
  day = Math.max(1, Math.min(28, day));
  const y = now.getUTCFullYear();
  const m = now.getUTCMonth();
  const d = now.getUTCDate();
  const iso = (yy, mm) =>
    `${yy}-${String(mm + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  if (d >= day) return iso(y, m);
  // Before the anniversary: the period started in the previous month. Using
  // the Date constructor for that step would re-parse through local time —
  // the exact hazard the hour-key fix documented — so the arithmetic is done
  // on the UTC fields directly.
  const pm = m === 0 ? { y: y - 1, m: 11 } : { y, m: m - 1 };
  return iso(pm.y, pm.m);
}
