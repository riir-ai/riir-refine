# ClippyCorpus — BLAKE3-committed lint fix corpus

The foundation (L0) of the clippy-healing pipeline. Each clippy lint is
distilled into a named rule with a bounded fix-space, stored as a
`(lint_name, buggy_code, fixed_code, lint_group)` tuple.

## ClippyFixEntry

```rust
pub struct ClippyFixEntry {
    pub lint_name: String,    // e.g. "needless_return"
    pub lint_group: String,   // e.g. "style", "complexity", "perf"
    pub buggy_code: String,   // the code that triggers the lint
    pub fixed_code: String,   // the code that resolves the lint
}
```

### Domain classification (sync boundary rule)

- `lint_name` + `lint_group` = physical (categorical, deterministic)
- `buggy_code` + `fixed_code` = physical (raw bytes, bit-identical for
  replay)

These are raw, deterministic, replayable values — NOT latent embeddings.

## ClippyCorpus

```rust
pub struct ClippyCorpus {
    pub entries: Vec<ClippyFixEntry>,
    pub commitment: [u8; 32],  // BLAKE3 hash of serialized entries
}
```

### BLAKE3 commitment

The commitment is computed by hashing all entries in sorted order
(by `lint_name`) with delimiter-separated fields:

```text
lint_name \0 lint_group \0 buggy_code \0 fixed_code \1
```

This is **tamper-evident**, not tamper-proof: `thaw()` returns the corpus
even if the commitment doesn't match, so the caller can decide how to
handle detected tampering. The pruner's syntax check (`is_patch_valid`)
is the load-bearing validation, not the commitment.

### Freeze / thaw

```rust
// Serialize to JSON bytes
let frozen: Vec<u8> = corpus.freeze()?;

// Deserialize from JSON bytes (commitment is advisory)
let thawed: ClippyCorpus = ClippyCorpus::thaw(&frozen)?;
```

The corpus is serialized via `serde_json`. The commitment is computed on
`new()`, verified on `verify()` / `thaw()`.

### Lookup

```rust
// Iterate fix entries for a specific lint
for entry in corpus.lookup("needless_return") {
    // ...
}

// All unique lint names (sorted, deduped)
let names: Vec<&str> = corpus.lint_names();
```

## Pattern source

Mirrors the `SealQuestCorpus` pattern from
`riir-games-quest/quest_grammar` but specialized for clippy lint fixes.
Reimplemented locally (~120 LOC) because riir-refine cannot depend on
riir-ai (boundary rule).

## See also

- [`seed_corpus.md`](seed_corpus.md) — the 10 built-in examples
- [`../03_drafter/clippy_ternary_drafter.md`](../03_drafter/clippy_ternary_drafter.md) — L1 consumes corpus entries
