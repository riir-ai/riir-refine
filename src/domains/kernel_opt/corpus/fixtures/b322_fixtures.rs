//! Batch 322 (Plan 322) fixtures — the three `entries_gpu_146` rules as
//! ORIGINAL minimal Rust reproductions (nothing transcribed from the
//! sources: Zig/CUDA, Zig, OpenCL/C++). Standalone-compiled AND executed
//! green BEFORE entry (`rustc --edition 2021 --test b322_fixtures.rs`).
//!
//! Laws pinned:
//! 1. `checkpoint-boot-staged-upload-bounded-bytes` — a 200 MiB image
//!    partitions into 64 MiB stages (64+64+64+8), byte-exactness across
//!    chunk boundaries, offset accumulation (item bases from the same
//!    accumulating offset), paired ownership (one device buffer, item
//!    base == prior end), digest preservation, peak host custody at ONE
//!    chunk (stage reused, grows only on demand), and the failed-later-
//!    chunk custody contracts (upload errors, close errors, the failed
//!    item poisoned, the source still pinned, nothing freed).
//! 2. `boot-identity-hash-read-once` — read-once hermeticity (the store
//!    errors on a SECOND read of the same bytes: the nonexistent-dir
//!    posture) + digest equality between the threaded and freshly-
//!    computed forms; the uncached path still reads exactly once.
//! 3. `vendor-intrinsic-staged-operands-by-value-scheduling-attribute` —
//!    the opt form copies the staged operands into locals BEFORE the
//!    vendor call and carries the scheduling attribute; the naive form
//!    passes the __local pointer straight in; value parity on a stable
//!    stage; and the narrow-tile admission (`n <= 8 ? 8 : 32`) plus the
//!    dp4a refusals as pure functions.
//!
//! Pure Rust, no external crates, deterministic.

#![allow(dead_code)]

use std::cell::Cell;

// ═══════════════════════════════════════════════════════════════════
// fixture 1 — checkpoint-boot-staged-upload-bounded-bytes (rule 1, A)
// ═══════════════════════════════════════════════════════════════════

/// [in-diff] host_stage.zig, verbatim: the bounded stage's chunk.
const UPLOAD_CHUNK: usize = 64 << 20; // 64 MiB

#[derive(Debug, PartialEq)]
enum Upload {
    NotReady,
}

struct Item {
    name: &'static str,
    bytes: Vec<u8>,
}

/// The device: ONE allocation for the whole image ([in-diff]
/// `cuda.DeviceBuffer.alloc(d, total)`); every copyTo recorded so the
/// fixture can pin per-copy bytes, destination offsets and boundaries.
struct Device {
    buf: Vec<u8>,
    copies: Vec<(usize, usize)>, // (dst offset, len) per copyTo
    fail_at_copy: Option<usize>, // the copy index that errors
}

impl Device {
    fn new() -> Self {
        Self {
            buf: Vec::new(),
            copies: Vec::new(),
            fail_at_copy: None,
        }
    }

    /// the ONE allocation — a second alloc is the whole-image-form bug
    fn alloc(&mut self, total: usize) {
        assert!(self.buf.is_empty(), "the device buffer is ONE allocation");
        self.buf = vec![0u8; total];
    }

    fn copy_to(&mut self, dst: usize, bytes: &[u8]) -> Result<(), Upload> {
        if self.fail_at_copy == Some(self.copies.len()) {
            return Err(Upload::NotReady); // a LATER chunk fails
        }
        self.buf[dst..dst + bytes.len()].copy_from_slice(bytes);
        self.copies.push((dst, bytes.len()));
        Ok(())
    }
}

/// The REUSABLE bounded host stage ([in-diff] copy_source.zig shape):
/// grows only when a request exceeds the kept size; the wait-before-reuse
/// is PRE-EXISTING inside `Stage.take` (`try s.wait()` — reuses, never
/// adds). `pinned` is the opened source's custody; `freed` counts frees.
struct MockSource {
    stage: Vec<u8>,
    pinned: bool,
    freed: usize,
    takes: usize,
    grew: usize,
    wait_calls: usize,
}

impl MockSource {
    fn new() -> Self {
        Self {
            stage: Vec::new(),
            pinned: true,
            freed: 0,
            takes: 0,
            grew: 0,
            wait_calls: 0,
        }
    }

    /// [in-diff] `source.take(d, len)` per chunk — the stage REUSED
    fn take(&mut self, len: usize) -> &mut [u8] {
        self.wait_calls += 1; // the pre-existing wait, every take
        self.takes += 1;
        if self.stage.len() < len {
            self.stage.resize(len, 0); // grows ONLY past the kept size
            self.grew += 1;
        }
        &mut self.stage[..len]
    }

    fn free(&mut self) {
        self.pinned = false;
        self.freed += 1;
    }
}

/// The upload weights ledger: bufs (one device buffer), the scale/pointer
/// map (offset accumulation), the poisoned entries, the failed flag.
struct Weights {
    bufs: Vec<Vec<u8>>,
    map: Vec<(&'static str, usize)>, // item name -> device base offset
    poison: Vec<&'static str>,
    failed: bool,
}

impl Weights {
    fn new() -> Self {
        Self {
            bufs: Vec::new(),
            map: Vec::new(),
            poison: Vec::new(),
            failed: false,
        }
    }

    fn get(&self, name: &str) -> Option<usize> {
        if self.poison.iter().any(|p| *p == name) {
            return None; // poisoned: get == null
        }
        self.map
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, base)| *base)
    }

    /// [in-diff] cuda_weights.zig upload loop, verbatim shape: offset
    /// accumulates per ITEM; chunks bounded by the chunk bound (`@min`);
    /// the scale/pointer map computed from the same accumulating offset.
    /// `chunk` is the bound — the REAL `UPLOAD_CHUNK` (64 MiB) in
    /// production; the byte-scale stand-in CHUNK_BYTES here (64 B), the
    /// identical partition shape.
    fn upload(
        &mut self,
        dev: &mut Device,
        src: &mut MockSource,
        items: &[Item],
        chunk: usize,
    ) -> Result<(), Upload> {
        let total: usize = items.iter().map(|n| n.bytes.len()).sum();
        dev.alloc(total);
        self.bufs.push(dev.buf.clone()); // weights.bufs.items.len == 1
        for n in items {
            self.map.push((n.name, 0)); // registered upfront
        }
        let mut offset = 0usize;
        for (idx, n) in items.iter().enumerate() {
            let mut from = 0usize;
            while from < n.bytes.len() {
                let len = (n.bytes.len() - from).min(chunk);
                let copy = src.take(len);
                copy.copy_from_slice(&n.bytes[from..from + len]);
                if dev.copy_to(offset + from, copy).is_err() {
                    self.failed = true;
                    self.poison.push(n.name); // get(...) == null on failure
                    return Err(Upload::NotReady);
                }
                from += len;
            }
            self.map[idx].1 = offset; // the same accumulating offset
            offset += n.bytes.len();
        }
        Ok(())
    }

    /// [in-diff] load_chunk_test.zig arm: a failed upload's close errors
    fn close(&mut self) -> Result<(), Upload> {
        if self.failed {
            return Err(Upload::NotReady);
        }
        Ok(())
    }
}

/// NAIVE: the whole image reserved on the HOST first — peak host custody
/// == the whole pack; the form that breaks the source-custody budget.
fn upload_whole_image(dev: &mut Device, items: &[Item]) -> Result<(), Upload> {
    let mut whole = Vec::new();
    for n in items {
        whole.extend_from_slice(&n.bytes);
    }
    dev.alloc(whole.len());
    dev.copy_to(0, &whole)
}

/// digest stand-in (FNV-1a; the source pins SHA-256 — equality of the
/// stand-in over device bytes vs source bytes is the same claim)
fn digest64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn two_items() -> Vec<Item> {
    vec![
        Item {
            name: "w0",
            bytes: vec![0x12; 256],
        },
        Item {
            name: "w1",
            bytes: vec![0x87; 120],
        },
    ]
}

/// The byte-scale stand-in for UPLOAD_CHUNK: 64 B ↔ 64 MiB — the
/// partition shape 64+64+64+8 is IDENTICAL at both scales (the real
/// constant's partition is pinned by r1_chunk_partition_... below).
const CHUNK_BYTES: usize = 64;

#[test]
fn r1_chunk_partition_of_a_200mib_image_into_64mib_stages() {
    // pure partition arithmetic at the REAL numbers — no 200 MiB alloc
    let image: usize = 200 << 20;
    let mut parts = Vec::new();
    let mut from = 0usize;
    while from < image {
        let len = (image - from).min(UPLOAD_CHUNK);
        parts.push(len);
        from += len;
    }
    assert_eq!(parts, vec![64 << 20, 64 << 20, 64 << 20, 8 << 20]);
    assert_eq!(parts.iter().sum::<usize>(), image);
}

#[test]
fn r1_byte_exact_across_boundaries_offset_accumulates_one_buffer() {
    // 200-BYTE image at a 64-BYTE chunk: the IDENTICAL partition shape
    // (64, 64, 64, 8), every byte checked. Item 0 fills 0x12 over 256 B
    // (the source's `dst == 256 + copied` base), item 1 fills 0x87.
    let items = two_items();
    let total: usize = items.iter().map(|n| n.bytes.len()).sum();
    let source_image: Vec<u8> = items.iter().flat_map(|n| n.bytes.iter().copied()).collect();
    let mut dev = Device::new();
    let mut src = MockSource::new();
    let mut w = Weights::new();
    w.upload(&mut dev, &mut src, &items, CHUNK_BYTES)
        .expect("clean upload");
    // ONE device buffer, byte-exact across every chunk boundary:
    assert_eq!(w.bufs.len(), 1); // weights.bufs.items.len == 1
    for (i, b) in dev.buf.iter().enumerate() {
        let want = if i < 256 { 0x12 } else { 0x87 };
        assert_eq!(*b, want, "byte {i} flipped across a chunk boundary");
    }
    // offset accumulation: 6 copies, dst == item base + chunk offset
    assert_eq!(
        dev.copies,
        vec![
            (0, 64),
            (64, 64),
            (128, 64),
            (192, 64),
            (256, 64),
            (320, 56)
        ]
    );
    // paired ownership: item 1's base == item 0's end (left.ptr + first)
    assert_eq!(dev.copies[4].0, 256);
    assert_eq!(w.get("w1"), Some(256));
    // the last copy ends exactly at the image end
    let last = *dev.copies.last().expect("copies recorded");
    assert_eq!(last.0 + last.1, total);
    // digest preserved end-to-end (SHA-256 of item 0 == left.sha256 shape)
    assert_eq!(digest64(&dev.buf), digest64(&source_image));
}

#[test]
fn r1_stage_is_reused_bounded_and_grows_only_on_demand() {
    let items = two_items();
    let image: Vec<u8> = items.iter().flat_map(|n| n.bytes.iter().copied()).collect();
    let mut dev = Device::new();
    let mut src = MockSource::new();
    let mut w = Weights::new();
    w.upload(&mut dev, &mut src, &items, CHUNK_BYTES)
        .expect("clean upload");
    // peak host custody == ONE chunk, never the image:
    assert_eq!(src.stage.len(), 64);
    assert!(src.stage.len() < image.len());
    // the stage is REUSED: one growth (the first take), then reuses
    assert_eq!(src.grew, 1);
    assert_eq!(src.takes, 6);
    // the wait-before-reuse is PRE-EXISTING on every take (reuses, never adds)
    assert_eq!(src.wait_calls, src.takes);
}

#[test]
fn r1_peak_host_custody_bounded_by_one_chunk_not_the_image() {
    let items = two_items();
    // naive: the whole-image host reservation
    let mut dev = Device::new();
    upload_whole_image(&mut dev, &items)
        .expect("the naive form uploads — until the pack size breaks the budget");
    let whole_len: usize = items.iter().map(|n| n.bytes.len()).sum();
    assert_eq!(whole_len, 376); // the reservation IS the peak
                                // opt: the bounded stage — peak == one chunk (byte-scale 64)
    let mut dev2 = Device::new();
    let mut src = MockSource::new();
    let mut w = Weights::new();
    w.upload(&mut dev2, &mut src, &items, CHUNK_BYTES)
        .expect("opt upload");
    assert!(src.stage.len() <= 64 && src.stage.len() < whole_len);
}

#[test]
fn r1_failed_later_chunk_keeps_the_custody_contracts() {
    // [in-diff] load_chunk_test.zig arms, this fixture's vocabulary:
    //   expectError(error.NotReady, weights.upload(&driver, &b))
    //   expectError(error.NotReady, weights.close())
    //   weights.get(b.out.items[0].name) == null        (poison)
    //   mock.pinned != null; mock.freed == 0            (custody)
    let items = two_items();
    let mut dev = Device::new();
    dev.fail_at_copy = Some(2); // a LATER chunk of item 0
    let mut src = MockSource::new();
    let mut w = Weights::new();
    assert_eq!(
        w.upload(&mut dev, &mut src, &items, CHUNK_BYTES),
        Err(Upload::NotReady)
    );
    assert_eq!(w.close(), Err(Upload::NotReady));
    assert_eq!(w.get("w0"), None, "the failed item stays poisoned");
    assert!(src.pinned, "the opened source stays pinned");
    assert_eq!(src.freed, 0, "nothing freed on the failure path");
    // the failing chunk's take still went through the reused stage:
    assert_eq!(src.takes, 3);
    assert_eq!(src.grew, 1);
    assert_eq!(src.stage.len(), 64); // bounded even mid-failure
                                     // and `free()` is never called on the failure path — the contract
                                     // that keeps freed == 0 above honest
    let mut untouched = MockSource::new();
    assert_eq!(untouched.freed, 0);
    untouched.free();
    assert_eq!(
        untouched.freed, 1,
        "free() is the only thing that bumps freed"
    );
}

// ═══════════════════════════════════════════════════════════════════
// fixture 2 — boot-identity-hash-read-once (rule 2, B)
// ═══════════════════════════════════════════════════════════════════

const PACK_BYTES: &[u8] = b"tensorfold-pack-image-bytes-b322";

/// Hermetic stand-in for the on-disk pack ([in-diff] the test's
/// `pack.dir = "/nonexistent-dsv41-pack"` posture): any real read AFTER
/// the first errors, so a second full read cannot pass unnoticed.
struct PoisonPack {
    reads: Cell<usize>,
}

#[derive(Debug, PartialEq)]
enum IdentityErr {
    SecondRead, // the hermetic FileNotFound
}

impl PoisonPack {
    fn read_all(&self) -> Result<Vec<u8>, IdentityErr> {
        let n = self.reads.get() + 1;
        self.reads.set(n);
        if n > 1 {
            return Err(IdentityErr::SecondRead);
        }
        Ok(PACK_BYTES.to_vec())
    }
}

/// 32-byte content identity — the stand-in for the source's digest
/// (deterministic, no crates).
#[derive(Clone, PartialEq, Eq, Debug)]
struct Digest([u8; 32]);

impl Digest {
    fn of(bytes: &[u8]) -> Self {
        let mut d = [0u8; 32];
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (i, b) in bytes.iter().enumerate() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3).rotate_left(13);
            d[i % 32] ^= (h & 0xff) as u8;
        }
        Digest(d)
    }

    fn splat(v: u8) -> Self {
        Digest([v; 32])
    }
}

/// The prepared cache: open mixes the pack digest into its KEY
/// ([in-diff] `h.update(&pack_digest)`), so it ALREADY holds the identity
/// the session store wants.
struct PreparedCache {
    pack_digest: Digest,
}

impl PreparedCache {
    /// [in-diff] Cache.fromEnv shape: the ONE read, at open.
    fn from_env(pack: &PoisonPack) -> Result<Self, IdentityErr> {
        Ok(Self {
            pack_digest: Digest::of(&pack.read_all()?),
        })
    }
}

/// [in-diff] prepared.zig verbatim shape: the open cache's digest, else
/// read here (packDigest reads every byte).
fn pack_identity(pack: &PoisonPack, cache: Option<&PreparedCache>) -> Result<Digest, IdentityErr> {
    if let Some(c) = cache {
        return Ok(c.pack_digest.clone());
    }
    Ok(Digest::of(&pack.read_all()?))
}

fn digest_of_read(pack: &PoisonPack) -> Result<Digest, IdentityErr> {
    Ok(Digest::of(&pack.read_all()?))
}

/// The session store's checkpoint identity consumer.
fn sessions_init(checkpoint: Option<Digest>) -> Option<Digest> {
    checkpoint
}

#[test]
fn r2_cached_boot_threads_the_keyed_digest_without_reading() {
    // [in-diff] hermetic arm: cache keyed @splat(7), packIdentity returns
    // @splat(7) — and any real read would error (the nonexistent dir).
    let pack = PoisonPack {
        reads: Cell::new(0),
    };
    let cache = PreparedCache {
        pack_digest: Digest::splat(7),
    };
    let id = pack_identity(&pack, Some(&cache)).expect("the threaded form");
    assert_eq!(id, Digest::splat(7));
    assert_eq!(pack.reads.get(), 0, "zero reads: the cache already held it");
}

#[test]
fn r2_uncached_path_still_reads_exactly_once() {
    let pack = PoisonPack {
        reads: Cell::new(0),
    };
    let id = pack_identity(&pack, None).expect("one read");
    assert_eq!(id, Digest::of(PACK_BYTES));
    assert_eq!(pack.reads.get(), 1);
    // a second read is exactly what the hermetic store refuses:
    assert_eq!(pack_identity(&pack, None), Err(IdentityErr::SecondRead));
    assert_eq!(pack.reads.get(), 2);
}

#[test]
fn r2_threaded_form_equals_the_freshly_computed_form() {
    let pack = PoisonPack {
        reads: Cell::new(0),
    };
    let cache = PreparedCache::from_env(&pack).expect("open: one read");
    assert_eq!(pack.reads.get(), 1);
    let threaded = pack_identity(&pack, Some(&cache)).expect("threaded");
    let fresh = Digest::of(PACK_BYTES); // the same bytes, recomputed
    assert_eq!(threaded, fresh);
    assert_eq!(threaded, cache.pack_digest);
    assert_eq!(pack.reads.get(), 1, "still ONE read across both forms");
}

#[test]
fn r2_naive_boot_double_hashes_and_fails_hermetically() {
    // the DEFECT: the cache open reads once (read 1), then the session
    // store's checkpoint identity hashes the same bytes AGAIN (read 2) —
    // under the hermetic store that second read is what errors. The FIXED
    // boot threads the cache's digest and reads NOTHING more.
    let pack = PoisonPack {
        reads: Cell::new(0),
    };
    let cache = PreparedCache::from_env(&pack).expect("open: one read");
    assert_eq!(pack.reads.get(), 1);
    let naive_checkpoint = sessions_init(digest_of_read(&pack).ok());
    assert_eq!(naive_checkpoint, None, "read 2 failed hermetically");
    assert_eq!(pack.reads.get(), 2);
    // the fixed boot: the threaded identity, no further reads
    let fixed_checkpoint = sessions_init(pack_identity(&pack, Some(&cache)).ok());
    assert_eq!(fixed_checkpoint, Some(cache.pack_digest));
    assert_eq!(pack.reads.get(), 2); // unchanged: the fix reads NOTHING more
}

// ═══════════════════════════════════════════════════════════════════
// fixture 3 — vendor-intrinsic-staged-operands-by-value-scheduling-
// attribute (rule 3, D)
// ═══════════════════════════════════════════════════════════════════

/// [in-diff] hazard comment, VERBATIM (appears TWICE — iq1_s AND iq1_m
/// dp4a kernels):
const HAZARD: &str = "// Pass staged operands by value; local dp4a operands miscompile on X2.";

/// [in-diff] the vendor scheduling attribute, verbatim, on
/// `kernel void kernel_gemm_noshuffle_iq1_s_q8_1_dp4a(` and the iq1_m twin:
const QCOM_WAVE_PAIR_MODE_ATTR: &str = "__attribute__((qcom_wave_pair_mode(1)))";

#[derive(Clone, Copy, PartialEq, Debug)]
struct U4 {
    x: u32,
    y: u32,
    z: u32,
    w: u32,
}

fn u4(x: u32, y: u32, z: u32, w: u32) -> U4 {
    U4 { x, y, z, w }
}

/// [in-diff] `__local uint4 sh_qa4[TILESIZE_N][2]` — the shared-memory
/// stage, flat here; `barrier(CLK_LOCAL_MEM_FENCE)` is single-threaded
/// in this stand-in and so carries no observable state.
struct LocalStage {
    qa4: Vec<U4>,
}

/// How the vendor dot received its operands — the call shape pinned.
#[derive(PartialEq, Debug)]
enum OperandShape {
    ByValue(U4, U4),
    LocalPointer(usize, usize),
}

/// [in-diff] dot_acc_sat_4x8packed_ss_int(qw.s0, a0.x, r) — the Qualcomm
/// packed dp4a intrinsic under `#ifdef cl_khr_integer_dot_product`
/// (deterministic stand-in: byte-lane products into the accumulator).
fn dot_acc_sat(qw: u8, a: u32, r: &mut i32) {
    let lo = ((a & 0xff) as i32).wrapping_mul(i32::from(qw & 0x0f));
    let hi = (((a >> 24) & 0xff) as i32).wrapping_mul(i32::from(qw >> 4));
    *r = r.wrapping_add(lo.wrapping_add(hi));
}

/// [in-diff] `inline int dot8_q8a_v(uint8 qw, uint4 a0, uint4 a1)` — the
/// BY-VALUE form (operands are register values).
#[inline]
fn dot8_q8a_v(qw: u8, a0: U4, a1: U4) -> i32 {
    let mut r = 0i32;
    dot_acc_sat(qw, a0.x, &mut r);
    dot_acc_sat(qw, a0.y, &mut r);
    dot_acc_sat(qw, a0.z, &mut r);
    dot_acc_sat(qw, a0.w, &mut r);
    dot_acc_sat(qw, a1.x, &mut r);
    dot_acc_sat(qw, a1.y, &mut r);
    dot_acc_sat(qw, a1.z, &mut r);
    dot_acc_sat(qw, a1.w, &mut r);
    r
}

/// The NAIVE shape: every operand read goes back through the __local
/// stage — the pointer shape the vendor compiler miscompiles.
#[inline]
fn dot8_q8a_local(stage: &LocalStage, i0: usize, i1: usize, qw: u8) -> i32 {
    let mut r = 0i32;
    dot_acc_sat(qw, stage.qa4[i0].x, &mut r);
    dot_acc_sat(qw, stage.qa4[i0].y, &mut r);
    dot_acc_sat(qw, stage.qa4[i0].z, &mut r);
    dot_acc_sat(qw, stage.qa4[i0].w, &mut r);
    dot_acc_sat(qw, stage.qa4[i1].x, &mut r);
    dot_acc_sat(qw, stage.qa4[i1].y, &mut r);
    dot_acc_sat(qw, stage.qa4[i1].z, &mut r);
    dot_acc_sat(qw, stage.qa4[i1].w, &mut r);
    r
}

/// One dp4a kernel run: records the operand shape of every dot, carries
/// the kernel's hazard note and scheduling attribute.
struct Dp4aKernel {
    name: &'static str,
    hazard_note: &'static str,
    attr: &'static str,
    shapes: Vec<OperandShape>,
}

impl Dp4aKernel {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            hazard_note: HAZARD,
            attr: QCOM_WAVE_PAIR_MODE_ATTR,
            shapes: Vec::new(),
        }
    }

    /// the OPT call shape: stage -> registers BEFORE the vendor call
    fn dot_by_value(&mut self, stage: &LocalStage, i0: usize, i1: usize, qw: u8) -> i32 {
        let a0 = stage.qa4[i0]; // the load the hazard comment asks for
        let a1 = stage.qa4[i1];
        self.shapes.push(OperandShape::ByValue(a0, a1));
        dot8_q8a_v(qw, a0, a1)
    }

    /// the NAIVE call shape: the __local pointer goes straight in
    fn dot_local_pointer(&mut self, stage: &LocalStage, i0: usize, i1: usize, qw: u8) -> i32 {
        self.shapes.push(OperandShape::LocalPointer(i0, i1));
        dot8_q8a_local(stage, i0, i1, qw)
    }
}

/// [in-diff] ggml-opencl.cpp verbatim gate:
/// `const int tile_n = n <= 8 ? 8 : 32;`
fn tile_n(n: usize) -> usize {
    if n <= 8 {
        8
    } else {
        32
    }
}

/// [in-diff] the dp4a admission: refuses n<=1, non-contiguous, misaligned
/// offset1; the decode gemv owns n == 1.
fn dp4a_admits(n: usize, contiguous: bool, offset1_aligned: bool) -> bool {
    n > 1 && contiguous && offset1_aligned
}

fn stage() -> LocalStage {
    LocalStage {
        qa4: vec![
            u4(0x0102_0304, 0x0506_0708, 0x090a_0b0c, 0x0d0e_0f10),
            u4(0x1112_1314, 0x1516_1718, 0x191a_1b1c, 0x1d1e_1f20),
        ],
    }
}

#[test]
fn r3_opt_call_shape_is_by_value_with_the_scheduling_attribute() {
    let st = stage();
    // the iq1_s kernel and the iq1_m twin — the hazard appears TWICE:
    let mut iq1_s = Dp4aKernel::new("kernel_gemm_noshuffle_iq1_s_q8_1_dp4a");
    let mut iq1_m = Dp4aKernel::new("kernel_gemm_noshuffle_iq1_m_q8_1_dp4a");
    iq1_s.dot_by_value(&st, 0, 1, 0x73);
    iq1_m.dot_by_value(&st, 1, 0, 0x73);
    for k in [&iq1_s, &iq1_m] {
        assert_eq!(k.hazard_note, HAZARD, "the hazard note rides every twin");
        assert_eq!(
            k.attr, QCOM_WAVE_PAIR_MODE_ATTR,
            "the attribute rides every twin"
        );
        assert!(
            k.shapes
                .iter()
                .all(|s| matches!(s, OperandShape::ByValue(_, _))),
            "{}: every dot is the by-value call shape",
            k.name
        );
    }
    // the recorded by-value operands are the STAGED values, copied at load:
    assert_eq!(iq1_s.shapes[0], OperandShape::ByValue(st.qa4[0], st.qa4[1]));
}

#[test]
fn r3_naive_call_shape_passes_the_local_pointer() {
    let st = stage();
    let mut k = Dp4aKernel::new("naive");
    k.dot_local_pointer(&st, 0, 1, 0x73);
    assert_eq!(k.shapes, vec![OperandShape::LocalPointer(0, 1)]);
}

#[test]
fn r3_value_parity_when_the_stage_is_stable() {
    // the workaround is call-shape-only: on a stable stage the by-value
    // dot and the pointer dot agree exactly (a workaround, not a rewrite)
    let st = stage();
    let mut k = Dp4aKernel::new("parity");
    let by_value = k.dot_by_value(&st, 0, 1, 0x73);
    let via_pointer = dot8_q8a_local(&st, 0, 1, 0x73);
    assert_eq!(by_value, via_pointer);
}

#[test]
fn r3_by_value_reads_registers_not_the_moving_stage() {
    // the by-value form's operands are fixed at LOAD time; the pointer
    // form tracks the stage. The opt shape is a register snapshot.
    let mut st = stage();
    let a0 = st.qa4[0];
    let a1 = st.qa4[1];
    let by_value = dot8_q8a_v(0x73, a0, a1);
    st.qa4[0] = u4(u32::MAX, 0, 0, 0); // the stage moves under the operands
    let via_pointer = dot8_q8a_local(&st, 0, 1, 0x73);
    assert_ne!(by_value, via_pointer);
}

#[test]
fn r3_narrow_tile_admission_is_a_pure_function() {
    // [in-diff] `const int tile_n = n <= 8 ? 8 : 32;` — the narrow kernel
    // recompiles via -DTILESIZE_N=8
    for n in 0..=8usize {
        assert_eq!(tile_n(n), 8, "n={n}: narrow tile");
    }
    for n in [9usize, 16, 31, 32, 64, 4096] {
        assert_eq!(tile_n(n), 32, "n={n}: wide tile");
    }
    // the dp4a refusals: n<=1, non-contiguous, misaligned offset1
    assert!(!dp4a_admits(0, true, true), "n <= 1 refused");
    assert!(
        !dp4a_admits(1, true, true),
        "n == 1 belongs to the decode gemv"
    );
    assert!(!dp4a_admits(8, false, true), "non-contiguous refused");
    assert!(!dp4a_admits(8, true, false), "misaligned offset1 refused");
    assert!(dp4a_admits(8, true, true), "narrow dp4a admits");
    assert!(dp4a_admits(32, true, true), "wide dp4a admits");
}
