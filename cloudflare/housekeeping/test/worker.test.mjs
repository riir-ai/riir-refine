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

function kvStub(seed = {}) {
  const store = new Map(Object.entries(seed));
  const ops = { get: 0, put: 0, list: 0 };
  return {
    store,
    ops,
    async get(k) { ops.get++; return store.has(k) ? store.get(k) : null; },
    async put(k, v) { ops.put++; store.set(k, v); },
    async list({ prefix = "", limit } = {}) {
      ops.list++;
      const keys = [...store.keys()].filter((k) => k.startsWith(prefix)).sort()
        .map((name) => ({ name }));
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

// Minimal CF GraphQL stub: one app burning a fixed footprint every hour.
function fetchStub() {
  return async (_url, init) => {
    const q = JSON.parse(init.body).query;
    const acc = {};
    if (q.includes("containersUsageAdaptiveGroups")) {
      acc.containersUsageAdaptiveGroups = [{
        dimensions: { applicationId: "a03a70ff-88e9-45c5-af80-339ab7707fbe" },
        sum: { allocatedMemory: 1073741824, allocatedDisk: 1e9, cpuTimeSec: 10, txBytes: 5 },
      }];
    } else if (q.includes("containersMetricsAdaptiveGroups")) {
      acc.containersMetricsAdaptiveGroups = [{
        dimensions: { applicationId: "a03a70ff-88e9-45c5-af80-339ab7707fbe" },
        sum: { containerUptime: 3600000 },
      }];
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
