# Plan 386 — Mozilla video v2: "refine → katopz" (project demo + team vision, 2-min re-do)

Status: EXECUTING 2026-11-18 — owner re-do of Plan 378 (v1 rejected: chart misplaced, flat voice, slide-show feel); storyboard + VO script + fact pins below; CLI `help` verb LANDED (`73e6cfc3`); VO v5 + assets + build DONE — `video-family` skill LANDED (riir-ai `931620cdb`) · **r1 fresh verdict = REVISE (7 blockers) → blockers 1–6 APPLIED, 7 half-applied (AI-voice chip at T1; owner's own voice = owner option)** — render `mozilla_v2.mp4` 120.47 s, sha `0f4f29d6…` (mozilla-video `c73c88e`) · **r2 (owner watch 2026-11-18): 'too excited, sometimes not, AI slop' → calm register re-cut — VO v6 ONE `[calm]` cue on every line (level spread −22.6..−25.4 dB vs v5's 6 dB swing; mix LRA 6.5→3.9 LU), no-overshoot motion, hype copy cut, install line → `cargo install riir-refine` (issue 244); render 120.00 s sha `b2ecea96…` (mozilla-video `5082379`); skill updated (riir-ai `670ce5c93`)** · **r3 (owner 2026-11-18): proof = the corpus grown by real use since August (dog-food start), not commit counts → heatmap dropped, corpus chart is the hero (412 @ 08-17 → 1,572); names respelled (Kat opz · R I I R, A I); render 122.07 s sha `79717f59…` (mozilla-video `191cac1`)** · awaiting owner watch + listen · supersedes Plan 378's §4/§5 (378's fact pins + TTS lane stay the record)
Branch: `develop`

The CFP line this answers: *"a link to a two-minute video showing the project and the
team's vision. The video does not need to be polished. Reviewers want to see the technology
working and hear directly from the team about why it matters."* Two halves, in that order:
**Project** (refine working, end to end) → **Team** (katopz, the vision).

## Tasks

- [x] Agent: storyboard + VO script + fact pins (this doc)
- [x] Agent: `cargo refine help` verb (alias of `--help`) — riir-refine CLI — landed `73e6cfc3` (stdout, exit 0, byte-identical to `--help`)
- [x] Agent: `cargo refine train` verdict — **DECLINED** (see §Verdicts V1); the video shows the trainer as a node, and the terminal uses the real `cargo refine update`
- [x] Agent: VO batch v5 on amo (Drama 3, inline `[excited]`/`[confident]`/`[warm]` tags, temp 0.8) — 17 takes, tags whisper-verified NOT spoken; t03 re-made ("Refine's"→"The suggestions from refine"); pace ≈1.8 words/s (voice-bound); names: katopz→"cat-ops", riir-ai→"rear AI" (owner listen)
- [x] Agent: assets — talk photo, riir-ai avatar, heatmap (34,446 owner-authored commits · 110/110 days · 40 repos, Aug 1→Nov 18) + corpus growth (412→1,572 `rule_id`s, weekly from git) — regeneration commands in the video README
- [x] Agent: fframes rebuild (`~/git/mozilla-video`, 15 scenes, `kit.rs` motion kit, `Xf` cross-fades) → inspect (clean but the deliberate Reflex code-wall bleed) → strips → render 120.13 s, −13.1 LUFS / −0.9 dBTP — mozilla-video `cda3e6c`
- [x] Agent: fresh Claude verdict on the render (facts + motion + slop) — r1 REVISE, applied (§Verdicts V5)
- [x] Agent: `video-family` skill (riir-ai `.agents/skills/video-family/`) — riir-ai `931620cdb`
- [ ] Owner: listen-check v5 takes + watch the render; swap in the two pasted Zed images if the measured charts differ from them (they did not reach the agent session — §Assets)

## Look (web-family → video)

- Background `#0b0d10` (family dark), terminal panel `#111418` with a 3-dot chrome bar.
- **`refine` is always green `#3fd68a`** (the `--c-refine` accent) and always the SAME glyph
  run — it moves, it never re-renders as a different element. `katopz` takes the complement
  (`#d63f8d`, magenta — hue 152°→332°).
- White `#f2f2f2` for URLs and commands, muted `#9aa0a6` for terminal noise.
- Highlight flash: cyan `#22d3ee` (the `--c-ai` hue) and violet `#a78bfa` (the `--c-rethink`
  hue) — used ONLY on the two sell lines (`🔍 Found …` / `🛠️ ~55% …`) and the fix result.
- Inter for words, JetBrains Mono for machine text. Flow figures follow the family §8 rule:
  numbered steps, lanes coloured by who runs them.

## Storyboard (target ≤ 120 s; scene times re-fit to the measured VO)

### Project

| # | Scene | Picture | VO (v5) |
|---|---|---|---|
| P1 | Title | `refine` center, green; `https://refine.riir.ai/` types in white beneath (per-char reveal, cursor) | "[excited] Meet refine." |
| P2 | Install | URL wipes out; `cargo install riir-` types in front of `refine` (V7) — the whole line re-centers, `refine` slides right with a spring | "One install, from crates.io." |
| P3 | Command | `install riir-` fades + collapses; `cargo ` and `refine` close the gap to a centered `cargo refine` | "One command." |
| P4 | Dry run | `cargo refine` flies to top-left as the prompt of a terminal that grows around it; the real capture (riir-dapps `src/data/refine_capture.txt`, v0.5.0) scrolls fast with a top-down motion blur, then brakes on the two sell lines, which pop (scale 1.0→1.12→1.0) with a cyan/violet flash | "[excited] A dry run, fully offline. Nine findings, and about fifty-five percent fixable right here, on your machine. So let's fix them!" |
| P5 | Fix | same terminal, `cargo refine fix` types; consent block blurs past; brakes on the verify lines + `6 edit(s) verified` + `100% fixed` (pop + flash) + a perf chip `perf  −18% allocs · demo` (labelled DEMO on screen — owner-authorised mock) | "[excited] cargo refine fix. Every edit has to compile, or it's rolled back. Six for six." |
| P6 | Screen | terminal zooms out to a card; a line draws from it through a **secret scan** gate (key icons struck out, `sk-…` → `████`) toward a node | "[confident] What it can't fix yet gets scanned for secrets first. Keys are dropped, never sent, and only if you opt in." |
| P7 | Fixer | terminal fades; FIXER node (container outline) — numbered steps 1 redacted span in → 2 LLM drafts a fix → 3 compile gate + real clippy → 4 proven pair out; arrow to TRAINER labelled `proven pair: rule · before/after span · verdict` | "Our fixer node drafts a fix with an LLM, inside a container, and keeps it only if it compiles and clippy goes quiet." |
| P8 | Trainer | TRAINER node (GPU lane): proven pairs → distill → rules → corpus epoch; fan-out arrows to many small terminals (the community) | "[excited] Then our GPUs distill the proven fixes into rules, and the new corpus goes back to everyone." |
| P9 | Update | the fan-out arrow curls back to the original terminal (fades in); `cargo refine update` → `corpus lease:  epoch 2973 installed atomically` (pop + flash on the epoch) | "cargo refine update, and your machine gets smarter." |
| P10 | Who | terminal zooms out; "Who made `refine`?" composes; `refine` slot-spins (top-down blur, glyphs roll, letters that already match lock first) and lands on `katopz` in magenta | "[curious] So, who made refine?" |

### Team

| # | Scene | Picture | VO (v5) |
|---|---|---|---|
| T1 | katopz | `katopz` center; `https://github.com/katopz` composes beneath; side text (on screen, not spoken): *Todsaporn Banjerdkit · Bangkok · Rust · AI for games · open source · community educator* | "[warm] Hi, I'm katopz, a Rust and game-AI developer from Bangkok." |
| T2 | Community | the talk photo (twimg `HG0p1A2bwAAJZnW`) Ken-Burns pan, caption chip "giving a community talk" | "I teach, and I build in the open." |
| T3 | Goodwill | two PR cards slide in: `mozilla/sccache #2977 · merged` and `#3025 · in review`; a two-bar chart (before 33.6 s / after 4.4 s) grows, the after bar snaps short, `7.6×` punches in — chip `this workload · medians of 3` | "[excited] Refine's suggestions became two pull requests to Mozilla's own sccache. One merged, one makes a stalled build seven times faster." |
| T4 | Family | riir-ai avatar as background (dimmed, slow zoom); `https://github.com/katopz/katgpt-rs` composes under it; the `katopz` segment flips (slot) to `riir-ai` → `https://github.com/riir-ai` | "Refine stands on katgpt-rs, our open inference core, and the riir-ai family." |
| T5 | Reflex | out-of-focus code scroll (fast, blurred) + reflex flow figure zooms in and out under a cut — the movie-trailer beat; `reflex.riir.ai` chip | "With Reflex deciding when to answer, and when to step back." |
| T6 | Invest | the two nodes again, small: FIXER (LLM calls) + TRAINER (GPU); contribution heatmap Aug→today fills cell by cell; the corpus growth line climbs; "self-funded" chip | "[confident] Today I pay for the LLM calls and the GPUs myself, every day since August, and the corpus keeps growing." |
| T7 | Vision | end card: `refine` + `katopz` + Mozilla-aligned line | "[warm] With Mozilla, we can make local AI that communities own. Better, faster, safer code, for everyone." |

End-card line (on screen, not spoken): *Voice generated with Fish Audio Drama 3 · script: owner-directed*
(the Plan 378 r14 disclosure carries over).

## Fact pins (every number on screen)

| Pin | Value | Source |
|---|---|---|
| F1 | dry run: `🔍 Found clippy 6 \| perf 3 — 9 total` / `🛠️ ~55% fixable via \`cargo refine fix\`` | riir-dapps `cloudflare/kat-service/src/data/refine_capture.txt` (v0.5.0 release binary, generated 2026-11-10) |
| F2 | fix: `kept 1 file(s) / 6 edit(s), reverted 0`, `🛠️ 100% fixed`, `⏱️ … refine 2.4s` | same capture |
| F3 | perf chip in P5 | **DEMO** — owner-authorised mock ("we can mock it up"); the chip says `demo` on screen; never quoted in the application |
| F4 | `cargo refine update` output shape | `src/points_lease.rs` (`corpus lease:  epoch N installed atomically`) — epoch number is illustrative (2972 is the capture's) |
| F5 | sccache #2977 merged 2026-11-07; #3025 open | GitHub API, read 2026-11-18 |
| F6 | 33.596 s → 4.394 s = 7.6× | `.benchmarks/128_mozilla_rustperf_day2/bench128_backend_backoff.svg` (standalone, medians of 3) — framed "this workload" per Bench 128's PR framing law |
| F7 | fixer = hosted-LLM drafts, cap-guarded, redaction-gated; gates = compile + real clippy | `.docs/01_orientation/role_flows.md` (fixer loop) |
| F8 | trainer = one container, cron-start/sleep-after, our rigs only today | same doc (trainer loop) |
| F9 | contribution heatmap + growth | measured from local git history, 2026-08-01 → 2026-11-18 (`media/contrib.json`, the generator command recorded in the video repo README) |

## Verdicts

- **V1 — `cargo refine train`: DECLINE (Claude verdict, 2026-11-18).** No user-side training
  exists — the client is modelless by mandate, and the trainer is an operator node. A `train`
  verb that only printed status would be a fake capability in a grant video. The terminal shows
  the real `cargo refine update` (corpus lease); the trainer appears as a node in P8/T6. If a
  status read is wanted later, it belongs under `cargo refine corpus` (already shows corpus state).
- **V2 — `help` verb: ADD.** `cargo refine help` falls through to the path parser today
  (treated as a directory named `help`); alias it to `--help` (stdout, exit 0).
- **V3 — voice energy.** v1's flatness came from the ref clip (a calm Common Voice read) and
  no direction tags. Drama 3 takes inline bracket cues (`[excited]`, `[confident]`, `[warm]`)
  at sentence start; v5 keeps the same pinned CC0 ref (voice continuity, provenance) and adds
  the cues + temperature 0.8.
- **V4 — container wording.** "inside a container" is pinned to the trainer window (F8) and the
  operator's node lane; the fixer VO keeps it to one clause — no sandbox-security claim beyond F7.

- **V5 — fresh render verdict r1 (2026-11-18): REVISE → applied (mozilla-video `c73c88e`).**
  Blockers: (1) "Refine's suggestions became two PRs" is FALSE by our own record — #2977 is
  goodwill (owner verdict 2026-11-02, Bench 127), #3025 is issue-driven, not healer-emitted
  (Bench 128 provenance) → the beat is now "Upstream goodwill → mozilla/sccache", VO "I also
  give back upstream"; (2) "seven times faster" overstated → VO "from thirty-three seconds to
  four", chip "test workload · 32 stalled reads"; (3) "only if you opt in" misstated the
  consent default (the prompt defaults to anonymous share) → "Nothing leaves until you're
  asked", chip "asked first · secret-blind"; (4) fixer "container" unsourced → dropped (the
  trainer window keeps it, F8); (5) the perf mock overrode the owner's "mock it" — the real
  `refine 2.4s` timing line replaces it (Claude verdict: an invented number in a
  "technology working" video costs more than it buys; owner may re-instate with the DEMO chip);
  (6) "34,446 commits · katopz" counts agent-authored commits → count dropped, heatmap labelled
  "human + AI-agent commits", linear intensity; (7) a synthetic voice says "Hi, I'm katopz" →
  legible AI-voice chip at T1 (agent half); recording t01/t07 in the owner's own voice is the
  owner's option. Bounded fixes taken: "6 of 6 fixed", jargon line cut, "With Mozilla —",
  growth relabelled "repair rules in refine's source". Kept (owner-asserted): trainer on our
  GPUs (p08).
- **F6 re-framed:** 33.596 s → 4.394 s on the Bench 128 test workload (32 stalled reads); the
  reporter's own reproduction measured ≈4.9× — never say "seven times faster" bare.

- **V6 — r2 calm register (owner, 2026-11-18).** The owner's watch verdict on r1: "too
  excited and sometimes no excite — looks like AI slop; should be consistent, calm energy,
  cool, smart, proud". Root cause measured: the per-line cue mix (`[excited]`/`[warm]`/…)
  swung take levels by 6 dB. Probe (p01/p04/t03 × 3 cues): `[calm]` held ±0.2 dB → the
  whole v6 script uses `[calm]`, temp 0.65 (amo `batch_v6.sh` + `lines_v6.txt`). Motion
  vocabulary de-hyped (no back-ease, no strobe, no shake/ring/white flash). Script v6 (calm,
  no exclamations) supersedes the §Storyboard VO column — the canonical text is amo
  `lines_v6.txt` and mozilla-video `media_src/v6/stt_v6.tsv`.
- **V7 — install name.** `cargo install riir-refine` (riir-refine issue 244: `refine` is taken
  on crates.io + homebrew-core; the binary stays `cargo-refine`). The crates.io publish is
  in flight — the video shows the post-publish command; re-verify `cargo install riir-refine`
  resolves before the video link is submitted.

- **V8 — r3 proof of use (owner, 2026-11-18).** "August is when we started using refine in
  real life; the corpus has stacked since that day — that proves real use, not code commits,
  which can be manipulated." F9 re-scoped: the commit heatmap is RETIRED; the T6 hero is the
  corpus rule count weekly from git history from the first dog-food week (2026-08-17: 412) to
  today (1,572). Local `.refine/blueprint_history.jsonl` fix-campaign records only reach back to
  2026-10-27 (96 campaigns, 418 edits workspace-wide) — too short for the August claim, so the
  corpus series carries it. Runtime 122.07 s: the calm voice (~2.3 words/s) + the owner's two
  must-say lines (dog-food since August + self-funded spend) cost ~2 s over 2:00 — accepted
  (the CFP asks for "a two-minute video", not a hard cap).

## Assets

- Talk photo: `https://pbs.twimg.com/media/HG0p1A2bwAAJZnW?format=jpg&name=4096x4096` → `media/talk.jpg`
- riir-ai avatar: `https://avatars.githubusercontent.com/u/351331031?s=512` → `media/riir_ai.png`
- ⚠ The two pasted Zed images (contribution graph, growth) did not reach this session — the
  charts are re-measured from git (F9). Owner: drop the originals into
  `~/git/mozilla-video/media/` if they must replace the measured ones.
