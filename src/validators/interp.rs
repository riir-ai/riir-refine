//! The fuel-bounded WASM MVP interpreter over the lowered basic-opcode set.
//!
//! Executes what Plan 199 T3.1 certified as the lane's machine:
//! `check_basic_only`-clean function bodies — i32 const/local/global access,
//! the load/store family, `eqz` + the ten signed/unsigned compares,
//! `add`/`sub`, and the structured control flow (`block`/`loop`/`if`/`else`/
//! `end`/`br`/`br_if`/`br_table`/`return`/`call`/`drop`/`select`/
//! `memory.size`/`unreachable`/`nop`). No floats reach this machine (none
//! exist in the basic set) — determinism by construction.
//!
//! Boundedness is the lane's safety property, and it is enforced here, not
//! assumed: a step budget (fuel), a call-depth cap, operand-stack and
//! label-depth caps, and bounds-checked memory on every access. A module
//! that would run forever instead reports [`Trap::OutOfFuel`].

// The machine IS the spec: i32 values over u32 addressing with wrapping
// arithmetic — sign loss, truncation, and wrap are defined wasm behavior
// here, not defects. Every usize cast below indexes a structure the
// admission gate has already bounded (ctrls/memory/locals), so the value is
// in range by construction.
#![allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
// `step` is the opcode dispatch — one arm per basic op; splitting it would
// hide the machine shape it exists to show.
#![allow(clippy::too_many_lines)]

use katgpt_percepta::wasm::{
    OP_BLOCK, OP_BR, OP_BR_IF, OP_BR_TABLE, OP_CALL, OP_DROP, OP_ELSE, OP_END, OP_GLOBAL_GET,
    OP_GLOBAL_SET, OP_I32_ADD, OP_I32_CONST, OP_I32_EQ, OP_I32_EQZ, OP_I32_GE_S, OP_I32_GE_U,
    OP_I32_GT_S, OP_I32_GT_U, OP_I32_LE_S, OP_I32_LE_U, OP_I32_LOAD, OP_I32_LOAD16_S,
    OP_I32_LOAD16_U, OP_I32_LOAD8_S, OP_I32_LOAD8_U, OP_I32_LT_S, OP_I32_LT_U, OP_I32_NE,
    OP_I32_STORE, OP_I32_STORE16, OP_I32_STORE8, OP_I32_SUB, OP_IF, OP_LOCAL_GET, OP_LOCAL_SET,
    OP_LOCAL_TEE, OP_LOOP, OP_MEMORY_SIZE, OP_NOP, OP_RETURN, OP_SELECT, OP_UNREACHABLE,
};

use super::ValidatorModule;

// ── Fixed ABI (the contract the validator sources compile against) ──────

/// The one export the admission gate requires: `validate(ptr, len) -> verdict`.
pub const ABI_FUNC: &str = "validate";

/// Where the host writes the input bytes before invoking [`ABI_FUNC`].
/// Placed at half the memory so build-time rodata (`wasm-ld` places data
/// sections in the low kilobytes) can never collide with it.
pub const ABI_INPUT_ADDR: u32 = 32768;

/// The 4-byte little-endian score cell the module may write; read by the
/// host after the call. Below the wasm-ld data base so rodata cannot
/// shadow it.
pub const ABI_SCORE_ADDR: u32 = 512;

/// Linear memory the interpreter owns: one MVP page. The decoder skips the
/// module's own memory declaration silently, so the machine's memory IS the
/// module's memory — data segments land here and every access is bounds
/// checked against this length.
pub const MEM_LEN: usize = 65536;

// ── Machine caps (the boundedness contract) ──────────────────────────────

/// Direct-call depth cap. Admission refuses imports, so every call target
/// is a declared function in the same module.
pub const MAX_CALL_DEPTH: usize = 32;
/// Operand-stack cap (per whole execution; label entry heights index into it).
pub const MAX_OPERAND_STACK: usize = 1024;
/// Structured-label depth cap (per frame).
pub const MAX_LABEL_DEPTH: usize = 256;
/// Total locals (params + declared) per function.
pub const MAX_LOCALS: u32 = 1024;
/// Max params per call — far beyond anything the authoring lane emits.
pub const MAX_PARAMS: usize = 32;
/// Default step budget: µs-class validators burn orders of magnitude less.
pub const DEFAULT_FUEL: u64 = 1_000_000;

// ── Traps ────────────────────────────────────────────────────────────────

/// Every way a bounded execution can stop without a verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    /// The step budget ran out — the module did not terminate in budget.
    OutOfFuel,
    /// A load/store addressed outside the 64 KiB linear memory.
    MemoryOob { addr: u32, access: u32 },
    /// Direct-call depth exceeded [`MAX_CALL_DEPTH`].
    CallDepth,
    /// `div`/`rem` by zero, or `div_s` of `i32::MIN` by `-1` (wasm
    /// semantics; the arithmetic itself is lowered away upstream — this
    /// arm is the defense if it ever reappears).
    DivByZero,
    /// The module executed `unreachable` (e.g. the `no_std` panic handler).
    Unreachable,
    /// `global.set` on an immutable global.
    GlobalImmutable,
    /// A branch depth named a label that does not exist, or another
    /// structural inconsistency the admission gate's control scan should
    /// have refused.
    MalformedBranch,
    /// A call target outside the declared function index space.
    CallTargetOob(u32),
    /// An opcode outside the basic set reached the machine (defense in
    /// depth — admission refuses such modules before they run).
    UnsupportedOpcode(u8),
    /// An instruction popped from an empty operand stack.
    StackUnderflow,
    /// The operand stack exceeded [`MAX_OPERAND_STACK`].
    StackOverflow,
    /// Structured-label depth exceeded [`MAX_LABEL_DEPTH`].
    LabelOverflow,
    /// A local or global index outside the frame's space.
    IndexOob,
    /// More params than [`MAX_PARAMS`] on a call.
    TooManyParams(u32),
    /// Input larger than the ABI input region.
    InputTooLarge { len: usize, capacity: usize },
}

// ── Outcome ──────────────────────────────────────────────────────────────

/// A completed validator run: the ABI verdict plus the module's score cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// The `validate` return value (0 = pass, per-class nonzero = refuse).
    pub verdict: i32,
    /// Little-endian u32 the module wrote at [`ABI_SCORE_ADDR`] (0 when it
    /// writes nothing).
    pub score: u32,
}

// ── Control precompute ──────────────────────────────────────────────

/// Per-`block`/`loop`/`if` structured-control facts, resolved once at
/// admission so the interpreter never rescans for matching `end`/`else`.
#[derive(Debug, Clone, Copy)]
pub struct Ctrl {
    /// pc of the matching `end`.
    pub end_pc: u32,
    /// pc of the matching `else` (only for `if`).
    pub else_pc: Option<u32>,
    /// Branch arity: 0 for void blocktypes, 1 for `(result i32)`.
    pub arity: u8,
}

// ── The machine ──────────────────────────────────────────────────────────

struct Frame {
    func_index: usize,
    pc: usize,
    entry_height: usize,
    locals: Vec<i32>,
    result_arity: usize,
    labels: Vec<Label>,
}

#[derive(Debug, Clone, Copy)]
struct Label {
    is_loop: bool,
    is_func: bool,
    start_pc: usize,
    end_pc: usize,
    entry_height: usize,
    /// Branch arity kept on `br` to this label: block/if keep their RESULT
    /// count; a loop keeps its PARAM count (always 0 in MVP); the implicit
    /// function label keeps the function's result count (`br` to it is a
    /// return).
    keep: u8,
}

/// A reusable interpreter machine. Allocation happens once per `Vm` (memory
/// + stacks); `run` recycles them, so hot-path reruns stay allocation-free.
#[derive(Default)]
pub struct Vm {
    memory: Vec<u8>,
    stack: Vec<i32>,
    globals: Vec<i32>,
    frames: Vec<Frame>,
    locals_pool: Vec<Vec<i32>>,
}

impl ValidatorModule {
    /// Execute `validate` over `input` with the given step budget.
    ///
    /// Deterministic: integer-only machine, fixed memory layout, no host
    /// state — the same bytes produce the same outcome on every host.
    ///
    /// # Errors
    /// [`Trap::InputTooLarge`] when the input exceeds the ABI input region;
    /// otherwise any execution [`Trap`] the machine can raise (fuel
    /// exhaustion, out-of-bounds memory, stack/call caps, a trapped div, an
    /// executed `unreachable`).
    pub fn run(&self, input: &[u8], fuel: u64, vm: &mut Vm) -> Result<Outcome, Trap> {
        let capacity = MEM_LEN - ABI_INPUT_ADDR as usize;
        if input.len() > capacity {
            return Err(Trap::InputTooLarge {
                len: input.len(),
                capacity,
            });
        }
        vm.reset(self, input);
        // The fixed ABI: `validate(ABI_INPUT_ADDR, len)`.
        vm.invoke(
            self,
            self.validate_idx,
            &[ABI_INPUT_ADDR as i32, input.len() as i32],
            fuel,
        )?;
        let s = ABI_SCORE_ADDR as usize;
        let score = u32::from_le_bytes([vm.memory[s], vm.memory[s + 1], vm.memory[s + 2], vm.memory[s + 3]]);
        let verdict = vm.stack.pop().unwrap_or(0);
        Ok(Outcome { verdict, score })
    }

    /// One-shot convenience: a fresh [`Vm`] per call.
    ///
    /// # Errors
    /// Exactly [`ValidatorModule::run`]'s.
    pub fn run_once(&self, input: &[u8], fuel: u64) -> Result<Outcome, Trap> {
        let mut vm = Vm::default();
        self.run(input, fuel, &mut vm)
    }
}

impl Vm {
    fn reset(&mut self, module: &ValidatorModule, input: &[u8]) {
        self.memory.clear();
        self.memory.resize(MEM_LEN, 0);
        for (off, data) in &module.data_segments {
            let off = *off as usize;
            self.memory[off..off + data.len()].copy_from_slice(data);
        }
        let base = ABI_INPUT_ADDR as usize;
        self.memory[base..base + input.len()].copy_from_slice(input);
        self.globals.clear();
        self.globals.extend_from_slice(&module.global_inits);
        self.stack.clear();
        self.frames.clear();
    }

    fn invoke(
        &mut self,
        module: &ValidatorModule,
        func_idx: usize,
        args: &[i32],
        fuel: u64,
    ) -> Result<(), Trap> {
        let mut fuel = fuel;
        self.push_frame(module, func_idx, args)?;
        while !self.frames.is_empty() {
            if fuel == 0 {
                return Err(Trap::OutOfFuel);
            }
            fuel -= 1;
            self.step(module)?;
        }
        Ok(())
    }

    fn push_frame(
        &mut self,
        module: &ValidatorModule,
        func_idx: usize,
        args: &[i32],
    ) -> Result<(), Trap> {
        if self.frames.len() >= MAX_CALL_DEPTH {
            return Err(Trap::CallDepth);
        }
        let func = &module.functions[func_idx];
        let mut locals = self.locals_pool.pop().unwrap_or_default();
        locals.clear();
        locals.extend_from_slice(args);
        locals.resize(func.num_locals, 0);
        let entry_height = self.stack.len();
        let end_pc = func.instrs.len();
        let result_arity = func.result_arity;
        self.frames.push(Frame {
            func_index: func_idx,
            pc: 0,
            entry_height,
            locals,
            result_arity,
            labels: vec![Label {
                is_loop: false,
                is_func: true,
                start_pc: 0,
                end_pc,
                entry_height,
                keep: result_arity as u8,
            }],
        });
        Ok(())
    }

    fn step(&mut self, module: &ValidatorModule) -> Result<(), Trap> {
        let (pc, func_idx) = {
            let frame = self.frames.last().expect("step with no frame");
            (frame.pc, frame.func_index)
        };
        let func = &module.functions[func_idx];
        let Some(instr) = func.instrs.get(pc) else {
            // Fell off the end of a body without its final `end` — treat as
            // return (defensive; admission's control scan makes this
            // unreachable for admitted modules).
            return self.ret();
        };
        let imm0 = instr.immediates.first().copied().unwrap_or(0);
        let next_pc = pc + 1;
        match instr.opcode {
            OP_NOP => self.jump(next_pc),
            OP_UNREACHABLE => Err(Trap::Unreachable),
            OP_DROP => {
                self.pop()?;
                self.jump(next_pc)
            }
            OP_I32_CONST => {
                self.push(imm0 as i32)?;
                self.jump(next_pc)
            }
            OP_LOCAL_GET => {
                let v = self.local(imm0 as usize)?;
                self.push(v)?;
                self.jump(next_pc)
            }
            OP_LOCAL_SET => {
                let v = self.pop()?;
                self.set_local(imm0 as usize, v)?;
                self.jump(next_pc)
            }
            OP_LOCAL_TEE => {
                let v = *self.stack.last().ok_or(Trap::StackUnderflow)?;
                self.set_local(imm0 as usize, v)?;
                self.jump(next_pc)
            }
            OP_GLOBAL_GET => {
                let v = *self.globals.get(imm0 as usize).ok_or(Trap::IndexOob)?;
                self.push(v)?;
                self.jump(next_pc)
            }
            OP_GLOBAL_SET => {
                if !module
                    .global_mutable
                    .get(imm0 as usize)
                    .copied()
                    .unwrap_or(false)
                {
                    return Err(Trap::GlobalImmutable);
                }
                let v = self.pop()?;
                *self.globals.get_mut(imm0 as usize).ok_or(Trap::IndexOob)? = v;
                self.jump(next_pc)
            }
            OP_I32_EQZ => {
                let a = self.pop()?;
                self.push(i32::from(a == 0))?;
                self.jump(next_pc)
            }
            OP_I32_EQ => self.eq_ne(next_pc, true),
            OP_I32_NE => self.eq_ne(next_pc, false),
            OP_I32_LT_S => self.cmp(next_pc, |a, b| a < b),
            OP_I32_LT_U => self.cmp(next_pc, |a, b| (a as u32) < (b as u32)),
            OP_I32_GT_S => self.cmp(next_pc, |a, b| a > b),
            OP_I32_GT_U => self.cmp(next_pc, |a, b| (a as u32) > (b as u32)),
            OP_I32_LE_S => self.cmp(next_pc, |a, b| a <= b),
            OP_I32_LE_U => self.cmp(next_pc, |a, b| (a as u32) <= (b as u32)),
            OP_I32_GE_S => self.cmp(next_pc, |a, b| a >= b),
            OP_I32_GE_U => self.cmp(next_pc, |a, b| (a as u32) >= (b as u32)),
            OP_I32_ADD => self.binop(next_pc, i32::wrapping_add),
            OP_I32_SUB => self.binop(next_pc, i32::wrapping_sub),
            OP_I32_LOAD => self.load(next_pc, instr.immediates[1], 4, LoadExt::None),
            OP_I32_LOAD8_S => self.load(next_pc, instr.immediates[1], 1, LoadExt::Sign8),
            OP_I32_LOAD8_U => self.load(next_pc, instr.immediates[1], 1, LoadExt::None),
            OP_I32_LOAD16_S => self.load(next_pc, instr.immediates[1], 2, LoadExt::Sign16),
            OP_I32_LOAD16_U => self.load(next_pc, instr.immediates[1], 2, LoadExt::None),
            OP_I32_STORE => self.store(next_pc, instr.immediates[1], 4),
            OP_I32_STORE8 => self.store(next_pc, instr.immediates[1], 1),
            OP_I32_STORE16 => self.store(next_pc, instr.immediates[1], 2),
            OP_MEMORY_SIZE => {
                self.push((MEM_LEN / 65536) as i32)?;
                self.jump(next_pc)
            }
            OP_SELECT => {
                let c = self.pop()?;
                let v2 = self.pop()?;
                let v1 = self.pop()?;
                self.push(if c != 0 { v1 } else { v2 })?;
                self.jump(next_pc)
            }
            OP_BLOCK | OP_LOOP => {
                let ctrl = *func.ctrls.get(pc).ok_or(Trap::MalformedBranch)?;
                // A loop label keeps nothing on `br` (MVP loops take no
                // params); a block keeps its result arity.
                let keep = if instr.opcode == OP_LOOP { 0 } else { ctrl.arity };
                self.push_label(instr.opcode == OP_LOOP, pc, ctrl.end_pc as usize, keep)?;
                self.jump(next_pc)
            }
            OP_IF => {
                let ctrl = *func.ctrls.get(pc).ok_or(Trap::MalformedBranch)?;
                let cond = self.pop()?;
                self.push_label(false, pc, ctrl.end_pc as usize, ctrl.arity)?;
                if cond == 0 {
                    let target = match ctrl.else_pc {
                        Some(e) => e as usize + 1,
                        None => ctrl.end_pc as usize,
                    };
                    self.jump(target)
                } else {
                    self.jump(next_pc)
                }
            }
            OP_ELSE => {
                // Reached only by the then-branch falling through: skip to
                // the matching `end` (which pops the if-label).
                let end_pc = self
                    .frames
                    .last()
                    .expect("step with no frame")
                    .labels
                    .last()
                    .ok_or(Trap::MalformedBranch)?
                    .end_pc;
                self.jump(end_pc)
            }
            OP_END => {
                let frame = self.frames.last_mut().expect("step with no frame");
                if frame.labels.pop().is_none() {
                    return Err(Trap::MalformedBranch);
                }
                if frame.labels.is_empty() {
                    // Popped the implicit function label: body complete.
                    return self.ret();
                }
                self.jump(next_pc)
            }
            OP_BR => self.branch(imm0 as u32),
            OP_BR_IF => {
                let cond = self.pop()?;
                if cond != 0 {
                    self.branch(imm0 as u32)
                } else {
                    self.jump(next_pc)
                }
            }
            OP_BR_TABLE => {
                let idx = self.pop()? as u32 as usize;
                let n = instr.immediates.len();
                if n == 0 {
                    return Err(Trap::MalformedBranch);
                }
                let depth = if idx < n - 1 {
                    instr.immediates[idx] as u32
                } else {
                    instr.immediates[n - 1] as u32
                };
                self.branch(depth)
            }
            OP_RETURN => self.ret(),
            OP_CALL => {
                let target = imm0 as u32 as usize;
                if target >= module.functions.len() {
                    return Err(Trap::CallTargetOob(target as u32));
                }
                let arity = module.functions[target].num_params;
                if arity > MAX_PARAMS {
                    return Err(Trap::TooManyParams(arity as u32));
                }
                let mut args = [0i32; MAX_PARAMS];
                for slot in args.iter_mut().rev().take(arity) {
                    *slot = self.pop()?;
                }
                self.push_frame(module, target, &args[..arity])
            }
            other => Err(Trap::UnsupportedOpcode(other)),
        }
    }

    // Uniform arm tails: plain arms end in `self.jump(next_pc)` as a Result
    // so the dispatch needs no per-arm Ok wrapping.
    #[allow(clippy::unnecessary_wraps)]
    fn jump(&mut self, pc: usize) -> Result<(), Trap> {
        self.frames.last_mut().expect("step with no frame").pc = pc;
        Ok(())
    }

    fn push_label(&mut self, is_loop: bool, start_pc: usize, end_pc: usize, keep: u8) -> Result<(), Trap> {
        let frame = self.frames.last_mut().expect("step with no frame");
        if frame.labels.len() >= MAX_LABEL_DEPTH {
            return Err(Trap::LabelOverflow);
        }
        let entry_height = self.stack.len();
        frame.labels.push(Label {
            is_loop,
            is_func: false,
            start_pc,
            end_pc,
            entry_height,
            keep,
        });
        Ok(())
    }

    fn local(&self, idx: usize) -> Result<i32, Trap> {
        self.frames
            .last()
            .expect("step with no frame")
            .locals
            .get(idx)
            .copied()
            .ok_or(Trap::IndexOob)
    }

    fn set_local(&mut self, idx: usize, v: i32) -> Result<(), Trap> {
        let locals = &mut self.frames.last_mut().expect("step with no frame").locals;
        let slot = locals.get_mut(idx).ok_or(Trap::IndexOob)?;
        *slot = v;
        Ok(())
    }

    fn push(&mut self, v: i32) -> Result<(), Trap> {
        if self.stack.len() >= MAX_OPERAND_STACK {
            return Err(Trap::StackOverflow);
        }
        self.stack.push(v);
        Ok(())
    }

    fn pop(&mut self) -> Result<i32, Trap> {
        self.stack.pop().ok_or(Trap::StackUnderflow)
    }

    fn cmp(&mut self, next_pc: usize, f: impl Fn(i32, i32) -> bool) -> Result<(), Trap> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(i32::from(f(a, b)))?;
        self.jump(next_pc)
    }

    fn eq_ne(&mut self, next_pc: usize, want_eq: bool) -> Result<(), Trap> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(i32::from((a == b) == want_eq))?;
        self.jump(next_pc)
    }

    fn binop(&mut self, next_pc: usize, f: impl Fn(i32, i32) -> i32) -> Result<(), Trap> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(f(a, b))?;
        self.jump(next_pc)
    }

    fn load(&mut self, next_pc: usize, offset_imm: i64, size: u32, ext: LoadExt) -> Result<(), Trap> {
        let addr = (self.pop()? as u32).wrapping_add(offset_imm as u32);
        let start = addr as usize;
        let end = start + size as usize;
        if end > MEM_LEN {
            return Err(Trap::MemoryOob { addr, access: size });
        }
        let v = match (size, ext) {
            (4, LoadExt::None) => i32::from_le_bytes(
                self.memory[start..start + 4].try_into().expect("static width"),
            ),
            (2, LoadExt::None) => i32::from(u16::from_le_bytes(
                self.memory[start..start + 2].try_into().expect("static width"),
            )),
            (2, LoadExt::Sign16) => i32::from(i16::from_le_bytes(
                self.memory[start..start + 2].try_into().expect("static width"),
            )),
            (1, LoadExt::None) => i32::from(self.memory[start]),
            (1, LoadExt::Sign8) => i32::from(self.memory[start] as i8),
            _ => return Err(Trap::MalformedBranch), // unreachable: admission pins the load shapes
        };
        self.push(v)?;
        self.jump(next_pc)
    }

    fn store(&mut self, next_pc: usize, offset_imm: i64, size: u32) -> Result<(), Trap> {
        let value = self.pop()?;
        let addr = (self.pop()? as u32).wrapping_add(offset_imm as u32);
        let start = addr as usize;
        let end = start + size as usize;
        if end > MEM_LEN {
            return Err(Trap::MemoryOob { addr, access: size });
        }
        let bytes = value.to_le_bytes();
        self.memory[start..end].copy_from_slice(&bytes[..size as usize]);
        self.jump(next_pc)
    }

    fn branch(&mut self, depth: u32) -> Result<(), Trap> {
        let depth = depth as usize;
        // Resolve the target immutably first, then mutate: the carry-pop and
        // the truncation must not race the label-stack borrow.
        let (target_idx, target) = {
            let frame = self.frames.last().expect("branch with no frame");
            if depth >= frame.labels.len() {
                return Err(Trap::MalformedBranch);
            }
            let idx = frame.labels.len() - 1 - depth;
            (idx, frame.labels[idx])
        };
        // Branch operands: the label's `keep` values ride on top; unwind the
        // rest back to the label's entry height, then put them back.
        let keep = target.keep as usize;
        let mut carried = [0i32; 1];
        if keep == 1 {
            carried[0] = self.pop()?;
        }
        if self.stack.len() < target.entry_height {
            return Err(Trap::MalformedBranch);
        }
        self.stack.truncate(target.entry_height);
        for &v in carried.iter().take(keep) {
            self.push(v)?;
        }
        if target.is_func {
            return self.ret();
        }
        let frame = self.frames.last_mut().expect("branch with no frame");
        if target.is_loop {
            // Re-enter the loop instruction: it pushes a fresh label.
            frame.labels.truncate(target_idx);
            frame.pc = target.start_pc;
        } else {
            // Keep the target label: the `end` at end_pc pops it.
            frame.labels.truncate(target_idx + 1);
            frame.pc = target.end_pc;
        }
        Ok(())
    }

    fn ret(&mut self) -> Result<(), Trap> {
        let arity = self.frames.last().expect("ret with no frame").result_arity;
        let mut results = [0i32; 1];
        if arity == 1 {
            results[0] = self.pop()?;
        } else if arity > 1 {
            // Multi-result types are refused at admission (MVP i32 ABI);
            // unreachable for admitted modules.
            return Err(Trap::MalformedBranch);
        }
        let frame = self.frames.pop().expect("ret with no frame");
        let entry_height = frame.entry_height;
        let locals = frame.locals;
        self.stack.truncate(entry_height);
        for &v in results.iter().take(arity) {
            self.push(v)?;
        }
        self.locals_pool.push(locals);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum LoadExt {
    None,
    Sign8,
    Sign16,
}
