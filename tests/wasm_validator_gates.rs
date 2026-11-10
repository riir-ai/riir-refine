//! Plan 199 T3.2 — the wasm validator lane's standing gates.
//!
//! The whole-file `#![cfg]` protects the count (713-T3); the
//! `required-features` row in Cargo.toml protects the reader. Gates:
//! - committed artifact: admission + verdict/score fixture family + rerun
//!   determinism (shared-`Vm` posture),
//! - boundedness: fuel exhaustion on a non-terminating module,
//! - memory safety: out-of-bounds store traps,
//! - admission refusals: host imports, non-basic opcodes, wrong ABI
//!   signature.
//!
//! The hand-built modules below are minimal MVP binaries assembled from the
//! same decoder constants the admission gate certifies against — small
//! enough to read, exact enough to pin each refusal class.

#![cfg(feature = "wasm_validator")]

use katgpt_percepta::wasm::{OP_BR, OP_DROP, OP_END, OP_I32_CONST, OP_I32_STORE, OP_LOOP};
use riir_refine::validators::{AdmissionError, DEFAULT_FUEL, Outcome, Trap, ValidatorModule, Vm};

/// `f32.const` — deliberately NOT in the decoder's re-export list (the lane
/// never executes floats); the admission test uses the raw opcode byte.
const OP_F32_CONST_RAW: u8 = 0x43;

const ARTIFACT: &str = "assets/validators/useless_format_brace.wasm";

fn committed_artifact() -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARTIFACT);
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("committed artifact missing at {}: {e}", path.display()))
}

// ── minimal MVP module encoder (hand-built fixtures) ─────────────────────

fn uleb(mut n: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let b = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
    out
}

fn section(id: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![id];
    out.extend(uleb(payload.len() as u64));
    out.extend_from_slice(payload);
    out
}

fn name_bytes(s: &str) -> Vec<u8> {
    let mut out = uleb(s.len() as u64);
    out.extend_from_slice(s.as_bytes());
    out
}

/// `t0 = (i32, i32) -> i32`, `t1 = () -> i32` — both present so a fixture
/// can point `validate` at the wrong one.
fn type_section() -> Vec<u8> {
    let mut payload = uleb(2);
    payload.extend_from_slice(&[0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F]); // t0
    payload.extend_from_slice(&[0x60, 0x00, 0x01, 0x7F]); // t1
    section(1, &payload)
}

fn func_section(type_indices: &[u32]) -> Vec<u8> {
    let mut payload = uleb(type_indices.len() as u64);
    for i in type_indices {
        payload.extend(uleb(*i as u64));
    }
    section(3, &payload)
}

fn export_validate(type_space_index: u32) -> Vec<u8> {
    let mut payload = uleb(1);
    payload.extend_from_slice(&name_bytes("validate"));
    payload.push(0x00); // func export
    payload.extend(uleb(type_space_index as u64));
    section(7, &payload)
}

fn code_section(bodies: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = uleb(bodies.len() as u64);
    for body in bodies {
        let mut entry = uleb(0); // no local groups
        entry.extend_from_slice(body);
        entry.push(OP_END);
        payload.extend(uleb(entry.len() as u64));
        payload.extend_from_slice(&entry);
    }
    section(10, &payload)
}

fn module_bytes(sections: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    for s in sections {
        out.extend_from_slice(s);
    }
    out
}

// ── gates ────────────────────────────────────────────────────────────────

#[test]
fn committed_artifact_admits_and_rules_the_fixture_family() {
    let bytes = committed_artifact();
    let module = ValidatorModule::admit(&bytes).expect("committed artifact must admit");

    // Fixture family: every class the validator claims to catch, plus the
    // clean class it must pass. (The T3.4 false-pass floor is the full
    // gate; this is the landing smoke over the same classes.)
    let clean: &[&[u8]] = &[
        b"format!(\"hello\")",
        b"x.to_string()",
        b"write!(f, \"literal only\")",
        b"",
    ];
    for span in clean {
        let out = module
            .run_once(span, DEFAULT_FUEL)
            .unwrap_or_else(|e| panic!("clean span {span:?} trapped: {e:?}"));
        assert_eq!(out.verdict, 0, "clean span {span:?} must pass");
        assert_eq!(out.score, 0, "clean span {span:?} must score 0");
    }

    let braced: &[(&[u8], u32)] = &[
        (b"format!(\"{}\", x)", 2),
        (b"write!(f, \"{:?}\")", 2),
        (b"let x = a { b };", 2),
        (b"{", 1),
        (b"}", 1),
    ];
    for (span, want) in braced {
        let out = module
            .run_once(span, DEFAULT_FUEL)
            .unwrap_or_else(|e| panic!("braced span {span:?} trapped: {e:?}"));
        assert_eq!(out.verdict, 1, "braced span {span:?} must be refused");
        assert_eq!(out.score, *want, "brace count for {span:?}");
    }
}

#[test]
fn reruns_are_deterministic_and_vm_reuse_is_stable() {
    let module = ValidatorModule::admit(&committed_artifact()).expect("admit");
    let mut vm = Vm::default();
    let first = module
        .run(b"format!(\"{}\", x)", DEFAULT_FUEL, &mut vm)
        .unwrap();
    for _ in 0..3 {
        let again = module
            .run(b"format!(\"{}\", x)", DEFAULT_FUEL, &mut vm)
            .unwrap();
        assert_eq!(again, first, "rerun must be byte-identical (shared Vm)");
    }
    let once = module
        .run_once(b"format!(\"{}\", x)", DEFAULT_FUEL)
        .unwrap();
    assert_eq!(once, first);
}

#[test]
fn non_terminating_module_exhausts_fuel() {
    // validate(ptr, len) { block { loop { br 0 } } const 0 } — the br
    // re-enters the loop forever; the step budget is the only exit.
    let body = [
        vec![0x02, 0x40],    // block void
        vec![OP_LOOP, 0x40], // loop void
        vec![OP_BR, 0x00],   // br 0 → loop label, forever
        vec![OP_END],        // end loop
        vec![OP_END],        // end block
        vec![OP_I32_CONST, 0x00],
    ]
    .concat();
    let bytes = module_bytes(&[
        type_section(),
        func_section(&[0]),
        export_validate(0),
        code_section(&[body]),
    ]);
    let module = ValidatorModule::admit(&bytes).expect("loop module admits");
    let err = module
        .run_once(b"", 1_000)
        .expect_err("infinite loop must exhaust fuel");
    assert_eq!(err, Trap::OutOfFuel);
}

#[test]
fn out_of_bounds_store_traps() {
    // validate(ptr, len) { i32.const 0; i32.const 1; i32.store offset=70000; const 0 }
    // — the effective address 0+70000 leaves the 64 KiB machine memory.
    let body = [
        vec![OP_I32_CONST, 0x00],
        vec![OP_I32_CONST, 0x01],
        {
            let mut v = vec![OP_I32_STORE];
            v.extend(uleb(2)); // align
            v.extend(uleb(70_000)); // offset
            v
        },
        vec![OP_I32_CONST, 0x00],
    ]
    .concat();
    let bytes = module_bytes(&[
        type_section(),
        func_section(&[0]),
        export_validate(0),
        code_section(&[body]),
    ]);
    let module = ValidatorModule::admit(&bytes).expect("oob module admits");
    let err = module
        .run_once(b"", DEFAULT_FUEL)
        .expect_err("oob store must trap");
    assert_eq!(
        err,
        Trap::MemoryOob {
            addr: 70_000,
            access: 4
        }
    );
}

#[test]
fn host_imports_are_refused() {
    let mut imports = uleb(1);
    imports.extend_from_slice(&name_bytes("env"));
    imports.extend_from_slice(&name_bytes("host_fn"));
    imports.push(0x00); // func import
    imports.extend(uleb(0)); // type 0
    let bytes = module_bytes(&[type_section(), section(2, &imports)]);
    let err = ValidatorModule::admit(&bytes).expect_err("imports must refuse");
    assert_eq!(err, AdmissionError::ImportsPresent(1));
}

#[test]
fn non_basic_opcodes_are_refused_after_lowering() {
    // f32.const is neither basic nor lowerable — check_basic_only must name it.
    let body = [
        {
            let mut v = vec![OP_F32_CONST_RAW];
            v.extend_from_slice(&0.0f32.to_le_bytes());
            v
        },
        vec![OP_DROP],
        vec![OP_I32_CONST, 0x00],
    ]
    .concat();
    let bytes = module_bytes(&[
        type_section(),
        func_section(&[0]),
        export_validate(0),
        code_section(&[body]),
    ]);
    let err = ValidatorModule::admit(&bytes).expect_err("f32.const must refuse");
    match err {
        AdmissionError::UnsupportedOps(bad) => {
            // The refusal class is the gate; the NAME may bucket to
            // "unknown" — check_basic_only only names tabled opcodes and
            // f32.const is outside the table (its own doc). Count 1 pins
            // the single offending instruction.
            assert_eq!(bad.len(), 1, "one offending op: {bad:?}");
            assert_eq!(bad[0].1, 1);
        }
        other => panic!("want UnsupportedOps, got {other:?}"),
    }
}

#[test]
fn wrong_abi_signature_is_refused() {
    // validate exported as `() -> i32` (type 1), not the fixed
    // `(i32, i32) -> i32`.
    let body = vec![OP_I32_CONST, 0x00];
    let bytes = module_bytes(&[
        type_section(),
        func_section(&[1]),
        export_validate(0),
        code_section(&[body]),
    ]);
    let err = ValidatorModule::admit(&bytes).expect_err("wrong signature must refuse");
    assert_eq!(err, AdmissionError::AbiSignature);
}

#[test]
fn score_cell_reads_little_endian_at_the_abi_address() {
    let module = ValidatorModule::admit(&committed_artifact()).expect("admit");
    let out: Outcome = module
        .run_once(b"{{{}}}", DEFAULT_FUEL)
        .expect("braced span runs");
    assert_eq!(out.verdict, 1);
    // 3 open + 3 close braces.
    assert_eq!(out.score, 6);
}
