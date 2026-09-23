# Issue 131 — the e2e self-evolve flow as a doc diagram + static SVG hero for the KAT network pages

**Status:** RESOLVED 2026-09-23 — T1–T4 landed (doc + SVG render + the KAT-pages hero + gates). Deploy note: the hero ships on the worker's next deploy — Plan 037's in-flight WIP is in the same build tree, so a deploy before it lands would ship half-done work (the hero is committed and pinned; T3.2 of Plan 037 carries it live).

## The ask

The self-evolve story is documented piecewise (`10_self_evolve/*` for the
local loop; the KAT network surface lives in riir-dapps' front page +
skill page) but no single artifact shows the WHOLE e2e flywheel:

```
local loop: scan → retrieve (corpus + memory) → draft → validate → fix
            → record trajectory → learn (EMA) → next run improves
fleet loop: mining batch (consent) → proof queue → epoch settle (KAT)
            → fleet corpus grows → leased back → retrieval improves
```

That flywheel IS the product pitch — it belongs on the network's front
page as a hero, and in `.docs` as the canonical diagram.

## Tasks

- [x] T1 — `.docs/10_self_evolve/self_evolve_flow.md`: the e2e flow doc
      with the mermaid source (compact hero variant + annotated full
      variant, honest terminology: modelless L0–L2, memory/learning
      oracle split, consent-gated mining, corpus leased back), README
      index line.
- [x] T2 — render the mermaid to a static SVG
      (`.docs/10_self_evolve/self_evolve_flow.svg`), dark-palette to
      match the KAT pages (#0d1117/#161b22/#3fb950), no JS, external
      `@import` stripped, aria-labeled. Verified visually in a page-
      CSS harness (qlmanage/WebKit thumbnail): every label fits.
- [x] T3 — riir-dapps kat-service: `src/self_evolve_flow.svg` (the
      pinned mirror, moved in the same commit) + `src/self_evolve_hero.rs`
      (`include_str!`, the heal_skill convention; tests pin the SVG
      shape, the CSS scoping, and the no-fabricated-numbers caption) +
      the `{HERO}` slot in `front_page_html` between the tagline and
      the FAQ + the front-page wiring pin in `front_page.rs` tests.
- [x] T4 — gates: kat-service lib 277/277 (273 + 4 new), clippy clean
      native + wasm32, direction gate OK; both repos committed on
      develop.

## Constraints

- The SVG is static bytes served from the binary — content changes only
  when the worker deploys (the `heal_skill.md` freshness rule).
- The canonical SVG home is THIS repo (`.docs/10_self_evolve/`); the
  dapps copy is a pinned mirror, never hand-edited there.
- Diagram claims must match shipped behavior: memory half runs every
  build, learning half is oracle-gated, mining is opt-in consent, the
  corpus lease refreshes clients (DomainOverlay).
