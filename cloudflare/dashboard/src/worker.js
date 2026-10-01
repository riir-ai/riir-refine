// riir-ops-dashboard — Issue 044 T2/T13 (riir-refine): auth gate in front of
// the generated ops HTML. Deliberately dependency-free (no build step;
// wrangler deploys this file).
//
// Authenticate once via ?token=… (301-exchanged for an HttpOnly cookie so the
// token leaves the URL bar), or present the cookie.
//
// The GATED branches are unreachable live while DASHBOARD_TOKEN is unset, so
// they are unit-gated instead — run `node cloudflare/dashboard/verify_auth.mjs`
// after ANY edit to this file.
//
// ── T13 (2026-08-28): the fail-open rationale was falsified ────────────────
// T2 failed OPEN when DASHBOARD_TOKEN was unset, on the rationale that "the
// HTML carries only aggregated numbers". A payload audit falsified that: the
// embedded `ds` JSON carries 19 FULL ledger rows with internal issue/proposal
// references, 10 PRIVATE repo names with per-repo unhealed-lint counts,
// per-repo rule-adoption stats, container app ids and cost, and fixer-quality
// internals — a private engineering profile, not aggregates.
//
// The fix is to SET the secret (one command, below). Two knobs exist because
// deploy order matters:
//
//   DASHBOARD_TOKEN (secret)    — set it and the gate is CLOSED. Until then
//                                 every request hits the unconfigured branch.
//     cd cloudflare/dashboard && npx wrangler secret put DASHBOARD_TOKEN
//
//   UNCONFIGURED_POLICY (var)   — what the unconfigured branch does:
//                                 "serve" (default) = T2 behavior, marked
//                                 `x-dashboard-auth: OPEN-INSECURE`;
//                                 "deny" = 503, never serve unauthenticated.
//
// Default is "serve" ONLY so that deploying this hardening cannot take the
// dashboard offline ahead of the secret. Target state: set the secret, then
// flip the var to "deny" so an accidentally deleted secret fails safe instead
// of silently republishing the payload. Once the secret exists the branch is
// unreachable either way.
//
// ── AUTH_MODE (2026-09-04, OWNER CALL): the dashboard is PUBLIC ─────────────
// The owner asked for the dashboard to open with no token ("i just want to
// open it and it should be public"). AUTH_MODE="public" serves the assets to
// EVERY caller, marked `x-dashboard-auth: public` — an explicit, recorded
// decision in the deployed config, NOT the silent fail-open branch (that one
// stays loud with OPEN-INSECURE for any future accidental state). Reversible:
// delete the AUTH_MODE var (or set it to anything else) and the token gate
// resumes with the SAME secret. The T13 payload-audit caveat now applies to
// anonymous visitors BY OWNER DECISION: the page carries the private
// engineering profile (repo names, lint counts, costs, fixer internals).

const COOKIE_NAME = "dashboard_token";
const HK_ORIGIN_DEFAULT = "https://riir-kat-housekeeping.foxfox.workers.dev";

// Length-INDEPENDENT compare: folding the length difference into the same
// accumulator means a wrong token cannot have its length recovered from
// response timing. (The prior `if (A.length !== B.length) return false`
// short-circuited and leaked exactly that.)
function timingSafeEqual(a, b) {
  const A = new TextEncoder().encode(String(a));
  const B = new TextEncoder().encode(String(b));
  let diff = A.length ^ B.length;
  const n = Math.max(A.length, B.length);
  for (let i = 0; i < n; i++) diff |= (A[i] || 0) ^ (B[i] || 0);
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

// Applied to EVERY response, including 401/503, so the posture never depends
// on which branch answered.
function harden(res, env) {
  const r = new Response(res.body, res);
  const hk = env.HK_ORIGIN || HK_ORIGIN_DEFAULT;
  // The HTML is fully self-contained (inline <style>/<script>, inline SVG,
  // zero external assets) and makes exactly ONE cross-origin call: the
  // housekeeping worker's /latest live badge. So everything else is denied.
  r.headers.set(
    "Content-Security-Policy",
    [
      "default-src 'none'",
      "script-src 'unsafe-inline'", // inline <script>, no nonce plumbing
      "style-src 'unsafe-inline'",
      `connect-src ${hk}`,
      "base-uri 'none'",
      "form-action 'none'",
      "frame-ancestors 'none'",
    ].join("; "),
  );
  // The response body is PRIVATE. The assets pipeline serves
  // `cache-control: public, max-age=0, must-revalidate` and CF was observed
  // edge-caching it (cf-cache-status: HIT). A shared cache holding a gated
  // body is an auth bypass whenever the cache key omits the cookie — so the
  // worker overrides to no-store and varies on Cookie.
  r.headers.set("Cache-Control", "private, no-store, max-age=0, must-revalidate");
  r.headers.set("Vary", "Cookie");
  r.headers.set("X-Content-Type-Options", "nosniff");
  r.headers.set("X-Frame-Options", "DENY");
  // no-referrer: the one-time ?token=… must never ride a Referer header out.
  r.headers.set("Referrer-Policy", "no-referrer");
  r.headers.set("X-Robots-Tag", "noindex, nofollow, noarchive");
  r.headers.set("Cross-Origin-Opener-Policy", "same-origin");
  r.headers.set("Permissions-Policy", "geolocation=(), camera=(), microphone=()");
  return r;
}

export default {
  async fetch(request, env) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return harden(
        new Response("405 — method not allowed", {
          status: 405,
          headers: { Allow: "GET, HEAD", "Content-Type": "text/plain; charset=utf-8" },
        }),
        env,
      );
    }

    // ── AUTH_MODE="public" (owner call, 2026-09-04) ── serve everyone.
    // Evaluated FIRST so it overrides both the unconfigured branch and the
    // token gate; ?token=… still exchanges for a cookie (harmless), but no
    // credential is REQUIRED. Deliberately distinct from the silent fail-open
    // branch below: this is a recorded config decision, marked `public`.
    if (String(env.AUTH_MODE || "") === "public") {
      const res = await env.ASSETS.fetch(request);
      const out = harden(res, env);
      out.headers.set("x-dashboard-auth", "public");
      return out;
    }

    // No secret configured → UNCONFIGURED_POLICY decides (see header).
    if (!env.DASHBOARD_TOKEN) {
      if (String(env.UNCONFIGURED_POLICY || "serve") === "deny") {
        return harden(
          new Response(
            "503 — dashboard not configured. The private payload is never served " +
              "unauthenticated; set the gate secret first:\n" +
              "  cd cloudflare/dashboard && npx wrangler secret put DASHBOARD_TOKEN\n",
            {
              status: 503,
              headers: {
                "Content-Type": "text/plain; charset=utf-8",
                "x-dashboard-auth": "unconfigured-denied",
              },
            },
          ),
          env,
        );
      }
      const res = await env.ASSETS.fetch(request);
      const out = harden(res, env);
      // Loud, not neutral: this response published a private payload. The old
      // value was a mild `open`; a curl / uptime check should read as a finding.
      out.headers.set("x-dashboard-auth", "OPEN-INSECURE");
      out.headers.set(
        "x-dashboard-warning",
        // ASCII only: HTTP header values are ByteStrings. workerd tolerated an
        // em-dash here, but Node/undici throws on it, so the header was
        // unportable and untestable off-workerd (caught by verify_auth.mjs).
        "DASHBOARD_TOKEN unset - private ops payload served unauthenticated (Issue 044 T13)",
      );
      return out;
    }

    const secret = String(env.DASHBOARD_TOKEN);
    const url = new URL(request.url);
    const queryToken = url.searchParams.get("token");

    // (a) ?token=… matches → one-time exchange: 301 to the same URL minus the
    // token param, planting the cookie (30d, HttpOnly).
    if (queryToken !== null && timingSafeEqual(queryToken, secret)) {
      url.searchParams.delete("token");
      return harden(
        new Response(null, {
          status: 301,
          headers: {
            Location: url.toString(),
            "Set-Cookie":
              `${COOKIE_NAME}=${queryToken}; Path=/; HttpOnly; Secure; ` +
              `SameSite=Lax; Max-Age=2592000`,
            "x-dashboard-auth": "gated-ok",
          },
        }),
        env,
      );
    }

    // (b) cookie matches → serve the assets.
    const cookie = cookieValue(request, COOKIE_NAME);
    if (cookie !== null && timingSafeEqual(cookie, secret)) {
      const res = await env.ASSETS.fetch(request);
      const out = harden(res, env);
      out.headers.set("x-dashboard-auth", "gated-ok");
      return out;
    }

    // Unauthorized. Never echo the token; hint the one-time query flow.
    return harden(
      new Response("401 — token-gated. Append ?token=… to authenticate once.\n", {
        status: 401,
        headers: {
          "Content-Type": "text/plain; charset=utf-8",
          "x-dashboard-auth": "denied",
        },
      }),
      env,
    );
  },
};
