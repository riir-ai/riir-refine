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

import test, { beforeEach } from "node:test";
import assert from "node:assert/strict";
import worker, { __resetMemoForTests } from "../src/worker.js";

// The worker's response/KV memo is module state with namespace-agnostic keys,
// so without this one test's `latest` row is servable to the next test's
// differently-seeded stub — a suite that passes only in this order.
beforeEach(() => __resetMemoForTests());

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
  CLOUDFLARE_OPS_API_TOKEN: "collector-token",
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
  const {
    omitHour = false, calls = null, webhooks = null,
    kvActions = null, kvFail = false, periodTotals = null,
  } = opts;
  // Defaults for the BILL queries (Issue 065 T7): account-wide period usage
  // shaped like .benchmarks/cf_bill_pin.json — the $8.08 Cloudflare actually
  // charged. An hourly-shaped answer here would make the budget tests pass
  // against a stub the real dataset never produces.
  const kvActs = kvActions || { read: 15310000, list: 1301970, write: 419160 };
  const pt = periodTotals || { memGibS: 122940, diskGbS: 246330, cpuS: 1020 };
  return async (_url, init) => {
    let body = {};
    try { body = JSON.parse(init.body); } catch (_) {}
    if (webhooks && body.content !== undefined) {
      webhooks.push(body.content);
      return { ok: true, async json() { return {}; } };
    }
    const q = body.query || "";
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
    if (q.includes("kvOperationsAdaptiveGroups")) {
      // the per-actionType period query (the bill's KV half). kvFail
      // reproduces a degraded collector so the uncollected path is testable.
      if (kvFail) throw new Error("kv dataset down");
      acc.kvOperationsAdaptiveGroups = Object.entries(kvActs).map(([actionType, requests]) => ({
        dimensions: { actionType },
        sum: { requests },
      }));
    } else if (q.includes("containersUsageAdaptiveGroups")) {
      if (q.includes("datetimeHour }")) {
        // per-hour grouped query (hourly cron + backfill)
        acc.containersUsageAdaptiveGroups = hours.map((h) => ({
          dimensions: dims(h),
          sum: { allocatedMemory: 1073741824, allocatedDisk: 1e9, cpuTimeSec: 10, txBytes: 5 },
        }));
      } else {
        // ungrouped account-wide period totals (the bill's container half)
        acc.containersUsageAdaptiveGroups = [{
          sum: {
            allocatedMemory: pt.memGibS * 1073741824,
            allocatedDisk: pt.diskGbS * 1e9,
            cpuTimeSec: pt.cpuS,
          },
        }];
      }
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

async function runHour(kv, iso, extra = {}, stubOpts = {}) {
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchStub(stubOpts);
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

// SUPERSEDES "unauthenticated /hours is redacted, not full rows". Redaction
// was the T8 answer to an unauthenticated caller reaching the bulk routes; the
// stronger answer is that it does not reach them at all. A redacted row still
// costs the full KV fan-out to produce, which was the actual exposure — see
// the gate comment in the worker. The projection itself is still pinned, on
// /status and /latest, by the two tests below.
test("unauthenticated /hours and /days are refused, and spend nothing", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const e = env(kv);          // no READ_TOKEN — the old fail-open window
  const before = { get: kv.ops.get, list: kv.ops.list };
  for (const path of ["/hours?n=168", "/days?n=90"]) {
    const res = await fetchAt(e, path);
    assert.equal(res.status, 403, `${path} must not fail open`);
    const body = await res.json();
    assert.match(body.error, /not fail open/);
  }
  // The point of the gate is COST, so assert the cost, not just the status.
  assert.equal(kv.ops.get - before.get, 0, "a refused request must issue no KV get");
  assert.equal(kv.ops.list - before.list, 0);
});

test("a wrong token cannot read a memo entry a valid one populated", async () => {
  const kv = kvStub();
  const opts = { READ_TOKEN: "s3cret", CLOUDFLARE_OPS_API_TOKEN: "t" };
  const e = env(kv, opts);
  await runHour(kv, "2026-08-28T12:00:06.549Z", opts);
  const warm = await (await fetchAt(e, "/hours?n=2", { token: "s3cret" })).json();
  assert.ok(warm.length >= 1, "the valid caller populates the memo");
  // The memo key is salted with a digest of the ACCEPTED token, so there is
  // no key a rejected caller can name.
  assert.equal((await fetchAt(e, "/hours?n=2", { token: "wrong" })).status, 401);
  assert.equal((await fetchAt(e, "/hours?n=2")).status, 401);
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
  const e = env(kv, { READ_TOKEN: "s3cret", CLOUDFLARE_OPS_API_TOKEN: "t" });
  await runHour(kv, "2026-08-28T12:00:06.549Z", { READ_TOKEN: "s3cret", CLOUDFLARE_OPS_API_TOKEN: "t" });
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

// Presents a token now that the route is hard-gated. Without one this test
// passed for the WRONG REASON: a 403 issues zero gets, so both "bounded" and
// "memoised" held trivially and the assertions measured nothing.
test("/hours fan-out is bounded and memoised per isolate", async () => {
  const kv = kvStub();
  const opts = { READ_TOKEN: "s3cret", CLOUDFLARE_OPS_API_TOKEN: "t" };
  const e = env(kv, opts);
  await runHour(kv, "2026-08-28T12:00:06.549Z", opts);
  const tok = { token: "s3cret" };
  const before = kv.ops.get;
  await fetchAt(e, "/hours?n=720", tok);
  const first = kv.ops.get - before;
  assert.ok(first > 0, "the measured path must actually issue gets");
  assert.ok(first <= 168, `n=720 must clamp to MAX_HOURS, issued ${first} gets`);
  const mid = kv.ops.get;
  await fetchAt(e, "/hours?n=720", tok);
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

// ---- T9f: exhausting `days` is a truncation, and must say so --------------
//
// A walk that completes `days` days without tripping a budget used to report
// `truncated:false, next_end:null` — indistinguishable from "the retrievable
// window is now fully archived". It is not: MAX_BACKFILL_DAYS (14) is less
// than half of GraphQL's ~31 d retention, so the runbook's `?days=14` from
// today never reaches days 15-31. Measured against prod on 2026-08-29:
// 2026-08-14 held 2 real container-usage hours (est $0.032696) that the
// default call could not see and the cron will never write, because the cron
// only ever writes the CURRENT hour. The archive was silently short by a day
// that cost real money.

test("a completed `days` walk still yields a cursor while retention remains", async () => {
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  // 14 days ending 2026-08-28 covers back to 2026-08-15; retention reaches
  // ~2026-07-29, so there is plenty left and the caller must be told.
  const body = await (await postBackfill(e, "?days=14&end=2026-08-28", { token: "s3cret" })).json();

  assert.equal(body.truncated, true, "a 14-day slice of a 31-day window is not complete");
  assert.equal(body.truncated_by, "max_days");
  // The cursor must name the next OLDER unvisited day, so a resume loop loses
  // nothing and repeats nothing.
  assert.equal(body.next_end, "2026-08-14");
  assert.ok(!body.per_day.some((r) => r.day === "20260814"),
    "cursor points at a day this call already walked — resuming would skip work");
  // And it must say what it actually covered, not just where retention ends.
  assert.deepEqual(body.covered, { from: "20260815", to: "20260828" });
});

test("the cursor stops at the retention floor instead of looping on dead days", async () => {
  // Symmetric failure: a cursor that never goes null turns the runbook's
  // resume loop into an unbounded spend against days GraphQL cannot answer.
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  // NOW is 2026-08-28, so the floor is 2026-07-29. Ending at 2026-08-11 with
  // 14 days covers back to 2026-07-29 — exactly the floor, nothing older.
  const body = await (await postBackfill(e, "?days=14&end=2026-08-11", { token: "s3cret" })).json();

  assert.equal(body.truncated, false, "nothing retrievable remains older than the floor");
  assert.equal(body.next_end, null);
});

test("the backfill note does not claim more coverage than one call gives", async () => {
  // The old note spoke only of the retention edge, which reads as "everything
  // still retrievable was covered". Prose is the surface an owner acts on.
  const kv = kvStub();
  const e = env(kv, { READ_TOKEN: "s3cret" });
  const body = await (await postBackfill(e, "?days=14&end=2026-08-28", { token: "s3cret" })).json();
  assert.match(body.note, /ONE call covers/);
  assert.match(body.note, /next_end/);
  assert.ok(!/genuinely gone\. Existing rows/.test(body.note),
    "the retention-only note is what made a partial walk read as complete");
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
  const noCollector = { ...env(kv, { READ_TOKEN: "s3cret" }), CLOUDFLARE_OPS_API_TOKEN: undefined };
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

// ---- Issue 065 T7: the budget alert reconciles the BILL, not marginal sums
// The old check summed per-day `est_usd` rows — a from-unit-zero MARGINAL
// quantity whose monthly allowance cannot be apportioned to a day, covering
// only the container family (1.0% of the real bill). The tests below pin the
// replacement: account-wide, period-scoped, allowances subtracted, KV's
// round-up applied, KV ops included, degraded collectors named as unwatched.

test("the monthly-budget alert reconciles the CF BILL against the period window", async () => {
  const webhooks = [];
  const calls = [];
  const kv = kvStub();
  await runHour(kv, "2026-08-28T10:00:06.549Z", {
    MONTHLY_BUDGET_USD: "8.08",   // the real bill's total — must fire
    ALERT_WEBHOOK_URL: "https://hooks.example/discord",
  }, { webhooks, calls });

  // Period scope: the default anniversary pin (16) puts the period at
  // 2026-08-16, and BOTH bill queries must be windowed from it.
  const key = "alert:bill:20260816";
  assert.ok(kv.store.has(key), "period-scoped dedupe key missing");
  assert.ok(calls.some((q) => q.includes("kvOperationsAdaptiveGroups") &&
    q.includes('date_geq: "2026-08-16"')), "KV bill query not period-scoped");
  assert.ok(calls.some((q) =>
    q.includes('datetimeHour_geq: "2026-08-16T00:00:00Z"')),
    "container bill query not period-scoped");

  // The message is the bill, with the top line attributed. The same usage
  // priced marginal-from-zero would read far higher — the message must not.
  assert.match(webhooks[0], /CF bill \$8\.08 ≥ monthly budget \$8\.08/);
  assert.match(webhooks[0], /top line: KV list ops \$5\.00/);
  assert.ok(!webhooks[0].includes("NOT counted"),
    "both collectors succeeded; nothing is unwatched");

  const row = JSON.parse(kv.store.get(key));
  assert.equal(row.kind, "monthly_budget");
  assert.equal(row.period_start, "2026-08-16");
  assert.deepEqual(row.uncollected, []);
  // 5.00 + 3.00 + 0.08235 = 8.08235 (toFixed(4) on the row; tolerance here
  // because the 4th decimal sits on a rounding boundary).
  assert.ok(Math.abs(row.bill_usd - 8.08235) < 0.001,
    `bill_usd ${row.bill_usd} vs 8.08235`);
  assert.equal(row.notified, true);
});

test("the bill scan stops running once the period alert has fired", async () => {
  const extra = { MONTHLY_BUDGET_USD: "0.000001" };   // trivially exceeded
  const calls1 = [];
  const kv = kvStub();
  await runHour(kv, "2026-08-28T10:00:06.549Z", extra, { calls: calls1 });
  assert.ok(kv.store.has("alert:bill:20260816"), "precondition: alert fired");
  assert.equal(calls1.filter((q) => q.includes("kvOperationsAdaptiveGroups")).length, 1,
    "precondition: the bill scan ran before the alert");

  // The dedupe check runs BEFORE the collectors, so a post-alert run spends
  // zero bill subrequests — the same discipline the old day-row scan had.
  const calls2 = [];
  await runHour(kv, "2026-08-28T11:00:06.549Z", extra, { calls: calls2 });
  assert.equal(calls2.filter((q) => q.includes("kvOperationsAdaptiveGroups")).length, 0,
    "post-alert run must not spend the bill subrequests");
  assert.equal(calls2.length, 3,
    "post-alert run still collects the three hourly datasets");
});

test("a failed KV collector leaves KV out of the bill and names it unwatched", async () => {
  const webhooks = [];
  const kv = kvStub();
  await runHour(kv, "2026-08-28T10:00:06.549Z", {
    MONTHLY_BUDGET_USD: "0.01",   // containers alone ($0.08) exceed this
    ALERT_WEBHOOK_URL: "https://hooks.example/discord",
  }, { webhooks, kvFail: true });

  const row = JSON.parse(kv.store.get("alert:bill:20260816"));
  assert.deepEqual(row.uncollected, ["kv_read", "kv_list", "kv_write", "kv_delete"],
    "a degraded collector must read as UNCOLLECTED, never as a zero");
  // The bill that fired counts containers only (0.08235) — and says so.
  assert.ok(Math.abs(row.bill_usd - 0.08235) < 0.001);
  assert.match(webhooks[0], /NOT counted \(collector failed\): KV read ops/);
});

test("no MONTHLY_BUDGET_USD: the bill collectors never run", async () => {
  const calls = [];
  const kv = kvStub();
  await runHour(kv, "2026-08-28T10:00:06.549Z", {}, { calls });
  assert.equal(calls.length, 3, "only the three hourly datasets");
  assert.ok(!calls.some((q) => q.includes("kvOperationsAdaptiveGroups")));
  assert.ok(!calls.some((q) =>
    q.includes("containersUsageAdaptiveGroups") && !q.includes("datetimeHour }")));
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
  // Reproduce the live shape: cron running, CLOUDFLARE_OPS_API_TOKEN unset.
  const noToken = { ...env(kv), CLOUDFLARE_OPS_API_TOKEN: undefined };
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
  const noToken = { ...env(kv), CLOUDFLARE_OPS_API_TOKEN: undefined };
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
  const noToken = { ...env(kv), CLOUDFLARE_OPS_API_TOKEN: undefined };
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

// ---- ROBUST: a corrupt KV value must degrade, not 500 --------------------
//
// Every JSON.parse here is over a value KV handed back — i.e. over data a
// previous deploy or a partial write could have left malformed. The aggregate
// and backfill paths were already tolerant of that; the two PUBLIC read
// routes were not, so one bad `latest` value turned the dashboard's liveness
// badge into an opaque runtime 500 with no line of our own in the logs.

test("a corrupt `latest` value degrades instead of 500ing the read routes", async () => {
  const kv = kvStub({ latest: "{ not json" });
  const e = env(kv);

  const status = await fetchAt(e, "/status");
  assert.equal(status.status, 200, "/status must still answer the liveness badge");
  const body = await status.json();
  assert.equal(body.latest, null, "an unparseable row reports as absent");
  assert.equal(body.worker, "riir-kat-housekeeping");
  assert.equal(body.token_set, true, "the config half still answers");

  const latest = await fetchAt(e, "/latest");
  assert.equal(latest.status, 200);
  assert.equal(await latest.json(), null);
});

test("a throwing KV binding answers 500 JSON, not an unhandled rejection", async () => {
  const kv = kvStub();
  kv.get = async () => { throw new Error("kv unavailable"); };
  const errs = [];
  const realErr = console.error;
  console.error = (...a) => errs.push(a.join(" "));
  try {
    const res = await fetchAt(env(kv), "/latest");
    assert.equal(res.status, 500);
    assert.deepEqual(await res.json(), { error: "internal error" });
    // The whole point is that it stops being SILENT.
    assert.ok(errs.some((l) => /fetch failed .*\/latest.*kv unavailable/s.test(l)),
      `the failure must be logged, got ${JSON.stringify(errs)}`);
    // A 500 is a private, uncacheable body — a shared cache holding it would
    // serve one caller's failure to everyone. (Kept inside the stub so the
    // worker's own log does not litter the suite's stderr.)
    const res2 = await fetchAt(env(kv), "/latest");
    assert.equal(res2.headers.get("Cache-Control"), "private, no-store");
  } finally { console.error = realErr; }
});

// ---- PERF-B: `latest` is the one key an anonymous caller can drive -------

test("repeated public reads of `latest` collapse to one KV get", async () => {
  const kv = kvStub();
  await runHour(kv, "2026-08-28T12:00:06.549Z");
  const e = env(kv);
  const before = kv.ops.get;
  for (let i = 0; i < 5; i++) await fetchAt(e, "/latest");
  for (let i = 0; i < 5; i++) await fetchAt(e, "/status");
  // Pre-fix: 10 gets. /status and /latest read the SAME key, so they share
  // one memo entry rather than one each.
  assert.equal(kv.ops.get - before, 1, "10 public reads must cost one KV get");
});

// ---- SEC-B: the token compare must not time out the secret's length -----

test("token acceptance is independent of token length", async () => {
  const kv = kvStub();
  // A 1-char and a 200-char secret must both work, and neither may be
  // unlocked by a guess of the other's length. The digest compare makes the
  // loop 64 hex chars wide in every case.
  for (const secret of ["x", "s3cret", "z".repeat(200)]) {
    const opts = { READ_TOKEN: secret, CLOUDFLARE_OPS_API_TOKEN: "t" };
    const e = env(kv, opts);
    __resetMemoForTests();
    await runHour(kv, "2026-08-28T12:00:06.549Z", opts);
    assert.equal((await fetchAt(e, "/hours?n=1", { token: secret })).status, 200,
      `a ${secret.length}-char secret must be accepted`);
    for (const guess of ["", "x", "z".repeat(199), "z".repeat(201), secret + "!"]) {
      if (guess === secret) continue;
      const res = await fetchAt(e, "/hours?n=1", { token: guess });
      assert.equal(res.status, 401,
        `guess of length ${guess.length} must be rejected against a ${secret.length}-char secret`);
    }
  }
});

// ---- T6: the completed-day mint-anchor POST (Issue 045 T6 / 044 T5) -------
//
// The kat-service ledger shell (riir-dapps Plan 013) accepts
// POST {url}/mint-anchor: first valid body for a day → 200 {recorded:true},
// byte-identical replay → 200 {recorded:false,reason:"already-recorded"},
// byte-different same-day body → 409. The cron must post the most recent
// COMPLETED UTC day (today is a moving target the exactly-once rule would
// wedge shut) and treat every failure as fail-soft.
//
// The stub models the server contract rather than a fixed script. One
// nuance found by this very test: the body carries `generated_at`, which
// necessarily DIFFERS on every hourly run — so a server keying strict
// byte-identity would 409 every re-post and the stateless design would
// fail. The replay key is therefore (date, neurons) — byte-identity applies
// to a transport retry of the same request; the semantic key is what makes
// "every later run just gets already-recorded" (the pinned design) true.
// Recorded in Issue 045 for the sibling implementing the server half.
function mintFetchStub(opts = {}) {
  const { posts = [], postMode = "ok", neuronsToday = 100, neuronsYesterday = 77,
    extraCounts = {} } = opts;
  const seen = new Map();   // date → neurons, the server's semantic key
  return async (url, init) => {
    if (String(url).endsWith("/mint-anchor")) {
      const body = JSON.parse(init.body);
      posts.push({ url: String(url), init, body });
      if (postMode === "reject") throw new Error("mint post network failure");
      if (postMode === "gen2") {
        return { ok: false, status: 401, async json() {
          return { error: "mint-anchor: this worker pins a genesis owner", code: "owner_sig_required" }; } };
      }
      if (postMode === "gen1-401") {
        return { ok: false, status: 401, async json() { return { error: "mint-anchor unauthorized" }; } };
      }
      if (postMode === "conflict") {
        return { ok: false, status: 409, async json() { return { recorded: false, reason: "superseded" }; } };
      }
      if (seen.has(body.date)) {
        if (seen.get(body.date) === body.neurons_today) {
          return { ok: true, status: 200, async json() { return { recorded: false, reason: "already-recorded" }; } };
        }
        return { ok: false, status: 409, async json() { return { recorded: false, reason: "neurons-mismatch" }; } };
      }
      seen.set(body.date, body.neurons_today);
      return { ok: true, status: 200, async json() { return { recorded: true }; } };
    }
    const q = JSON.parse(init.body).query;
    const geq = (q.match(/date_geq: "([^"]+)"/) || [])[1];
    const acc = {};
    if (q.includes("containersUsageAdaptiveGroups")) acc.containersUsageAdaptiveGroups = [];
    else if (q.includes("containersMetricsAdaptiveGroups")) acc.containersMetricsAdaptiveGroups = [];
    else acc.aiInferenceAdaptiveGroups = [{ sum: {
      totalNeurons: geq === "2026-08-27" ? neuronsYesterday
        : (extraCounts[geq] ?? neuronsToday) } }];
    return { ok: true, async json() { return { data: { viewer: { accounts: [acc] } } }; } };
  };
}

async function runHourMint(kv, iso, fetchImpl, extra = {}) {
  const realFetch = globalThis.fetch;
  globalThis.fetch = fetchImpl;
  try {
    return await atUtc(iso, async () => {
      const c = ctxStub();
      await worker.scheduled({}, env(kv, extra), c);
      await c.drain();
      return JSON.parse(kv.store.get("latest"));
    });
  } finally { globalThis.fetch = realFetch; }
}

const MINT = { KAT_SERVICE_URL: "https://kat.example", KAT_MINT_TOKEN: "mint-t0k" };

test("mint env unset: no POST, and the truthful status strings are pinned", async () => {
  const kv = kvStub();
  const posts = [];
  const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z", mintFetchStub({ posts }));
  assert.equal(posts.length, 0, "no KAT_* env → no POST at all");
  assert.equal(row.mint_post, undefined, "no mint_post field when unarmed");
  // The old strings described a shell that now EXISTS. Pin the truthful ones
  // so a regression back to `pending-worker-shell` fails here.
  assert.equal(row.mint, "anchor-only");
  assert.equal(row.mint_anchor.status, "accruing");
  assert.equal(row.mint_anchor.date, "2026-08-28");
  assert.equal(row.mint_anchor.neurons_today, 100);
  assert.equal(row.mint_anchor.kat, null);
  // Same field shape as before the swap-in — no new keys on the local anchor.
  assert.deepEqual(Object.keys(row.mint_anchor).sort(),
    ["date", "kat", "neurons_today", "note", "status"]);

  // The heartbeat arm has its own truthful string: nothing collected, so
  // nothing is accruing.
  const kv2 = kvStub();
  const hb = await runHourMint(kv2, "2026-08-28T12:00:06.549Z", mintFetchStub({ posts }),
    { CLOUDFLARE_OPS_API_TOKEN: undefined });
  assert.equal(hb.mint, "idle");
  assert.equal(hb.mint_anchor, undefined);
  assert.equal(posts.length, 0, "heartbeat mode never POSTs either");
});

test("mint env set: exactly one POST per run, carrying YESTERDAY's UTC day and its distinct count", async () => {
  const kv = kvStub();
  const posts = [];
  const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z",
    mintFetchStub({ posts }), MINT);
  assert.equal(posts.length, 1, "one cron run = exactly one mint POST");
  const p = posts[0];
  assert.equal(p.url, "https://kat.example/mint-anchor");
  assert.equal(p.init.method, "POST");
  assert.equal(p.init.headers.Authorization, "Bearer mint-t0k");
  assert.equal(p.init.headers["Content-Type"], "application/json");
  // The completed-day contract: the run at 12:00 on the 28th posts the 27th,
  // with the 27th's OWN count (77) — not today's in-progress 100. A stub
  // that could not tell the windows apart would have let a window bug pass.
  assert.equal(p.body.date, "2026-08-27");
  assert.equal(p.body.neurons_today, 77);
  assert.equal(typeof p.body.neurons_today, "number");
  assert.ok(Number.isInteger(p.body.neurons_today));
  assert.equal(p.body.source, "riir-kat-housekeeping");
  assert.ok(!Number.isNaN(Date.parse(p.body.generated_at)), "generated_at must be RFC3339");
  assert.equal(row.mint_post.status, "recorded");
  assert.equal(row.mint_post.date, "2026-08-27");
  assert.equal(row.mint_post.neurons_yesterday, 77);
  // The local anchor is untouched by the POST — still today, still accruing.
  assert.equal(row.mint_anchor.date, "2026-08-28");
  assert.equal(row.mint_anchor.status, "accruing");
});

test("mint replay: the second hourly run gets already-recorded, statelessly", async () => {
  const kv = kvStub();
  const posts = [];
  const stub = mintFetchStub({ posts });
  const r1 = await runHourMint(kv, "2026-08-28T12:00:06.549Z", stub, MINT);
  const r2 = await runHourMint(kv, "2026-08-28T13:00:06.549Z", stub, MINT);
  assert.equal(r1.mint_post.status, "recorded");
  // The 13:00 run re-derives the same completed day (2026-08-27) with the
  // same count → byte-identical body → the server (modelled here) answers
  // already-recorded. No client-side dedupe, no extra KV checkpoint.
  assert.equal(r2.mint_post.status, "already-recorded");
  assert.equal(posts.length, 2);
  assert.equal(posts[0].body.date, "2026-08-27");
  assert.equal(posts[1].body.date, "2026-08-27");
});

test("mint 409: recorded as an error status, never thrown, cron row still ok", async () => {
  const kv = kvStub();
  const errs = [];
  const realWarn = console.warn;
  console.warn = (...a) => errs.push(a.join(" "));
  try {
    const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z",
      mintFetchStub({ postMode: "conflict" }), MINT);
    assert.equal(row.mint_post.status, "error: http 409 (superseded)");
    assert.equal(row.ok, true, "a mint failure must not fail collection");
    assert.ok(kv.store.has("hour:2026082811"), "the cron row is written regardless");
    // "Log one line + record status": the warn lands in Workers Logs.
    assert.ok(errs.some((l) => l.includes("mint-anchor 2026-08-27")),
      `the 409 must be logged, got ${JSON.stringify(errs)}`);
  } finally { console.warn = realWarn; }
});

test("mint fetch rejects: fail-soft, row still written", async () => {
  const kv = kvStub();
  const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z",
    mintFetchStub({ postMode: "reject" }), MINT);
  assert.equal(row.mint_post.status, "error: mint post network failure");
  assert.equal(row.ok, true, "network failure must not fail collection");
  assert.ok(kv.store.has("hour:2026082811"));
  assert.ok(kv.store.has("latest"));
});

test("mint window rolls over at UTC midnight: the 00:00 run posts the day that just completed", async () => {
  const kv = kvStub();
  const posts = [];
  // 00:00:06 on the 29th UTC — the 28th completed four seconds ago. The stub
  // gives the 28th its OWN count (88), so the assertion proves the boundary
  // maps to the right GraphQL window, not a local-time drift.
  const row = await runHourMint(kv, "2026-08-29T00:00:06.549Z",
    mintFetchStub({ posts, extraCounts: { "2026-08-28": 88 } }), MINT);
  assert.equal(posts[0].body.date, "2026-08-28");
  assert.equal(posts[0].body.neurons_today, 88);
  assert.equal(row.mint_post.status, "recorded");
  assert.equal(row.mint_post.neurons_yesterday, 88);
});

// ---- riir-dapps Plan 058 T7b: a gen-2 target attests in-worker ------------
test("mint gen-2 target: 401 owner_sig_required is a skip, never an error or a warn", async () => {
  const kv = kvStub();
  const errs = [];
  const realWarn = console.warn;
  console.warn = (...a) => errs.push(a.join(" "));
  try {
    const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z",
      mintFetchStub({ postMode: "gen2" }), MINT);
    assert.ok(row.mint_post.status.startsWith("skipped: gen-2 target"), row.mint_post.status);
    assert.equal(row.ok, true);
    assert.equal(errs.filter((l) => l.includes("mint-anchor")).length, 0,
      `a gen-2 skip must not warn, got ${JSON.stringify(errs)}`);
  } finally { console.warn = realWarn; }
});

test("mint gen-1 401 (wrong bearer) stays an error", async () => {
  const kv = kvStub();
  const row = await runHourMint(kv, "2026-08-28T12:00:06.549Z",
    mintFetchStub({ postMode: "gen1-401" }), MINT);
  assert.equal(row.mint_post.status, "error: http 401");
});
