# The corpus PR lane (devnet → mainnet)

> **Scope:** the `cargo refine corpus-pr` client half of the corpus PR lane
> (riir-dapps Plan 050, S1.11) — the refine-integrated PR builder +
> submitter. The server half (the ingest lib + the worker routes + the
> PR-attestation gate) lives in riir-dapps `src/kat/corpuspr.rs` +
> `cloudflare/kat-service/src/routes.rs`; the ops-path builder twin is
> riir-dapps `cloudflare/kat-service/scripts/build_corpus_pr.py`.

## The model, one line

Devnet is the dev branch, mainnet is main, a corpus PR is an
owner-attested batch of mined pattern pairs carried from one ledger to
the other — rebase-before-merge, mainnet-first conflict resolution
(MAINNET WINS, the PR never overwrites), batch acceptance in chunks,
monotone pr_seq per `(src, dst)` pair (the burn-watermark idiom).

## The command

```sh
# the read-only divergence gauge (no key, no writes — the dark-era posture)
cargo refine corpus-pr --src devnet --dst mainnet --submitter <32hex>

# draft (writes the submission files + binds the T2b stamps)
cargo refine corpus-pr --src devnet --dst mainnet --submitter <32hex> --key-file .corpuspr.key

# submit (POST envelope, then chunks in order; replays idempotent)
cargo refine corpus-pr --src devnet --dst mainnet --submitter <32hex> \
    --key-file .corpuspr.key --seq 1 --submit
```

Needs `kat_transport` (even the gauge reads both leases over the
network). The key file is the `kat_corpus_sign --gen-corpuspr` format
(64 hex chars, the raw 32-byte seed) — the DEDICATED PR-attestation
class, never the anchor key.

## The S1.0 rebase (the owner round-2 amendment)

The builder's FIRST step, before any attestation: read the destination's
serving lease (a PUBLIC read — T10/T2b gate the PR WRITE, never reads),
key every source row by `record_id` + `blake3(fix)`, drop what the
destination already carries or supersedes, and print

```
rebase: kept K of N rows (d duplicates, c superseded)
```

Three laws:

- **(a)** rebase is an optimization + hygiene layer, NEVER the
  correctness defense — the TOCTOU window between rebase and ingest
  closes SERVER-side (exactly-once + the three-arm + the recomputed-root
  check), exactly as a git rebase does not replace merge-time conflict
  resolution.
- **(b)** rebase never touches the source ledger — rows, credit and
  history stay committed there; only the outgoing patch is reshaped.
- **(c)** an all-duplicate result refuses loudly: `nothing new for
  mainnet — mine on mainnet instead` (saves the caps budget; serves the
  incentive gradient).

## The gates (every PR write)

| Gate | Law |
|---|---|
| **T10 attested-intent** | the dst URL names mainnet AND its `GET /` attests `env: mainnet` + a ledger id (`kat_net_intent`'s fail-closed judge); the src is judged by its own URL's intent + must serve its ledger id |
| **T2b stamps** | the per-pair state records both hosts + both ledger ids; ANY drift refuses loudly — a reset is a fresh namespace, the remedy is adjudicating the state file, never auto-rebirth |
| **Net pins** | both dials pass `ensure_net_allowed` — a pinned build never dials outside its pin |

## The layout (per-net, the T1 law)

```
.refine/net/<dst-key>/corpuspr/<src-key>/
  state.json          the T2b pair stamp + the last registered pr_seq
  pr/<seq>/envelope.json, chunk_NNNN.json, manifest.json
  submissions.jsonl   the append-only submit log
```

`last_pr_seq` (the client-local tick of S1.0) bumps AT REGISTRATION — a
draft does not consume the seq; a crash mid-chunk resumes at the same
seq (a byte-identical envelope answers already-registered; an applied
chunk answers idempotently — the exactly-once law).

## The mirror law

riir-refine may NOT depend on riir-dapps (the boundary allowlist). The
wire — the signing domain `riir-kat-corpuspr:v1:`, the
leaf/chunk/top-root derivations, the record identity, the caps, the two
route bodies — mirrors riir-dapps `src/kat/corpuspr.rs` BYTE-FOR-BYTE,
pinned by the SAME frozen golden vectors that pin the dapps lib and the
Python builder (`build_corpus_pr.py --selftest`): a **three-way pin**.
A drift on any side reds that side's vectors.

## The live findings (on record)

- **2026-10-13, the first live read:** devnet's serving corpus contains
  rows the wire caps REFUSE — the `doc_lazy_continuation` pack serves an
  empty `buggy_code` (6 rows src + 3 dst at the reading). The client
  SKIPS them with a disclosure line (the ingest secret-scan precedent:
  the row is skipped, the chunk stays valid); the server's own
  `validate` would refuse the whole chunk at ingest, and by the
  contiguity law that would block the PR — the python builder has this
  defect latent. The same read measured the live divergence:
  `kept 5 of 138 rows (133 duplicates, 0 superseded)` — mainnet already
  carries most of devnet's corpus (the Stage-0 carry); **5 rows are
  genuinely new for mainnet** and wait on the owner's key mint (S1.12).

## The dark-era posture

The lane ships DARK until the owner mints the PR-attestation key
(`kat_corpus_sign --gen-corpuspr` from the M3) and deploys
`KAT_CORPUSPR_PUBKEY_HEX`. Until then: the gauge is the monitoring
posture (read-only, no key), and a `--submit` against an unconfigured
worker fails at its 503 gate — loud, named, never a silent pass.
