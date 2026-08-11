# Freeze/thaw pattern — BLAKE3-committed corpus

The `ClippyCorpus` uses a freeze/thaw pattern with BLAKE3 commitment for
tamper-evidence.

## The pattern

```rust
// Build (computes commitment)
let corpus = ClippyCorpus::new(entries);

// Freeze (serialize to bytes)
let frozen: Vec<u8> = corpus.freeze()?;

// Thaw (deserialize from bytes — commitment is advisory)
let thawed: ClippyCorpus = ClippyCorpus::thaw(&frozen)?;

// Verify (check commitment matches)
let ok: bool = corpus.verify();
```

## BLAKE3 commitment

The commitment is a 32-byte BLAKE3 hash of all entries in sorted order
(by `lint_name`), with delimiter-separated fields:

```text
lint_name \0 lint_group \0 buggy_code \0 fixed_code \1
```

BLAKE3 is chosen over SHA-256 per the global AGENTS.md rule: "Use blake3
as possible instead of SHA1, SHA256".

## Tamper-evident, not tamper-proof

The commitment is **advisory**: `thaw()` returns the corpus even if the
commitment doesn't match. The caller decides how to handle detected
tampering. The pruner's syntax check (`is_patch_valid`) is the
load-bearing validation, not the commitment.

This is deliberate: a corrupted corpus entry that still parses as valid
Rust might still produce a useful fix. The commitment tells you the data
changed; the pruner tells you whether the change matters.

## Why not Merkle-tree commitment?

The corpus is small (~10 entries, ~2KB serialized). A flat BLAKE3 hash
over the whole serialized blob is sufficient. Merkle trees are for
large datasets where you need partial verification (proving one entry
without revealing the rest). The corpus doesn't need that.

## Pattern source

Mirrors the `SealQuestCorpus` pattern from
`riir-games-quest/quest_grammar` + the `NeuronShard` freeze/thaw pattern
from `riir-neuron-db`. Both use BLAKE3 for tamper-evidence.

## See also

- [`../02_corpus/clippy_corpus.md`](../02_corpus/clippy_corpus.md) — the corpus type
