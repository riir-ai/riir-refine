#!/usr/bin/env node
// verify_token.mjs — does this token actually do the job? (Issue 045 T4)
//
//   node cloudflare/housekeeping/verify_token.mjs <token>
//   CF_API_TOKEN=... node cloudflare/housekeeping/verify_token.mjs
//
// Run this BEFORE `wrangler secret put CF_API_TOKEN`. A token with the wrong
// permission still deploys fine and then fails silently at 00:00 UTC — the
// worker degrades to heartbeat rows and the only symptom is `ok:false` in a
// KV row nobody reads. This exercises the three queries `worker.js` actually
// issues, so a PASS here means the cron will collect.
//
// The token is read from argv/env and only ever placed in a request header.
// It is never logged, and never written to disk.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const GRAPHQL = "https://api.cloudflare.com/client/v4/graphql";

const token = process.argv[2] || process.env.CF_API_TOKEN;
if (!token) {
  console.error("usage: node verify_token.mjs <token>   (or set CF_API_TOKEN)");
  process.exit(2);
}

// Single source of truth for the account id — same file the worker deploys with.
const toml = readFileSync(join(HERE, "wrangler.toml"), "utf8");
const acct = (toml.match(/^CF_ACCOUNT_ID\s*=\s*"([^"]+)"/m) || [])[1];
if (!acct) {
  console.error("could not read CF_ACCOUNT_ID from wrangler.toml");
  process.exit(2);
}

const day = new Date(Date.now() - 86400e3).toISOString().slice(0, 10);
const hourEnd = new Date().toISOString().slice(0, 13) + ":00:00Z";
const hourStart = new Date(Date.now() - 26 * 3600e3).toISOString().slice(0, 13) + ":00:00Z";

// Exactly the three queries worker.js issues, same shape, same filters.
const QUERIES = [
  ["containers usage (billing)", `{ viewer { accounts(filter: {accountTag: "${acct}"}) {
      containersUsageAdaptiveGroups(limit: 100,
        filter: {datetimeHour_geq: "${hourStart}", datetimeHour_lt: "${hourEnd}"}) {
        dimensions { applicationId datetimeHour }
        sum { allocatedMemory allocatedDisk cpuTimeSec txBytes } } } } }`],
  ["containers metrics (uptime)", `{ viewer { accounts(filter: {accountTag: "${acct}"}) {
      containersMetricsAdaptiveGroups(limit: 200,
        filter: {datetimeHour_geq: "${hourStart}", datetimeHour_lt: "${hourEnd}"}) {
        dimensions { applicationId datetimeHour }
        sum { containerUptime } } } } }`],
  ["workers-ai neurons", `{ viewer { accounts(filter: {accountTag: "${acct}"}) {
      aiInferenceAdaptiveGroups(limit: 2, filter: {date_geq: "${day}", date_leq: "${day}"}) {
        sum { totalNeurons } } } } }`],
];

const auth = { Authorization: `Bearer ${token}`, "Content-Type": "application/json" };
let failed = 0;

// Informational only: an OAuth token 401s here while still querying GraphQL
// fine, so a failure is not by itself disqualifying — the GraphQL probes below
// are the real gate.
//
// Two endpoints, because account-owned and user tokens verify at different
// paths and each 4xxs on the other's token. Account-owned is the recommended
// shape for this job (a durable service principal that does not die with a
// user), so try that first.
const VERIFY = [
  [`accounts/${acct}/tokens/verify`, "account-owned"],
  ["user/tokens/verify", "user"],
];
for (const [path, kind] of VERIFY) {
  try {
    const r = await fetch(`https://api.cloudflare.com/client/v4/${path}`, { headers: auth });
    const j = await r.json();
    if (j?.result?.status === "active") {
      console.log(`  token/verify   ACTIVE (${kind} token)`);
      break;
    }
    console.log(`  token/verify   ${kind}: ${r.status} ${JSON.stringify(j?.errors ?? j?.result?.status)} (informational)`);
  } catch (e) {
    console.log(`  token/verify   ${kind}: unreachable: ${e.message} (informational)`);
  }
}

for (const [label, query] of QUERIES) {
  const pad = label.padEnd(28);
  try {
    const res = await fetch(GRAPHQL, {
      method: "POST", headers: auth, body: JSON.stringify({ query }),
      signal: AbortSignal.timeout(20000),
    });
    const body = await res.json().catch(() => null);
    if (!res.ok) {
      // A permission problem surfaces HERE, not at token/verify.
      console.log(`✗ ${pad} HTTP ${res.status} ${JSON.stringify(body?.errors ?? "").slice(0, 200)}`);
      failed++; continue;
    }
    if (body?.errors) {
      console.log(`✗ ${pad} ${JSON.stringify(body.errors).slice(0, 220)}`);
      failed++; continue;
    }
    const acc = body?.data?.viewer?.accounts?.[0];
    if (!acc) {
      console.log(`✗ ${pad} no account in response — token is not scoped to ${acct.slice(0, 8)}…`);
      failed++; continue;
    }
    // An empty row set is a PASS: it means "authorised, nothing billed in the
    // window". Only an error means the token cannot do the job.
    const rows = Object.values(acc)[0] ?? [];
    console.log(`✓ ${pad} ${rows.length} row(s)`);
  } catch (e) {
    console.log(`✗ ${pad} ${e.message}`);
    failed++;
  }
}

console.log(failed === 0
  ? "\nPASS — this token can run every query the cron issues.\n  next: cd cloudflare/housekeeping && npx wrangler secret put CF_API_TOKEN"
  : `\nFAIL — ${failed}/3 queries rejected. Do not set this as the secret;\n  the worker would deploy fine and then collect nothing.`);
process.exit(failed === 0 ? 0 : 1);
