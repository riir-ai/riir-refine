# Issue 129 — cargo-heal scoop-bucket 0.1.4 manifest 404 (URL pointed at the v0.1.3 tag)

**Status:** RESOLVED 2026-09-22 — fixed + download-hash-verified the same day (`gist-rs/scoop-bucket` `9f70496`); filed as the durable record. Found during the riir-reflex Plan 606 T2.3 follow-up session (previously carried as "noted, not mine" in the Phase-2 handoff summary — this issue is the make-it-trackable step).

## Defect

`bucket/cargo-heal.json` in [gist-rs/scoop-bucket](https://github.com/gist-rs/scoop-bucket) declared
`"version": "0.1.4"` but its download URL fetched from the **v0.1.3** tag:

```
https://github.com/gist-rs/cargo-heal/releases/download/v0.1.3/cargo-heal-v0.1.4-x86_64-pc-windows-gnu.zip
```

The v0.1.3 tag carries no `cargo-heal-v0.1.4-*` asset → **HTTP 404 for every
scoop install/upgrade of cargo-heal 0.1.4 on Windows** since the 0.1.4 bump
(`170caa7`, "cargo-heal 0.1.4"). The `hash` field was already correct
(`0e985284…`) — only the URL's tag was stale, which is exactly the shape a
hand-edit produces: version string bumped, URL string missed.

## Fix + verification

- One-line fix: URL tag `v0.1.3` → `v0.1.4` (scoop-bucket `9f70496`).
- Verified END-TO-END, not by eyeball: `curl -sL` the fixed URL →
  `sha256 == 0e9852844b56d885a553109a61cad26f86cdba0149f6f4870aee0b5452460fed`
  — byte-identical to the manifest's hash AND to the v0.1.4 release's own
  `SHA256SUMS` row for the windows zip.

## Why it could happen (the lane note)

`release.yml` (Plan 105) owns tap/bucket PRs on `v*` tags, but the 0.1.4
bucket row landed as a manual commit — a hand path that has no mechanical
coupling between the version string and the URL tag. Prevention: prefer the
release.yml bucket-PR lane for every bump; when hand-editing is unavoidable,
copy BOTH `url` and `hash` from the freshly cut release's assets (never
retype the tag), and curl the URL once before pushing — a 404 in a bucket
manifest is a broken install for every Windows user, not a docs nit.

## Cross-references

- Discovery context: riir-reflex Plan 606 (`katgpt-rs/.plans/606_reflex_phase2_site_distribution.md`) — T2.3 session, 2026-09-22; the same session
  landed `bucket/riir-reflex.json` (`e9319bb`) under the "manifest lands WITH
  the first Windows asset" rule.
- Fix commit: `gist-rs/scoop-bucket@9f70496`.
- The reflex-side record: riir-reflex `e5dcfc9` (AGENTS.md/README of the
  606 close-out session).
