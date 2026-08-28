// Issue 044 T13 — auth-gate unit gate for src/worker.js.
//
// The live dashboard has no DASHBOARD_TOKEN set (owner decision), so the
// GATED branches cannot be exercised against production. This exercises them
// locally so that setting the secret cannot surface a first-use bug.
//   node cloudflare/dashboard/verify_auth.mjs
import worker from "./src/worker.js";

const SECRET = "s3cret-token-value";
const HK = "https://hk.example";
// Distinctive sentinel: a generic phrase collided with the 503 body text
// ("the private payload is never served"), which read as a leak.
const ASSET_BODY = "<html>SENTINEL_ASSET_BODY_9f3a</html>";
const assets = { fetch: async () => new Response(ASSET_BODY, { headers: { "Content-Type": "text/html", "Cache-Control": "public, max-age=0, must-revalidate" } }) };
const envOpen = { ASSETS: assets, HK_ORIGIN: HK };
const envDeny = { ASSETS: assets, HK_ORIGIN: HK, UNCONFIGURED_POLICY: "deny" };
const envGated = { ASSETS: assets, HK_ORIGIN: HK, DASHBOARD_TOKEN: SECRET };

const req = (url, { method = "GET", cookie } = {}) =>
  new Request(url, { method, headers: cookie ? { Cookie: cookie } : {} });
const U = "https://d.example/";

const fails = [];
const ok = (c, m) => { if (!c) fails.push(m); };
let n = 0;
const t = async (name, fn) => { n++; try { await fn(); } catch (e) { fails.push(`${name}: THREW ${e}`); } };

// ── hardening headers must be present on EVERY branch ──
const HARD = ["Content-Security-Policy", "Cache-Control", "X-Content-Type-Options",
  "X-Frame-Options", "Referrer-Policy", "X-Robots-Tag", "Vary"];
const assertHardened = (r, where) => {
  for (const h of HARD) ok(r.headers.get(h), `${where}: missing ${h}`);
  ok(/no-store/.test(r.headers.get("Cache-Control") || ""), `${where}: Cache-Control not no-store`);
  ok(r.headers.get("Referrer-Policy") === "no-referrer", `${where}: Referrer-Policy`);
  const csp = r.headers.get("Content-Security-Policy") || "";
  ok(csp.includes(`connect-src ${HK}`), `${where}: CSP connect-src not pinned to HK_ORIGIN`);
  ok(csp.includes("default-src 'none'"), `${where}: CSP default-src`);
};

await t("unconfigured/serve", async () => {
  const r = await worker.fetch(req(U), envOpen);
  ok(r.status === 200, `unconfigured serve status ${r.status}`);
  ok(r.headers.get("x-dashboard-auth") === "OPEN-INSECURE", "unconfigured marker not loud");
  ok(r.headers.get("x-dashboard-warning"), "unconfigured warning header absent");
  assertHardened(r, "unconfigured/serve");
});

await t("unconfigured/deny", async () => {
  const r = await worker.fetch(req(U), envDeny);
  ok(r.status === 503, `deny status ${r.status}`);
  ok(!(await r.text()).includes("SENTINEL_ASSET_BODY_9f3a"), "DENY LEAKED THE PAYLOAD");
  assertHardened(r, "unconfigured/deny");
});

await t("gated: no credential → 401", async () => {
  const r = await worker.fetch(req(U), envGated);
  ok(r.status === 401, `no-cred status ${r.status}`);
  const body = await r.text();
  ok(!body.includes("SENTINEL_ASSET_BODY_9f3a"), "401 LEAKED THE PAYLOAD");
  ok(!body.includes(SECRET), "401 ECHOED THE SECRET");
  assertHardened(r, "gated/401");
});

await t("gated: wrong token → 401, no echo", async () => {
  for (const bad of ["", "x", SECRET + "x", SECRET.slice(0, -1), SECRET.toUpperCase()]) {
    const r = await worker.fetch(req(`${U}?token=${encodeURIComponent(bad)}`), envGated);
    ok(r.status === 401, `wrong token ${JSON.stringify(bad)} → ${r.status}`);
    ok(!(await r.text()).includes("SENTINEL_ASSET_BODY_9f3a"), `wrong token ${JSON.stringify(bad)} LEAKED`);
  }
});

await t("gated: wrong cookie → 401", async () => {
  const r = await worker.fetch(req(U, { cookie: `dashboard_token=${SECRET}x` }), envGated);
  ok(r.status === 401, `wrong cookie status ${r.status}`);
  ok(!(await r.text()).includes("SENTINEL_ASSET_BODY_9f3a"), "wrong cookie LEAKED");
});

await t("gated: correct token → 301 + cookie, token stripped", async () => {
  const r = await worker.fetch(req(`${U}?token=${SECRET}&keep=1`), envGated);
  ok(r.status === 301, `exchange status ${r.status}`);
  const loc = r.headers.get("Location") || "";
  ok(!loc.includes("token="), `Location still carries the token: ${loc}`);
  ok(loc.includes("keep=1"), "exchange dropped unrelated query params");
  const sc = r.headers.get("Set-Cookie") || "";
  for (const attr of ["HttpOnly", "Secure", "SameSite=Lax", "Path=/", "Max-Age=2592000"])
    ok(sc.includes(attr), `Set-Cookie missing ${attr}`);
  ok((await r.text()) === "", "301 had a body");
});

await t("gated: correct cookie → 200 payload", async () => {
  const r = await worker.fetch(req(U, { cookie: `dashboard_token=${SECRET}` }), envGated);
  ok(r.status === 200, `cookie-auth status ${r.status}`);
  ok((await r.text()).includes("SENTINEL_ASSET_BODY_9f3a"), "cookie-auth did NOT serve the payload");
  ok(r.headers.get("x-dashboard-auth") === "gated-ok", "cookie-auth marker");
  assertHardened(r, "gated/200");
});

await t("cookie parsing: multi-cookie jar", async () => {
  const r = await worker.fetch(req(U, { cookie: `a=1; dashboard_token=${SECRET}; z=2` }), envGated);
  ok(r.status === 200, `multi-cookie status ${r.status}`);
});

await t("non-GET rejected on every env", async () => {
  for (const [name, env] of [["open", envOpen], ["deny", envDeny], ["gated", envGated]]) {
    for (const m of ["POST", "PUT", "DELETE"]) {
      const r = await worker.fetch(req(U, { method: m }), env);
      ok(r.status === 405, `${name}/${m} → ${r.status}`);
    }
  }
});

await t("HEAD allowed when authorized", async () => {
  const r = await worker.fetch(req(U, { method: "HEAD", cookie: `dashboard_token=${SECRET}` }), envGated);
  ok(r.status === 200, `HEAD status ${r.status}`);
});

if (fails.length) {
  console.error(`FAIL (${fails.length}):`);
  for (const f of fails) console.error("  - " + f);
  process.exit(1);
}
console.log(`PASS: ${n} auth scenarios — unconfigured serve/deny, 401 paths (no leak, no echo), token exchange strips token + sets HttpOnly/Secure/SameSite, cookie auth serves, 405 on non-GET, hardening headers on every branch`);
