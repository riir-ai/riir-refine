//! Batch 362 — the `ThreeRT` sealed driver pipeline-cache `rust_perf` rule
//! (Plan 373 / Research 406 R6): a compile-artifact cache handed to a
//! DRIVER is sealed, identity-keyed, atomically installed, and hard-capped
//! before the driver sees it. The `kernel_opt` half of this batch is
//! `entries_gpu_157` (8 rules). File number 78 (not 77) deliberately: the
//! live b361 polars lane (zed-amo-b361-polars, amo) takes `entries_77` for its
//! rows — the gap avoids the same-name collision at push, the B353
//! coexistence discipline.
//!
//! Source: `meshy-dev/ThreeRT` @ `0a190f83c559d1dec33ee70b763433593dd67de3`
//! (main tip 2026-11-14T15:49+08:00, MIT © Meshy LLC; T0 re-pin — the
//! verdict pin `10fdc773` moved two commits the same day; every quote below
//! byte-verified at the NEW pin this session; the `pipeline_cache.rs` module
//! was untouched by both new commits). ORIGINAL Rust fixtures
//! (`corpus/fixtures/b362_fixtures.rs`, standalone-compiled AND executed
//! before entry — rustc --edition 2021 --test), never pasted source.
//! Batch 362 / Plan 373 / Research 406.

#[allow(clippy::too_many_lines)] // the corpus entry IS the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── io: a compile-artifact cache whose consumer is a DRIVER — a
        // trust boundary that need not check what it is given — is sealed
        // (magic + length + digest), keyed by the target identity, installed
        // atomically, and hard-capped before it is handed over. ──────────
        // Source: meshy-dev/ThreeRT @ 0a190f83…, renderer/src/pipeline_cache.rs.
        crate::domains::rust_perf::PerfRule {
            rule_id: "driver-pipeline-cache-sealed-adapter-keyed".to_string(),
            name: "a compile-artifact cache whose consumer is a DRIVER (or any loader that need not check what it is given) is sealed — magic, declared length, and a digest of the payload BEFORE the payload — and a file that is cut short, damaged, or not ours counts as NO FILE, never a load attempt; the file is keyed by the target identity (adapter/device), installed by pid-suffixed atomic rename (multi-process last-wins with a bounded recompile loss), hard-capped (past the cap the cache starts empty, never truncated mid-read), and a save of unchanged data writes nothing".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: the compiled-pipeline cache is written and read as bare
// driver bytes under a shared name. A truncated or corrupted file
// (a crash mid-write, a full disk, another app's file in the dir) is
// handed to the driver, which "need not check what it is given" —
// the load fails obscurely or, worse, the driver trusts garbage. Two
// processes save to the same path (a torn interleaved write); every
// save rewrites the whole file even when nothing changed; a cache
// nobody clears grows forever until reads slow to a crawl.

use std::path::{Path, PathBuf};

pub struct NaivePipelineCache {
    path: PathBuf,
}

impl NaivePipelineCache {
    pub fn load(&self) -> Option<Vec<u8>> {
        std::fs::read(&self.path).ok() // bare bytes: no seal, no identity, no cap
    }

    pub fn save(&self, data: &[u8]) -> std::io::Result<()> {
        // a torn write leaves a half-file that the next load() hands over
        std::fs::write(&self.path, data)
    }
}

pub fn cache_path(dir: &Path) -> PathBuf {
    dir.join("pipeline-cache.bin") // one name: two GPUs share, two processes clobber
}"#.to_string(),
            fixed_code: r#"// FIXED: the four laws around the driver boundary.
// (1) SEAL: magic + declared length + SHA-256(payload) BEFORE the
//     payload; unseal verifies all three — anything else "counts as
//     no file".
// (2) IDENTITY: the file is named for the target (adapter key:
//     backend/vendor/device) so two targets never read each other's;
//     the payload's own header still carries the driver-version
//     identity, and a mismatch is an empty cache, not an error.
// (3) ATOMIC MULTI-PROCESS INSTALL: each save writes the whole file
//     under its own pid-counted name beside the real one, then
//     renames over it — a reader finds the old file or the new one,
//     whole, and a process stopped halfway leaves the old one. Last
//     save wins: "what the other process compiled meanwhile is
//     compiled once more, by whoever misses it" — the loss is a
//     bounded recompile, never corruption.
// (4) CAP + NO-OP SAVE: past MAX_BYTES the cache starts EMPTY
//     (never truncated mid-read); a save of data whose digest matches
//     the last one writes NOTHING.

const MAGIC: &[u8; 8] = b"KPIPE001";
const HEADER: usize = 8 + 8 + 32; // magic + len + digest
pub const MAX_BYTES: u64 = 256 << 20;

fn seal(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + data.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&digest(data));
    out.extend_from_slice(data);
    out
}

fn unseal(file: &[u8]) -> Option<&[u8]> {
    let (head, data) = file.split_at_checked(HEADER)?;
    let len = u64::from_le_bytes(head[8..16].try_into().ok()?);
    (&head[..8] == MAGIC && len == data.len() as u64 && head[16..] == digest(data)[..]).then_some(data)
}

fn digest(data: &[u8]) -> [u8; 32] {
    // stand-in for sha2::Sha256 in this fixture model; the real rule
    // pins the real digest crate.
    let mut h = [0u8; 32];
    for (k, b) in data.iter().enumerate() {
        h[k % 32] ^= b.wrapping_add(k as u8);
    }
    h
}

pub struct PipelineCache {
    path: PathBuf,
    saved: [u8; 32], // the digest of the data last read or written
}

impl PipelineCache {
    pub fn load(&self) -> Option<Vec<u8>> {
        // the cap check reads the METADATA first: past the cap the cache
        // starts empty — a huge file is never read and never truncated.
        if std::fs::metadata(&self.path).ok()?.len() > MAX_BYTES {
            return None;
        }
        let file = std::fs::read(&self.path).ok()?;
        unseal(&file).map(|d| d.to_vec()) // damaged = no file, never a load attempt
    }

    pub fn save(&self, data: &[u8]) -> std::io::Result<()> {
        let hash = digest(data);
        if hash == self.saved {
            return Ok(()); // a save of unchanged data writes nothing
        }
        let mut name = self.path.file_name().unwrap_or_default().to_os_string();
        name.push(format!(".{}.0.tmp", std::process::id())); // pid-counted own name
        let tmp = self.path.with_file_name(name);
        let done = std::fs::write(&tmp, &seal(data)).and_then(|_| std::fs::rename(&tmp, &self.path));
        if done.is_err() {
            let _ = std::fs::remove_file(&tmp); // a failed save leaves nothing
        }
        done
    }
}

pub fn cache_path(dir: &Path, adapter_key: &str) -> PathBuf {
    // (2) IDENTITY: adapter-keyed — "two GPUs never read each other's".
    dir.join(format!("{adapter_key}.bin"))
}

pub fn open(path: &Path, data_read: Option<&[u8]>) -> PipelineCache {
    PipelineCache { path: path.to_path_buf(), saved: data_read.map_or([0; 32], digest) }
}

#[test]
fn a_damaged_file_counts_as_no_file() {
    let data: Vec<u8> = (0..1000u32).flat_map(|k| k.to_le_bytes()).collect();
    let file = seal(&data);
    assert_eq!(unseal(&file), Some(&data[..]));
    assert_eq!(unseal(&seal(&[])), Some(&[][..]));
    assert_eq!(unseal(&file[..file.len() - 1]), None); // cut short
    assert_eq!(unseal(&file[..HEADER - 1]), None); // under the header
    assert_eq!(unseal(&[]), None);
    assert_eq!(unseal(&data), None); // the bare payload without the seal
    let mut flipped = file.clone();
    *flipped.last_mut().unwrap() ^= 1;
    assert_eq!(unseal(&flipped), None); // one flipped byte
    let mut longer = file.clone();
    longer.push(0);
    assert_eq!(unseal(&longer), None); // a trailing byte
    let mut other = file;
    other[0] = b'X';
    assert_eq!(unseal(&other), None); // another magic: not ours
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the win is the driver's cold compile — the source's own cite is Adreno 740 'ten' seconds per kernel variant, re-paid per variant per run without the cache; no validator hosts a driver compile; the dial9/Batch-136 annotation). ROUTING: rust_perf — a HOST-side artifact/integrity law over files; kernel_opt loses (no kernel we author changes), sec loses (this is not attacker-facing input validation — the adversary is entropy: crashes, torn writes, full disks, mixed devices; the seal exists for INTEGRITY not adversarial authentication — a hostile attacker who can rewrite the file can rewrite the digest too, and the rule never claims otherwise). SOURCE (meshy-dev/ThreeRT @ 0a190f83…, MIT, T0 byte-verified; renderer/src/pipeline_cache.rs read in full). THE LAW IN-SOURCE, verbatim: ':10-12 Its bytes are what wgpu::PipelineCache::get_data returned, sealed: a magic, the length and the data's SHA-256 before them. A file that is cut short, damaged or not ours is not handed to the driver (which need not check what it is given); it counts as no file.' — the CONSUMER-IS-A-NONVALIDATOR clause is the load-bearing framing: the driver is a trust boundary that performs no validation, so the HOST owns integrity end to end. IDENTITY, :7-9 'Its name is wgpu's key for the adapter (backend, vendor, device), so two GPUs never read each other's. Inside, wgpu's own header holds the driver's cache identity: data from another driver version is not used (fallback: an empty cache then, and the next save replaces the file)' — identity is TWO-LAYER: the filename keys the target (our half), the payload's own header keys the driver version (the loader's half); a mismatch is an EMPTY cache, never an error, and the next save replaces the file. MULTI-PROCESS, :13-16 'Two processes may share it (two games of one app, a game opened twice). A save writes the whole file under a name of its own (process id, a count) beside it and renames it over the old one: a reader finds the old file or the new one, whole, and a process stopped halfway leaves the old one. The last save wins; what the other process compiled meanwhile is compiled once more, by whoever misses it, and saved then.' — last-wins is SOUND here because the lost data is recomputable at bounded cost (a recompile, seconds); the rule names the soundness condition, not just the mechanism. CAP, :25-26 'MAX_BYTES 256 << 20 — past it the cache starts empty (every kernel variant of every game ever opened adds to it)' — the cap check reads the file's METADATA before any read; start-empty, never truncate-mid-read. NO-OP SAVE (PipelineCache::save, :126-136): 'saved — the SHA-256 of the data last read or written: a save of the same data writes nothing' + 'A failure to write is not an error: the next run compiles again.' — cache-write failures are deliberately non-fatal (the cache is an optimization; its failure mode is a recompile, and the law says so). STALE-SAVE SWEEP: saves' own pid-suffixed temps a stopped process left are removed on open past STALE = 600 s (:27-28, :68-79) — 'A save's own file that old was left by a process that stopped: removed when the cache is opened'. REGIME: any compile-artifact / driver-artifact cache (shader pipelines, kernel binaries, JIT caches, driver-blob caches) shared across runs or processes; any loader-consumer that does not validate its inputs. ANTI-REGIME: caches whose consumer validates (the seal is redundant — add only the atomic install); adversarial-threat models (this is integrity, not authentication — see the routing note); single-use artifacts (the cap and the no-op save have nothing to gate). ADJUDICATIONS (do not double-count): execution-artifact-trust-ladder-before-deserialize (sec, B345) owns the TRUST-LADDER axis — an execution artifact is trusted by its provenance tier before deserialization; THIS owns the AT-REST ARTIFACT half (seal + identity + install + cap around a NONVALIDATING consumer — a driver, not a deserializer), and the positive co-instance posture: the same ladder's at-rest arm; untrusted-download-verify-then-atomic-install (rust_perf entries_70) owns the DOWNLOAD axis — verify-before-promote of NETWORK-sourced bytes with the use-site re-verify; THIS shares the atomic-rename install shape but its intake is the process's OWN compile output (no network, no adversary — the threat is entropy), and it adds the multi-process last-wins + adapter-keyed identity + cap + no-op-save halves that law does not carry; our own index_cache (riir-refine src/index_cache.rs, issue 226) is the POSITIVE-INSTANCE adjacency: refuse-loud-and-rebuild on a digest mismatch vs THIS's count-as-no-file — both refuse to consume a damaged cache; the difference is the consumer (a retrieval index can rebuild loud; a driver hand-over fails obscure, so THIS degrades to 'no cache', silently-fast) — carried in-rule as the posture split; B324 reserve-then-commit-capacity-accounting owns the reserve/commit LEDGER inside one store — not an at-rest artifact law. FIXTURES (b362_fixtures, rust_perf half): seal/unseal (whole, empty, cut-short, under-header, flipped-byte, trailing-byte, wrong-magic, bare-data arms); adapter-keyed naming; concurrent saves leave one whole file (N threads x M saves + a reader loop, no torn read, no tmp leftovers); the stale-sweep ages; the cap refuses before read; the no-op save. Cite: @ 0a190f83… — renderer/src/pipeline_cache.rs:6-29 (the module doc, quoted above), :31-45 (seal/unseal), :52-79 (write_whole + sweep), :82-88 (load + cap), :100-136 (open/save + the saved-digest no-op). Batch 362 / Plan 373 / Research 406 (R6).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

#[test]
fn an_adapter_keyed_cache_path_names_the_target() {
    // Repair (Plan 377 landing, 2026-11-14, disclosed): the helper this arm
    // asserts lived only inside the raw-string corpus shapes above — the
    // lib-test target never compiled (the authoring session's "compiles
    // clean" ran cargo check, not --all-targets). The helper here is the
    // FIXED shape's identity law verbatim:
    // dir.join(format!("{adapter_key}.bin")).
    fn cache_path(dir: &std::path::Path, adapter_key: &str) -> std::path::PathBuf {
        dir.join(format!("{adapter_key}.bin"))
    }
    let a = cache_path(std::path::Path::new("/tmp"), "vulkan-5143-43050a01");
    let b = cache_path(std::path::Path::new("/tmp"), "vulkan-5143-43050a02");
    assert_ne!(a, b, "two GPUs never read each other's file");
    assert!(a.to_string_lossy().ends_with("vulkan-5143-43050a01.bin"));
}
