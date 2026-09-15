# Issue 112 (2026-09-16) — Issue 110's repair is correct and unwitnessed; and the shipped numbers CAN be adjudicated after all

**Status:** RESOLVED 2026-09-16 — witness landed at all three sites, proven
by perturbation against a real cargo run; the retrospective question Issue
110 left open is answered for every artifact this repo actually shipped
**Severity:** MEDIUM (the defect is fixed; this is the tripwire that makes a
recurrence loud) + the audit half, which is HIGH-value because Issue 110
recorded it as unanswerable
**Sites:** `src/domains/probe_witness.rs` (new),
`src/domains/clippy_lints/verify.rs`, `src/domains/rust_perf/verify.rs`,
`src/domains/rust_perf/bench.rs`

---

## Half A — the shipped artifacts, adjudicated

Issue 110 closed with:

> ⛔ **UNDETERMINED, deliberately not assumed either way:** whether any
> shipped number — a `fix_verify` verdict, a corpus label, a `.benchmarks/`
> perf row — was produced under a shared target dir. Nothing recorded the
> environment, so this cannot be answered from the artifacts.

The first clause is right and the second is **too strong**. Nothing recorded
the *environment*, but two of the three sites emit a quantity that the
collision necessarily destroys, and those quantities are in the committed
rows. The question was never "what does the provenance field say"; it was
"is there anything in the output that a replay could not have produced".

### `ClippyVerifier` — clean, by the `control_fires` scaffold

`cf_workers_ai::score_case` calls `verify()` **twice per case**, `before` on
the buggy module and `after` on the merged one, and `control_fires` asserts
that the **buggy** input trips its own target lint. Under the collision every
`verify()` after the first in a run replays the first's diagnostics, so from
case 2 onward `control_fires` asks whether case 1's module trips case *k*'s
lint — which is false unless the two cases share a lint. **A contaminated
multi-case run drives `control_fires` to ~1.**

Every committed row in `.benchmarks/fixer_quality_history.jsonl`:

| ts_unix | arm | model | n | control_fires |
|---|---|---|---:|---:|
| 1787852344 | G | llama-3.3-70b | 9 | 9 |
| 1787852373 | B | llama-3.3-70b | 9 | 9 |
| 1787852397 | A | llama-3.3-70b | 9 | 9 |
| 1787935952 | A | glm-5.3-flash | 9 | 9 |
| 1787936019 | B | glm-5.3-flash | 9 | 9 |
| 1788585040 | B | glm-5.3-flash | 25 | 24 |
| 1788585260 | B | llama-3.3-70b | 25 | 24 |
| 1788585325 | G2 | llama-3.3-70b | 25 | 24 |

8 of 8 at `n` or `n − 1`. The three 24/25 rows are the same shortfall across
**two different models**, which is a per-sample property (one fixture whose
buggy module does not trip its declared lint), not a replay — a replay is a
property of the run, not of the sample, and would not reproduce at the same
count under a different generator.

⚠ Read the direction honestly: what the scaffold proves is that the
**`before`** probes were real. It was built for a different purpose (Bench
039: *"without it a non-compiling crate reads as resolved"*) and it turns out
to be the exact witness this question needed — which is an argument for
positive controls in general, not a licence to skip the witness in Half B.

### `PerfBenchValidator` — clean, by value distinctness

`.benchmarks/013_rust_perf_bench_pareto_goat.md` is the GOAT that PROMOTED
`rust_perf` to default-on, so it is the highest-stakes row in the repo. Under
the bin collision every rule times **one** binary, so the per-size numbers
would be identical across rules. Measured over the file's 39 rule rows: **31
carry numeric speedup profiles and all 31 are distinct**; the only repeated
"profile" is the empty one, shared by the 8 non-numeric rows (the
`NotBenchable` set the same document reports). The recorded invocation is
`cargo run --example perf_bench_demo`, which sets no `CARGO_TARGET_DIR`.

### `PerfVerifier` — clean by absence

No shipped artifact was ever produced by it. Its only appearance across
`.benchmarks/`, `.docs/` and `.plans/` is Bench 019's G3 row recording that
`--features rust_perf_verify` **type-checks** — a compile assertion, not a
verdict corpus. There is nothing to adjudicate.

### What stays open

The `data/` miss-shape corpus rows Issue 110 also named belong to
**riir-train**, and its Issue 555 is the place that question gets answered;
nothing here can settle it. And this audit is about the rows that were
**committed** — a local `cargo heal --verify` run somebody did under an
isolated target dir left no artifact and never can be adjudicated. That is
the residual, and it is the one Half B removes for the future.

## Half B — the witness

Issue 110's repair (key the package name with the directory's BLAKE3 stem)
makes the collision impossible **by construction**, and a construction that
nothing observes can be quietly re-opened: a de-keyed name, a reachable
`.cargo/config.toml` with `build.target-dir`, a cargo freshness change. The
failure mode would be exactly as silent as before — `present_lints = []` →
`lint_resolved = true`, exit 0.

### What is observable, measured rather than assumed

Two crates named `clippy_verify v0.0.0` in one `CARGO_TARGET_DIR`, A =
`pub fn f() -> i32 { return 1; }`, B = `pub fn g() -> i32 { return 2; }`,
reading B's `--message-format=json`:

| field | says | usable? |
|---|---|---|
| `package_id` | `…/probewit/b#clippy_verify@0.0.0` | ⛔ **lies** — cargo rewrites it to the manifest asked for |
| `target.src_path` | `…/probewit/b/src/lib.rs` | ⛔ **lies**, same reason |
| span `file_name` | `src\lib.rs` | ⛔ relative, identical for every probe |
| span `text[].text` | `pub fn f() -> i32 { return 1; }` | ⚠ the truth — but it is **A's** source line, and it exists only when a diagnostic fires |
| `compiler-artifact.fresh` | `true` | ✅ the witness |

The two fields a reader reaches for first are precisely the ones cargo
rewrites. The span text is the only field that betrays the replay outright,
and it is **absent in the silent case that matters** (zero diagnostics), so
it cannot be the witness.

### The predicate

`fresh: true` alone over-fires: it is legitimate on a repeat probe of
identical code, which is what the BLAKE3-keyed directory exists to enable.
The exact predicate is the **join** with a fact only the probe knows —
whether it just *changed* `src/lib.rs`. Cargo cannot legitimately call a unit
fresh whose source moved under it.

| | `fresh` | not `fresh` |
|---|---|---|
| **source changed** | ⛔ `StaleAfterWrite` | `Compiled` |
| **source unchanged** | `CachedUnchanged` | `Compiled` |

`write_probe_source` reads before writing, which both supplies the flag and
leaves the mtime alone on an unchanged repeat — without that, the cache hit
the directory exists to provide stops being one.

⛔ **The fourth arm was found by the arms, not by design.** The first version
had three, and the `--ignored` real-cargo tests immediately red: a probe whose
code does not compile emits **no artifact at all**, which that version read as
blindness, so both verifiers refused on broken input — the input they are most
often handed, and the case
`verify_compilation_error_reports_not_resolved` exists to assert.
`CompileFailed` (no artifact, unsuccessful `build-finished`, ≥1 error-level
message) is a **verdict**, not blindness. Its residual is named rather than
assumed away: a replay whose donor ALSO failed lands here carrying the donor's
errors — rarer (a failed unit has no successful fingerprint to replay from)
and conservative (it rejects a fix rather than accepting one). The
silent-false-PASS direction cannot reach it: that has an artifact and no
errors, so it lands on the freshness arm regardless of what `build-finished`
says, and an arm pins exactly that.

### The `cargo run` site needs a different witness, and gets a stronger one

`bench.rs` spawns `cargo run --release` and reads the child's own stdout —
there is no cargo JSON stream to read `fresh` out of. But the executed
**program** can state its own identity, which is a stronger assertion than
unit freshness: the generated `main` prints `{"probe":"<stem>"}` as its first
line and `run_bench` refuses unless it matches the stem it generated. An
absent line is **not** agreement (a stale binary predating the echo yields
`None`), matching `NoArtifact`'s standing.

### One module, three sites

`src/domains/probe_witness.rs`, gated on the union of `clippy_verify`,
`rust_perf_verify` and `rust_perf_bench` rather than living inside one of
them. A copy per site is a second thing to get wrong — which is the shape
Issue 110 was, and the shape riir-train's port of it was.

## Verification (2026-09-16, shikuwa)

**Perturbation, against a real cargo run** — the witness is only a witness if
its own perturbation reds it. `generate_main_rs` made to stamp a foreign key
(`"0000000000000000"`), as a de-keyed `[[bin]]` collision would:

```text
batch-135 'atomic-vec-zero-cost-convert' bench must run: Some("probe identity
mismatch for `atomic-vec-zero-cost-convert`: generated `737d13ddff218032`, the
binary that ran said `0000000000000000` — these nanoseconds describe other
code (Issue 110 scratch-crate unit collision) …")
```

That is the end-to-end path: temp crate → `cargo build` → `cargo run` →
parser. Restored, green.

**Arms** — 10 in `probe_witness` (both freshness directions, the cache-hit
licence, `CompileFailed` vs warnings-only vs errors-without-`build-finished`,
the collision's inability to hide in `CompileFailed`, multi-unit `cargo test`
streams, non-JSON lines, the read-before-write mtime property, id round-trip
and absence) plus 4 added to the existing `prepare_crate` arms at the three
sites (the change flag on first write, the unchanged repeat, the substituted
placeholder, the printed key).

**Suites**

```text
--lib (default)                                            1541 passed; 0 failed
--lib --features clippy_verify,rust_perf_verify,rust_perf_bench
                                                           1604 passed; 0 failed; 11 ignored
    … -- --ignored (the real-cargo oracles, end to end)      11 passed; 0 failed
clippy --all-targets (same features)                        0 findings
rustfmt --check (touched files)                             clean
```

## ⚠ The installed binary does not carry this

The Issue 110 owed item was discharged the same morning — the M3 global
`cargo-heal` was rebuilt from `3cfa4faf` with the release set, and AGENTS.md's
contradictory global-binary feature list was corrected at `78c3c42f`. That
rebuild carries 110's keyed-name FIX; it predates this commit, so it does not
carry this witness. Nothing is broken by that — the fix is the thing that
prevents the collision and the witness only makes a recurrence loud — but the
next rebuild is what puts the tripwire in the fleet binary.

## Cross-repo

riir-train carries the same port (Issue 110's fix went both ways —
riir-train `61d85d62` / here `9443ef1a`), and Half B's **join** is landed
there too: riir-train `3f8ce65a` (verify with
`git -C ../riir-train cat-file -e 3f8ce65a`). Its oracle already re-probed
from cold on `fresh && messages == 0`, and neither rule subsumes the other —
the message-count rule is blind to a replay whose donor crate *linted*, the
first-write rule does not cover a poisoned scratch dir from an interrupted
process — so it re-probes on `fresh && (wrote_new || messages == 0)` rather
than importing this module. Measured there: 1635 lib / 13 miss_shape,
perturbation of the read-before-write reds its arm.

⛔ **Half A does not carry across, and that is the part to remember.** Its
oracle has no `control_fires` scaffold, so whether any row of
`data/clippy_l4_rlvr.jsonl` was generated under a shared target dir is
UNDETERMINED there and the repair is **regeneration, not re-reading**. The
adjudication above was possible only because somebody, for an unrelated
reason, had built a positive control into the harness.
See riir-train Issue 555.

Landed here: see the commit that closes this issue.
