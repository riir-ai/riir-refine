//! Plan 199 T3.2 — the first bounded validator module.
//!
//! Predicate class: the P21 oracle-edits `useless_format` brace-escape guard,
//! expressed as a wasm32 module. Inlining a string literal into a
//! `format!`-family call is only safe when the covered call's span text
//! carries no braces — a brace means format arguments are in use and the
//! inline would corrupt the format string.
//!
//! Fixed ABI (host: `riir_refine::validators`):
//! - export `validate(ptr: i32, len: i32) -> i32`
//! - input bytes at linear memory `[ptr, ptr+len)` (host writes them before
//!   the call; the host passes `ABI_INPUT_ADDR` as `ptr`)
//! - verdict return: `0` = clean (inline-safe), `1` = brace present (refuse)
//! - score: brace count, little-endian i32 at `ABI_SCORE_ADDR`
//!
//! `no_std` + no allocation: the emitted module stays inside the bounded
//! i32 opcode set (loads/compares/add/branch) that `check_basic_only`
//! certifies — no floats, no i64, determinism by construction.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

/// Score cell: brace count, little-endian i32, written before returning.
const ABI_SCORE_ADDR: i32 = 512;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // A validator that panics has a bug: stop loudly inside the bounded
    // machine (the host observes it as a halted module — admission keeps
    // panicking modules out of the corpus in the first place).
    core::arch::wasm32::unreachable()
}

/// The brace-escape predicate over an input byte span.
#[no_mangle]
pub extern "C" fn validate(ptr: i32, len: i32) -> i32 {
    let mut i: i32 = 0;
    let mut braces: i32 = 0;
    while i < len {
        // SAFETY: the host bounds the input span inside the 64 KiB linear
        // memory (ABI: `ptr + len <= MEM_LEN`); `i < len` keeps every read in
        // that span.
        let b = unsafe { ((ptr + i) as *const u8).read() };
        if b == b'{' || b == b'}' {
            braces += 1;
        }
        i += 1;
    }
    // SAFETY: ABI_SCORE_ADDR + 4 <= MEM_LEN is a fixed-ABI constant.
    unsafe { (ABI_SCORE_ADDR as *mut i32).write_unaligned(braces) };
    if braces > 0 {
        1
    } else {
        0
    }
}
