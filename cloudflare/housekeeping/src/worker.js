// riir-kat-housekeeping — Issue 045 (riir-clippy).
// Hourly cron: container cost snapshot + KAT mint anchor → KV.
// Deliberately dependency-free (no build step; wrangler deploys this file).

const GRAPHQL = "https://api.cloudflare.com/client/v4/graphql";

const DASHBOARD_URL = "https://riir-ops-dashboard.foxfox.workers.dev/";

// CORS for the read routes (the ops dashboard fetches these cross-origin).
const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
  "Access-Control-Max-Age": "86400",
};

function cors(response) {
  const r = new Response(response.body, response);
  for (const [k, v] of Object.entries(CORS_HEADERS)) r.headers.set(k, v);
  return r;
}

// ---- helpers -------------------------------------------------------------

function utc(d) {
  return d.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "");
}

function hourKey(d) { return utc(d).slice(0, 10); }        // YYYYMMDDHH
function dayKey(d) { return utc(d).slice(0, 8); }          // YYYYMMDD

function costUsd(env, memByteS, diskByteS, cpuS) {
  const mem = (memByteS || 0) / 1073741824 * parseFloat(env.MEM_USD_PER_GIB_S);
  const disk = (diskByteS || 0) / 1e9 * parseFloat(env.DISK_USD_PER_GB_S);
  const cpu = (cpuS || 0) * parseFloat(env.CPU_USD_PER_S);
  return mem + disk + cpu;
}

// Fire-and-forget Discord-compatible webhook. Never throws — a webhook
// failure must not break the housekeeping run.
async function notify(env, message) {
  if (!env.ALERT_WEBHOOK_URL) return false;
  try {
    await fetch(env.ALERT_WEBHOOK_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content: message }),
    });
    return true;
  } catch (_) {
    return false;
  }
}

async function gql(env, query) {
  const res = await fetch(GRAPHQL, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.CF_API_TOKEN}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query }),
  });
  if (!res.ok) throw new Error(`graphql ${res.status}`);
  const d = await res.json();
  if (d.errors) throw new Error(`graphql errors: ${JSON.stringify(d.errors).slice(0, 200)}`);
  return d.data.viewer.accounts[0];
}

// ---- collectors ----------------------------------------------------------

// Trailing-hour container usage (billing dataset) grouped per app.
// Bounds are [hourStart, hourEnd) — the trailing complete hour.
async function containerUsageHour(env, hourStartISO, hourEndISO) {
  const acc = await gql(env, `{ viewer { accounts(filter: {accountTag: "${env.CF_ACCOUNT_ID}"}) {
    containersUsageAdaptiveGroups(limit: 100,
      filter: {datetimeHour_geq: "${hourStartISO}", datetimeHour_lt: "${hourEndISO}"}) {
      dimensions { applicationId datetimeHour }
      sum { allocatedMemory allocatedDisk cpuTimeSec txBytes }
    } } } }`);
  const groups = acc.containersUsageAdaptiveGroups || [];
  const apps = {};
  for (const g of groups) {
    const app = (g.dimensions.applicationId || "?").slice(0, 8);
    const s = g.sum;
    const slot = apps[app] = apps[app] || { mem: 0, disk: 0, cpu: 0, tx: 0 };
    slot.mem += s.allocatedMemory || 0;
    slot.disk += s.allocatedDisk || 0;
    slot.cpu += s.cpuTimeSec || 0;
    slot.tx += s.txBytes || 0;
  }
  return apps;
}

// Awake seconds per app for the trailing hour (metrics dataset).
async function containerAwakeHour(env, hourStartISO, hourEndISO) {
  const acc = await gql(env, `{ viewer { accounts(filter: {accountTag: "${env.CF_ACCOUNT_ID}"}) {
    containersMetricsAdaptiveGroups(limit: 200,
      filter: {datetimeHour_geq: "${hourStartISO}", datetimeHour_lt: "${hourEndISO}"}) {
      dimensions { applicationId datetimeHour }
      sum { containerUptime }
    } } } }`);
  const groups = acc.containersMetricsAdaptiveGroups || [];
  const awake = {};
  for (const g of groups) {
    const app = (g.dimensions.applicationId || "?").slice(0, 8);
    awake[app] = (awake[app] || 0) + (g.sum.containerUptime || 0) / 1000.0;
  }
  return awake;
}

// Today's Workers-AI neurons (billing truth; the mint-anchor input).
async function aiNeuronsToday(env, dateISO) {
  const acc = await gql(env, `{ viewer { accounts(filter: {accountTag: "${env.CF_ACCOUNT_ID}"}) {
    aiInferenceAdaptiveGroups(limit: 2, filter: {date_geq: "${dateISO}", date_leq: "${dateISO}"}) {
      sum { totalNeurons }
    } } } }`);
  const g = (acc.aiInferenceAdaptiveGroups || [])[0];
  return g ? (g.sum.totalNeurons || 0) : 0;
}

// ---- the hourly run ------------------------------------------------------

async function runHousekeeping(env) {
  const now = new Date();
  const hourStart = new Date(now.getTime() - 60 * 60 * 1000);
  const hourStartISO = hourStart.toISOString().slice(0, 13) + ":00:00Z";
  const hourEndISO = now.toISOString().slice(0, 13) + ":00:00Z";
  const dateISO = now.toISOString().slice(0, 10);

  const row = {
    kind: "housekeeping",
    ts: now.toISOString(),
    hour: hourStartISO,
    ok: true,
    notes: [],
  };

  if (!env.CF_API_TOKEN) {
    // Heartbeat mode: prove the cron is alive; data collection waits on the
    // one-time owner step (wrangler secret put CF_API_TOKEN).
    row.ok = false;
    row.notes.push("CF_API_TOKEN secret not set — heartbeat only");
    row.mint = "pending-worker-shell";
    row.mint_note = "phase-1 sync surface landed 2026-08-27; live mint awaits the deferred Worker/DO ledger shell (proposal-005 phase-2 defer / phase-3 owner gate)";
    await env.HOUSEKEEPING_KV.put(`hour:${hourKey(now)}`, JSON.stringify(row));
    await env.HOUSEKEEPING_KV.put("latest", JSON.stringify(row));
    return row;
  }

  try {
    const [usage, awake, neurons] = await Promise.all([
      containerUsageHour(env, hourStartISO, hourEndISO),
      containerAwakeHour(env, hourStartISO, hourEndISO),
      aiNeuronsToday(env, dateISO),
    ]);
    let cost = 0;
    const apps = {};
    for (const app of new Set([...Object.keys(usage), ...Object.keys(awake)])) {
      const u = usage[app] || { mem: 0, disk: 0, cpu: 0, tx: 0 };
      const c = costUsd(env, u.mem, u.disk, u.cpu);
      cost += c;
      apps[app] = { awake_s: Math.round(awake[app] || 0), est_usd: +c.toFixed(6) };
    }
    row.containers = { est_usd: +cost.toFixed(6), apps };
    row.ai = { neurons_today: neurons };

    // KAT mint anchor (Issue 045 T3): the daily trainer window's committed
    // ledger rows are the mint function's input. Proposal 005 §Economics is
    // a POLICY ("emission tracks credited work"), not a formula — no KAT
    // amount math lives in this worker. Phase 1 (the sync surface) landed
    // 2026-08-27 (T1.5/T1.6); the LIVE mint awaits the deferred Worker/DO
    // HTTP ledger shell (proposal-005 phase-2 defer / Plan 031 Phase B gate
    // / phase-3 owner gate) — until then we record the anchor only.
    // Swap-in point: this object becomes the mint call's payload.
    row.mint_anchor = {
      date: dateISO,
      neurons_today: neurons,
      kat: null,
      status: "pending-worker-shell",
      note: "phase-1 sync surface landed 2026-08-27; live mint awaits the deferred Worker/DO ledger shell (proposal-005 phase-2 defer / phase-3 owner gate)",
    };
    row.mint = "pending-worker-shell";

    // ---- budget alerting (Issue 044 T4) ----------------------------------
    // Runs only in the token-present path: neurons are only known there.
    // Dedupe key `alert:<date>:<threshold>` is written AFTER the webhook
    // attempt regardless of POST success — a failed POST is NOT retried on
    // the next hourly run (deliberate: no hourly retry storms). One alert
    // per threshold per day.
    const cap = parseFloat(env.NEURON_CAP);
    if (cap > 0 && neurons >= 0) {
      const pct = neurons / cap;
      row.ai.pct = +pct.toFixed(4);
      for (const t of [0.5, 0.9, 1.0]) {
        if (pct < t) continue;
        const dedupeKey = `alert:${dateISO}:${t}`;
        if (await env.HOUSEKEEPING_KV.get(dedupeKey)) continue;
        const pctStr = (pct * 100).toFixed(1);
        let msg;
        if (t === 1.0) {
          msg = `⚠ ops budget alert ${dateISO}: CAP EXCEEDED — ${neurons} neurons (${pctStr}%) of the ${cap}/day cap, new generations hard-fail until 00:00 UTC — ${DASHBOARD_URL}`;
        } else {
          msg = `⚠ ops budget alert ${dateISO}: ${neurons} neurons (${pctStr}%) of the ${cap}/day cap, ${Math.max(0, Math.floor(cap - neurons))} remaining — ${DASHBOARD_URL}`;
        }
        const sent = await notify(env, msg);
        await env.HOUSEKEEPING_KV.put(dedupeKey, JSON.stringify({
          ts: now.toISOString(),
          level: t,
          neurons,
          pct: +pct.toFixed(4),
          notified: sent,
        }));
      }
    }

    // ---- monthly container budget (optional secret) ----------------------
    if (env.MONTHLY_BUDGET_USD) {
      try {
        const ym = dateISO.slice(0, 7).replace("-", ""); // YYYYMM
        let mtd = 0;
        const listed = await env.HOUSEKEEPING_KV.list({ prefix: `day:${ym}` });
        for (const k of listed.keys) {
          try {
            const day = JSON.parse(await env.HOUSEKEEPING_KV.get(k.name) || "{}");
            mtd += day.est_usd || 0;
          } catch (_) { /* tolerate malformed rows */ }
        }
        const budget = parseFloat(env.MONTHLY_BUDGET_USD);
        if (budget > 0 && mtd >= budget) {
          const dedupeKey = `alert:${ym}:budget`;
          if (!(await env.HOUSEKEEPING_KV.get(dedupeKey))) {
            const msg = `⚠ ops budget alert ${ym}: month-to-date container est cost $${mtd.toFixed(2)} ≥ monthly budget $${budget.toFixed(2)} — ${DASHBOARD_URL}`;
            const sent = await notify(env, msg);
            await env.HOUSEKEEPING_KV.put(dedupeKey, JSON.stringify({
              ts: now.toISOString(),
              kind: "monthly_budget",
              mtd_usd: +mtd.toFixed(6),
              budget_usd: budget,
              notified: sent,
            }));
          }
        }
      } catch (_) { /* monthly budget check is best-effort */ }
    }
    // ---- end budget alerting ---------------------------------------------
  } catch (e) {
    row.ok = false;
    row.notes.push(`collect failed: ${e.message}`);
  }

  await env.HOUSEKEEPING_KV.put(`hour:${hourKey(now)}`, JSON.stringify(row));
  await env.HOUSEKEEPING_KV.put("latest", JSON.stringify(row));

  // Day aggregate (rewritten each hour — derived, never authoritative).
  try {
    const dk = dayKey(now);
    const prev = JSON.parse((await env.HOUSEKEEPING_KV.get(`day:${dk}`)) || '{"est_usd":0,"hours":0}');
    await env.HOUSEKEEPING_KV.put(`day:${dk}`, JSON.stringify({
      date: dateISO,
      est_usd: +(Math.max(prev.est_usd || 0, 0) + (row.containers ? row.containers.est_usd : 0)).toFixed(6),
      hours: (prev.hours || 0) + 1,
      updated: now.toISOString(),
    }));
  } catch (_) { /* aggregate is best-effort */ }

  return row;
}

// ---- worker entry points -------------------------------------------------

export default {
  async scheduled(event, env, ctx) {
    ctx.waitUntil(runHousekeeping(env));
  },
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const kv = env.HOUSEKEEPING_KV;
    // Preflight for any path — the read routes are the only CORS consumers,
    // but a global 204 keeps the surface simple and harmless.
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }
    if (url.pathname === "/status") {
      const latest = await kv.get("latest");
      let alerts_today = 0;
      try {
        const listed = await kv.list({ prefix: `alert:${new Date().toISOString().slice(0, 10)}:`, limit: 100 });
        alerts_today = listed.keys.length;
      } catch (_) { /* best-effort count */ }
      return cors(Response.json({
        worker: "riir-kat-housekeeping",
        cron: "0 * * * *",
        token_set: !!env.CF_API_TOKEN,
        cap: parseFloat(env.NEURON_CAP) || 0,
        thresholds: [0.5, 0.9, 1.0],
        has_webhook: !!env.ALERT_WEBHOOK_URL,
        has_monthly_budget: !!env.MONTHLY_BUDGET_USD,
        alerts_today,
        latest: latest ? JSON.parse(latest) : null,
      }));
    }
    if (url.pathname === "/latest") {
      const v = await kv.get("latest");
      return cors(new Response(v || "null", { headers: { "Content-Type": "application/json" } }));
    }
    if (url.pathname === "/hours" || url.pathname === "/days") {
      const n = Math.min(parseInt(url.searchParams.get("n") || "24", 10) || 24, 720);
      const keys = [];
      const now = new Date();
      for (let i = 0; i < n; i++) {
        const d = new Date(now.getTime() - (url.pathname === "/hours" ? i * 3600e3 : i * 86400e3));
        keys.push((url.pathname === "/hours" ? "hour:" : "day:") +
          (url.pathname === "/hours" ? hourKey(d) : dayKey(d)));
      }
      const rows = await Promise.all(keys.map(async (k) => ({ key: k, row: await kv.get(k) })));
      return cors(Response.json(rows.filter((r) => r.row !== null)
        .map((r) => JSON.parse(r.row))));
    }
    return new Response("riir-kat-housekeeping (issue 045) — /status /latest /hours?n= /days?n=",
      { status: 404 });
  },
};
