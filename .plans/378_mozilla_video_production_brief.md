# Plan 378 — Mozilla video: "The corpus is the model" (2-min fframes production brief)

Status: DRAFT — awaiting owner GO · Claude verdict r1 = **REVISE** — all revisions APPLIED (§0)
Branch: `develop`

- [ ] Owner: pick the VO lane (A: record own voice — recommended / B: Kokoro TTS / C: ElevenLabs) — §7
- [ ] Owner: capture the 7 screen assets — §9 (re-capture LIVE pins on recording day, §2)
- [ ] Agent: scaffold the fframes project + build S01–S13 + end card per §4/§8
- [ ] Agent: captions track (word-timed JSON) + loudness gate + export per §6/§7/§8
- [ ] Owner review (`preview` + strips) → final render → attach link to the application
- [ ] Re-verify the LIVE fact pins on recording day (§2, rows marked 🔴)

## 0. Verdict record

Claude delegated-owner verdict r1 (2026-11-14, session `0ab6f11f`): **REVISE** — every
revision below is applied in this revision of the brief:

- **S06 re-anchored**: "one-fourteenth the disk" compared against the WRONG model (that
  ratio is vs the 9B); vs the 27B teacher the sentence names it is 55.62/0.48 ≈ **116×** →
  VO now says "over a hundred times smaller"; "a thousand times faster" scoped **"at the
  median"** (p50 geomean; the escalated path is far below that — own-encoder multiple
  21×–36×); board URL on screen; evaluation regime scoped ("on the public classification
  suites"); suite-count reconciliation pinned (P6–P8).
- **S08**: "thirteen of our own repos" (was ambiguous external-sounding); private-game
  clause dropped (unverifiable, adds nothing).
- **S11**: "written by hand" added (kills the by-juxtaposition "the AI wrote it" read);
  "a second PR" demoted to on-screen chip pending the #3025 provenance pin.
- **S03**: "studied 580 papers; ninety-plus survived the gates" (the honest form is the
  stronger form); ★141 star chip added; 201-download floor stays off the video.
- **S04**: unit fix — 161 is default-on **features**, not lanes.
- **S13**: "two to three contributors"; "one engineer today" said on camera; "no external
  users yet" said out loud (naming the decisive risk = the same honesty as publishing gaps).
- **S07**: VO removed — the method pipeline is on-screen only; hard **2:00** target,
  **≤ 290 words**.
- **§7**: TTS options demoted to **scratch-track only** (timing the cut before the owner
  records); any TTS reaching the shipping cut must be disclosed on the end card.
- Testimonial: "3 friends adopted it" stays CUT — Proposal 025 states "no external users
  yet" twice; the only compliant revival is a named person + public repo + consent, which
  would also change the written answers.

## 1. What this is

The two-minute video for the Mozilla Human-Scale AI Incubator application
([Proposal 025](../.proposals/025_mozilla_human_scale_ai_incubator.md), deadline 2026-11-20).
Reviewer brief: *"a link to a two-minute video showing the project and the team's vision …
Reviewers want to see the technology working and hear directly from the team about why it
matters. The video does not need to be polished."* → real screen captures + the owner's real
voice. Honest and concrete beats polished.

Produced with **fframes** — the skill is installed at
`../riir-ai/.agents/skills/fframes-video/` (read `SKILL.md` first, then `references/design.md`,
`references/audio.md`, `references/captions.md`). Format: 1920×1080 landscape, 30 fps,
target ~118 s + ~3 s end card = ≤ 2:12 (the brief says "two-minute"; 2:10 is fine, never 3 min).

Project home: `../mozilla-video/` (sibling scratch dir, `cargo fframes new mozilla-video
--format landscape --fps 30 --yes`; no remote — the deliverable is the mp4 + this plan's record).

## 2. Fact pins — every number in the script, with source

⛔ The video agent must never invent a number. Everything below is pinned. 🔴 = re-measure on
recording day (the corpus and board move weekly).

| # | Claim (script wording) | Value | Source |
|---|---|---|---|
| P1 | "580 published papers distilled" | **649 research notes on disk, 580 citing arXiv** (grep-verified 2026-11-14); highwater 669 incl. removed files | `katgpt-rs/.research/*.md` count + `grep -li arxiv` |
| P2 | "ninety-plus gated Rust lanes" | GOAT table: 99 lane rows, 96 with gate verdicts | `katgpt-rs/README.md` L447–550 |
| P3 | "161 shipped default-on" | 668 feature flags, 161 default-on — repo's own count gate PASS | `katgpt-rs/scripts/count_features.py` |
| P4 | "57 theorems machine-checked in Lean" | `EXPECTED_THEOREMS=57`, 7 modules, axiom budget + sorry-refusal enforced | `katgpt-rs/.proofs/KatgptProof/`, `scripts/proof_gate.sh:72` |
| P5 | MIT + crates.io (substrate) | katgpt-rs MIT, github.com/katopz/katgpt-rs; **katgpt-core 0.5.0 live on crates.io** | `katgpt-rs/LICENSE`, crates.io API 2026-11-11 |
| P6 | "answers in under a millisecond" | G2 gate p99 ≤ 1 ms per decision set; measured p50 0.024–0.762 ms across 13 reflex harness suites (its own suite set); cheap-lane p50 84–391 µs | `riir-reflex/benches/decision_set_goat.rs`, `riir-reflex/.benchmarks/137_modelless_timing_quiet/RECORD.md` |
| P7 | Board beat: "outscores a 27B teacher" | **Aggregate index: Rethink 73.1% (areas) / 76.4% (benchmarks) vs mammoth-1 27B 67.4% / 70.6% vs Clef-flash 9B 60.6% / 65.1%**; disk **0.48 GB vs 6.80 vs 55.62 GB** → vs the 27B = **~116× smaller**, vs the 9B = 14× (never swap the two ratios); latency p50 geomean **1.6 ms vs 681 ms vs 2506 ms** → ~1567× **at the median only** (escalated-path multiple is far lower — own-encoder 21×–36×; never quote >1000× without "at the median"); Rethink serves **6/8 board spokes, 2 fallback**. ⚠ **Evaluation regime UNPINNED** — confirm from rethink bench records how the teacher was run (zero-/few-shot) before recording; the VO scopes the claim to "the public classification suites" and names datasets on screen | 🔴 LIVE board page (owner's screenshot; republish before capture; capture the URL for the on-screen chip) + `riir-rethink/AGENTS.md` sizing FAQ (Q8 447.7 MB ≈ 0.48 GB) |
| P8 | Per-suite gaps published | **Two suite sets — never mix them on screen:** the board = 8 benchmark spokes (ag_news, massive_intent_en, banking77, sst5, emotion, xnli_en, prompt_injections, typed_decisions); the wider record set trails on 11 suites (worst hellaswag −57.7, winogrande −23.4; closest ag_news −0.2, incl. thai_wisesight/thai_sib200 + the JDI suites). On-screen gap table = the board's own per-spoke display only | same board; `riir-rethink/.benchmarks/096`, `086`, `087` |
| P9 | Escalation containment | ESC serves a pinned **[15%, 60%]** band with a production rate guard; measured armed rates 42.7–52.8% | `riir-instinct/tests/serve_gates.rs`, `riir-rethink/.benchmarks/089` |
| P10 | Method sentence | paired A/B, one frozen read, pre-registered bars, Wilson 95% bounds, non-inferiority δ = max(1.0 pp, 2.5·SE) | `riir-instinct/src/stats.rs`, `riir-rethink/.benchmarks/089.../RESULTS.md` |
| P11 | "default lint fixer since August, 13 repos" | 13 workspace repos' AGENTS.md carry the "`cargo refine` before manual fixes" convention; in daily production since 2026-08; private game production = the seal runtime repo (never name the game) | grep `"Lint fixing" + "cargo refine"` across `/Users/kat/git/*/AGENTS.md` |
| P12 | Corpus: "1,669 rules · 8 domains · from 562 · ~100/week" | 🔴 manifest today: **1,669 rules / 8 domains** (clippy 120 + rust_perf 338 + kernel_opt 1,089 + sec 69 + docker 31 + dist 8 + shader 8 + ane-preview 6); history first row **2026-08-28: 562**; growth ≈ **99–100 rules/week** | 🔴 `riir-refine/.benchmarks/pins_manifest.json` + `corpus_history.jsonl` (re-sum on recording day) |
| P13 | Heal rate: "85% then, 96 now, 254 runs" | 🔴 score-bench `heal_rate`: first recorded run **84.6%** (2026-08-19) → latest **96.4%**, parse-safety 1.0, **254 recorded runs**; say the trend, never "increases each day" | 🔴 `riir-refine/.benchmarks/score_history.jsonl` |
| P14 | Distribution | **crates.io `cargo-refine` 0.5.1 (2026-11-11)**; GitHub releases + Homebrew tap + Scoop bucket (Sept 2026) | `riir-refine/AGENTS.md` §Open posture, issue 221 |
| P15 | Mozilla upstream | **sccache #2977 MERGED** (six review rounds, merge `907230be`) — **written by hand** (owner verdict: goodwill class; say "our first upstream fix, written by hand", NEVER "the AI wrote it"); **#3025 open, CI 50 pass / 0 fail** — ⚠ **provenance UNPINNED (tool-found vs hand-written)**: confirm before any "a second" VO line; until pinned it stays an ON-SCREEN CHIP ONLY | 🔴 `.benchmarks/127_mozilla_wetrun/README.md`, `.benchmarks/128_mozilla_rustperf_day2/README.md` |
| P16 | Transparency sentence | contribution points non-transferable, expiring, no cash value; **not a blockchain**: per-op conservation invariant `Σ balances + healed + expired == reservoir + topups` (test-pinned + Lean-proven `Kat/Tuna.lean`), per-epoch Merkle anchor roots published hourly (`GET /points/anchor` 200-verified); resettable client UUID, hardware fingerprinting refused | Proposal 025 §points plane; `riir-dapps/AGENTS.md`, `.proofs/RiirDappsProof/Kat/Tuna.lean` |
| P17 | The ask | $300K: ~60% people (2–3 contributors), ~15% pilot, ~15% infra+audit, ~10% contingency; GPU spend = training the decision tier (W7) + our corpus production lane; fleet tooling opens MIT; never imply paid "fixer nodes" (uptime earns 0 points by design) | Proposal 025 §Year-1 workplan |

**DO-NOT-SAY list (hard):**
- ~~"blockchain / distributed ledger / autonomous DAO"~~ for refine — the points ledger is
  deliberately a service plane + Merkle anchors; the DAO is advisory-only (recorded verdict)
  for a frozen lane, outside this project.
- ~~"the AI wrote the merged Mozilla fix"~~ — #2977 is hand-written goodwill.
- ~~"beats GPT / frontier models"~~ — never measured; name models + public datasets when
  citing the board (mammoth-1 27B MIT, Clef-flash 9B; ag_news, banking77, sst5, xnli…).
- ~~"mine / mining / token / gas / payment"~~ — vocabulary law: contribute / contribution
  points / starter allowance / anti-spam deposit.
- ~~"resolving rate increases each day"~~ — say the measured trend (P13).
- ~~"600+ papers"~~ — use 580+ (P1).
- ~~binary size in MB for reflex~~ — unmeasured; say "one static binary".
- ~~"we win every suite"~~ — the 11 open gaps are published on purpose; keep them visible.
  That disclosure IS the goodwill.

## 3. Creative direction

- **One thing to remember:** *"The corpus is the model."* Everything else supports it.
- **Structure = before/after, three acts, 1-2-3 + close:** (1) the substrate that distills
  papers into proven small primitives, (2) the decision stack that beats bigger models on the
  public board, (3) the product already in daily production — then the ask.
- **Cohort fit (silent, for the owner's confidence — not spoken):** the three preselected
  anchor projects (Tiles: small local models + offline installers + open governance;
  Family Intelligence: local-first memory you can export and verify; DAIR: shared
  values-aligned compute + published operator guides) are all the same species as cargo
  refine — local-first, small, community-governed, open. The video's beats (offline binary,
  local repair memory you can inspect and purge, MIT corpus + authoring/governance guides)
  speak the anchors' language without naming them.
- **Style:** near-black blue field `#0B0E14`, paper-white text `#E8ECF3`, ONE accent amber
  `#FFB454`; red `#E5484D` reserved for "before"/loss, green `#3DD68C` for "after"/pass.
  Grotesk for prose (Inter, OFL), mono for every number and terminal (JetBrains Mono, OFL) —
  both font files in `media/`.
- **The before/after device:** every act opens on a red-tinted "before" card and resolves to
  the dark "after" state. Diagonal wipe between them, once per act — not on every beat.
- **Motion:** springs/ease-out 300–600 ms in, 200–300 ms out, 60–120 ms staggers; counters
  tick in mono; one accent element per scene; screen captures sit in a subtle window frame
  with a slow 1.02× scale-in. No beat-spam, no full-frame zooms, nothing fades up 40 px with
  the same easing everywhere (the skill's no-slop list is binding).
- **Text discipline:** ≤ 8 words per line, ≤ 3 lines per card; the VO carries the story,
  on-screen text only anchors numbers and names.
- All motion synced to the VO, not to a metronome: place the VO first, cut to its phrases.

## 4. Scenes — time / VO / on-screen / graphics

Target durations below are pre-VO; **final timing derives from the recorded VO files** (each
scene owns its wav; scene duration = VO length + 0.6 s settle).

| id | t (target) | VO (see §5) | On-screen | Graphics + motion prompt |
|---|---|---|---|---|
| S01 | 0:00–0:09 | hook | `BEFORE: subscription · datacenter · your code uploaded` / `AFTER: offline · free · code stays` | Diagonal split. Left card red-tinted: cloud, `$` and upload-arrow icons drop in staggered; right card dark: unplugged-plug icon springs in. Wipe resolves left→right on "We built the opposite." |
| S02 | 0:09–0:15 | corpus-is-model | `THE CORPUS IS THE MODEL` | Giant type (130 px) with per-word spring-in; small rule tiles (`collapsible_if`, `needless_range_loop`, `map_clone`…) fly in and stack into a book shape; a GPU icon sits dimmed with a strike-through. |
| S03 | 0:15–0:24 | katgpt-rs papers | `katgpt-rs · MIT · ★141` + ticker `580 papers studied` + chip `90+ survived the gates` | Live capture: README GOAT-table slow scroll (or crates.io katgpt-core page). Ticker counts 0→580 in mono; the survival chip stamps in on "survived". Three primitive chips pop on the beats: `sigmoid (never softmax) ✓`, `ternary SIMD matvec`, `freeze / thaw`. |
| S04 | 0:24–0:33 | GOAT gates | `G1 CORRECT · G2 PERF · G3 NO-REG · G4 ZERO-ALLOC` + `161 features default-on` + `57 Lean theorems ✓` | Four stamps slam in sequence (thud SFX, 120 ms apart); counters tick up beside them; tiny terminal line `proof gate: 57 theorems · axioms ✓` types itself. |
| S05 | 0:33–0:41 | decision stack | `reflex · instinct · rethink` + `0.4 ms` stopwatch + `abstain` | Wire diagram: query card slides in → fast lane (stopwatch icon spins to `0.4 ms`) → amber `abstain` branch lights on "doesn't know". Labels `choice / score / noul` under the wire. |
| S06 | 0:41–0:53 | board beat | Live board + chip `board: <url>` + `ag_news · banking77 · sst5 · xnli_en …` + bars `0.48 GB vs 55.6 GB` · `1.6 ms vs 2506 ms (median)` | Live capture of the board page, Rethink row highlighted with the accent. Bars grow on the numbers (log axis like the site); dataset names line up under the radar; footnote chip: `per-suite gaps: published ↓` ghosting the board's own gap rows for 2 s. ⛔ Ratios: "~116× smaller, ~1500× at the median" vs the 27B — never the 14× (that's vs the 9B). |
| S07 | 0:53–0:57 | — (no VO; 4 s breather) | `pre-register → commit → one frozen read → Wilson bounds` | On-screen only: the pipeline builds left→right; a bracket labeled `LB95 > 0` snaps around the result; a losing lever gets stamped `DEMOTED · public` (red stamp, slight rotation). Music carries the beat. |
| S08 | 0:57–1:05 | refine production | `cargo refine` terminal + `13 of our own repos name it the default` | Live capture: `cargo refine` dry-run scrolling findings in a terminal frame. A constellation of 13 repo chips lights up around it, one per beat. |
| S09 | 1:05–1:15 | flywheel | `scan → retrieve → draft → compile-gate → record → learn` + `opt-in share · secret-scanned` | Redraw of the site's flywheel as 6 nodes on a ring, rotating slowly; the compile-gate node flashes ✓ and once ✗→revert; the share branch (dashed, dimmer) unlocks only when an `opt-in` toggle flips. |
| S10 | 1:15–1:24 | numbers | `562 → 1,669 rules · 8 domains` · `+100/week` · `84.6% → 96.4%` | Three counters tick in mono; underneath, a line chart climbs 84.6→96.4 with 254 tiny dots (the recorded runs) scattered along the axis. |
| S11 | 1:24–1:32 | upstream | `mozilla/sccache #2977 MERGED · written by hand · 6 review rounds` + `crates.io · brew · scoop` (+ chip `#3025 CI 50 ✓ 0 ✗` only if P15 provenance gets pinned) | PR card slides in; a green `MERGED` stamp hits #2977; three installer badges underline at the end. The #3025 chip stays OFF the VO — chip only, and only after the provenance pin. |
| S12 | 1:32–1:40 | transparency | `Σ balances + healed + expired = reservoir + topups ✓` · `Merkle anchor roots · public` · `fingerprinting: refused` | The conservation equation types out and its `=` pulses green with a ✓; a small Merkle tree roots into a hash chip labeled `published hourly`; a shield icon stamps `refused` on a fingerprint icon. |
| S13 | 1:40–1:52 | ask + close | donut `60% people · 15% pilot · 15% infra · 10% contingency` + chips `one engineer today` · `0 external users — pilot recruiting` · `GPU: decision tier + corpus` | Donut segments sweep in; the honesty chips stamp in on "we say so"; a GPU chip orbits to the decision-tier segment. On "Small by design." the frame clears to the close line; on "Yours by default." it holds. |
| END | 1:52–1:56 | — | `cargo refine — the corpus is the model` + `refine.riir.ai · github.com/katopz/katgpt-rs · crates.io/cargo-refine` + `MIT` (+ `narration: [owner name]` — or a TTS disclosure line if Option B ever ships) | Quiet end card, one fade, nothing else moves. |

## 5. Full VO script (the transcription — 297 words, hard 2:00)

> S01 — Cloud AI fixes your code — for a subscription, on a datacenter, with your source
> leaving the machine. We built the opposite.
>
> S02 — cargo refine heals code with no model in the hot path. The corpus is the model.
>
> S03 — katgpt-rs — public, MIT — studied five hundred eighty papers; ninety-plus survived
> the gates as Rust lanes: sigmoid scoring, ternary SIMD, freeze-and-thaw.
>
> S04 — Each lane earns default-on through a GOAT gate — correctness, speed, no regressions,
> zero allocations. One sixty-one features on; fifty-seven theorems machine-checked in Lean.
>
> S05 — That substrate also powers our decision stack — reflex, instinct, Rethink. It
> answers in under a millisecond, and abstains when it doesn't know.
>
> S06 — Where it abstains, it escalates sparingly. Our board is public: this half-gigabyte
> stack outscores a twenty-seven-billion-parameter teacher — over a hundred times smaller,
> a thousand times faster at the median — gaps published.
>
> S07 — (no VO — the method pipeline plays on screen over music)
>
> S08 — That discipline became cargo refine: the default lint fixer in daily production
> since August — thirteen of our own repos name it.
>
> S09 — It writes a fix only if your crate still compiles, learns what worked — and only if
> you opt in, shares secret-scanned fixes that improve every machine.
>
> S10 — The corpus: sixteen sixty-nine rules, eight domains — from five sixty-two in August,
> a hundred new a week. Bench heal rate: eighty-five then, ninety-six now.
>
> S11 — It's on crates dot i-o, Homebrew, and Scoop. Our first upstream fix — written by
> hand, to Mozilla's own sccache — merged after six review rounds.
>
> S12 — Contribution points are reputation — never payment. No blockchain: every point
> conserved per operation, anchored in public Merkle roots. Fingerprinting? Refused by
> design.
>
> S13 — Today it's one engineer — and no external users yet; we say so. The grant funds two
> to three contributors, a recruiting pilot, and the GPU behind the decision tier. Small by
> design. Yours by default.

If VO still runs past 2:00: cut S13 "and the GPU behind the decision tier" last — never
restore the S07 VO.

## 6. Captions (subtitle-ready)

- **Track:** one word-timed JSON caption track in `media/captions.json` (the skill's
  `references/captions.md`): cues = the 13 VO lines, words = per-word timestamps from the
  transcription step (§7). ⛔ Never split a phrase evenly to fake word timings — either real
  word times (whisper) or plain VTT phrases.
- **Render:** one `Captions` component built in the video constructor:
  `Captions::from_media("captions.json", CaptionRect::new(160.0, 880.0, 1600.0, 150.0))`,
  font Inter 46 / weight 600, `CaptionMode::Highlight`, spoken `#E8ECF3`, active `#FFB454`,
  outline `#0B0E14` 3 px. Insert once in `render_frame`; clock = `CaptionClock::Global`.
- **Fallback:** if word timings are unavailable, ship the same lines as plain VTT
  (`captions.vtt`) — phrase mode, same rect.
- **SRT deliverable** (for the application link's player, timings = the table above, shift
  after VO):

```srt
1
00:00:00,200 --> 00:00:08,800
Cloud AI fixes your code — for a subscription, on a datacenter, with your source leaving the machine. We built the opposite.

2
00:00:09,400 --> 00:00:14,800
cargo refine heals code with no model in the hot path. The corpus is the model.

3
00:00:15,400 --> 00:00:23,800
katgpt-rs — public, MIT — studied 580 papers; 90-plus survived the gates as Rust lanes: sigmoid scoring, ternary SIMD, freeze-and-thaw.

4
00:00:24,400 --> 00:00:32,800
Each lane earns default-on through a GOAT gate — correctness, speed, no regressions, zero allocations. 161 features on; 57 theorems machine-checked in Lean.

5
00:00:33,400 --> 00:00:40,800
That substrate also powers our decision stack — reflex, instinct, Rethink. It answers in under a millisecond, and abstains when it doesn't know.

6
00:00:41,400 --> 00:00:52,800
Where it abstains, it escalates sparingly. Our board is public: this half-gigabyte stack outscores a 27-billion-parameter teacher — over a hundred times smaller, a thousand times faster at the median — gaps published.

7
00:00:57,400 --> 00:01:04,800
That discipline became cargo refine: the default lint fixer in daily production since August — 13 of our own repos name it.

8
00:01:05,400 --> 00:01:14,800
It writes a fix only if your crate still compiles, learns what worked — and only if you opt in, shares secret-scanned fixes that improve every machine.

9
00:01:15,400 --> 00:01:23,800
The corpus: 1,669 rules, 8 domains — from 562 in August, a hundred new a week. Bench heal rate: 85 then, 96 now.

10
00:01:24,400 --> 00:01:31,800
It's on crates.io, Homebrew, and Scoop. Our first upstream fix — written by hand, to Mozilla's own sccache — merged after six review rounds.

11
00:01:32,400 --> 00:01:39,800
Contribution points are reputation — never payment. No blockchain: every point conserved per operation, anchored in public Merkle roots. Fingerprinting? Refused by design.

12
00:01:40,400 --> 00:01:51,800
Today it's one engineer — and no external users yet; we say so. The grant funds two to three contributors, a recruiting pilot, and the GPU behind the decision tier. Small by design. Yours by default.
```

(12 cues — S07 carries no VO; shift all timings to the real VO after recording.)

## 7. Voiceover — recommendation + pipeline

**Recommendation: Option A — the owner records it.** The reviewer brief says *"hear directly
from the team"*; Proposal 025's Team answer says the project is built by one principal
engineer — so "the team's voice" IS the owner's voice, and TTS in the shipping cut defeats
the stated purpose of the video. Phone-memo quality is explicitly fine ("does not need to be
polished"). Record per scene (12 short takes, not one long read — retakes stay cheap), quiet
room, phone 20 cm away, read the §5 lines as written. Name the files `vo_s01.wav` …
`vo_s13.wav` (S07 has none).

Fallbacks — **scratch-track only** (for timing the cut before the owner records; never the
shipping voice):
- **Option B — Kokoro-82M** (open weights, Apache-2.0, runs CPU via onnx; HF:
  hexgrad/Kokoro-82M). Free, offline, decent.
- **Option C — ElevenLabs** (best quality, paid, cloud; send the script text only — no code).
- ⚠ If TTS ever reached the shipping cut anyway, it MUST be disclosed on the end card or in
  the link description — undisclosed synthetic narration in a video about trustworthy AI is
  an unpriced credibility risk for zero gain. macOS `say`: not at all.

**Pipeline (either option):**
1. Produce the 13 VO files → drop into `media/`.
2. Word timestamps: run the final VO through whisper with word timestamps
   (`whisperx` or `openai-whisper --word_timestamps True`) → emit `media/captions.json`
   in the skill's cue format (§6). One clock, absolute seconds.
3. Scene timing: each scene's `duration()` = its VO length + 0.6 s settle; the audio map
   places each `vo_sNN.wav` at its scene start with `.voice()`.
4. Music bed (owner picks; suggest ~100–110 BPM so cuts land):
   `AudioTrack::new("music.mp3", Second(0.)..Eof).gain_db(-18.).fade_in(1.).fade_out(2.).duck_under_voice()`.
   Minimal SFX: stamp thuds on S04/S11/S12, ticks on counters — `.gain_db(-8.)`, times from
   the same constants that drive the animations.
5. Gate: `audio analyze` ≈ **−14 LUFS** integrated, true peak < −1 dBTP, no unintended
   silence; listen once in `preview`.

## 8. Build notes for the fframes agent

1. Read `../riir-ai/.agents/skills/fframes-video/SKILL.md` + `references/design.md` +
   `references/audio.md` + `references/captions.md` before any code. Plan first (the skill's
   three rules); the scene table in §4 is the plan.
2. Scaffold: `cargo install --locked cargo-fframes && cargo fframes new mozilla-video
   --format landscape --fps 30 --yes` in `../mozilla-video/`; start `cargo build --release`
   in the background immediately.
3. One scene struct per §4 row (`S01`–`S13` + `EndCard`), `Overlap::Previous(0.4)` on the
   scene transitions that carry an element across the cut (S03→S04 table scroll, S05→S06 wire).
   Replace every template leftover — the starter layout/colors are the first thing to go.
4. Media: `media/` gets the 13 VO wavs, music, the 2 fonts, `captions.json`, and the screen
   captures (§9) as trimmed MP4 clips played through `Clip::new(file, a..b)` inside their
   scenes — one scene per spoken segment, `fn audio` places that segment's VO so picture and
   voice move together. ⛔ No ffmpeg/shell cutting scripts — everything through the crate.
5. Every number on screen comes from §2. If a LIVE pin moved on recording day (corpus count,
   heal rate, board numbers), update the pin table here first, then the code — never hardcode
   from memory.
6. Checks, in order: `timeline` (pacing) → `inspect --fail-on warning` → `strip <scene> -n 12`
   per scene → `frame` crops for the dense cards (S06 bars, S12 equation, S10 counters) →
   `inspect --pops` → `audio analyze` → `preview` for the owner. Then `render -o mozilla.mp4`
   and verify with `ffprobe -v error -show_entries
   stream=codec_type,width,height,nb_frames,duration mozilla.mp4`.
7. Deliverables: `mozilla.mp4` (≤ 2:12, 1080p30, H.264 + AAC), `captions.srt` (§6, shifted to
   the real VO), and the strip contact sheets for the owner's review.

## 9. Screen assets to capture (owner, before the build — dark theme, 1920×1080 or 2× retina)

1. `cargo refine` dry run in a real repo — findings list scrolling (S08).
2. `cargo refine fix` — a fix applying + one compile-gate revert moment if easy (S09 backup).
3. The live board page (the screenshot the owner pasted: Rethink / Clef-flash / mammoth-1,
   radar + footprint + latency) — 🔴 republish first if the numbers moved (S06).
4. refine.riir.ai hero — the flywheel section + live counters (S09 reference for the redraw).
5. katgpt-rs README GOAT table slow scroll (S03) — or the crates.io katgpt-core page.
6. `cargo test -p katgpt-core --lib` tail + `scripts/proof_gate.sh` pass line (S04 backup).
7. mozilla/sccache PR #2977 (merged, six rounds) + #3025 CI green (S11).

Capture tips: `just capture` exists for editor windows but these are terminal/browser —
use `ffmpeg -f avfoundation` or OBS, cursor hidden, and keep each take ≥ the scene length.

## 10. Sources

- Proposal 025 (all posture + workplan + compliance facts): `../.proposals/025_mozilla_human_scale_ai_incubator.md`
- Fact sheets (this plan's audit, 2026-11-14): four read-only subagent passes over
  katgpt-rs, riir-reflex/riir-instinct/riir-rethink, riir-refine, riir-dao/riir-dapps —
  the pin table above is their consolidated output; owner corrections folded in
  (board beat = aggregate index win + published gaps; papers = 580 arXiv-citing research notes).
- fframes skill: `../riir-ai/.agents/skills/fframes-video/` (copied from
  github.com/dmtrKovalenko/fframes `skills/fframes-video`).
