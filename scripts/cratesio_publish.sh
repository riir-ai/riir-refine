#!/bin/sh
# cratesio_publish.sh — the publish-time manifest surgery + standalone gate (issue 221 P2).
#
# Generates a publishable manifest from a git ref (tag or HEAD), then proves it
# STANDALONE: zero sibling checkouts, zero path deps, the publish feature set
# (default + fix_verify + update_check) built from the REGISTRY only.
#
# The dev workspace is never touched: everything happens in a stage dir
# (/tmp/refine-cratesio-<name>-<version>). The surgery is deterministic text
# rewriting — same ref + same args -> byte-identical manifest.
#
# The surgery, item by item (issue 221 Findings B + the P2a world):
#   [package]     canonical rows: name -> --name, version -> --version,
#                 publish = true; license = "MIT" rides from the dev manifest
#                 (P0(c)); + the include list (src/**, build.rs,
#                 build_commit.rs, the kernel_opt wgsl fixture the default
#                 build embeds, the pins manifest the health views embed,
#                 README, LICENSE) + autodiscovery off (autobins/autotests/
#                 autoexamples/autobenches = false).
#   [lib]         kept as-is (name stays riir_refine).
#   [workspace]   dropped (the stage package stands alone).
#   [dependencies] katgpt-core -> registry row at --kc-version (the P1
#                 refresh; 0.4.1 CANNOT satisfy it — no rating/step_noise
#                 modules there); every private optional row dropped
#                 (riir-kat, riir-ledger, riir-rag, riir-rag-bonsai,
#                 riir-neuron-db, riir-llm, riir-infer-core/-gpu,
#                 reflexer-vessel, katgpt-percepta, katgpt-micro-belief).
#   [dev-dependencies] dropped (path rows would fail the standalone resolve).
#   [features]    closure to a fixpoint: every row naming a dropped dep or a
#                 dropped feature is removed (33 rows); fix_verify is
#                 REWRITTEN to ["compile_oracle", "scoped_recheck"] (it
#                 implies latent_retrieval in the dev tree — the retrieval
#                 leg is binaries-only); default GAINS fix_verify +
#                 update_check so `cargo install cargo-refine` yields the
#                 full registry product.
#   targets       all [[test]]/[[bench]]/[[example]] rows dropped (their
#                 dirs stay out of the include list; their required-features
#                 name removed features); [[bin]] keeps ONLY the --name bin
#                 (cargo-refine) as a canonical block with required-features
#                 pruned of removed features; corpus_diagnose +
#                 wasm-validator-build are binaries-only bins.
#   [patch.*]     both dropped (sibling vendor forks — dead weight in a
#                 published manifest).
#   [profile.*]   dropped (workspace tables, not crate metadata).
#
# Gates (registry mode): cargo metadata with ZERO path deps; release build of
# lib + the cargo-refine bin at default+fix_verify+update_check; cargo
# package --list audited for internal surfaces; cargo publish --dry-run.
# Sibling mode (--kc-source sibling) keeps the katgpt-core path dep so the
# surgery itself can be gated BEFORE the P1 refresh lands — the path-dep
# assertion and the dry-run are skipped there, loudly.
#
# PUBLISHING is refused by default (P3 is owner-gated): pass
# CARGO_REFINE_PUBLISH_CONFIRM=yes and a real token env to actually upload.
#
# Usage:
#   scripts/cratesio_publish.sh                       # full gate, registry mode
#   scripts/cratesio_publish.sh --kc-source sibling   # pre-P1 surgery gate
#   scripts/cratesio_publish.sh --ref v0.4.8 --name cargo-refine \
#       --version 0.4.8 --kc-version 0.4.2
#
# Exit codes: 0 green · 1 any gate red · 2 usage/refusal.

set -eu

NAME=cargo-refine
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
KC_VERSION=0.4.2
KC_SOURCE=registry
REF=HEAD
KEEP_STAGE=0

while [ $# -gt 0 ]; do
    case "$1" in
        --name) NAME=$2; shift 2 ;;
        --version) VERSION=$2; shift 2 ;;
        --kc-version) KC_VERSION=$2; shift 2 ;;
        --kc-source) KC_SOURCE=$2; shift 2 ;;
        --ref) REF=$2; shift 2 ;;
        --keep-stage) KEEP_STAGE=1; shift ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

[ "$KC_SOURCE" = registry ] || [ "$KC_SOURCE" = sibling ] || {
    echo "--kc-source must be registry|sibling" >&2; exit 2; }

STAGE="/tmp/refine-cratesio-$NAME-$VERSION"
REPO=$(git rev-parse --show-toplevel)
KC_PATH=$(cd "$REPO" && realpath "../katgpt-rs/crates/katgpt-core" 2>/dev/null || echo "")
if [ "$KC_SOURCE" = sibling ] && [ ! -d "$KC_PATH" ]; then
    echo "FAIL: --kc-source sibling needs the katgpt-rs checkout beside the repo" >&2; exit 2
fi

echo "== cratesio_publish: name=$NAME version=$VERSION kc=$KC_VERSION@$KC_SOURCE ref=$REF"

# ── 1. stage the ref tree ────────────────────────────────────────────────────
rm -rf "$STAGE"
mkdir -p "$STAGE"
git -C "$REPO" archive "$REF" | tar -x -C "$STAGE"
# Nested workspaces / non-packaged dirs the stage must not carry (their
# manifests reference the [workspace] table the surgery drops, or they are
# excluded lanes with their own cargo projects).
rm -rf "$STAGE/crates" "$STAGE/vendor" "$STAGE/validator_src" \
       "$STAGE/.refine" "$STAGE/.proofs" "$STAGE/target"
echo "   staged $STAGE"

# ── 2. the manifest surgery ─────────────────────────────────────────────────
python3 - "$STAGE/Cargo.toml" "$NAME" "$VERSION" "$KC_VERSION" "$KC_SOURCE" "$KC_PATH" <<'PYEOF'
import re, sys

path, name, version, kc_version, kc_source, kc_path = (
    sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6])
lines = open(path).read().splitlines(keepends=True)

# The dropped PRIVATE dep crate names (issue 221 Findings B table, post-P2a).
DROPPED_DEPS = {
    "riir-kat", "riir-ledger", "riir-rag", "riir-rag-bonsai", "riir-neuron-db",
    "riir-llm", "riir-infer-core", "riir-infer-gpu", "reflexer-vessel",
    "katgpt-percepta", "katgpt-micro-belief",
}
# The feature row that is REWRITTEN, not dropped: its purpose (the compile
# gate on fixes) is retrieval-free; only the latent_retrieval leg dies.
REWRITE = {"fix_verify": ["compile_oracle", "scoped_recheck"]}

out = []            # emitted lines (drop-mode lines never land here)
mode = "outside"    # outside|package|passthrough|deps|features|drop
feat_items = []     # ordered features-section items: ("c", line) | ("f", fname)
feat_rows = {}      # feature -> entries
feat_pos = None     # where the rendered features body inserts into out
license_emitted = False
dep_comments = []   # comment lines buffered ahead of the next dep row
dep_depth = 0       # brace depth inside the current dep row
dep_priv = False    # current dep row belongs to a dropped crate
dep_kc = False      # current dep row is katgpt-core
dep_first = False   # on the row's first line
bins = []           # buffered [[bin]] blocks

i = 0
while i < len(lines):
    line = lines[i]
    s = line.strip()

    # ── section headers (never while inside a multi-line dep row) ──
    if s.startswith("[") and not (mode == "deps" and dep_depth > 0) \
            and not s.startswith("[["):
        hdr_m = re.match(r"\[([^\]]+)\]", s)
        hdr = hdr_m.group(1) if hdr_m else ""
        if hdr == "package":
            mode = "package"
            out.append(line)
        elif hdr in ("lib", "lints"):
            mode = "passthrough"
            out.append(line)
        elif hdr == "dependencies" or (
                hdr.startswith("target.") and hdr.endswith(".dependencies")):
            mode = "deps"
            dep_comments, dep_depth, dep_priv, dep_kc, dep_first = [], 0, False, False, False
            out.append(line)
        elif hdr == "features":
            mode = "features"
            feat_items = []
            feat_pos = len(out) + 1  # after the header line appended below
            out.append(line)
        elif hdr == "dev-dependencies" or hdr == "workspace" \
                or hdr.startswith("patch.") or hdr.startswith("profile."):
            mode = "drop"            # header + body both swallowed
        else:
            mode = "passthrough"     # unknown single tables ride through
            out.append(line)
        i += 1
        continue

    if s.startswith("[["):
        m = re.match(r"\[\[(bin|test|bench|example)\]\]", s)
        kind = m.group(1) if m else "?"
        if kind == "bin":
            blk = [line]
            i += 1
            while i < len(lines):
                s2 = lines[i].strip()
                if s2.startswith("["):
                    break
                blk.append(lines[i])
                i += 1
            bins.append(blk)
            continue
        mode = "drop"
        i += 1
        continue

    # ── body lines ──
    if mode == "drop":
        i += 1
        continue

    if mode == "package":
        st = s
        if st.startswith("#"):
            pass  # canonical generated package section: prose comments stay out
        elif st.startswith("license-file"):
            # the EULA is binaries-only terms — replaced by the crate licence
            if not license_emitted:
                out.append('license = "MIT"\n')
                license_emitted = True
        elif st.startswith("license"):
            # normalize regardless of the ref's posture (self-contained from
            # any tag: P0(c) landed in the dev tree, older tags still carry
            # the EULA license-file row)
            if not license_emitted:
                out.append('license = "MIT"\n')
                license_emitted = True
        elif st.startswith("name ="):
            out.append(f'name = "{name}"\n')
        elif st.startswith("version ="):
            out.append(f'version = "{version}"\n')
        elif st.startswith("publish"):
            out.append("# Publish posture (issue 221 P2): the crates.io crate.\n")
            out.append("publish = true\n")
        else:
            out.append(line)
        i += 1
        continue

    if mode == "passthrough":
        out.append(line)
        i += 1
        continue

    if mode == "deps":
        if not s:
            if dep_depth == 0:
                out.append(line)
            i += 1
            continue
        if s.startswith("#"):
            if dep_depth == 0:
                dep_comments.append(line)
            i += 1
            continue
        if dep_depth == 0:
            dep_name = s.split("=")[0].strip()
            dep_priv = dep_name in DROPPED_DEPS
            dep_kc = dep_name == "katgpt-core"
            dep_first = True
        if dep_priv:
            dep_depth += line.count("{") - line.count("}")
            if dep_depth <= 0:
                dep_depth = 0
                dep_comments = []   # comments above a dropped row die with it
            i += 1
            continue
        if dep_first and dep_comments:
            out.extend(dep_comments)
            dep_comments = []
        if dep_kc and kc_source == "sibling" and dep_first:
            fm = re.search(r"features\s*=\s*\[([^\]]*)\]", line)
            feats = fm.group(1) if fm else ""
            out.append(f'katgpt-core = {{ path = "{kc_path}", '
                       f'default-features = false, features = [{feats}] }}\n')
        elif dep_kc and kc_source == "registry" and dep_first:
            fm = re.search(r"features\s*=\s*\[([^\]]*)\]", line)
            feats = fm.group(1) if fm else ""
            out.append(f'katgpt-core = {{ version = "{kc_version}", '
                       f'default-features = false, features = [{feats}] }}\n')
        else:
            out.append(line)
        dep_first = False
        dep_depth += line.count("{") - line.count("}")
        if dep_depth <= 0:
            dep_depth = 0
        i += 1
        continue

    if mode == "features":
        if not s:
            feat_items.append(("c", line))
            i += 1
            continue
        s_nc = s.split("#")[0].strip()
        m = re.match(r"^([A-Za-z0-9_\-]+)\s*=\s*\[(.*)\]\s*$", s_nc)
        if m:
            fname = m.group(1)
            entries = [e.strip().strip('"').strip("'")
                       for e in m.group(2).split(",") if e.strip()]
            feat_rows[fname] = entries
            feat_items.append(("f", fname))
        else:
            feat_items.append(("c", line))
        i += 1
        continue

    i += 1

# ── feature closure to a fixpoint ────────────────────────────────────────────
removed_features = set()
changed = True
while changed:
    changed = False
    for fname, entries in feat_rows.items():
        if fname in removed_features or fname in REWRITE:
            continue
        for e in entries:
            if e.startswith("dep:"):
                dead = e[4:].split("/", 1)[0] in DROPPED_DEPS
            elif "/" in e:
                dead = e.split("/", 1)[0] in DROPPED_DEPS
            else:
                dead = e in removed_features
            if dead:
                removed_features.add(fname)
                changed = True
                break

def render(fname, entries):
    return fname + " = [" + ", ".join('"%s"' % e for e in entries) + "]\n"

# ── re-render the features body in its original order ───────────────────────
rendered = []
for kind, payload in feat_items:
    if kind == "c":
        rendered.append(payload)
    elif payload in removed_features:
        continue
    elif payload in REWRITE:
        rendered.append(render(payload, REWRITE[payload]))
    else:
        rendered.append(render(payload, feat_rows[payload]))
out[feat_pos:feat_pos] = rendered

# ── prune optional dep rows whose every dep:X feature reference was removed ──
kept_feats = {f: (REWRITE[f] if f in REWRITE else feat_rows[f])
              for f in feat_rows if f not in removed_features}
used_deps = set()
for es in kept_feats.values():
    for e in es:
        if e.startswith("dep:"):
            used_deps.add(e[4:].split("/", 1)[0])
dep_start = next(i for i, l in enumerate(out) if l.strip() == "[dependencies]")
dep_end = len(out)
for i2 in range(dep_start + 1, len(out)):
    if out[i2].strip().startswith("["):
        dep_end = i2
        break
sec = out[dep_start:dep_end]
new_sec = []
depth = 0
cur_drop = False
pruned_deps = []
for l in sec:
    st = l.strip()
    if st == "[dependencies]":
        new_sec.append(l)
        continue
    if depth == 0 and (not st or st.startswith("#")):
        new_sec.append(l)
        continue
    if depth == 0:
        m = re.match(r"^([A-Za-z0-9_\-]+)\s*=", st)
        rname = m.group(1) if m else None
        is_opt = "optional = true" in st
        cur_drop = bool(rname and is_opt and rname not in used_deps
                        and rname != "katgpt-core")
        if cur_drop:
            pruned_deps.append(rname)
            while new_sec and new_sec[-1].strip().startswith("#"):
                new_sec.pop()   # its comment block dies with it
        depth += st.count("{") - st.count("}")
        if depth <= 0:
            depth = 0
            if not cur_drop:
                new_sec.append(l)
            cur_drop = False
        elif not cur_drop:
            new_sec.append(l)
        continue
    # continuation of a multi-line row
    depth += st.count("{") - st.count("}")
    if depth <= 0:
        depth = 0
        if not cur_drop:
            new_sec.append(l)
        cur_drop = False
    elif not cur_drop:
        new_sec.append(l)
out[dep_start:dep_end] = new_sec
if pruned_deps:
    print(f"   pruned unreferenced optional dep rows: {', '.join(sorted(pruned_deps))}")

# ── default GAINS fix_verify + update_check (the registry product = the
#    release product minus the hosted planes; issue 221 lane design) ─────────
for idx, line in enumerate(out):
    mm = re.match(r"^default = \[(.*)\]\s*$", line.strip())
    if mm:
        have = [e.strip().strip('"') for e in mm.group(1).split(",") if e.strip()]
        for add in ("fix_verify", "update_check"):
            if add not in have:
                have.append(add)
        out[idx] = "default = [" + ", ".join('"%s"' % h for h in have) + "]\n"
        break

# ── bins: canonical block for the kept bin only ──────────────────────────────
kept_bin = None
for blk in bins:
    nm = re.search(r'name\s*=\s*"([^"]+)"', "".join(blk))
    if nm and nm.group(1) == name:
        rf = re.search(r"required-features\s*=\s*\[(.*?)\]", "".join(blk))
        req = [e.strip().strip('"') for e in rf.group(1).split(",")] if rf else []
        kept_bin = [r for r in req if r not in removed_features]
if kept_bin is not None:
    out.append("# The publish bin — the unified CLI. required-features pruned at\n")
    out.append("# surgery (removed features are binaries-only; issue 221 P2).\n")
    out.append("[[bin]]\n")
    out.append(f'name = "{name}"\n')
    out.append('path = "src/bin/heal.rs"\n')
    if kept_bin:
        out.append("required-features = [" + ", ".join('"%s"' % r for r in kept_bin) + "]\n")

# ── the [package] additions: include list + autodiscovery off ───────────────
include_block = (
    "\n"
    "# Issue 221 P2: tight package surface — internal docs, corpora-side\n"
    "# fixtures and the binaries-only lanes stay out of the .crate.\n"
    'include = [\n'
    '    "src/**",\n'
    '    "build.rs",\n'
    '    "build_commit.rs",\n'
    '    "tests/fixtures/m2p_attention_postfix_701.wgsl",\n'
    '    ".benchmarks/pins_manifest.json",\n'
    '    "README.md",\n'
    '    "LICENSE",\n'
    ']\n'
    'autobins = false\n'
    'autotests = false\n'
    'autoexamples = false\n'
    'autobenches = false\n'
)
text = "".join(out)
anchor = re.search(r'(^description = .*$)', text, re.M)
assert anchor, "description row not found"
text = text[: anchor.end()] + "\n" + include_block + text[anchor.end():]

open(path, "w").write(text)
print(f"   surgery: package={name} v{version}; dropped features ({len(removed_features)}):")
for f in sorted(removed_features):
    print(f"     - {f}")
print(f"   dropped deps: {', '.join(sorted(DROPPED_DEPS))}")
PYEOF

# sanity: the generated manifest carries exactly the surgery's decisions
grep -q '^name = "'$NAME'"' "$STAGE/Cargo.toml" || { echo "FAIL: name row missing" >&2; exit 1; }
[ -f "$STAGE/src/bin/heal.rs" ] || { echo "FAIL: bin source missing" >&2; exit 1; }
grep -q '^\[\[bin\]\]' "$STAGE/Cargo.toml" || { echo "FAIL: bin block missing" >&2; exit 1; }
grep -q 'required-features = \["rust_perf", "clippy_lints"\]' "$STAGE/Cargo.toml" \
  || { echo "FAIL: bin required-features rewrite missing" >&2; exit 1; }
grep -q '^fix_verify = \["compile_oracle", "scoped_recheck"\]' "$STAGE/Cargo.toml" \
  || { echo "FAIL: fix_verify rewrite missing" >&2; exit 1; }
grep -qE '^default = .*"fix_verify".*"update_check"' "$STAGE/Cargo.toml" \
  || { echo "FAIL: default gain missing" >&2; exit 1; }
if grep -qE '^\[workspace\]|^\[patch\.|^path = "\.\./|^\[dev-dependencies\]|^\[profile' "$STAGE/Cargo.toml"; then
  echo "FAIL: workspace/patch/dev/profile/sibling-path residue in the generated manifest" >&2; exit 1
fi
grep -qE '^license = "MIT"' "$STAGE/Cargo.toml" || { echo "FAIL: license row missing" >&2; exit 1; }
if grep -q 'license-file' "$STAGE/Cargo.toml"; then
  echo "FAIL: license-file survived the surgery" >&2; exit 1
fi
grep -q '^publish = true' "$STAGE/Cargo.toml" || { echo "FAIL: publish flag missing" >&2; exit 1; }
if grep -qE '^\[\[test\]\]|^\[\[bench\]\]|^\[\[example\]\]' "$STAGE/Cargo.toml"; then
  echo "FAIL: test/bench/example target rows survived" >&2; exit 1
fi
for gone in latent_retrieval points kat_service kat_transport score_bench ternary_inference \
            vessel_artifacts wasm_validator bonsai_retrieval hosted_expert nonergodic_poc \
            net-mainnet points_drain frontier_miner pav_data; do
  if grep -qE "^$gone = " "$STAGE/Cargo.toml"; then
    echo "FAIL: removed feature $gone still declared" >&2; exit 1
  fi
done
for kept in elo_k_eff gw_corpus coverage_gap_event seat_rerank l4_lora kernel_opt \
            fix_verify compile_oracle scoped_recheck update_check default; do
  if ! grep -qE "^$kept = " "$STAGE/Cargo.toml"; then
    echo "FAIL: kept feature $kept missing" >&2; exit 1
  fi
done

cd "$STAGE"
export CARGO_TARGET_DIR="$STAGE/target"

# The surgery changes the dep set, so the staged lock is stale by
# construction — drop it here and let metadata/build regenerate a lock that
# matches the GENERATED manifest (verdict round-2 fix 3: publish rides
# --locked, which needs an existing, matching lock; this is where it gets
# both).
rm -f "$STAGE/Cargo.lock"

# ── 3. metadata resolve + the ZERO path-dep assertion (registry mode) ───────
cargo metadata --format-version 1 > "$STAGE/metadata.json" 2>/dev/null \
  || { echo "FAIL: cargo metadata could not resolve the generated manifest" >&2; exit 1; }
PATH_DEPS=$(python3 -c '
import json
md = json.load(open("'"$STAGE"'/metadata.json"))
paths = sorted({d["name"] for p in md["packages"] for d in p["dependencies"] if d.get("path")})
print(",".join(paths))')
if [ "$KC_SOURCE" = registry ]; then
    [ -z "$PATH_DEPS" ] || { echo "FAIL: path deps survived the surgery: $PATH_DEPS" >&2; exit 1; }
    echo "   metadata: zero path deps ✓"
else
    echo "   metadata: SIBLING MODE — path deps present by design: ${PATH_DEPS:-none}"
fi

# ── 4. the standalone build gate (default+fix_verify+update_check) ──────────
echo "   building (release, default+fix_verify+update_check)…"
if ! cargo build --release --quiet > "$STAGE/build.log" 2>&1; then
    echo "FAIL: build red; tail:" >&2
    tail -30 "$STAGE/build.log" >&2
    exit 1
fi
[ -x "$CARGO_TARGET_DIR/release/$NAME" ] || { echo "FAIL: bin not built" >&2; exit 1; }
echo "   build ✓"
# Verdict round-2 fix 1: a binary that builds but won't run fails the gate —
# never a silent pass (the --version leg exercises the build-stamp path).
if ! "$CARGO_TARGET_DIR/release/$NAME" --version > "$STAGE/bin_version.log" 2>&1; then
    echo "FAIL: the built bin refuses --version; tail:" >&2
    tail -5 "$STAGE/bin_version.log" >&2
    exit 1
fi
cat "$STAGE/bin_version.log"

# ── 5. package surface audit ────────────────────────────────────────────────
cargo package --list --allow-dirty > "$STAGE/pkglist.txt" 2>/dev/null \
  || { echo "FAIL: cargo package --list red" >&2; exit 1; }
if grep -qE '^(\.docs/|\.plans/|\.issues/|\.distill/|\.research/|\.agents/|release/|scripts/|assets/|corpus/|\.refine/|\.proposals/)' "$STAGE/pkglist.txt"; then
    echo "FAIL: internal surfaces in the package:" >&2
    grep -E '^(\.docs/|\.plans/|\.issues/|\.distill/|\.research/|\.agents/|release/|scripts/|assets/|corpus/|\.refine/|\.proposals/)' "$STAGE/pkglist.txt" | head -10 >&2
    exit 1
fi
if grep -qE '^\.benchmarks/' "$STAGE/pkglist.txt"; then
    if ! grep -qE '^\.benchmarks/pins_manifest\.json$' "$STAGE/pkglist.txt"; then
        echo "FAIL: unexpected .benchmarks entries:" >&2
        grep '^\.benchmarks/' "$STAGE/pkglist.txt" | head -5 >&2
        exit 1
    fi
fi
echo "   package list ✓ ($(grep -c . "$STAGE/pkglist.txt") entries)"
# Verdict round-2 fix 3b: a binary crate ships its lockfile so installs are
# reproducible — assert the REGENERATED lock survived the include list (cargo
# force-includes it for bin packages, but the assertion is the proof).
if ! grep -q '^Cargo\.lock$' "$STAGE/pkglist.txt"; then
    echo "FAIL: Cargo.lock not in the package (bin crates must ship it)" >&2
    exit 1
fi

# ── 6. cargo publish --dry-run (registry mode; needs the P1 refresh live) ───
if [ "$KC_SOURCE" = registry ]; then
    echo "   cargo publish --dry-run (--locked)…"
    if cargo publish --locked --dry-run > "$STAGE/publish_dry.log" 2>&1; then
        echo "   dry-run ✓"
    else
        echo "DRY-RUN RED (expected while P1's katgpt-core refresh is unpublished); tail:" >&2
        tail -12 "$STAGE/publish_dry.log" >&2
        exit 1
    fi
else
    echo "   dry-run SKIPPED (sibling mode)"
fi

# ── 7. the real publish — owner-gated (P3) ──────────────────────────────────
if [ "${CARGO_REFINE_PUBLISH_CONFIRM:-}" = "yes" ] && [ "$KC_SOURCE" = registry ]; then
    echo "   CARGO_REFINE_PUBLISH_CONFIRM=yes — publishing (--locked)"
    cargo publish --locked
    echo "   published $NAME@$VERSION"
else
    echo "   publish REFUSED (owner-gated P3): set CARGO_REFINE_PUBLISH_CONFIRM=yes to run"
fi

if [ "$KEEP_STAGE" = 0 ] && [ -n "${CARGO_TARGET_DIR:-}" ]; then
    rm -rf "$CARGO_TARGET_DIR"
fi
if [ "$KC_SOURCE" = registry ]; then
    echo "== VERDICT: green"
else
    echo "== VERDICT: green (sibling: dry-run UNSEEN)"
fi
