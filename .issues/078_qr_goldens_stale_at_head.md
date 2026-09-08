# Issue 078 — the QR goldens are stale at HEAD: `qr_commitment_golden` + `render_qr_ascii` byte-identity both fail with the work tree untouched

**Status:** OPEN — found 2026-09-09 while validating the 075 soft gate; proven PRE-EXISTING (fails with the session's work `git stash`ed)

## The finding

Two `kat_transport`-gated lib tests in `src/kat_lease_client.rs` fail at
develop HEAD (first seen under `a7878ed`, still true at `de729c9`):

| test | line | symptom |
|---|---|---|
| `qr_commitment_golden` | ~2242 | `hex32(qr_commitment(QR_GOLDEN_URL))` = `a06a9976…` vs pinned `QR_COMMITMENT_GOLDEN` `de761701…` |
| `render_qr_ascii_is_byte_identical_to_the_pre_t14_call_shape` | ~2359 | render digest `4f0f6ee5…` vs pinned `213caa9e…` |

The INDEPENDENT halves pass: `qr_render_params_are_the_pinned_ones` is
green, and `qr_commitment_golden`'s own independent preimage recompute
(the "pin every reimplemented derivation" block built from
`QrCode::with_error_correction_level` directly) AGREES with the fn's
output — the preimage layout did NOT drift. What drifted is the
**pinned digest vs what the current render actually produces**: the
goldens record a render this build can no longer reproduce, i.e. the
qrcode crate's OUTPUT (module matrix or version encoding) changed under
a transitive bump while the pins stayed.

Likely era: the `f7cbe31` v0.1.2 batch flipped
`DEFAULT_SERVICE_URL`/`QR_GOLDEN_URL` to `heal.gist.rs` and updated
`QR_COMMITMENT_GOLDEN` then — but the goldens have rotted again since
(suspect a `Cargo.lock` refresh moving `qrcode` inside the pinned
`0.14` requirement, changing cell ordering/`code.width()`).

## Why it matters

These two tests are the only guards that the top-up QR a user scans is
byte-for-byte the QR the server-issued URL encodes — a silent render
change is exactly the "two QRs, one account" drift they exist to catch.
Right now the guard is ALARMING (red), not protecting; a red-for-a-month
golden teaches everyone to ignore it.

## The fix shape (when picked up)

1. `cargo tree -i qrcode` + `git log -p Cargo.lock` to date the bump.
2. Decide: re-pin BOTH goldens to the current render (the drift is
   upstream rendering, not a preimage break — the independent recompute
   proves the layout), AND add a `qrcode` VERSION pin assertion next to
   `QRCODE_PINNED_REQUIREMENT` so the next transitive move fails loudly
   at the version line instead of at a digest.
3. If instead the drift is IN this repo (a render param change that
   skipped the "edit both" rule), that's a contract break — restore the
   old render, not the pin.

Not gated by any CI lane: `kat_transport` lib tests run in the
`ci_feature_guard.sh` feature matrix (layer 2/3) — verify which pinned
suite actually compiles this target before closing (`scripts/` +
`.github/workflows/rust.yml`; the Gate-Coverage lesson applies).

## Related

- Found during riir-clippy `.issues/075` soft-gate validation
  (`de729c9`); the 075 LANDED section names both tests as
  proven-pre-existing.
- `f7cbe31` (the URL flip that last touched the goldens).
