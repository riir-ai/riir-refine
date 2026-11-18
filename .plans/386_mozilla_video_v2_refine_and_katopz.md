# Plan 386 — Mozilla video v2: "refine → katopz" (project demo + team vision, 2-min re-do)

Status: EXECUTING 2026-11-18 — owner re-do of Plan 378 (v1 rejected: chart misplaced, flat voice, slide-show feel); storyboard + VO script + fact pins below; CLI `help` verb LANDED (`73e6cfc3`); VO batch + assets + build in flight · supersedes Plan 378's §4/§5 (378's fact pins + TTS lane stay the record)
Branch: `develop`

The CFP line this answers: *"a link to a two-minute video showing the project and the
team's vision. The video does not need to be polished. Reviewers want to see the technology
working and hear directly from the team about why it matters."* Two halves, in that order:
**Project** (refine working, end to end) → **Team** (katopz, the vision).

## Tasks

- [x] Agent: storyboard + VO script + fact pins (this doc)
- [x] Agent: `cargo refine help` verb (alias of `--help`) — riir-refine CLI — landed `73e6cfc3` (stdout, exit 0, byte-identical to `--help`)
- [x] Agent: `cargo refine train` verdict — **DECLINED** (see §Verdicts V1); the video shows the trainer as a node, and the terminal uses the real `cargo refine update`
- [ ] Agent: VO batch v5 on amo (Drama 3, inline `[excited]`/`[confident]`/`[warm]` tags, temp 0.8) — energy fix
- [ ] Agent: assets — talk photo, riir-ai avatar, contribution heatmap + growth data measured from git
- [ ] Agent: fframes rebuild (`~/git/mozilla-video`, new scene set) → inspect → strip → render
- [ ] Agent: fresh Claude verdict on the render (facts + motion + slop)
- [ ] Agent: `video-family` skill (riir-ai `.agents/skills/video-family/`)
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
| P2 | Install | URL wipes out; `cargo install cargo-` types in front of `refine` — the whole line re-centers, `refine` slides right with a spring | "One install, from crates.io." |
| P3 | Command | `install cargo-` fades + collapses; `cargo ` and `refine` close the gap to a centered `cargo refine` | "One command." |
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

## Assets

- Talk photo: `https://pbs.twimg.com/media/HG0p1A2bwAAJZnW?format=jpg&name=4096x4096` → `media/talk.jpg`
- riir-ai avatar: `https://avatars.githubusercontent.com/u/351331031?s=512` → `media/riir_ai.png`
- ⚠ The two pasted Zed images (contribution graph, growth) did not reach this session — the
  charts are re-measured from git (F9). Owner: drop the originals into
  `~/git/mozilla-video/media/` if they must replace the measured ones.
