// Regression tests for the Issue 045 housekeeping Worker.
//
// These exist because the T8 audit found a bug that had been live for two
// days and was invisible to every prior "live-verified" check: the endpoints
// all returned 200 with plausible-looking JSON while the hour keys collided.
// A shape check on the KEYS, not on the HTTP status, is what catches it.
//
//   node --test cloudflare/housekeeping/test/*.test.mjs
//
// (a bare directory arg fails MODULE_NOT_FOUND — node --test wants file paths)
//
// The worker is dependency-free ESM and every global it touches (Response,
// URL, TextEncoder, fetch) exists in Node 18+, so it runs unmodified here.

import test from "node:test";
import assert from "node:assert/strict";
import worker from "../src/worker.js";

// ---- stubs ---------------------------------------------------------------

// The real KV returns per-key metadata from list(), and the backfill relies on
// it to classify rows without a get(). A stub that dropped metadata would make
// the fallback path the only path under test, so it is modelled here.
// `seed` entries may be a bare string (a legacy, metadata-less row) or
// `{ value, metadata }`.
function kvStub(seed = {}) {
  const store = new Map();
  const meta = new Map();
  for (const [k, v] of Object.entries(seed)) {
    if (v && typeof v === "object") { store.set(k, v.value); meta.set(k, v.metadata); }
    else store.set(k, v);
  }
  const ops = { get: 0, put: 0, list: 0 };
  return {
    store,
    meta,
    ops,
    async get(k) { ops.get++; return store.has(k) ? store.get(k) : null; },
    async put(k, v, opts) {
      ops.put++;
      store.set(k, v);
      if (opts && opts.metadata) meta.set(k, opts.metadata); else meta.delete(k);
    },
    async list({ prefix = "", limit } = {}) {
      ops.list++;
      const keys = [...store.keys()].filter((k) => k.startsWith(prefix)).sort()
        .map((name) => (meta.has(name) ? { name, metadata: meta.get(name) } : { name }));
      return { keys: limit ? keys.slice(0, limit) : keys };
    },
  };
}

const env = (kv, extra = {}) => ({
  HOUSEKEEPING_KV: kv,
  CF_ACCOUNT_ID: "acct",
  CF_API_TOKEN: "collector-token",
  MEM_USD_PER_GIB_S: "0.0000025",
  DISK_USD_PER_GB_S: "0.00000007",
  CPU_USD_PER_S: "0.000020",
  NEURON_CAP: "10000",
  ALLOWED_ORIGINS: "https://riir-ops-dashboard.foxfox.workers.dev",
  ...extra,
});

function ctxStub() {
  const pending = [];
  return { waitUntil: (p) => pending.push(p), drain: () => Promise.all(pending) };
}

// The worker reads wall-clock via bare `new Date()` / `Date.now()`, so the
// only way to drive 24 distinct hours is to pin the global.
async function atUtc(iso, fn) {
  const Real = Date;
  class Fake extends Real {
    constructor(...a) { if (a.length === 0) super(iso); else super(...a); }
    static now() { return new Real(iso).getTime(); }
  }
  globalThis.Date = Fake;
  try { return await fn(); } finally { globalThis.Date = Real; }
}

// Same as atUtc, but `Date.now()` ADVANCES by `stepMs` on every read. The
// fixed-instant clock makes any elapsed-time guard inert — it would pass a
// wall-clock test by measuring zero, which is the "verify the instrument"
// trap. This one lets a guard actually trip.
async function atUtcAdvancing(iso, stepMs, fn) {
  const Real = Date;
  let t = new Real(iso).getTime();
  class Fake extends Real {
    constructor(...a) { if (a.length === 0) super(iso); else super(...a); }
    static now() { const v = t; t += stepMs; return v; }
  }
  globalThis.Date = Fake;
  try { return await fn(); } finally { globalThis.Date = Real; }
}

const APP = "a03a70ff-88e9-45c5-af80-339ab7707fbe";

// CF GraphQL stub: one app burning a fixed footprint every hour.
//
// It ECHOES `datetimeHour`, one group per hour in the requested window, which
// is what the real dataset does — the earlier stub omitted the dimension it
// asked for, so a whole-day query looked identical to a one-hour query and no
// test could have caught an hour-grouping bug. `opts.omitHour` reproduces the
// old shape on purpose, to pin the cron's fallback.
function fetchStub(opts = {}) {
  const { omitHour = false, calls = null } = opts;
  return async (_url, init) => {
    const q = JSON.parse(init.body).query;
    if (calls) calls.push(q);
    const geq = (q.match(/datetimeHour_geq: "([^"]+)"/) || [])[1];
    const lt = (q.match(/datetimeHour_lt: "([^"]+)"/) || [])[1];
    const hours = [];
    if (geq && lt) {
      for (let t = Date.parse(geq); t < Date.parse(lt); t += 3600e3) {
        hours.push(new Date(t).toISOString().slice(0, 13) + ":00:00Z");
      }
    }
    const dims = (h) => (omitHour ? { applicationId: APP } : { applicationId: APP, datetimeHour: h });
    const acc = {};
    if (q.includes("containersUsageAdaptiveGroups")) {
      acc.containersUsageAdaptiveGroups = hours.map((h) => ({
        dimensions: dims(h),
        sum: { allocatedMemory: 1073741824, allocatedDisk: 1e9, cpuTimeSec: 10, txBytes: 5 },
      }));
    } else if (q.includes("containersMetricsAdaptiveGroups")) {
      acc.containersMetricsAdaptiveGroups = hours.map((h) => ({
        dimensions: dims(h),
        sum: { containerUptime: 3600000 },
      }));
    } else {
      acc.aiInferenceAdaptiveGroups = [{ sum: { totalNeurons: 100 } }];
    }
    return {
      ok: true,
      async json() { return { data: { viewer: { accounts: [acc] } } }; },
    };
  };
}

const NOW = "2026-08-28T12:30:00.000Z";   // 30 min after the last runHour

// Reads are clock-dependent (`/hours` walks back from the last complete hour),
// so they need the same pinned clock the writes used.
const fetchAt = (e, path, opts, iso = NOW) =>
  atUtc(iso, () => worker.fetch(get(path, opts), e, ctxStub()));

const get = (path, { token, origin } = {}) => new Request(
  `https://riir-kat-housekeeping.foxfox.workers.dev${path}`,
  { headers: {
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(origin ? { Origin: origin } : {}),
  } },
);

async function runHour(kv, iso, extra = {}) {
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  try {
    await atUtc(iso, async () => {
      const c = ctxStub();
      await worker.scheduled({}, env(kv, extra), c);
      await c.drain();
    });
  } finally { globalThis.fetch = realFetch; }
}

// ---- BUG-1: hour-key collision ------------------------------------------

test("every hourly run writes a distinct YYYYMMDDHH key", async () => {
  const kv = kvStub();
  for (let h = 0; h < 24; h++) {
    await runHour(kv, `2026-08-28T${String(h).padStart(2, "0")}:00:06.549Z`);
  }
  const hourKeys = [...kv.store.keys()].filter((k) => k.startsWith("hour:"));
  // Pre-fix this was 3: `slice(0, 10)` kept the "T" plus one hour digit, so
  // 00-09 -> T0, 10-19 -> T1, 20-23 -> T2, each overwriting the last.
  assert.equal(hourKeys.length, 24, `expected 24 hour rows, got ${hourKeys.length}`);
  for (const k of hourKeys) assert.match(k, /^hour:\d{10}$/, `malformed key ${k}`);
});

test("the hour key names the hour the row DESCRIBES, not the run time", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const [k] = [...kv.store.keys()].filter((x) => x.startsWith("hour:"));
  assert.equal(k, "hour:2026082811");                       // not ...12
  assert.equal(JSON.parse(kv.store.get(k)).hour, "2026-08-28T11:00:00Z");
});

test("the day aggregate counts every hour of the day", async () => {
  const kv = kvStub();
  for (let h = 1; h <= 12; h++) {
    await runHour(kv, `2026-08-28T${String(h).padStart(2, "0")}:00:06.549Z`);
  }
  const day = JSON.parse(kv.store.get("day:20260828"));
  assert.equal(day.hours, 12);          // pre-fix: 1
  assert.ok(day.est_usd > 0, "aggregate should sum a nonzero cost");
});

test("the 00:00 run aggregates the PREVIOUS day's final hour", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-29T00:00:06.549Z");
  assert.ok(kv.store.has("hour:2026082823"), "23:00 row must be filed under the 28th");
  assert.ok(kv.store.has("day:20260828"), "and must aggregate into the 28th, not the 29th");
  assert.equal(JSON.parse(kv.store.get("day:20260828")).date, "2026-08-28");
});

test("legacy collided keys are not summed alongside the fixed rows", async () => {
  // The legacy key shares the `hour:20260828` prefix the aggregate scans.
  const kv = kvStub({
    "hour:20260828T1": JSON.stringify({ containers: { est_usd: 99 } }),
  });
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const day = JSON.parse(kv.store.get("day:20260828"));
  assert.equal(day.hours, 1, "only the well-formed row counts");
  assert.ok(day.est_usd < 1, `legacy row leaked into the sum: ${day.est_usd}`);
});

// ---- SEC-A: the redaction must hold on every read route ------------------

test("unauthenticated /hours is redacted, not full rows", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const res = await fetchAt(env(kv), "/hours?n=3");
  const rows = await res.json();
  assert.ok(rows.length >= 1);
  for (const r of rows) {
    assert.equal(r.redacted, true);
    assert.equal(r.containers.apps, undefined, "per-app breakdown must not be public");
    assert.equal(r.mint_anchor, undefined, "mint anchor must not be public");
    assert.equal(r.notes, undefined, "collector notes must not be public");
    assert.ok(typeof r.containers.est_usd === "number", "the scalar stays public");
  }
});

test("unauthenticated /status echoes the projection, never the full row", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const body = await (await fetchAt(env(kv), "/status")).json();
  assert.equal(body.latest.redacted, true);
  assert.equal(body.latest.containers.apps, undefined);
  assert.equal(body.alerts_today, undefined, "the KV-list field needs a real token");
  // The T4 diagnostic path survives the fail-open window.
  assert.equal(body.token_set, true);
  assert.equal(body.read_gated, false);
});

test("a presented token returns the full detail on every route", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret", CF_API_TOKEN: "t" });
  await runHour(kv, "2026-08-28T12:00:06.549Z", { READ_TOKEN: "s3cret", CF_API_TOKEN: "t" });
  for (const path of ["/latest", "/status"]) {
    const body = await (await fetchAt(e, path, { token: "s3cret" })).json();
    const row = path === "/status" ? body.latest : body;
    assert.equal(row.redacted, undefined, `${path} should be unredacted`);
    assert.ok(row.mint_anchor, `${path} should carry the mint anchor`);
  }
  const rows = await (await fetchAt(e, "/hours?n=2", { token: "s3cret" })).json();
  assert.ok(rows[0].containers.apps, "/hours should carry the per-app breakdown");
});

test("a wrong token is rejected once READ_TOKEN is armed", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  await runHour(kv, "2026-08-28T12:00:06.549Z", { READ_TOKEN: "s3cret" });
  assert.equal((await fetchAt(e, "/hours")).status, 401);
  assert.equal((await fetchAt(e, "/hours", { token: "wrong" })).status, 401);
  // /latest never 401s (the dashboard badge needs it) but must stay redacted.
  const latest = await (await fetchAt(e, "/latest", { token: "wrong" })).json();
  assert.equal(latest.mint_anchor, undefined, "a wrong token must not unlock detail");
});

// ---- PERF-A: the unauthenticated read path must not spend list ops -------

test("unauthenticated /status spends zero KV list ops", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const before = kv.ops.list;
  for (let i = 0; i < 10; i++) await fetchAt(env(kv), "/status");
  // Pre-fix: one `list` per request, against a 1,000/day free-tier cap.
  assert.equal(kv.ops.list - before, 0);
});

test("/hours fan-out is bounded and memoised per isolate", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const e = env(kv);
  const before = kv.ops.get;
  await fetchAt(e, "/hours?n=720");
  const first = kv.ops.get - before;
  assert.ok(first <= 168, `n=720 must clamp to MAX_HOURS, issued ${first} gets`);
  const mid = kv.ops.get;
  await fetchAt(e, "/hours?n=720");
  assert.equal(kv.ops.get - mid, 0, "the repeat request must be served from the memo");
});

// ---- CORS ---------------------------------------------------------------

test("CORS echoes only the allowlisted origin", async () => {
  const kv = kvStub();
  const ok = await fetchAt(env(kv), "/latest",
    { origin: "https://riir-ops-dashboard.foxfox.workers.dev" });
  assert.equal(ok.headers.get("Access-Control-Allow-Origin"),
    "https://riir-ops-dashboard.foxfox.workers.dev");
  const bad = await fetchAt(env(kv), "/latest", { origin: "https://evil.example" });
  assert.equal(bad.headers.get("Access-Control-Allow-Origin"), null);
});

// ---- T9: the backfill ----------------------------------------------------
//
// T8 recorded the hours it lost as UNRECOVERABLE. They are not: CF's GraphQL
// datasets retain ~31 days and expose `datetimeHour`, so KV is a cache of a
// still-live source. These pin the repair path — and, more importantly, pin
// that it cannot make things worse than the bug it repairs.

const postBackfill = (e, qs = "", { token, iso = NOW, stub = {} } = {}) => {
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub(stub);
  const req = new Request(
    `https://riir-kat-housekeeping.foxfox.workers.dev/backfill${qs}`,
    { method: "POST", headers: token ? { Authorization: `Bearer ${token}` } : {} },
  );
  return atUtc(iso, () => worker.fetch(req, e, ctxStub()))
    .finally(() => { globalThis.fetch = realFetch; });
};

test("backfill reconstructs every missing hour of a day from GraphQL", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const res = await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" });
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(body.written, 24, `expected 24 rebuilt hours, got ${body.written}`);
  const keys = [...kv.store.keys()].filter((k) => k.startsWith("hour:2026")).sort();
  assert.equal(keys.length, 24);
  assert.equal(keys[0], "hour:2026082700");
  assert.equal(keys[23], "hour:2026082723");
  for (const k of keys) assert.match(k, /^hour:\d{10}$/);
  // The whole point: the day aggregate now counts 24, not the 3-4 the bug left.
  const day = JSON.parse(kv.store.get("day:20260827"));
  assert.equal(day.hours, 24);
  assert.ok(day.est_usd > 0);
});

test("backfill NEVER overwrites a live-cron row", async () => {
  // A cron row carries `ai` + `mint_anchor`, which billing cannot rebuild.
  // Clobbering one to "repair" it would destroy information.
  const kv = kvStub();
  await runHour(kv, "2026-08-27T13:00:06.549Z");     // writes hour:2026082712
  const before = kv.store.get("hour:2026082712");
  assert.ok(JSON.parse(before).mint_anchor, "precondition: cron row has a mint anchor");

  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 1, "the existing hour must be skipped, not rewritten");
  assert.equal(body.written, 23);
  assert.equal(kv.store.get("hour:2026082712"), before, "live row was mutated");
  assert.equal(JSON.parse(kv.store.get("hour:2026082700")).source, "backfill");
});

test("backfill is idempotent and cheap on a second run", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" });
  const snapshot = new Map(kv.store);
  const opsBefore = { ...kv.ops };

  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.written, 0, "a repeat run must write nothing");
  assert.equal(body.skipped, 24);
  assert.equal(kv.ops.put, opsBefore.put, "a repeat run must issue zero KV puts");
  // Existence is decided by ONE list per day, never 24 gets.
  assert.equal(kv.ops.get, opsBefore.get, "a repeat run must issue zero KV gets");
  assert.equal(kv.ops.list, opsBefore.list + 1);
  for (const [k, v] of snapshot) assert.equal(kv.store.get(k), v, `row ${k} changed`);
});

test("backfill is bounded: days clamps and the window walks back", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=999&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.days, 14, "days must clamp to MAX_BACKFILL_DAYS");
  // 700-subrequest budget / 52 worst-case per day => 13 days, then a cursor.
    assert.ok(body.subrequests_spent <= 700, `budget blown: ${body.subrequests_spent}`);
  if (body.truncated) assert.match(body.next_end, /^\d{4}-\d{2}-\d{2}$/);
  const days = new Set([...kv.store.keys()]
    .filter((k) => k.startsWith("hour:")).map((k) => k.slice(5, 13)));
  assert.ok(days.has("20260827") && days.has("20260826"), "must walk back day by day");
  for (const d of days) assert.ok(d <= "20260827", `wrote a day after \`end\`: ${d}`);
});

test("backfill drops groups whose datetimeHour is missing, never guesses", async () => {
  // A whole day's usage filed under a guessed hour is worse than a gap.
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27",
    { token: "s3cret", stub: { omitHour: true } })).json();
  assert.equal(body.written, 0);
  assert.equal([...kv.store.keys()].filter((k) => k.startsWith("hour:")).length, 0);
});

test("the hourly cron still collects when datetimeHour is absent", async () => {
  // The mirror of the test above: the cron's window IS one hour, so it may
  // fall back to the hour it asked for rather than lose the row.
  const kv = kvStub();
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub({ omitHour: true });
  try {
    await atUtc("2026-08-28T12:00:06.549Z", async () => {
      const c = ctxStub();
      await worker.scheduled({}, env(kv), c);
      await c.drain();
    });
  } finally { globalThis.fetch = realFetch; }
  const row = JSON.parse(kv.store.get("hour:2026082811"));
  assert.ok(row.containers.est_usd > 0, "fallback hour key must keep the row");
});

test("backfill is a write route: POST only, and it never fails open", async () => {
  const kv = kvStub();

  // No READ_TOKEN set -> 403, NOT the fail-open the read routes use.
  const open = env(kv);
  assert.equal((await postBackfill(open, "?days=1")).status, 403);
  assert.equal(kv.ops.put, 0);

  const e = env(kv, { READ_TOKEN: "s3cret" });
  assert.equal((await postBackfill(e, "?days=1")).status, 401, "no token -> 401");
  assert.equal((await postBackfill(e, "?days=1", { token: "wrong" })).status, 401);
  assert.equal(kv.ops.put, 0, "an unauthorized call must not write");

  // GET is rejected: a link prefetch or crawler must not spend GraphQL quota.
  const g = await atUtc(NOW, () => worker.fetch(get("/backfill", { token: "s3cret" }), e, ctxStub()));
  assert.equal(g.status, 405);

  // Token present but no collector token -> 503, not a silent empty success.
  const noCollector = { ...env(kv, { READ_TOKEN: "s3cret" }), CF_API_TOKEN: undefined };
  assert.equal((await postBackfill(noCollector, "?days=1", { token: "s3cret" })).status, 503);
  assert.equal(kv.ops.put, 0);
});

// ---- T9b: cache posture, budget-scan cost, wall-clock guard --------------

test("an authenticated body is never marked publicly cacheable", async () => {
  // RFC 9111 §3.5: `public` is the exact directive that GRANTS a shared cache
  // the storage of an Authorization-bearing response that it would otherwise
  // refuse. Stamping it on the full rows handed account telemetry to every
  // cache on the path. `Vary: Authorization` is a mitigation, not a licence.
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const e = env(kv, { READ_TOKEN: "s3cret" });

  for (const path of ["/latest", "/status", "/hours?n=4", "/days?n=4"]) {
    const priv = await fetchAt(e, path, { token: "s3cret" });
    assert.equal(priv.headers.get("Cache-Control"), "private, no-store",
      `${path} leaked an authenticated body into shared caches`);
    assert.match(priv.headers.get("Vary") || "", /Authorization/);
  }
  // The redacted anonymous projection stays publicly cacheable — that is the
  // whole point of having a projection.
  for (const [path, secs] of [["/latest", 60], ["/status", 60]]) {
    const pub = await fetchAt(e, path);
    assert.equal(pub.headers.get("Cache-Control"), `public, max-age=${secs}`);
  }
});

test("the monthly-budget scan stops running once the alert has fired", async () => {
  const extra = { MONTHLY_BUDGET_USD: "0.000001" };   // trivially exceeded
  // A prior day row must exist: the month-to-date scan reads the `day:`
  // aggregates, and this run writes its own only after the budget check.
  const seed = { "day:20260827": JSON.stringify({ date: "2026-08-27", est_usd: 1, hours: 24 }) };
  const kv = kvStub(seed);
  await runHour(kv, "2026-08-28T10:00:06.549Z", extra);
  assert.ok(kv.store.has("alert:202608:budget"), "precondition: alert fired");

  // The scan is 1 list + up to 31 gets, and it was re-running EVERY hour for
  // the rest of the month to recompute a number that could only re-fire an
  // alert already sent.
  const before = { ...kv.ops };
  await runHour(kv, "2026-08-28T11:00:06.549Z", extra);
  const withAlert = { list: kv.ops.list - before.list, get: kv.ops.get - before.get };

  const kv2 = kvStub(seed);
  await runHour(kv2, "2026-08-28T10:00:06.549Z");          // no budget configured
  const before2 = { ...kv2.ops };
  await runHour(kv2, "2026-08-28T11:00:06.549Z");
  const noBudget = { list: kv2.ops.list - before2.list, get: kv2.ops.get - before2.get };

  assert.equal(withAlert.list, noBudget.list,
    "a post-alert run must not spend the month-to-date list op");
  assert.ok(withAlert.get <= noBudget.get + 1,
    `post-alert run scanned the month anyway: ${withAlert.get} vs ${noBudget.get}`);
});

test("a long backfill stops on a wall-clock budget with a resumable cursor", async () => {
  // A 524 mid-run is safe (writes are committed, the route is idempotent) but
  // it is indistinguishable from a broken worker and it loses the cursor.
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  let body;
  try {
    const req = new Request(
      "https://riir-kat-housekeeping.foxfox.workers.dev/backfill?days=14&end=2026-08-27",
      { method: "POST", headers: { Authorization: "Bearer s3cret" } },
    );
    // ~5 s of simulated wall clock per Date.now() read: the 45 s guard trips
    // after a couple of days, long before the 700-subrequest budget would.
    body = await (await atUtcAdvancing(NOW, 5000,
      () => worker.fetch(req, e, ctxStub()))).json();
  } finally { globalThis.fetch = realFetch; }

  assert.equal(body.truncated, true);
  assert.equal(body.truncated_by, "wall_clock");
  assert.ok(body.subrequests_spent < 700, "must stop on time, not on budget");
  assert.match(body.next_end, /^\d{4}-\d{2}-\d{2}$/);
  // The cursor must name a day that was NOT completed, so resuming loses none.
  assert.ok(!kv.store.has(`hour:${body.next_end.replace(/-/g, "")}00`),
    "cursor points at an already-written day — resuming would skip real work");
  // And it stopped on a day boundary: every day it did touch has all 24 rows.
  const byDay = {};
  for (const k of kv.store.keys()) {
    if (k.startsWith("hour:")) byDay[k.slice(5, 13)] = (byDay[k.slice(5, 13)] || 0) + 1;
  }
  for (const [d, n] of Object.entries(byDay)) {
    assert.equal(n, 24, `day ${d} was left half-written: ${n} rows`);
  }
});

// ---- T9c: protect DATA, not mere existence -------------------------------
//
// Found by reading prod, not the code: `/status` reports `token_set:false`,
// so every archived row is a heartbeat. An exists-based protect rule would
// have declared the entire archive untouchable and repaired nothing.

test("backfill replaces heartbeat rows — existence is not data", async () => {
  const kv = kvStub();
  // Reproduce the live shape: cron running, CF_API_TOKEN unset.
  const noToken = { ...env(kv), CF_API_TOKEN: undefined };
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  try {
    for (let h = 1; h <= 4; h++) {
      await atUtc(`2026-08-27T0${h}:00:06.549Z`, async () => {
        const c = ctxStub();
        await worker.scheduled({}, noToken, c);
        await c.drain();
      });
    }
  } finally { globalThis.fetch = realFetch; }
  const beats = [...kv.store.keys()].filter((k) => /^hour:\d{10}$/.test(k));
  assert.equal(beats.length, 4, "precondition: heartbeat rows exist");
  assert.equal(JSON.parse(kv.store.get(beats[0])).ok, false);
  for (const k of beats) assert.equal(kv.meta.get(k).d, 0, "heartbeat must be marked d:0");

  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 0, "a heartbeat must never be treated as protected");
  assert.equal(body.written, 24);
  const repaired = JSON.parse(kv.store.get("hour:2026082701"));
  assert.equal(repaired.ok, true);
  assert.ok(repaired.containers.est_usd > 0, "the heartbeat hour now carries real cost");
  assert.ok(repaired.replaced, "the placeholder it displaced is recorded");
  assert.equal(JSON.parse(kv.store.get("day:20260827")).hours, 24);
});

test("backfill still protects a row that really collected", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-27T13:00:06.549Z");          // token present => real collect
  const key = "hour:2026082712";
  assert.equal(JSON.parse(kv.store.get(key)).ok, true);
  assert.equal(kv.meta.get(key).d, 1, "a real collection must be marked d:1");
  const before = kv.store.get(key);

  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 1);
  assert.equal(kv.store.get(key), before, "a real collection was clobbered");
});

test("an all-collectors-failed row is replaceable despite having containers", async () => {
  // It gets `containers: {est_usd: 0, apps: {}}` because the collectors return
  // empty maps rather than throwing. Treating that false zero as data would
  // wedge the hour shut forever, so `ok` is the discriminator, not `containers`.
  const kv = kvStub({
    "hour:2026082705": JSON.stringify({
      kind: "housekeeping", ok: false, ts: "2026-08-27T06:00:00Z",
      notes: ["usage failed: graphql 500"],
      containers: { est_usd: 0, apps: {} },
      ai: { neurons_today: 4242 },        // this collector DID succeed
    }),
  });
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 0);
  const row = JSON.parse(kv.store.get("hour:2026082705"));
  assert.ok(row.containers.est_usd > 0, "the false zero must be replaced");
  // Merge-forward: the neuron count that DID collect must survive the repair.
  assert.equal(row.ai.neurons_today, 4242, "a successful collector's data was lost");
});

test("classifying marked rows costs no extra gets", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" });   // all marked d:1
  const before = { ...kv.ops };
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 24);
  assert.equal(kv.ops.get, before.get, "metadata in list() must avoid the get fan-out");
  assert.equal(kv.ops.put, before.put);
});

test("a legacy row with no metadata is classified by a get, not assumed", async () => {
  // Rows written before this deploy carry no marker. Assuming either way is
  // wrong: assume-data and the repair silently no-ops; assume-placeholder and
  // a real collection gets clobbered.
  const kv = kvStub({
    "hour:2026082703": JSON.stringify({ ok: true, containers: { est_usd: 7, apps: {} } }),
    "hour:2026082704": JSON.stringify({ ok: false, notes: ["heartbeat only"] }),
  });
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-27", { token: "s3cret" })).json();
  assert.equal(body.skipped, 1, "the legacy real collection must be protected");
  assert.equal(JSON.parse(kv.store.get("hour:2026082703")).containers.est_usd, 7);
  assert.equal(JSON.parse(kv.store.get("hour:2026082704")).ok, true, "legacy heartbeat repaired");
});

test("a throwing housekeeping run is logged, not swallowed", async () => {
  const kv = kvStub();
  kv.put = async () => { throw new Error("kv unavailable"); };
  const errs = [];
  const realErr = console.error;
  console.error = (...a) => errs.push(a.join(" "));
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  try {
    await atUtc("2026-08-28T12:00:06.549Z", async () => {
      const c = ctxStub();
      await worker.scheduled({}, env(kv), c);
      await c.drain();                    // must not reject
    });
  } finally { console.error = realErr; globalThis.fetch = realFetch; }
  assert.equal(errs.length, 1, "the failure must reach the log");
  assert.match(errs[0], /housekeeping run failed/);
  assert.match(errs[0], /kv unavailable/);
});

// ---- T9d: what a live KV inventory exposed -------------------------------
//
// 2026-08-29: the namespace held EIGHT keys after three days of hourly runs —
// 7 legacy collided `hour:` rows and `latest`. Not one `day:` row. The
// heartbeat path was returning before the aggregate, so that code had never
// executed in production and every fix to it was stub-only.

test("heartbeat runs still write a day aggregate", async () => {
  const kv = kvStub();
  const noToken = { ...env(kv), CF_API_TOKEN: undefined };
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  try {
    for (let h = 1; h <= 5; h++) {
      await atUtc(`2026-08-28T0${h}:00:06.549Z`, async () => {
        const c = ctxStub();
        await worker.scheduled({}, noToken, c);
        await c.drain();
      });
    }
  } finally { globalThis.fetch = realFetch; }

  const day = kv.store.get("day:20260828");
  assert.ok(day, "a heartbeat day must still produce a day: row — `/days` served [] for three days");
  const d = JSON.parse(day);
  // The honest shape: rows exist, none of them are data. Reporting hours:5
  // with est_usd:0 would be a false zero that looks like a measurement.
  assert.equal(d.hours, 0, "a heartbeat must not count as a collected hour");
  assert.equal(d.hours_present, 5, "but its presence must still be visible");
  assert.equal(d.est_usd, 0);
});

test("a collected day reports hours and hours_present distinctly", async () => {
  const kv = kvStub();
  const noToken = { ...env(kv), CF_API_TOKEN: undefined };
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub();
  try {
    // Two heartbeat hours, then the token arrives and three hours collect.
    for (const h of [1, 2]) {
      await atUtc(`2026-08-28T0${h}:00:06.549Z`, async () => {
        const c = ctxStub();
        await worker.scheduled({}, noToken, c);
        await c.drain();
      });
    }
  } finally { globalThis.fetch = realFetch; }
  for (const h of [3, 4, 5]) await runHour(kv, `2026-08-28T0${h}:00:06.549Z`);

  const d = JSON.parse(kv.store.get("day:20260828"));
  assert.equal(d.hours_present, 5);
  assert.equal(d.hours, 3, "only the collected hours count toward the cost");
  assert.ok(d.est_usd > 0);
});

test("the 7 live legacy keys stay inert across every path", async () => {
  // Exactly the shapes measured in prod. They match the `hour:<dk>` prefix the
  // aggregate scans, so an unfiltered path would sum or clobber them.
  const seed = {};
  for (const n of ["hour:20260828T0", "hour:20260828T1", "hour:20260828T2"]) {
    seed[n] = JSON.stringify({ ts: "2026-08-28T17:00:07.321Z", hour: "2026-08-28T16:00:00Z", ok: false });
  }
  const kv = kvStub(seed);
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=1&end=2026-08-28", { token: "s3cret" })).json();
  assert.equal(body.written, 24, "legacy keys must not block any hour");
  assert.equal(body.skipped, 0);
  for (const n of Object.keys(seed)) {
    assert.equal(kv.store.get(n), seed[n], `legacy key ${n} was mutated`);
  }
  const d = JSON.parse(kv.store.get("day:20260828"));
  assert.equal(d.hours, 24, "legacy rows must not inflate the collected count");
  assert.equal(d.hours_present, 24, "nor the present count");
});
