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

// `authenticated` selects the cache posture, and it is not cosmetic.
//
// RFC 9111 §3.5: a shared cache MUST NOT reuse a response to a request
// carrying an Authorization header UNLESS the response says `public` /
// `s-maxage` / `must-revalidate`. So `public` is not a neutral hint here — it
// is the exact directive that GRANTS shared caches the storage the default
// would deny, and this worker was stamping it on the token-bearing full rows
// (per-app ids, per-app cost, the mint anchor) on /latest, /status, /hours
// and /days. `Vary: Authorization` keeps a *conformant* cache from serving
// one caller's body to another, but it is a mitigation layered on an
// unnecessary grant, and it depends on every intermediary honouring Vary.
//
// The isolate memo already got this right — "an authenticated body must never
// be served from a cache an unauthenticated caller can also hit" — and then
// the HTTP header handed the same body to every cache on the path. Same bug
// class, one layer down. Authenticated responses are now `private, no-store`;
// only the redacted anonymous projection stays publicly cacheable.
function cors(response, request, env, cacheSeconds, authenticated) {
  const r = new Response(response.body, response);
  for (const [k, v] of Object.entries(corsHeaders(request, env))) r.headers.set(k, v);
  if (authenticated) r.headers.set("Cache-Control", "private, no-store");
  else if (cacheSeconds) r.headers.set("Cache-Control", `public, max-age=${cacheSeconds}`);
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

// The public projection of an hour row: exactly the fields the dashboard
// badge and cap alert consume. Per-app ids, per-app cost, the mint anchor and
// the collector notes are account telemetry and need a presented token.
//
// This is applied by /latest AND by /hours AND by the /status `latest` echo.
// It previously lived only in /latest, so `GET /hours?n=1` and `GET /status`
// both returned the full row to an unauthenticated caller — two silent
// bypasses of the redaction /latest documents as unconditional. Harmless
// while the rows are heartbeats; it arms itself the moment T4 sets
// CF_API_TOKEN and the rows start carrying per-app breakdown.
function publicRow(r) {
  if (!r) return r;
  return {
    ts: r.ts,
    hour: r.hour,
    ok: r.ok,
    mint: r.mint,
    ai: r.ai ? { neurons_today: r.ai.neurons_today, pct: r.ai.pct } : undefined,
    containers: r.containers ? { est_usd: r.containers.est_usd } : undefined,
    redacted: true,
  };
}

// Per-isolate response memo for the KV fan-out routes.
//
// NOT the Cache API, deliberately. Checked the docs rather than assuming
// either way: developers.cloudflare.com/workers/runtime-apis/cache/ names the
// three contexts where Cache API operations do nothing (dashboard editor,
// Playground previews, Access-fronted Workers) and *.workers.dev is not among
// them — but the availability sentence enumerates custom domains and Pages and
// stays SILENT on workers.dev. That is an omission, not a grant. This worker
// is on workers.dev, so a `caches.default` win here would rest on an
// unconfirmed premise.
//
// An isolate-local memo needs no such premise. Past hour rows are immutable
// once written, so a short TTL is safe, and it collapses repeated fan-outs
// within an isolate instead of re-issuing up to 168 KV gets per request.
const MEMO_TTL_MS = 60000;
const MEMO_MAX = 64;
const memo = new Map();

function memoGet(k) {
  const e = memo.get(k);
  if (e && e.exp > Date.now()) return e.v;
  if (e) memo.delete(k);
  return null;
}

function memoPut(k, v) {
  if (memo.size >= MEMO_MAX) memo.clear();
  memo.set(k, { v, exp: Date.now() + MEMO_TTL_MS });
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

// YYYYMMDDHH. `utc()` yields YYYYMMDD"T"HHMMSS, so the old `slice(0, 10)`
// took the literal "T" plus ONE hour digit: every hour 00-09 wrote
// `hour:YYYYMMDDT0`, 10-19 wrote `T1`, 20-23 wrote `T2`. 24 hourly runs
// collapsed into 3 keys, each overwriting the last. Measured in prod
// 2026-08-28 before the fix: `day:20260828` = {"hours":1} after 13 cron
// runs, and `/hours?n=24` returned 24 rows with 4 distinct timestamps.
// The day aggregate — the cost number this whole issue exists to report —
// was therefore summing ~3 of 24 hours, understating daily spend ~8x.
function hourKey(d) { const u = utc(d); return u.slice(0, 8) + u.slice(9, 11); }
function dayKey(d) { return utc(d).slice(0, 8); }          // YYYYMMDD

// Strict shape of a post-fix hour key. The legacy colliding keys
// (`hour:YYYYMMDDT<digit>`) still match the day aggregate's `hour:YYYYMMDD`
// prefix scan, so without this filter they would be summed ALONGSIDE the
// correct rows — a double count layered on top of the undercount.
const HOUR_KEY_RE = /^hour:\d{10}$/;

// Does this hour row carry a REAL collection, or is it a placeholder?
//
// `ok` is load-bearing, not decorative. Two kinds of row have a `containers`
// object and no data behind it: the heartbeat row written while CF_API_TOKEN
// is unset (no `containers` at all), and the all-collectors-failed row, which
// DOES get `containers: {est_usd: 0, apps: {}}` because the collectors return
// empty maps rather than throwing. Treating that false zero as data would
// wedge the hour shut forever. A row that says ok:false is telling us it did
// not collect; take it at its word.
function hasCollectedData(row) {
  return !!(row && row.ok === true && row.containers);
}

// Marker written alongside every hour row so the backfill can decide
// protect-vs-replace straight from `list()`, which returns per-key metadata —
// the KV docs recommend exactly this over a list()+get() pair. Legacy rows
// (written before this deploy) carry no metadata; the backfill falls back to
// a `get` for those and only those.
// Two bits, because the backfill asks two different questions:
//   d — does this row carry a real collection? (protect it)
//   m — does it carry anything worth MERGING FORWARD if it is replaced?
//       A failed-collect row can hold a real neuron count, because the AI
//       collector may have succeeded while the container ones failed.
// Without `m` the metadata shortcut would be self-defeating: skipping the
// `get` also discards the body we would need to merge. With it, the common
// case — a pure heartbeat, which is 100% of the current archive — still
// costs zero gets, and only the rare partial row is fetched.
function hourMeta(row) {
  return {
    metadata: {
      d: hasCollectedData(row) ? 1 : 0,
      m: row && (row.ai || row.mint_anchor) ? 1 : 0,
    },
  };
}

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

// `datetimeHour` comes back as an ISO-ish "YYYY-MM-DDTHH:00:00Z". Derive the
// key by SLICING THE STRING, never via `new Date(...)`: V8 parses the
// space-separated / unsuffixed variant as LOCAL time, which would shift every
// backfilled row by the host offset — the same one-hour class of defect T8
// already cost us once. Returns null for a shape we do not recognise; the
// caller skips those groups rather than filing them under a guessed hour.
function hourKeyFromDim(v, fallback) {
  const m = String(v || "").match(/^(\d{4})-(\d{2})-(\d{2})[T ](\d{2})/);
  if (m) return m[1] + m[2] + m[3] + m[4];
  // Unparseable or absent. The hourly cron KNOWS the answer — its window is
  // one hour wide — so it passes that hour as the fallback and keeps the row.
  // The backfill cannot know, so it passes nothing and the group is dropped:
  // filing a whole day's usage under a guessed hour is worse than losing it.
  return fallback || null;
}

// Container usage (billing dataset), grouped per HOUR and then per app.
//
// One shape serves both callers: the hourly cron passes a 1-hour window and
// reads the single bucket, the backfill passes a whole day and reads 24. The
// `datetimeHour` dimension was ALREADY being requested here and then summed
// away — so a wide window silently collapsed into one bucket and the dataset's
// per-hour truth was discarded at the door. Keying by it is what makes the
// T9 backfill possible at all.
async function containerUsageByHour(env, startISO, endISO, limit, fallbackHk) {
  const acc = await gql(env, `{ viewer { accounts(filter: {accountTag: "${env.CF_ACCOUNT_ID}"}) {
    containersUsageAdaptiveGroups(limit: ${limit},
      filter: {datetimeHour_geq: "${startISO}", datetimeHour_lt: "${endISO}"}) {
      dimensions { applicationId datetimeHour }
      sum { allocatedMemory allocatedDisk cpuTimeSec txBytes }
    } } } }`);
  const byHour = new Map();
  for (const g of acc.containersUsageAdaptiveGroups || []) {
    const hk = hourKeyFromDim(g.dimensions.datetimeHour, fallbackHk);
    if (!hk) continue;
    // Full 36-char id, not a truncated stub: the `[:8]` form is not
    // addressable against the API and caused the Issue 045 T5 misdiagnosis.
    const app = g.dimensions.applicationId || "?";
    let apps = byHour.get(hk);
    if (!apps) byHour.set(hk, apps = {});
    const s = g.sum;
    const slot = apps[app] = apps[app] || { mem: 0, disk: 0, cpu: 0, tx: 0 };
    slot.mem += s.allocatedMemory || 0;
    slot.disk += s.allocatedDisk || 0;
    slot.cpu += s.cpuTimeSec || 0;
    slot.tx += s.txBytes || 0;
  }
  return byHour;
}

// Awake seconds per app, grouped per hour (metrics dataset). Same contract.
async function containerAwakeByHour(env, startISO, endISO, limit, fallbackHk) {
  const acc = await gql(env, `{ viewer { accounts(filter: {accountTag: "${env.CF_ACCOUNT_ID}"}) {
    containersMetricsAdaptiveGroups(limit: ${limit},
      filter: {datetimeHour_geq: "${startISO}", datetimeHour_lt: "${endISO}"}) {
      dimensions { applicationId datetimeHour }
      sum { containerUptime }
    } } } }`);
  const byHour = new Map();
  for (const g of acc.containersMetricsAdaptiveGroups || []) {
    const hk = hourKeyFromDim(g.dimensions.datetimeHour, fallbackHk);
    if (!hk) continue;
    const app = g.dimensions.applicationId || "?";
    let apps = byHour.get(hk);
    if (!apps) byHour.set(hk, apps = {});
    apps[app] = (apps[app] || 0) + (g.sum.containerUptime || 0) / 1000.0;
  }
  return byHour;
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
  // Key by the hour the row DESCRIBES (hourStart), not by the run time.
  // The run at 12:00 collects 11:00-12:00, so keying on `now` filed that
  // row under 12 — every point on the hourly chart was shifted one hour off
  // its own `row.hour` field. Same reason the day aggregate below keys on
  // hourStart: the 00:00 run collects the PREVIOUS day's 23:00 hour, and
  // aggregating `dayKey(now)` left that last hour permanently unsummed.
  const hk = hourKey(hourStart);
  const dk = dayKey(hourStart);

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
    await env.HOUSEKEEPING_KV.put(`hour:${hk}`, JSON.stringify(row), hourMeta(row));
    await env.HOUSEKEEPING_KV.put("latest", JSON.stringify(row));
    return row;
  }

  try {
    // allSettled, not all: one failing dataset used to discard the other two
    // and record a bare ok=false row, losing an hour of collectable data.
    const [rUsage, rAwake, rNeurons] = await Promise.allSettled([
      containerUsageByHour(env, hourStartISO, hourEndISO, 100, hk),
      containerAwakeByHour(env, hourStartISO, hourEndISO, 200, hk),
      aiNeuronsToday(env, dateISO),
    ]);
    for (const [label, r] of [["usage", rUsage], ["awake", rAwake], ["neurons", rNeurons]]) {
      if (r.status === "rejected") {
        row.ok = false;
        row.notes.push(`${label} failed: ${(r.reason && r.reason.message) || r.reason}`);
      }
    }
    // One-hour window => at most one bucket; `|| {}` covers an idle hour.
    const usage = (rUsage.status === "fulfilled" ? rUsage.value.get(hk) : null) || {};
    const awake = (rAwake.status === "fulfilled" ? rAwake.value.get(hk) : null) || {};
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
        const dedupeKey = `alert:${ym}:budget`;
        // Check the dedupe key BEFORE the scan, not after. The alert fires at
        // most once a month, but the scan under it — 1 KV list + up to 31 gets
        // — was running EVERY HOUR for the rest of the month to recompute a
        // number that could only re-fire an alert already sent. Workers count
        // every KV binding call as a subrequest, so this was ~32 of them per
        // run, hourly, to reach a guaranteed no-op.
        const alreadyAlerted = await env.HOUSEKEEPING_KV.get(dedupeKey);
        let mtd = 0;
        const listed = alreadyAlerted
          ? { keys: [] }
          : await env.HOUSEKEEPING_KV.list({ prefix: `day:${ym}` });
        const vals = await mapLimit(listed.keys.map((k) => k.name), KV_CONCURRENCY,
          (n) => env.HOUSEKEEPING_KV.get(n));
        for (const v of vals) {
          try {
            mtd += (JSON.parse(v || "{}").est_usd) || 0;
          } catch (_) { /* tolerate malformed rows */ }
        }
        const budget = parseFloat(env.MONTHLY_BUDGET_USD);
        if (!alreadyAlerted && budget > 0 && mtd >= budget) {
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
      } catch (_) { /* monthly budget check is best-effort */ }
    }
    // ---- end budget alerting ---------------------------------------------
  } catch (e) {
    row.ok = false;
    row.notes.push(`collect failed: ${e.message}`);
  }

  await env.HOUSEKEEPING_KV.put(`hour:${hk}`, JSON.stringify(row), hourMeta(row));
  await env.HOUSEKEEPING_KV.put("latest", JSON.stringify(row));

  // Day aggregate — one definition, shared with the backfill (see
  // recomputeDay). Seeded with the row we just wrote so we neither re-read it
  // nor depend on KV `list` having caught up with the `put` above.
  try {
    await recomputeDay(env, dk, {
      [`hour:${hk}`]: (row.containers && row.containers.est_usd) || 0,
    });
  } catch (_) { /* aggregate is best-effort */ }

  return row;
}

// Day aggregate — RECOMPUTED from the hour rows, not accumulated.
// The previous read-modify-write (`prev.est_usd + this hour`) was both
// non-idempotent (a cron double-fire or a manual invoke double-counted the
// hour) and a lost-update race on an eventually-consistent store. Summing the
// authoritative hour:<dk>* rows is idempotent and converges.
//
// `seed` maps an already-in-memory `hour:` key to its est_usd. It buys two
// things, not one:
//   * a skipped KV get per seeded row (the cron seeds 1, the backfill seeds
//     every hour it just wrote — up to 24 gets saved against a subrequest cap
//     that counts every KV binding call), and
//   * correctness under KV's eventual consistency. `list` is NOT read-your-
//     writes: the cron previously listed immediately after putting `hour:<hk>`
//     and could legitimately not see it, silently dropping the very hour it
//     had just collected from that hour's own day total. Union-ing the seed
//     keys with the listed names closes that window.
async function recomputeDay(env, dk, seed) {
  const listed = await env.HOUSEKEEPING_KV.list({ prefix: `hour:${dk}` });
  // Legacy collided keys (`hour:YYYYMMDDT<digit>`) share this prefix; sum
  // only well-formed YYYYMMDDHH rows or the fix double-counts its own past.
  const names = new Set(listed.keys.map((k) => k.name).filter((n) => HOUR_KEY_RE.test(n)));
  for (const n of Object.keys(seed)) if (HOUR_KEY_RE.test(n)) names.add(n);
  const need = [...names].filter((n) => !(n in seed));
  const raw = await mapLimit(need, KV_CONCURRENCY, (n) => env.HOUSEKEEPING_KV.get(n));
  const fetched = new Map(need.map((n, i) => [n, raw[i]]));
  let est = 0, hours = 0;
  for (const n of names) {
    if (n in seed) { est += seed[n] || 0; hours += 1; continue; }
    const r = fetched.get(n);
    if (!r) continue;
    try {
      const h = JSON.parse(r);
      hours += 1;
      est += (h.containers && h.containers.est_usd) || 0;
    } catch (_) { /* tolerate a malformed hour row */ }
  }
  const out = {
    date: `${dk.slice(0, 4)}-${dk.slice(4, 6)}-${dk.slice(6, 8)}`,
    est_usd: +est.toFixed(6),
    hours,
    updated: new Date().toISOString(),
  };
  await env.HOUSEKEEPING_KV.put(`day:${dk}`, JSON.stringify(out));
  return out;
}

// ---- backfill (Issue 045 T9) ---------------------------------------------
//
// T8 recorded the collapsed hours as UNRECOVERABLE. That is true of the KV
// ROWS and false of the DATA: KV here is a cache of a source that is still
// live. Cloudflare's GraphQL analytics datasets retain ~31 days, and
// `datetimeHour` — the dimension the collectors were already requesting and
// then summing away — is exactly the per-hour truth the archive lost. So the
// same deploy that stops the bleeding can also repair the past.
//
// Invariants, in order of how much they would cost to get wrong:
//
//  * Never overwrite an hour row that CARRIES A REAL COLLECTION. Such a row
//    has `ai` + `mint_anchor`, neither of which is per-hour reconstructible
//    from billing, so clobbering it would destroy information in the name of
//    repairing it. But "exists" is NOT the same test as "carries data", and
//    getting that wrong was a live bug (found 2026-08-29 by reading prod:
//    `/status` says `token_set:false`, so EVERY archived row is a heartbeat).
//    An exists-based rule would have declared the whole archive protected and
//    silently repaired nothing — the placeholder rows written between this
//    deploy and T4 would each wedge their hour shut permanently. The rule is
//    `hasCollectedData()`; a heartbeat or failed-collect row is replaceable,
//    and anything worth keeping on it is merged forward.
//  * Idempotent. Existence is decided by ONE KV `list` per day rather than 24
//    `get`s — 24x cheaper against both the subrequest cap and the 1,000/day
//    list tier that T8's SEC/PERF pass was already defending. A second run
//    over the same window writes nothing and costs 3 subrequests per day.
//  * Bounded. Workers count EVERY KV binding call as a subrequest (50 free,
//    1,000+ paid), so an unbounded "repair the month" call dies mid-write
//    with a partially-written day. Hence MAX_BACKFILL_DAYS plus a running
//    budget that stops on a day boundary and hands back a `next_end` cursor.
//  * The day aggregate is recomputed only for days that actually changed, and
//    only via recomputeDay() — the same function the cron uses. Two
//    implementations of "what a day costs" is how the numbers drift apart.
const MAX_BACKFILL_DAYS = 14;
const GRAPHQL_RETENTION_DAYS = 31;
// Worst case for one changed day: 1 list + 2 gql + 24 puts + 24 gets + 1 put.
const DAY_SUBREQUEST_WORST_CASE = 52;
const BACKFILL_SUBREQUEST_BUDGET = 700;
// Cloudflare's edge closes a client connection that waits too long (524), and
// a 14-day window is 28 sequential GraphQL round-trips. A timeout here is
// SAFE — every KV write already committed and the route is idempotent, so the
// owner can just re-run — but it is indistinguishable from a broken worker
// and it loses the cursor. Stopping ourselves at 45 s turns that into a clean
// `truncated:true` + `next_end` the runbook already knows how to resume from.
const BACKFILL_WALL_CLOCK_MS = 45000;

async function backfillRange(env, endDay, days) {
  const report = { written: 0, skipped: 0, truncated: false, next_end: null, per_day: [] };
  const endMs = Date.UTC(+endDay.slice(0, 4), +endDay.slice(5, 7) - 1, +endDay.slice(8, 10));
  const started = Date.now();
  let spent = 0;

  for (let i = 0; i < days; i++) {
    const d = new Date(endMs - i * 86400e3);
    const dateISO = d.toISOString().slice(0, 10);
    const dk = dayKey(d);
    const outOfTime = Date.now() - started > BACKFILL_WALL_CLOCK_MS;
    if (outOfTime || spent + DAY_SUBREQUEST_WORST_CASE > BACKFILL_SUBREQUEST_BUDGET) {
      // Stop on a day boundary, never mid-day: a half-written day would look
      // complete to the aggregate. The cursor lets the owner resume exactly.
      report.truncated = true;
      report.truncated_by = outOfTime ? "wall_clock" : "subrequest_budget";
      report.next_end = dateISO;
      break;
    }

    const startISO = `${dateISO}T00:00:00Z`;
    const endISO = `${new Date(d.getTime() + 86400e3).toISOString().slice(0, 10)}T00:00:00Z`;

    const listed = await env.HOUSEKEEPING_KV.list({ prefix: `hour:${dk}` });
    spent += 1;
    // Partition the existing rows into protected (real collection) and
    // replaceable (heartbeat / failed collect). The marker rides in `list()`
    // metadata, so this costs nothing for any row written by this build;
    // only pre-marker legacy rows need a `get` to classify.
    const existing = listed.keys.filter((k) => HOUR_KEY_RE.test(k.name));
    // Fetch a body only when the marker cannot answer on its own: an unmarked
    // legacy row (classify it), or a marked placeholder that has something to
    // merge forward. A pure heartbeat needs neither.
    const toFetch = existing.filter((k) =>
      !k.metadata || (!k.metadata.d && k.metadata.m));
    const probed = await mapLimit(toFetch, KV_CONCURRENCY,
      (k) => env.HOUSEKEEPING_KV.get(k.name));
    spent += toFetch.length;
    const bodies = new Map(toFetch.map((k, i) => [k.name, probed[i]]));
    const protectedRows = new Set();
    const replaceable = new Map();
    for (const k of existing) {
      let row = null;
      if (bodies.has(k.name)) {
        try { row = JSON.parse(bodies.get(k.name) || "null"); } catch (_) { /* malformed */ }
      }
      const isData = k.metadata ? !!k.metadata.d : hasCollectedData(row);
      if (isData) protectedRows.add(k.name);
      else replaceable.set(k.name, row);
    }

    // allSettled, matching the cron: a metrics outage must not cost us the
    // billing rows, which are the ones the cost number is built from.
    const [rU, rA] = await Promise.allSettled([
      containerUsageByHour(env, startISO, endISO, 2000),
      containerAwakeByHour(env, startISO, endISO, 2000),
    ]);
    spent += 2;
    if (rU.status === "rejected" && rA.status === "rejected") {
      report.per_day.push({ day: dk, error: String((rU.reason && rU.reason.message) || rU.reason) });
      continue;
    }
    const usage = rU.status === "fulfilled" ? rU.value : new Map();
    const awake = rA.status === "fulfilled" ? rA.value : new Map();

    const seed = {};
    let wrote = 0, skipped = 0;
    for (const hk of new Set([...usage.keys(), ...awake.keys()])) {
      // Defensive: only ever write inside the day whose keys we just listed.
      // A row outside it was not existence-checked, so writing it could
      // clobber a live-cron row in an adjacent day.
      if (!hk.startsWith(dk)) continue;
      const name = `hour:${hk}`;
      if (protectedRows.has(name)) { skipped += 1; continue; }
      const u = usage.get(hk) || {};
      const a = awake.get(hk) || {};
      let cost = 0;
      const apps = {};
      for (const app of new Set([...Object.keys(u), ...Object.keys(a)])) {
        const g = u[app] || { mem: 0, disk: 0, cpu: 0, tx: 0 };
        const c = costUsd(env, g.mem, g.disk, g.cpu);
        cost += c;
        apps[app] = { awake_s: Math.round(a[app] || 0), est_usd: +c.toFixed(6) };
      }
      // Replacing a placeholder must not lose whatever it did carry. A
      // heartbeat has no `ai`, but a failed-collect row can hold a real
      // neuron count (that collector may have succeeded while the container
      // ones failed) — so merge forward rather than overwrite wholesale.
      const prior = replaceable.get(name) || null;
      const row = {
        kind: "housekeeping",
        source: "backfill",
        ts: new Date().toISOString(),
        hour: `${hk.slice(0, 4)}-${hk.slice(4, 6)}-${hk.slice(6, 8)}T${hk.slice(8, 10)}:00:00Z`,
        ok: true,
        notes: ["backfilled from GraphQL billing rows; ai + mint_anchor are not per-hour reconstructible"],
        containers: { est_usd: +cost.toFixed(6), apps },
        ai: (prior && prior.ai) || null,
        mint: (prior && prior.mint) || "n/a-backfill",
      };
      if (prior && prior.mint_anchor) row.mint_anchor = prior.mint_anchor;
      // Provenance is recorded from the KEY's presence, not the body — a pure
      // heartbeat is deliberately never fetched, so `prior` is null for it.
      if (replaceable.has(name)) {
        row.replaced = prior
          ? { ts: prior.ts, ok: prior.ok, notes: prior.notes }
          : { placeholder: true };
      }
      await env.HOUSEKEEPING_KV.put(name, JSON.stringify(row), hourMeta(row));
      spent += 1;
      seed[name] = row.containers.est_usd;
      wrote += 1;
    }

    const entry = { day: dk, written: wrote, skipped };
    if (wrote > 0) {
      try {
        entry.day_row = await recomputeDay(env, dk, seed);
      } catch (e) {
        entry.aggregate_error = String((e && e.message) || e);
      }
      spent += 2 + existing.length;
    }
    report.per_day.push(entry);
    report.written += wrote;
    report.skipped += skipped;
  }

  report.subrequests_spent = spent;
  report.elapsed_ms = Date.now() - started;
  return report;
}

// ---- worker entry points -------------------------------------------------

export default {
  async scheduled(event, env, ctx) {
    // runHousekeeping's internal try covers COLLECTION; the KV writes after it
    // are outside that guard, so a KV outage rejected straight into
    // waitUntil() and the run vanished — no row, no log, and (with
    // observability unset in wrangler.toml) nothing in the dashboard either.
    // For a worker whose every documented failure mode is silence, an
    // unobserved throw is the worst possible shape.
    ctx.waitUntil(runHousekeeping(env).catch((e) => {
      console.error("housekeeping run failed", (e && e.stack) || String(e));
    }));
  },
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const kv = env.HOUSEKEEPING_KV;
    // Two gate levels, deliberately distinct:
    //   ok       — fail-OPEN while READ_TOKEN is unset (the diagnostic path
    //              the owner needs to verify the T4 secret step landed).
    //   hasToken — hard: a token must be SET and PRESENTED. Anything that is
    //              account telemetry, or that costs a KV list, needs this.
    const ok = authed(request, env);
    const hasToken = !!env.READ_TOKEN && ok;

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders(request, env) });
    }
    if (url.pathname === "/backfill") {
      // The only WRITE route on this worker, and the only one that spends
      // upstream GraphQL quota. Three deliberate differences from the read
      // routes:
      //   * POST, not GET — a GET write is reachable by a link prefetch, a
      //     crawler, or a browser address bar, and this one costs money.
      //   * It NEVER fails open. The read routes fail open while READ_TOKEN
      //     is unset so the owner keeps a diagnostic path; an unset token on
      //     a write route is a 403, not an open door.
      //   * No CORS-driven browser access is intended; the allowlist echo
      //     stays only so a cross-origin call fails loudly, not silently.
      if (request.method !== "POST") {
        return cors(new Response("use POST", { status: 405 }), request, env);
      }
      if (!env.READ_TOKEN) {
        return cors(Response.json(
          { error: "READ_TOKEN is not set; the write route does not fail open" },
          { status: 403 }), request, env);
      }
      if (!ok) return cors(new Response("unauthorized", { status: 401 }), request, env);
      if (!env.CF_API_TOKEN) {
        return cors(Response.json(
          { error: "CF_API_TOKEN is not set; there is no source to backfill from" },
          { status: 503 }), request, env);
      }
      const reqDays = parseInt(url.searchParams.get("days") || "7", 10);
      const days = Math.min(Number.isFinite(reqDays) && reqDays > 0 ? reqDays : 7, MAX_BACKFILL_DAYS);
      const endParam = url.searchParams.get("end") || "";
      const end = /^\d{4}-\d{2}-\d{2}$/.test(endParam)
        ? endParam : new Date().toISOString().slice(0, 10);
      const report = await backfillRange(env, end, days);
      return cors(Response.json({
        end,
        days,
        retention_days: GRAPHQL_RETENTION_DAYS,
        note: `walks back from \`end\`; hours older than ~${GRAPHQL_RETENTION_DAYS} d are past GraphQL retention and are genuinely gone. Existing rows are never overwritten.`,
        ...report,
      }), request, env);
    }

    if (request.method !== "GET" && request.method !== "HEAD") {
      return cors(new Response("method not allowed", { status: 405 }), request, env);
    }

    if (url.pathname === "/status") {
      // Liveness is public (the dashboard badge needs it). Config posture goes
      // behind the fail-open gate so the T4 verification commands still work
      // before any token exists. Two things moved to the HARD gate:
      //
      //  * `latest` — this echoed the FULL row, so /status was a complete
      //    bypass of the projection /latest documents as unconditional. It
      //    now echoes the same projection; the full row needs a token.
      //  * `alerts_today` — it issues a KV `list` on EVERY request, and the
      //    gate above it was fail-open, so an unauthenticated caller drove
      //    one list op per request. The free tier allows 1,000 list ops/day
      //    (against 100,000 reads), so ~1,000 requests exhaust it — and the
      //    two things that then break are list-based: the day-aggregate
      //    recompute and the monthly-budget check. That is a cheaper and
      //    more damaging exhaustion than the read amplification T7 fixed.
      const latest = await kv.get("latest");
      const parsed = latest ? JSON.parse(latest) : null;
      const body = {
        worker: "riir-kat-housekeeping",
        cron: "0 * * * *",
        ts: parsed ? parsed.ts : null,
        ok: parsed ? parsed.ok : null,
        latest: publicRow(parsed),
      };
      if (ok) {
        Object.assign(body, {
          token_set: !!env.CF_API_TOKEN,
          cap: parseFloat(env.NEURON_CAP) || 0,
          thresholds: [0.5, 0.9, 1.0],
          has_webhook: !!env.ALERT_WEBHOOK_URL,
          has_monthly_budget: !!env.MONTHLY_BUDGET_USD,
          read_gated: !!env.READ_TOKEN,
        });
      }
      if (hasToken) {
        let alerts_today = 0;
        try {
          const listed = await kv.list({
            prefix: `alert:${new Date().toISOString().slice(0, 10)}:`, limit: 100 });
          alerts_today = listed.keys.length;
        } catch (_) { /* best-effort count */ }
        Object.assign(body, { alerts_today, latest: parsed });
      }
      return cors(Response.json(body), request, env, 60, hasToken);
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
      if (hasToken) return cors(Response.json(full), request, env, 60, true);
      return cors(Response.json(publicRow(full)), request, env, 60);
    }

    if (url.pathname === "/hours" || url.pathname === "/days") {
      if (!ok) return cors(new Response("unauthorized", { status: 401 }), request, env);
      const isHours = url.pathname === "/hours";
      const max = isHours ? MAX_HOURS : MAX_DAYS;
      const req = parseInt(url.searchParams.get("n") || "24", 10);
      const n = Math.min(Number.isFinite(req) && req > 0 ? req : 24, max);
      // Only the public variant is memoised — an authenticated body must never
      // be served from a cache an unauthenticated caller can also hit.
      const memoKey = `${url.pathname}:${n}`;
      if (!hasToken) {
        const hit = memoGet(memoKey);
        if (hit) return cors(Response.json(hit), request, env, 300);
      }
      const now = new Date();
      // Hours walk back from the last COMPLETE hour: rows are written for
      // hourStart, so including the current partial hour guaranteed one miss
      // per request and shifted every row one slot in the chart.
      const lastHour = new Date(Math.floor(now.getTime() / 3600e3) * 3600e3 - 3600e3);
      const keys = Array.from({ length: n }, (_, i) => {
        if (!isHours) return `day:${dayKey(new Date(now.getTime() - i * 86400e3))}`;
        return `hour:${hourKey(new Date(lastHour.getTime() - i * 3600e3))}`;
      });
      const raw = await mapLimit(keys, KV_CONCURRENCY, (k) => kv.get(k));
      const rows = [];
      for (const r of raw) {
        if (!r) continue;
        try { rows.push(JSON.parse(r)); } catch (_) { /* skip malformed */ }
      }
      // Day rows are already a bare {date, est_usd, hours} aggregate — the
      // same scalar /latest publishes. Hour rows carry the per-app breakdown.
      const out = (isHours && !hasToken) ? rows.map(publicRow) : rows;
      if (!hasToken) memoPut(memoKey, out);
      return cors(Response.json(out), request, env, 300, hasToken);
    }

    return cors(new Response(
      "riir-kat-housekeeping (issue 045) — /status /latest /hours?n= /days?n= | POST /backfill?days=&end= (token)",
      { status: 404 }), request, env);
  },
};
