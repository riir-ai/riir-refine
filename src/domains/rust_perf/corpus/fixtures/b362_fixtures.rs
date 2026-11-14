//! Batch 362 fixtures — the rust_perf half (Plan 373 / Research 406 R6):
//! the ThreeRT sealed driver pipeline-cache law. ORIGINAL std-only model,
//! never pasted source. Standalone-compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b362_fixtures.rs && ./b362_fixtures`
//! (the B353/B359/B360/B361 pre-entry discipline). The kernel_opt half of
//! this batch lives in `src/domains/kernel_opt/corpus/fixtures/b362_fixtures.rs`.
//!
//! Arms (per the rule's FIXTURES note):
//! - seal/unseal: whole, empty, cut-short, under-header, flipped-byte,
//!   trailing-byte, wrong-magic, bare-data — a damaged file counts as NO
//!   file, never a load attempt;
//! - adapter-keyed naming: two targets never read each other's file;
//! - concurrent saves: N threads × M saves + a reader loop — a reader finds
//!   the old file or the new one, WHOLE, and no tmp litter remains;
//! - the stale sweep ages: an orphaned pid-suffixed tmp past STALE is
//!   removed on open, a fresh one survives;
//! - the cap refuses BEFORE read: past MAX_BYTES the cache starts empty,
//!   never truncated mid-read;
//! - the no-op save: a save of unchanged data writes nothing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"KPIPE001";
const HEADER: usize = 8 + 8 + 32; // magic + len + digest

/// stand-in for sha2::Sha256 in this fixture model; the real rule pins the
/// real digest crate.
fn digest(data: &[u8]) -> [u8; 32] {
    let mut h = [0u8; 32];
    for (k, b) in data.iter().enumerate() {
        h[k % 32] ^= b.wrapping_add(k as u8);
    }
    h
}

/// (1) SEAL: magic + declared length + digest BEFORE the payload.
fn seal(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + data.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&digest(data));
    out.extend_from_slice(data);
    out
}

/// unseal verifies all three — anything else counts as NO FILE.
fn unseal(file: &[u8]) -> Option<&[u8]> {
    let (head, data) = file.split_at_checked(HEADER)?;
    let len = u64::from_le_bytes(head[8..16].try_into().ok()?);
    (&head[..8] == MAGIC && len == data.len() as u64 && head[16..] == digest(data)[..])
        .then_some(data)
}

/// (2) IDENTITY: the file is named for the target — two GPUs never read
/// each other's.
fn cache_path(dir: &Path, adapter_key: &str) -> PathBuf {
    dir.join(format!("{adapter_key}.bin"))
}

/// (3) ATOMIC MULTI-PROCESS INSTALL: each save writes the whole file under
/// its own pid-counted name beside the real one, then renames over it —
/// a reader finds the old file or the new one, whole; a failed save leaves
/// nothing.
fn save_atomic(path: &Path, sealed: &[u8], worker: u32) {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.{}.tmp", std::process::id(), worker));
    let tmp = path.with_file_name(name);
    let done = std::fs::write(&tmp, sealed).and_then(|_| std::fs::rename(&tmp, path));
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
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
}

#[test]
fn an_adapter_keyed_cache_path_names_the_target() {
    let a = cache_path(Path::new("/tmp"), "vulkan-5143-43050a01");
    let b = cache_path(Path::new("/tmp"), "vulkan-5143-43050a02");
    assert_ne!(a, b, "two GPUs never read each other's file");
    assert!(a.to_string_lossy().ends_with("vulkan-5143-43050a01.bin"));
}

#[test]
fn concurrent_saves_leave_one_whole_file_and_no_litter() {
    let dir = std::env::temp_dir().join(format!("b362_cache_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = Arc::new(cache_path(&dir, "test-adapter"));
    let mut handles = Vec::new();
    for t in 0..4u32 {
        let path = Arc::clone(&path);
        handles.push(std::thread::spawn(move || {
            for m in 0..25u32 {
                let data = format!("worker {t} save {m}").into_bytes();
                save_atomic(&path, &seal(&data), t);
            }
        }));
    }
    // a reader loop alongside: every read sees a WHOLE sealed file or nothing
    let mut reads = 0usize;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);
    while std::time::Instant::now() < deadline {
        if let Ok(bytes) = std::fs::read(&*path) {
            reads += 1;
            assert!(unseal(&bytes).is_some(), "a reader saw a torn file");
        }
    }
    for h in handles {
        h.join().unwrap();
    }
    assert!(reads > 0);
    // the final file is whole; no tmp litter remains
    let final_bytes = std::fs::read(&*path).unwrap();
    assert!(unseal(&final_bytes).is_some());
    let litter: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains(".tmp"))
        .collect();
    assert!(litter.is_empty(), "tmp litter: {litter:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// the sweep predicate: a save's own pid-suffixed file older than STALE was
/// left by a process that stopped — removed when the cache is opened.
fn sweep_stale(dir: &Path, now: std::time::SystemTime, stale_ms: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.contains(".tmp") {
            continue;
        }
        let Ok(meta) = e.metadata() else {
            continue;
        };
        let Ok(age) = now.duration_since(meta.modified().unwrap_or(now)) else {
            continue;
        };
        if age.as_millis() as u64 >= stale_ms {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[test]
fn the_stale_sweep_ages_a_stopped_process_litter() {
    let dir = std::env::temp_dir().join(format!("b362_stale_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = cache_path(&dir, "test-adapter");
    // a save's own file a STOPPED process left: a pid-suffixed tmp beside the cache
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.0.tmp", 999_999)); // a pid nobody owns any more
    let litter = path.with_file_name(name);
    std::fs::write(&litter, b"orphan").unwrap();
    let now = std::time::SystemTime::now();
    sweep_stale(&dir, now, 0); // the aged posture: any age at all is past the window
    assert!(!litter.exists(), "the orphan was removed");
    // a FRESH tmp within the window survives
    std::fs::write(&litter, b"fresh").unwrap();
    sweep_stale(&dir, now, 60_000); // a one-minute window
    assert!(litter.exists(), "a fresh tmp is not litter");
    let _ = std::fs::remove_dir_all(&dir);
}

/// the cap check reads the METADATA first: past the cap the cache starts
/// empty — a huge file is never read and never truncated. `reads` counts
/// real reads so the test can pin the BEFORE-any-read ordering.
fn load_capped(path: &Path, max_bytes: u64, reads: &mut usize) -> Option<Vec<u8>> {
    if std::fs::metadata(path).ok()?.len() > max_bytes {
        return None;
    }
    *reads += 1;
    let file = std::fs::read(path).ok()?;
    unseal(&file).map(|d| d.to_vec()) // damaged = no file, never a load attempt
}

#[test]
fn the_cap_refuses_before_any_read() {
    let dir = std::env::temp_dir().join(format!("b362_cap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = cache_path(&dir, "test-adapter");
    std::fs::write(&path, vec![0u8; 4096]).unwrap();
    let cap = 1024u64; // the model's MAX_BYTES, small for the test
    let mut reads = 0usize;
    let out = load_capped(&path, cap, &mut reads);
    assert_eq!(out, None, "past the cap the cache starts empty");
    assert_eq!(
        reads, 0,
        "the metadata check refused BEFORE any read — never truncated mid-read"
    );
    // under the cap: it reads
    std::fs::write(&path, seal(b"small")).unwrap();
    let out = load_capped(&path, cap, &mut reads);
    assert_eq!(out, Some(b"small".to_vec()));
    assert_eq!(reads, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// (4) NO-OP SAVE: `saved` carries the digest of the data last read or
/// written — a save of the same data writes nothing. The write count is
/// OWNED by the cache so the test can pin the no-op without a borrow.
struct PipelineCache {
    path: PathBuf,
    saved: Option<[u8; 32]>,
    writes: usize,
}

impl PipelineCache {
    fn save(&mut self, data: &[u8]) -> std::io::Result<()> {
        let hash = digest(data);
        if Some(hash) == self.saved {
            return Ok(()); // a save of the same data writes nothing
        }
        self.writes += 1;
        save_atomic(&self.path, &seal(data), 0);
        self.saved = Some(hash);
        Ok(())
    }
}

#[test]
fn a_save_of_unchanged_data_writes_nothing() {
    let dir = std::env::temp_dir().join(format!("b362_noop_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = cache_path(&dir, "test-adapter");
    let mut cache = PipelineCache {
        path: path.clone(),
        saved: None,
        writes: 0,
    };
    cache.save(b"payload").unwrap();
    assert_eq!(cache.writes, 1, "the first save writes");
    cache.save(b"payload").unwrap(); // the same digest: writes nothing
    assert_eq!(cache.writes, 1, "a save of the same data writes nothing");
    cache.save(b"changed").unwrap();
    assert_eq!(cache.writes, 2, "the changed payload wrote once more");
    let _ = std::fs::remove_dir_all(&dir);
}
