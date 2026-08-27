// riir-ops-dashboard — Issue 044 T2 (riir-clippy): token query gate in
// front of the generated aggregate HTML. Deliberately dependency-free
// (no build step; wrangler deploys this file).
//
// Fail-open rationale: until the owner sets the DASHBOARD_TOKEN secret
// (one-time `npx wrangler secret put DASHBOARD_TOKEN`) every request
// passes straight through — identical to v1's assets-only behavior,
// because the HTML carries only aggregated numbers. Once the secret
// exists the gate is CLOSED: authorized via ?token=… (exchanged once
// for an HttpOnly cookie so it leaves the URL bar) or the cookie itself.

const COOKIE_NAME = "dashboard_token";

// Constant-time-ish compare: length check first, then XOR-accumulate over
// every byte — never short-circuit on an early mismatch.
function tokensMatch(a, b) {
  const A = new TextEncoder().encode(String(a));
  const B = new TextEncoder().encode(String(b));
  if (A.length !== B.length) return false;
  let diff = 0;
  for (let i = 0; i < A.length; i++) diff |= A[i] ^ B[i];
  return diff === 0;
}

function cookieValue(request, name) {
  for (const part of (request.headers.get("Cookie") || "").split(";")) {
    const eq = part.indexOf("=");
    if (eq < 0) continue;
    if (part.slice(0, eq).trim() === name) return part.slice(eq + 1).trim();
  }
  return null;
}

function withHeader(res, name, value) {
  const r = new Response(res.body, res);
  r.headers.set(name, value);
  return r;
}

export default {
  async fetch(request, env, ctx) {
    // Fail-open: no secret configured → v1 behavior, observably marked so
    // a curl can tell which mode is live.
    if (!env.DASHBOARD_TOKEN) {
      return withHeader(await env.ASSETS.fetch(request), "x-dashboard-auth", "open");
    }

    const secret = String(env.DASHBOARD_TOKEN);
    const url = new URL(request.url);
    const queryToken = url.searchParams.get("token");

    // (b) ?token=… matches → one-time exchange: 301 to the same URL minus
    // the token param, planting the cookie (30d, HttpOnly).
    if (queryToken !== null && tokensMatch(queryToken, secret)) {
      url.searchParams.delete("token");
      return new Response(null, {
        status: 301,
        headers: {
          Location: url.toString(),
          "Set-Cookie": `${COOKIE_NAME}=${queryToken}; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000`,
          "x-dashboard-auth": "gated-ok",
        },
      });
    }

    // (a) cookie matches → pass through to the assets.
    const cookie = cookieValue(request, COOKIE_NAME);
    if (cookie !== null && tokensMatch(cookie, secret)) {
      return withHeader(await env.ASSETS.fetch(request), "x-dashboard-auth", "gated-ok");
    }

    // Unauthorized. Never echo the token; hint the one-time query flow.
    return new Response(
      "401 — this dashboard is token-gated. Append ?token=… to authenticate once.",
      { status: 401, headers: { "Content-Type": "text/html; charset=utf-8" } },
    );
  },
};
