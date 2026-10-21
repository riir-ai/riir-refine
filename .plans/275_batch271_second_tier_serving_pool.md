# Plan 275 — Batch 271: the splash-wave second-tier serving pool

**Status:** COMPLETE — landed 2026-10-21 (shikuwa/4090, riir-refine idle loop; the claim
`2cfcb104d` discharged by the landing row; 4 authoring slice commits + the landing
commit; Research 307 is the record).
Source: incoai/splash, the B268/B270 staged second tier — 13 B/B− candidates
(`253506f`, `88d0e57`, `91d4ddb`, `4d17995`, `fa63d97`+`593f2a6`, `68b9e70`, `81ae003`,
`ed1092f`+`2f5fe8d`, `dab8841`+`6fcd9b1`+`25502fd`+`7e51fcb`, `5ea4043`+`82b93c5`,
`b7844dc`, `4fc7465`, `e2c0af2`) enumerated in Research 305 §STAGED + Research 306
§staged. All rust_perf-lane. Honest drops taken at the T0 re-read: `6fcd9b1`
(error-copy), `25502fd` (DRY, the commit's own "Effect: none"), `7e51fcb`
(error-copy); 4d17995's third mechanism dropped as lifetime plumbing. Landed: +12 OWN
rust_perf rules (corpus 254→266, entries_43-46) + 4 count-neutral rider faces.

## Tasks

- [x] T0 — clone splash to `.raw/b271/splash`, verify all 13 staged commit ids present;
      read each candidate's subject+diff, grade own-rule / rider / drop (dedup grep
      against rust_perf 254 + kernel_opt first, 3+ vocabulary variants) — 3 grading
      subagents, 19/19 ids verified, verdicts 12 OWN + 4 faces + 4 drops
- [x] T1 — claim row committed + pushed BEFORE authoring (`2cfcb104d`)
- [x] T2 — author rules via disjoint-slice subagents (≤3 rules per slice, standalone
      compile + execute fixtures before landing; ALL NotBenchable annotated; every
      quote machine-verified at the pin) — slices A `7de3d543b` / B `e88a86ea7` /
      C `e498d07d8` / D (coordinator-committed at landing); fixtures 6/6 per slice
      green standalone
- [x] T3 — wire entries files + batch271 needle gate; count pins via ONE shape-asserted
      script pass (the B265 lesson: never fuzzy-edit multi-site pins) — mod.rs ×4
      registrations + header docs; pins total 266 / without_spec 145 / annotated 213
      (+ the drafter-propagation form the lib found); `batch271_rules_landed` landed
      FIRST-RUN PASS (12 rules + rider faces); 3 rider faces authored on the hosts
      (253506f + 4fc7465 → entries_39, 4d17995-buffer → entries_02)
- [x] T4 — gates: default lib **2729/0**; rust_perf_bench lib **2760/0**;
      clippy −D clean ×4 postures (default / no-default / rust_perf_bench /
      kernel_opt+latent — the 4th swept 3 PRE-EXISTING B270 residues:
      batch251/batch268 gates' too_many_lines + retrieval_eval doc_markdown);
      retrieval floors HELD 22/22 no re-pin (462 s)
- [x] T5 — freeze gen2+gen3 e2es PASS (28.6 s + 24.7 s) — no table movement (the
      freeze corpus builder is the clippy_lints seed; a rust_perf-only delta moves
      nothing — the P30/B250/B270 line, run anyway per the claim)
- [x] T6 — score-bench appended (**#198** heal 0.9636 FLAT / parse 1.0, gen-1
      SATURATED); release-set binary rebuilt + installed (build == the landing HEAD,
      clean stamp) BEFORE push (the §0 bin-age law)
- [x] T7 — docs: Research 307 + this plan all [x] + AGENTS.md rust_perf bullets (BOTH
      — the Role bullet was stale at the pre-B270 242, repaired here) + README row +
      `.docs/11_domains/rust_perf.md` + snapshot landing row; `.raw/b271` removed
- [x] T8 — the landing commit + push (slices A/B/C committed their own files
      en-route; the landing registers + pins + docs in one commit)
