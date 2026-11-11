//! `gateway-ctor-security-check-coverage` extension lives in
//! `gateway_ctor.rs` — this module is the FACE carrier documented in
//! the shell rule: the Batch-351 program-identity note.
//!
//! Batch 351 record: R6 `command-new-untrusted-program-path` was
//! RESCOPED-DROPPED at authoring (the research's own face-or-drop
//! gate): foxguard's detector flags every dynamic `Command::new`
//! argument, and dynamic program paths are ubiquitous and legitimate
//! (configurable tool paths) — the FP pressure is structural, not
//! tunable. The hazard CLASS (binary planting, CWE-426/427 — the
//! program resolves from PATH or user choice) is carried as the
//! count-neutral FACE on `shell-string-command-interpolation`
//! (the shell rule owns the metacharacter layer; the face names the
//! program-identity layer beside it), never as a rule.
//!
//! This module intentionally carries NO detector: the face is concept
//! text on the host rule (the B350 weak-hash co-occurrence shape), and
//! a detector for the program axis was the dropped candidate's exact
//! body. Batch 351 / Plan 361 / Research 394 (0sec-labs/foxguard @
//! `fbeab16`, MIT OR Apache-2.0).
//!
//! Recorded DROPS from the same source (the gate worked — the full
//! ledger in Research 394): the jwt `decode-without-verify` face
//! dropped honestly — the WRONG-API axis in Rust IS
//! `jsonwebtoken::dangerous::insecure_decode`, which
//! `jwt-insecure-validation-flags` already names (the JS `jwt.decode`
//! parse-only library has no Rust equivalent that the existing rule
//! does not cover).
