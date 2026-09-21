# The fix-compile attempt ring (fixseq)

One `run-<ts>.jsonl` per `cargo heal --fix-compile --write` run: the
ORDERED per-attempt sequence (seq, error code, span_hash, strategy,
outcome, errors before→after) as jsonl lines. The trajectory store
holds per-FAMILY counters — attempt ORDER lives only here.

Consumers: the Issue 91 T2 oscillation detector (same `(code,
span_hash)` returning after an intervening fix) and the Issue 093
per-run strategy posterior (the strategy sequence per run).
Append-only; never edited in place.
