#!/bin/sh
# vendor_ragplane.sh — the rag modelless substrate vendoring (issue 221 P2b).
#
# Vendors the MODELLESS retrieval slices the structural matcher consumes into
# src/ragplane/ (the katplane pattern: provenance header, sibling --check):
#
#   from riir-neuron-db/src/bm25.rs            -> ragplane/bm25.rs
#     (the production Bm25Index the current floors were pinned under —
#      std + papaya only; papaya is already a refine dep)
#   from riir-neuron-db/crates/riir-rag/src/:
#     embedder.rs  -> ragplane/embedder.rs    (Embedder + ModellessEmbedder +
#                                              embed_modelless + the DenseEmbedder
#                                              trait the bonsai arm binds; the
#                                              embed_contextual half keeps its
#                                              upstream cfg)
#     query.rs     -> ragplane/query.rs       (LATENT_DIM, LatentQuery,
#                                              RetrievedChunk, ChunkSource)
#     chunker.rs   -> ragplane/chunker.rs     (Chunker trait + RawChunk +
#                                              WindowChunker)
#     ast_chunker.rs -> ragplane/ast_chunker.rs (the Rust AST chunker; needs
#                                              `quote`, already a dep)
#
# NOT vendored: retriever.rs (the NeuronDbRetriever — replaced by the fresh
# in-repo seam), packer/rrf/slate/graph_fusion/evidence_eval/doc_pool/
# heading_chunker/math/score_norm (not consumed by the structural path).
#
# Internal paths are rewritten: crate::query -> crate::ragplane::query,
# crate::chunker -> crate::ragplane::chunker (sibling module refs). All four
# files are FULL vendoring (no partial slices) — --check diffs whole files
# modulo the provenance header.
#
# The private-dep extras stay sibling-side: the bonsai dense arm keeps
# importing riir_rag_bonsai + riir_neuron_db::dense_embed behind
# #[cfg(feature = "bonsai_retrieval")]; traj_store keeps riir_neuron_db
# behind latent_retrieval.
#
# Usage: scripts/vendor_ragplane.sh <ndb-sha> | --check <ndb-sha>
# Exit: 0 green · 1 drift · 2 usage.

set -eu

DEST=src/ragplane
NDB=../riir-neuron-db
RAG=$NDB/crates/riir-rag/src

usage() {
    echo "usage: $0 <ndb-sha> | $0 --check <ndb-sha>" >&2
    exit 2
}

[ $# -ge 1 ] || usage
MODE="write"
case "$1" in
    --check) [ $# -eq 2 ] || usage; SHA="$2"; MODE="check" ;;
    -*) usage ;;
    *) [ $# -eq 1 ] || usage; SHA="$1" ;;
esac

[ -d "$NDB" ] || { echo "✗ sibling missing: $NDB (read-only source)" >&2; exit 1; }
for pair in "$NDB/src/bm25.rs:bm25.rs" "$RAG/embedder.rs:embedder.rs" \
            "$RAG/query.rs:query.rs" "$RAG/chunker.rs:chunker.rs" \
            "$RAG/ast_chunker.rs:ast_chunker.rs"; do
    src=${pair%%:*}
    [ -f "$src" ] || { echo "✗ sibling module missing: $src" >&2; exit 1; }
done

header() {
    printf '%s\n' \
"//! Vendored from \`riir-neuron-db\` (\`$SHA\`, vendored 2026-11-09, issue" \
"//! 221 P2b — the publish posture's modelless retrieval substrate)." \
"//! std-only up to the crate's own deps (blake3 / papaya / quote — all" \
"//! already in refine's tree). The canonical copy stays in the private" \
"//! sibling; drift is caught by \`scripts/vendor_ragplane.sh --check\`" \
"//! and the pins in \`tests/ragplane_parity.rs\` — never hand-edit one" \
"//! side. Upstream-internal paths rewritten: crate::query ->" \
"//! crate::ragplane::query, crate::chunker -> crate::ragplane::chunker." \
"//!" \
"//! LINT POSTURE (vendored-code disclosure): upstream never carried" \
"//! refine's clippy -D bar (lib OR --all-targets, where the vendored test" \
"//! modules also get graded), so the divergent lint classes are allowed at" \
"//! file scope rather than hand-patched — the cast-precision findings are" \
"//! INHERENT to the upstream algorithms (fixing them would change the" \
"//! pinned vectors/floors), the doc-backtick and style classes are churn" \
"//! with zero behavior content. The parity pins + the upstream tests that" \
"//! ship inside each file are the correctness authority, not lint shape." \
"#![allow(clippy::doc_markdown, clippy::cast_precision_loss," \
"        clippy::cast_possible_truncation, clippy::cast_sign_loss," \
"        clippy::cast_possible_wrap, clippy::cast_lossless," \
"        clippy::missing_const_for_fn, clippy::missing_panics_doc," \
"        clippy::missing_errors_doc, clippy::missing_fields_in_debug," \
"        clippy::module_name_repetitions, clippy::must_use_candidate," \
"        clippy::inline_always, clippy::let_and_return," \
"        clippy::manual_let_else, clippy::explicit_iter_loop," \
"        clippy::match_same_arms, clippy::single_char_pattern," \
"        clippy::format_push_string, clippy::ignore_without_reason," \
"        clippy::assertions_on_constants, clippy::too_many_lines," \
"        clippy::assert_is_empty)]"
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# (source, dest) pairs; the header notes which upstream file each came from.
emit() {
    src=$1
    { header; echo; sed \
        -e 's/crate::query/crate::ragplane::query/g' \
        -e 's/crate::chunker/crate::ragplane::chunker/g' \
        -e 's/crate::ast_chunker/crate::ragplane::ast_chunker/g' \
        -e 's/crate::math/crate::ragplane::math/g' \
        -e 's/\[crate::Embedder\]/[crate::ragplane::embedder::Embedder]/g' \
        -e 's/`(crate::Embedder)`/`Embedder`/g' \
        -e 's/\[crate::WindowChunker\]/[WindowChunker]/g' \
        -e 's/`(crate::WindowChunker)`/`WindowChunker`/g' \
        -e 's/\[crate::heading_chunker::HeadingChunker\]/[HeadingChunker]/g' \
        -e 's/`crate::heading_chunker::HeadingChunker`/`HeadingChunker`/g' \
        -e 's/\[crate::ast_chunker::AstChunker\]/[AstChunker]/g' \
        -e 's/budget: crate::TokenBudget/budget: crate::ragplane::query::TokenBudget/g' \
        -e 's/`crate::TokenBudgetPacker`/`TokenBudgetPacker`/g' \
        "$src"; } > "$tmp/$2"
}

# query.rs gains the 1-line TokenBudget tuple (upstream it lives in
# packer.rs — vendoring just the type the vendored RetrievalProvider trait
# names, not the packer).
TOKEN_BUDGET_BLOCK='
/// The token budget a retrieval may spend (vendored from upstream packer.rs
/// with the type — the packer itself stays sibling-side; the vendored
/// `RetrievalProvider` trait names it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudget(pub usize);
'
emit_query() {
    emit "$RAG/query.rs" query.rs.tmp
    cat "$tmp/query.rs.tmp" > "$tmp/query.rs"
    printf '%s\n' "$TOKEN_BUDGET_BLOCK" >> "$tmp/query.rs"
}

emit "$NDB/src/bm25.rs" bm25.rs
emit "$RAG/embedder.rs" embedder.rs
emit_query
emit "$RAG/chunker.rs" chunker.rs
emit "$RAG/ast_chunker.rs" ast_chunker.rs

if [ "$MODE" = "check" ]; then
    rc=0
    for f in bm25.rs embedder.rs query.rs chunker.rs ast_chunker.rs; do
        if ! diff -u "$DEST/$f" "$tmp/$f" >/dev/null 2>&1; then
            echo "Δ $f differs from sibling @$SHA (module the vendored header rewrote?)"
            rc=1
        fi
    done
    if [ "$rc" -eq 0 ]; then
        echo "✓ ragplane matches riir-neuron-db @$SHA (modulo provenance headers + path rewrites)"
    fi
    exit "$rc"
fi

mkdir -p "$DEST"
for f in bm25.rs embedder.rs query.rs chunker.rs ast_chunker.rs; do
    cp "$tmp/$f" "$DEST/$f"
    echo "vendored $f @$SHA"
done
echo "→ run: rustfmt --config skip_children=true src/ragplane/*.rs, then the parity pins + clippy -D"
