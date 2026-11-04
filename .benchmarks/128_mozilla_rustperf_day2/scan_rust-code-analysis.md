cargo-refine 0.4.2 — help: `cargo refine help`
scan root: /Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis — 92 source file(s)
Refining…
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/pdf_js_test.rs:1-1 (routed: clippy)
  [clippy] needless_bool_assign             score     3.26  →  fn f(c: bool) { let mut x = false; x = c; }
  [clippy] unused_unit                      score     3.20  →  struct S;
  [clippy] chunks_exact_to_as_chunks        score     2.96  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    11.16  →  #![allow(dead_code)]
  [perf  ] per-read-timeout-vs-request-deadline score     8.99  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score     8.63  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/pdf_js_test.rs:3-3 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/pdf_js_test.rs:5-131 (routed: perf)
  [clippy] manual_checked_ops               score  1870.21  →  fn share(total: u64, n: u64) -> u64 {
  [clippy] doc_overindented_list_items      score   335.73  →  /// - the item text.
  [clippy] len_without_is_empty             score   327.82  →  pub struct LwA { items: Vec<u8> }
  [perf  ] generation-counter-stale-timer-invalidation score  3929.84  →  #![allow(dead_code)]
  [perf  ] exclusive-mkdir-temp-resource-reservation score  3800.46  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score  3476.37  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/serde_test.rs:1-1 (routed: clippy)
  [clippy] needless_bool_assign             score     3.26  →  fn f(c: bool) { let mut x = false; x = c; }
  [clippy] unused_unit                      score     3.20  →  struct S;
  [clippy] chunks_exact_to_as_chunks        score     2.96  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    11.16  →  #![allow(dead_code)]
  [perf  ] per-read-timeout-vs-request-deadline score     8.99  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score     8.63  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/serde_test.rs:3-3 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/serde_test.rs:5-8 (routed: clippy)
  [clippy] semicolon_if_nothing_returned    score     7.62  →  fn greet() {}
  [clippy] print_with_newline               score     6.45  →  fn ready() { println!("ready"); }
  [clippy] single_char_pattern              score     6.28  →  fn f(s: &str) -> bool { s.contains('x') }
  [perf  ] wasm-unwrap-panic-machinery-elimination score    18.27  →  // The same two sites behind a trait whose wasm arm compiles to a
  [perf  ] nonmax-sentinel-option-niche     score    16.11  →  // OPT: usize::MAX is IMPOSSIBLE for an offset (Rust guarantees
  [perf  ] kept-prompt-states-pinned-host-cache-budget score    15.83  →  // OPT: a save copies the stream's state to PINNED HOST memory and the device
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/deepspeech_test.rs:1-1 (routed: clippy)
  [clippy] needless_bool_assign             score     3.26  →  fn f(c: bool) { let mut x = false; x = c; }
  [clippy] unused_unit                      score     3.20  →  struct S;
  [clippy] chunks_exact_to_as_chunks        score     2.96  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    11.16  →  #![allow(dead_code)]
  [perf  ] per-read-timeout-vs-request-deadline score     8.99  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score     8.63  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/deepspeech_test.rs:3-3 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/tests/deepspeech_test.rs:5-21 (routed: perf)
  [clippy] manual_checked_ops               score   210.17  →  fn share(total: u64, n: u64) -> u64 {
  [clippy] doc_overindented_list_items      score    61.09  →  /// - the item text.
  [clippy] mismatched_bit_width_type        score    43.57  →  fn mbwt(w: u8) -> u32 { w.bit_width() }
  [perf  ] reject-plan-runs-before-setup-with-outliving-side-effects score   320.02  →  #![allow(dead_code)]
  [perf  ] generation-counter-stale-timer-invalidation score   300.56  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score   296.77  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:1-1 (routed: perf)
  [clippy] unnecessary_get_then_check       score    30.73  →  fn ugtc(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { !m.contains_key(&k) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    15.85  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] sorted-input-previous-row-fast-path score    73.36  →  use std::collections::HashMap;
  [perf  ] papaya-lockfree                  score    53.05  →  use papaya::HashMap;
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    49.80  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:2-2 (routed: perf)
  [clippy] write_with_newline               score    17.61  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    13.44  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score    10.51  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] mtime-encoded-lru-recency-persistence score    99.73  →  #![allow(dead_code)]
  [perf  ] exclusive-mkdir-temp-resource-reservation score    32.64  →  #![allow(dead_code)]
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    27.71  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:3-3 (routed: perf)
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    15.85  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    79.81  →  #![allow(dead_code)]
  [perf  ] keyword-map-to-match             score    49.44  →  #[derive(Clone, Copy)]
  [perf  ] sender-per-work-item-terminates-channel score    49.44  →  // Every work item CARRIES a clone of the sender; the original is
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:4-4 (routed: perf)
  [clippy] write_with_newline               score    20.36  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    11.93  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score    11.75  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] batch-reserve-monotonic-counters-offload-bulk score    51.84  →  #![allow(dead_code)]
  [perf  ] child-piped-stderr-drain-for-process-lifetime score    30.05  →  #![allow(dead_code)]
  [perf  ] per-read-timeout-vs-request-deadline score    30.05  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:6-6 (routed: perf)
  [clippy] write_with_newline               score     9.76  →  use std::fmt::Write as _;
  [clippy] from_str_radix_10                score     6.83  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [clippy] io_other_error                   score     6.61  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] sender-per-work-item-terminates-channel score    24.23  →  // Every work item CARRIES a clone of the sender; the original is
  [perf  ] persistent-digest-memo-stat-identity-keyed score    22.89  →  // OPT: the digest is memoized ON DISK keyed by the file's full stat
  [perf  ] channel-send-batching-unpark     score    19.96  →  fn record_measurement(buf: &mut Vec<Sample>, tx: &crossbeam_channel::Sender<Vec<Sample>>, s: Sample) {
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:7-7 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:8-8 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] redundant_field_names            score     2.88  →  struct Point { x: i32, y: i32 }
  [clippy] match_ref_pats                   score     2.59  →  pub enum Fruit { Apple, Pear }
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.14  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    14.14  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:10-10 (routed: clippy)
  [clippy] io_other_error                   score    44.32  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score    20.86  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [clippy] from_str_radix_10                score    19.79  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] write-verify-readback-before-publish score    74.58  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] exclusive-mkdir-temp-resource-reservation score    73.11  →  #![allow(dead_code)]
  [perf  ] buffered-io                      score    70.96  →  use std::io::Read;
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:12-13 (routed: clippy)
  [clippy] assign_op_pattern                score    19.49  →  fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y += 1; y }
  [clippy] needless_late_init               score    17.79  →  fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }
  [clippy] unnecessary_get_then_check       score    16.37  →  fn ugtc(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { !m.contains_key(&k) }
  [perf  ] refusal-quarantines-entry-transient-passes score    68.72  →  #![allow(dead_code)]
  [perf  ] exclusive-mkdir-temp-resource-reservation score    53.01  →  #![allow(dead_code)]
  [perf  ] box-str-immutable                score    50.33  →  struct Config { labels: Vec<Box<str>> }
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:15-15 (routed: clippy)
  [clippy] assign_op_pattern                score    11.70  →  fn bump(x: i32, hits: &mut Vec<i32>) -> i32 { hits.push(x); let mut y = x; y += 1; y }
  [clippy] needless_late_init               score    10.98  →  fn late(hits: &mut Vec<u8>) -> u32 { hits.push(0); let x = 5u32; x + x }
  [clippy] manual_let_else                  score    10.02  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [perf  ] unset-control-arm-silent-default-parity score    42.60  →  // The control value is EXPLICIT and validated to differ from the
  [perf  ] tokio-guard-across-await         score    35.31  →  async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    31.86  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:18-18 (routed: clippy)
  [clippy] match_like_matches_macro         score    28.77  →  fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); matches!(x, 0) }
  [clippy] write_with_newline               score    24.60  →  use std::fmt::Write as _;
  [clippy] mem_replace_with_default         score    23.27  →  fn mrw(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::take(dst)) }
  [perf  ] refusal-quarantines-entry-transient-passes score    73.09  →  #![allow(dead_code)]
  [perf  ] mem-take                         score    66.68  →  fn take_buffer(buf: &mut Vec<String>) -> Vec<String> {
  [perf  ] exclusive-mkdir-temp-resource-reservation score    66.54  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:19-19 (routed: clippy)
  [clippy] match_same_arms                  score    16.23  →  fn msa(x: &str) -> u8 { match x { "a" | "b" => 1, _ => 2 } }
  [clippy] match_like_matches_macro         score    15.64  →  fn f(hits: &mut Vec<u8>, x: u32) -> bool { hits.push(0); matches!(x, 0) }
  [clippy] single_match                     score    13.64  →  fn use_v(v: u8) {}
  [perf  ] unset-control-arm-silent-default-parity score    56.93  →  // The control value is EXPLICIT and validated to differ from the
  [perf  ] resume-revalidates-provenance-before-reuse score    44.37  →  // The manifest records the build IDENTITY (source + calibration +
  [perf  ] tokio-guard-across-await         score    42.77  →  async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:21-25 (routed: dist)
  [clippy] manual_slice_fill                score    10.23  →  struct Terrain { data: Vec<f32> }
  [clippy] len_without_is_empty             score    10.10  →  pub struct LwA { items: Vec<u8> }
  [clippy] into_iter_on_ref                 score     9.85  →  fn f(hits: &mut Vec<u8>, v: Vec<u64>) -> Option<u64> { hits.push(0); v.iter().next().copied() }
  [perf  ] utf8-fast-lane-cold-diagnostics  score    60.34  →  // OPT: the fast lane validates BARE — no tracking state at all — and
  [perf  ] tokio-guard-across-await         score    52.36  →  async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {
  [perf  ] tier-write-defers-to-urgent-reads-except-a-read-behind-it score    48.88  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:27-27 (routed: clippy)
  [clippy] manual_map                       score    15.33  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] needless_match                   score    12.05  →  fn f(x: Option<u8>) -> Option<u8> { x }
  [clippy] question_mark                    score    11.68  →  fn f(x: Option<u8>) -> Option<u8> { x?; Some(x.unwrap() + 1) }
  [perf  ] snapshot-then-release-shared-drafter-context score    45.82  →  // OPT: the own lists are releasable AT THE SNAPSHOT SEAM because the
  [perf  ] tokio-guard-across-await         score    39.86  →  async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {
  [perf  ] page-miss-blocks-one-ranged-read-scatter score    36.65  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:28-28 (routed: clippy)
  [clippy] manual_map                       score    15.33  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] needless_match                   score    12.05  →  fn f(x: Option<u8>) -> Option<u8> { x }
  [clippy] question_mark                    score    11.68  →  fn f(x: Option<u8>) -> Option<u8> { x?; Some(x.unwrap() + 1) }
  [perf  ] snapshot-then-release-shared-drafter-context score    45.82  →  // OPT: the own lists are releasable AT THE SNAPSHOT SEAM because the
  [perf  ] tokio-guard-across-await         score    39.86  →  async fn update(cache: &tokio::sync::Mutex<Config>, key: String) {
  [perf  ] page-miss-blocks-one-ranged-read-scatter score    36.65  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/src/concurrent_files.rs:30-46 (routed: perf)
  [clippy] while_let_on_iterator            score    61.56  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [clippy] redundant_pattern_matching       score    61.25  →  fn f(r: Result<u8, u8>) -> bool { if r.is_ok() { true } else { false } }
  [clippy] question_mark                    score    61.11  →  fn f(x: Option<u8>) -> Option<u8> { x?; Some(x.unwrap() + 1) }
  [perf  ] pinned-entry-tombstone-freed-at-last-unpin score   182.30  →  // OPT: the pin spans the ASYNC operation -- pin at lookup, unpin at
  [perf  ] exclusive-mkdir-temp-resource-reservation score   124.84  →  #![allow(dead_code)]
  [perf  ] buffered-io                      score   117.57  →  use std::io::Read;
  … and 801 more span(s) — the summary below is the full count
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/rust-code-analysis-cli/Cargo.toml:11 [dist  ] pgo-training-set-covers-real-workload-hint — [[bin]]
           ↳ every manifest-level size/tuning knob this domain advises is set here — the next build-time lever is out-of-manifest: IF you use (or add) PGO, the training set must cover the SHIPPED workload mix (rustc #159091: adding rustdoc benches to the PGO training set = 2.85% mean wall-time across rustdoc benchmarks, >6% best; 'PGO is fiddly to set up but it can really make a difference, and the training set matters'); train on the real workload corpus, not a narrow benchmark — measure before/after
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/enums/Cargo.toml:23 [dist  ] pgo-training-set-covers-real-workload-hint — [profile.release]
           ↳ every manifest-level size/tuning knob this domain advises is set here — the next build-time lever is out-of-manifest: IF you use (or add) PGO, the training set must cover the SHIPPED workload mix (rustc #159091: adding rustdoc benches to the PGO training set = 2.85% mean wall-time across rustdoc benchmarks, >6% best; 'PGO is fiddly to set up but it can really make a difference, and the training set matters'); train on the real workload corpus, not a narrow benchmark — measure before/after
/Users/kat/git/riir-refine/.raw/mozilla-heal/rust-code-analysis/rust-code-analysis-web/Cargo.toml:1 [dist  ] pgo-training-set-covers-real-workload-hint — [[bin]]
           ↳ every manifest-level size/tuning knob this domain advises is set here — the next build-time lever is out-of-manifest: IF you use (or add) PGO, the training set must cover the SHIPPED workload mix (rustc #159091: adding rustdoc benches to the PGO training set = 2.85% mean wall-time across rustdoc benchmarks, >6% best; 'PGO is fiddly to set up but it can really make a difference, and the training set matters'); train on the real workload corpus, not a narrow benchmark — measure before/after
secrets: 0 finding(s) (+3 digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed)
🔍 Found clippy 75 | perf 75 | dist 3 — 153 total
🛠️ ~49% fixable via `cargo refine fix` (compile-gated; the suggest-only rows stay manual)

scanned 84 files · 826 spans · 826 with suggestions (top-3 per domain)
