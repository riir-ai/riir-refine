//! The WASM validator lane (Plan 199 Phase 3): bounded fix-validators as
//! committed wasm32 modules, executed in-process at µs class as a pre-oracle
//! gate.
//!
//! Pipeline per module: `admit` (decode via the percepta decoder → lower i64
//! and hard ops to the basic set → `check_basic_only` → structural scan →
//! data-layout + ABI checks) → `run` (the fuel-bounded interpreter in
//! [`interp`]). Determinism by construction: the executed opcode set has no
//! floats, the machine is integer-only with a fixed memory layout, and the
//! same bytes produce the same outcome on every host.
//!
//! Threat model: the module is OUR authored, BLAKE3-committed artifact; the
//! INPUT is untrusted. Boundedness (fuel, call/stack/label caps, bounds-
//! checked memory) defends against hostile inputs and runaway modules; the
//! admission gate guarantees honest semantics for what we chose to run.

// Admission arithmetic casts between decoder widths (usize counts, u32
// offsets, i64 segment bounds). Every cast feeds a bound check in the same
// expression — the value is validated where it is consumed.
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]

pub mod interp;
pub mod store;

pub use interp::{
    Outcome, Trap, Vm, ABI_FUNC, ABI_INPUT_ADDR, ABI_SCORE_ADDR, DEFAULT_FUEL, MEM_LEN,
};
pub use store::{
    advantage_certified, withdrawal_excludes, AdmittedValidator, StoreError, ValidatorManifest,
    ValidatorRecord, ValidatorState, ValidatorStore, MANIFEST_FILE,
};

use katgpt_percepta::wasm::{
    check_basic_only, decode, lower_hard_ops, lower_i64_ops, DataSegment, DecodeError, ExportKind,
    ImportKind, WasmInstr, OP_BLOCK, OP_ELSE, OP_END, OP_IF, OP_LOOP, VALTYPE_I32,
};
use std::collections::HashMap;

use interp::{Ctrl, MAX_LOCALS, MAX_PARAMS};

/// Every way a module can fail admission — the lane's honest-refusal surface.
/// An artifact that cannot be certified is never executed, and the reason is
/// named (the audit is the record, not a silent skip).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// The percepta MVP decoder refused the bytes.
    Decode(String),
    /// Host imports present — the fixed ABI is bytes-in/verdict-out, no
    /// host functions.
    ImportsPresent(usize),
    /// No `validate` function export.
    MissingExport,
    /// The `validate` export names a function index outside the declared
    /// function space.
    ExportOob(u32),
    /// `validate` is not `(i32, i32) -> i32`.
    AbiSignature,
    /// No declared functions.
    NoFunctions,
    /// A function signature (or global) uses a non-i32 value type.
    TypeUnsupported(u8),
    /// A structured block/loop/if carries a blocktype other than void or
    /// `(result i32)`.
    BlockTypeUnsupported(i64),
    /// After lowering, non-basic opcodes remain.
    UnsupportedOps(Vec<(&'static str, usize)>),
    /// The structured control flow does not close (unbalanced
    /// block/loop/if/else/end).
    MalformedControl,
    /// A data segment would land below address 0.
    DataOffsetNegative(i32),
    /// A data segment extends past the linear memory.
    DataTooLarge { offset: u32, len: u32 },
    /// A data segment intersects the ABI score cell or input region.
    DataOverlap { offset: u32, len: u32 },
    /// A function's total locals exceed the machine cap.
    LocalsCap { function: usize, locals: u32 },
    /// A function takes more params than the machine cap.
    TooManyParams { function: usize, params: usize },
}

impl std::fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdmissionError::Decode(e) => write!(f, "wasm decode failed: {e}"),
            AdmissionError::ImportsPresent(n) => {
                write!(f, "module carries {n} host imports (fixed ABI refuses all)")
            }
            AdmissionError::MissingExport => write!(f, "no `{ABI_FUNC}` function export"),
            AdmissionError::ExportOob(i) => write!(
                f,
                "`{ABI_FUNC}` export names function {i} outside the declared space"
            ),
            AdmissionError::AbiSignature => {
                write!(f, "`{ABI_FUNC}` is not `(i32, i32) -> i32`")
            }
            AdmissionError::NoFunctions => write!(f, "module declares no functions"),
            AdmissionError::TypeUnsupported(t) => write!(f, "non-i32 value type 0x{t:02x}"),
            AdmissionError::BlockTypeUnsupported(b) => {
                write!(f, "blocktype 0x{b:02x} unsupported (void/i32 only)")
            }
            AdmissionError::UnsupportedOps(bad) => {
                write!(f, "non-basic ops remain after lowering: {bad:?}")
            }
            AdmissionError::MalformedControl => {
                write!(f, "structured control flow does not close")
            }
            AdmissionError::DataOffsetNegative(o) => {
                write!(f, "data segment at negative offset {o}")
            }
            AdmissionError::DataTooLarge { offset, len } => {
                write!(
                    f,
                    "data segment [{offset}, {}) exceeds {MEM_LEN} bytes",
                    offset + len
                )
            }
            AdmissionError::DataOverlap { offset, len } => write!(
                f,
                "data segment [{offset}, {}) intersects the ABI score cell or input region",
                offset + len
            ),
            AdmissionError::LocalsCap { function, locals } => {
                write!(
                    f,
                    "function {function} has {locals} locals (cap {MAX_LOCALS})"
                )
            }
            AdmissionError::TooManyParams { function, params } => {
                write!(
                    f,
                    "function {function} takes {params} params (cap {MAX_PARAMS})"
                )
            }
        }
    }
}

impl From<DecodeError> for AdmissionError {
    fn from(e: DecodeError) -> Self {
        AdmissionError::Decode(format!("{e:?}"))
    }
}

/// A certified, ready-to-run validator module.
///
/// Built once per artifact via [`ValidatorModule::admit`]; reusable across
/// runs (share a [`Vm`] to keep reruns allocation-free).
#[derive(Debug)]
pub struct ValidatorModule {
    pub(crate) functions: Vec<PreparedFunc>,
    pub(crate) global_inits: Vec<i32>,
    pub(crate) global_mutable: Vec<bool>,
    pub(crate) data_segments: Vec<(u32, Vec<u8>)>,
    pub(crate) validate_idx: usize,
}

/// One lowered, structurally-scanned function body.
#[derive(Debug)]
pub struct PreparedFunc {
    pub(crate) instrs: Vec<WasmInstr>,
    pub(crate) ctrls: Vec<Ctrl>,
    pub(crate) num_params: usize,
    pub(crate) num_locals: usize,
    pub(crate) result_arity: usize,
}

impl ValidatorModule {
    /// Decode + lower + certify a module. The admission gate: imports
    /// refused, `(i32, i32) -> i32` export, basic-opcode-only bodies,
    /// balanced control flow, data segments clear of the ABI regions, i32
    /// value types throughout.
    ///
    /// # Errors
    /// Every refusal is a named [`AdmissionError`] variant: decode failure,
    /// host imports, non-i32 value types, a missing or mis-typed `validate`
    /// export, non-basic opcodes surviving the lowering, unbalanced control
    /// flow, data segments outside the ABI-legal regions, or a
    /// locals/params cap breach.
    // The admission GATE: one sequential refusal ladder over the decoded
    // module — every arm is a named security contract (splitting it would
    // scatter the fail-closed order the Lemma-4.2 exclusion reads).
    #[allow(clippy::too_many_lines)]
    pub fn admit(bytes: &[u8]) -> Result<Self, AdmissionError> {
        let module = decode(bytes)?;

        // Fixed ABI: no host functions, no host memory, nothing imported.
        if !module.imports.is_empty() {
            let funcs = module
                .imports
                .iter()
                .filter(|i| i.kind == ImportKind::Func)
                .count();
            return Err(AdmissionError::ImportsPresent(funcs));
        }

        // The machine is i32-only: every signature i32→(≤1) i32.
        for t in &module.types {
            for v in t.params.iter().chain(t.results.iter()) {
                if *v != VALTYPE_I32 {
                    return Err(AdmissionError::TypeUnsupported(*v));
                }
            }
            if t.results.len() > 1 {
                return Err(AdmissionError::TypeUnsupported(t.results[0]));
            }
        }

        if module.functions.is_empty() {
            return Err(AdmissionError::NoFunctions);
        }

        // The one required export, with its ABI signature.
        let validate_idx = module
            .exports
            .iter()
            .find(|e| e.kind == ExportKind::Func && e.name == ABI_FUNC)
            .map(|e| e.index as usize)
            .ok_or(AdmissionError::MissingExport)?;
        if validate_idx >= module.functions.len() {
            return Err(AdmissionError::ExportOob(validate_idx as u32));
        }
        let vtype = &module.types[module.func_type_indices[validate_idx] as usize];
        if vtype.params.len() != 2 || vtype.results.len() != 1 {
            return Err(AdmissionError::AbiSignature);
        }

        // Per-function: lower i64 then hard ops, certify the basic set,
        // resolve the structured control flow.
        let mut functions = Vec::with_capacity(module.functions.len());
        for (i, body) in module.functions.iter().enumerate() {
            let ty = &module.types[module.func_type_indices[i] as usize];
            let num_params = ty.params.len();
            if num_params > MAX_PARAMS {
                return Err(AdmissionError::TooManyParams {
                    function: i,
                    params: num_params,
                });
            }
            let lowered_i64 = lower_i64_ops(body);
            let lowered = lower_hard_ops(&lowered_i64, num_params as u32);
            let bad: Vec<(&'static str, usize)> = {
                let bad: HashMap<&'static str, usize> = check_basic_only(&lowered);
                let mut bad: Vec<_> = bad.into_iter().collect();
                bad.sort_unstable();
                bad
            };
            if !bad.is_empty() {
                return Err(AdmissionError::UnsupportedOps(bad));
            }
            let total_locals = num_params as u32 + lowered.num_locals;
            if total_locals > MAX_LOCALS {
                return Err(AdmissionError::LocalsCap {
                    function: i,
                    locals: total_locals,
                });
            }
            let ctrls = build_ctrls(&lowered.instructions)?;
            functions.push(PreparedFunc {
                instrs: lowered.instructions,
                ctrls,
                num_params,
                num_locals: total_locals as usize,
                result_arity: ty.results.len(),
            });
        }

        // Data segments must fit the machine's memory and stay clear of the
        // ABI score cell + input region (the host writes input there; a
        // segment under it would be silently clobbered).
        let mut data_segments = Vec::with_capacity(module.data_segments.len());
        for DataSegment { offset, data } in &module.data_segments {
            let off = i64::from(*offset);
            let end = off + data.len() as i64;
            if off < 0 {
                return Err(AdmissionError::DataOffsetNegative(*offset));
            }
            if end > MEM_LEN as i64 {
                return Err(AdmissionError::DataTooLarge {
                    offset: *offset as u32,
                    len: data.len() as u32,
                });
            }
            let over_score = off < i64::from(ABI_SCORE_ADDR) + 4 && end > i64::from(ABI_SCORE_ADDR);
            let over_input = off < MEM_LEN as i64 && end > i64::from(ABI_INPUT_ADDR);
            if over_score || over_input {
                return Err(AdmissionError::DataOverlap {
                    offset: *offset as u32,
                    len: data.len() as u32,
                });
            }
            data_segments.push((*offset as u32, data.clone()));
        }

        // Globals: i32 only, init captured for the per-run reset.
        let mut global_inits = Vec::with_capacity(module.globals.len());
        let mut global_mutable = Vec::with_capacity(module.globals.len());
        for g in &module.globals {
            if g.valtype != VALTYPE_I32 {
                return Err(AdmissionError::TypeUnsupported(g.valtype));
            }
            global_inits.push(g.init);
            global_mutable.push(g.mutable != 0);
        }

        Ok(ValidatorModule {
            functions,
            global_inits,
            global_mutable,
            data_segments,
            validate_idx,
        })
    }
}

/// Resolve every `block`/`loop`/`if` to its matching `end`/`else` + branch
/// arity, once, so the interpreter jumps O(1). Also the structural
/// well-formedness check: every structured instruction closes, `else` binds
/// to an `if`, and the body's final `end` is the last instruction.
fn build_ctrls(instrs: &[WasmInstr]) -> Result<Vec<Ctrl>, AdmissionError> {
    #[derive(Clone, Copy)]
    struct Open {
        pc: usize,
        arity: u8,
        is_if: bool,
    }
    let mut ctrls = vec![
        Ctrl {
            end_pc: 0,
            else_pc: None,
            arity: 0,
        };
        instrs.len()
    ];
    // The implicit function frame (closed by the body's final `end`).
    let mut open: Vec<Option<Open>> = vec![None];
    for (pc, ins) in instrs.iter().enumerate() {
        match ins.opcode {
            OP_BLOCK | OP_LOOP | OP_IF => {
                let arity = match ins.immediates.first() {
                    Some(0x40) => 0u8,
                    Some(0x7F) => 1u8,
                    other => {
                        return Err(AdmissionError::BlockTypeUnsupported(
                            other.copied().unwrap_or(-1),
                        ))
                    }
                };
                open.push(Some(Open {
                    pc,
                    arity,
                    is_if: ins.opcode == OP_IF,
                }));
            }
            OP_ELSE => match open.last() {
                Some(Some(Open {
                    pc, is_if: true, ..
                })) => {
                    if ctrls[*pc].else_pc.is_some() {
                        return Err(AdmissionError::MalformedControl);
                    }
                    ctrls[*pc].else_pc = Some(*pc as u32);
                }
                _ => return Err(AdmissionError::MalformedControl),
            },
            OP_END => match open.pop() {
                Some(Some(o)) => {
                    ctrls[o.pc].end_pc = pc as u32;
                    ctrls[o.pc].arity = o.arity;
                }
                Some(None) => {
                    if pc + 1 != instrs.len() {
                        return Err(AdmissionError::MalformedControl);
                    }
                }
                None => return Err(AdmissionError::MalformedControl),
            },
            _ => {}
        }
    }
    if !open.is_empty() {
        return Err(AdmissionError::MalformedControl);
    }
    Ok(ctrls)
}
