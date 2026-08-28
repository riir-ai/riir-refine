// riir-kat-housekeeping — Issue 045 (riir-clippy).
// Hourly cron: container cost snapshot + KAT mint anchor → KV.
// Deliberately dependency-free (no build step; wrangler deploys this file).

const GRAPHQL = "https://api.cloudflare.com/client/v4/graphql";

const DASHBOARD_URL = "https://riir-ops-dashboard.foxfox.workers.dev/";

// Read-route limits. `/hours?n=` used to accept n=720 and fan that out as
// 720 unbounded concurrent KV gets on an UNAUTHENTICATED route: 138 requests
// (~14 s of trivial traffic) exhaust the 100k/day KV read tier, which then
// breaks the dashboard AND makes the alert-dedupe `get` throw — degrading
// budget alerting exactly when it matters. Cap the window and the fan-out.
const MAX_HOURS = 168;   // 7 d of hourly rows
const MAX_DAYS = 90;     // a quarter of daily rows
const KV_CONCURRENCY = 16;

// CORS: echo only an allowlisted origin (was `*`, which let any page in any
// browser read this account's cost telemetry cross-origin).
function allowedOrigins(env) {
  return (env.ALLOWED_ORIGINS || DASHBOARD_URL)
    .split(",").map((o) => o.trim().replace(/\/$/, "")).filter(Boolean);
}

function corsHeaders(request, env) {
  const origin = request.headers.get("Origin");
  const h = {
    "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
    "Access-Control-Allow-Headers": "Authorization",
    "Access-Control-Max-Age": "86400",
    Vary: "Origin, Authorization",
  };
  if (origin && allowedOrigins(env).includes(origin.replace(/\/$/, ""))) {
    h["Access-Control-Allow-Origin"] = origin;
  }
  return h;
}

function cors(response, request, env, cacheSeconds) {
  const r = new Response(response.body, response);
  for (const [k, v] of Object.entries(corsHeaders(request, env))) r.headers.set(k, v);
  if (cacheSeconds) r.headers.set("Cache-Control", `public, max-age=${cacheSeconds}`);
  return r;
}

// Length-independent compare so a wrong token cannot be recovered byte-wise
// from response timing.
function timingSafeEqual(a, b) {
  const ea = new TextEncoder().encode(a), eb = new TextEncoder().encode(b);
  let diff = ea.length ^ eb.length;
  const n = Math.max(ea.length, eb.length);
  for (let i = 0; i < n; i++) diff |= (ea[i] || 0) ^ (eb[i] || 0);
  return diff === 0;
}

// Detail routes are gated ONLY when READ_TOKEN is set — fails OPEN until the
// owner sets it, matching the existing DASHBOARD_TOKEN posture. `/latest`
// stays public but returns a minimised projection (below) either way, so the
// public surface never carries per-app ids or cost breakdown.
function authed(request, env) {
  if (!env.READ_TOKEN) return true;
  const h = request.headers.get("Authorization") || "";
  const m = h.match(/^Bearer\s+(.+)$/);
  return !!m && timingSafeEqual(m[1], env.READ_TOKEN);
}

// Bounded-concurrency map — replaces Promise.all over an attacker-sized list.
async function mapLimit(items, limit, fn) {
  const out = new Array(items.length);
  let i = 0;
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (i < items.length) {
      const idx = i++;
      out[idx] = await fn(items[idx], idx);
    }
  }));
  return out;
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
      // A hanging webhook must not stall the cron run.
      signal: AbortSignal.timeout(5000),
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
    signal: AbortSignal.timeout(10000),
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
    // Full 36-char id, not a truncated stub: the `[:8]` form is not
    // addressable against the API and caused the Issue 045 T5 misdiagnosis.
    const app = g.dimensions.applicationId || "?";
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
    const app = g.dimensions.applicationId || "?";
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
    // allSettled, not all: one failing dataset used to discard the other two
    // and record a bare ok=false row, losing an hour of collectable data.
    const [rUsage, rAwake, rNeurons] = await Promise.allSettled([
      containerUsageHour(env, hourStartISO, hourEndISO),
      containerAwakeHour(env, hourStartISO, hourEndISO),
      aiNeuronsToday(env, dateISO),
    ]);
    for (const [label, r] of [["usage", rUsage], ["awake", rAwake], ["neurons", rNeurons]]) {
      if (r.status === "rejected") {
        row.ok = false;
        row.notes.push(`${label} failed: ${(r.reason && r.reason.message) || r.reason}`);
      }
    }
    const usage = rUsage.status === "fulfilled" ? rUsage.value : {};
    const awake = rAwake.status === "fulfilled" ? rAwake.value : {};
    const neurons = rNeurons.status === "fulfilled" ? rNeurons.value : null;
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
    if (cap > 0 && typeof neurons === "number" && neurons >= 0) {
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
        const vals = await mapLimit(listed.keys.map((k) => k.name), KV_CONCURRENCY,
          (n) => env.HOUSEKEEPING_KV.get(n));
        for (const v of vals) {
          try {
            mtd += (JSON.parse(v || "{}").est_usd) || 0;
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

  // Day aggregate — RECOMPUTED from the hour rows, not accumulated.
  // The previous read-modify-write (`prev.est_usd + this hour`) was both
  // non-idempotent (a cron double-fire or a manual invoke double-counted the
  // hour) and a lost-update race on an eventually-consistent store. Summing
  // the authoritative hour:<dk>* rows is idempotent and converges.
  try {
    const dk = dayKey(now);
    const listed = await env.HOUSEKEEPING_KV.list({ prefix: `hour:${dk}` });
    const names = listed.keys.map((k) => k.name);
    const raw = await mapLimit(names, KV_CONCURRENCY, (n) => env.HOUSEKEEPING_KV.get(n));
    let est = 0, hours = 0;
    for (const r of raw) {
      if (!r) continue;
      try {
        const h = JSON.parse(r);
        hours += 1;
        est += (h.containers && h.containers.est_usd) || 0;
      } catch (_) { /* tolerate a malformed hour row */ }
    }
    await env.HOUSEKEEPING_KV.put(`day:${dk}`, JSON.stringify({
      date: dateISO,
      est_usd: +est.toFixed(6),
      hours,
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
    const ok = authed(request, env);

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders(request, env) });
    }
    if (request.method !== "GET" && request.method !== "HEAD") {
      return cors(new Response("method not allowed", { status: 405 }), request, env);
    }

    if (url.pathname === "/status") {
      // Liveness is public (the dashboard badge needs it). Config posture —
      // cap, thresholds, whether alerting/webhook/budget are wired, how many
      // alerts fired — is operational detail and goes behind the token.
      const latest = await kv.get("latest");
      const parsed = latest ? JSON.parse(latest) : null;
      const body = {
        worker: "riir-kat-housekeeping",
        cron: "0 * * * *",
        ts: parsed ? parsed.ts : null,
        ok: parsed ? parsed.ok : null,
      };
      if (ok) {
        let alerts_today = 0;
        try {
          const listed = await kv.list({
            prefix: `alert:${new Date().toISOString().slice(0, 10)}:`, limit: 100 });
          alerts_today = listed.keys.length;
        } catch (_) { /* best-effort count */ }
        Object.assign(body, {
          token_set: !!env.CF_API_TOKEN,
          cap: parseFloat(env.NEURON_CAP) || 0,
          thresholds: [0.5, 0.9, 1.0],
          has_webhook: !!env.ALERT_WEBHOOK_URL,
          has_monthly_budget: !!env.MONTHLY_BUDGET_USD,
          read_gated: !!env.READ_TOKEN,
          alerts_today,
          latest: parsed,
        });
      }
      return cors(Response.json(body), request, env, 60);
    }

    if (url.pathname === "/latest") {
      // Public projection: exactly the fields the dashboard live badge and
      // cap alert consume (ts / ok / neurons_today / pct). Per-app ids and
      // the cost breakdown are account telemetry — token only.
      //
      // NOTE this route redacts UNCONDITIONALLY for an unauthenticated
      // caller — unlike /hours, /days and the /status config half, it does
      // NOT fail open when READ_TOKEN is unset. It can afford not to: the
      // projection is by construction everything the dashboard consumes, so
      // hard-gating the detail costs no functionality. The other routes fail
      // open so the owner keeps a diagnostic path (notably `token_set`, the
      // check for whether the T4 CF_API_TOKEN step landed) before any token
      // exists to present.
      const v = await kv.get("latest");
      if (!v) return cors(new Response("null", { status: 200,
        headers: { "Content-Type": "application/json" } }), request, env, 60);
      const full = JSON.parse(v);
      const hasToken = !!env.READ_TOKEN && ok;
      if (hasToken) return cors(Response.json(full), request, env, 60);
      return cors(Response.json({
        ts: full.ts,
        hour: full.hour,
        ok: full.ok,
        mint: full.mint,
        ai: full.ai ? { neurons_today: full.ai.neurons_today, pct: full.ai.pct } : undefined,
        containers: full.containers ? { est_usd: full.containers.est_usd } : undefined,
        redacted: true,
      }), request, env, 60);
    }

    if (url.pathname === "/hours" || url.pathname === "/days") {
      if (!ok) return cors(new Response("unauthorized", { status: 401 }), request, env);
      const isHours = url.pathname === "/hours";
      const max = isHours ? MAX_HOURS : MAX_DAYS;
      const req = parseInt(url.searchParams.get("n") || "24", 10);
      const n = Math.min(Number.isFinite(req) && req > 0 ? req : 24, max);
      const now = new Date();
      const keys = Array.from({ length: n }, (_, i) => {
        const d = new Date(now.getTime() - i * (isHours ? 3600e3 : 86400e3));
        return (isHours ? `hour:${hourKey(d)}` : `day:${dayKey(d)}`);
      });
      const raw = await mapLimit(keys, KV_CONCURRENCY, (k) => kv.get(k));
      const rows = [];
      for (const r of raw) {
        if (!r) continue;
        try { rows.push(JSON.parse(r)); } catch (_) { /* skip malformed */ }
      }
      return cors(Response.json(rows), request, env, 300);
    }

    return cors(new Response(
      "riir-kat-housekeeping (issue 045) — /status /latest /hours?n= /days?n=",
      { status: 404 }), request, env);
  },
};
