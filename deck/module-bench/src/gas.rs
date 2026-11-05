//! Gas metering, the mutable-global variant of wasm-instrument (#306), re-done on wasmparser + wasm-encoder (the
//! Bytecode Alliance crates wasmtime itself uses): wasm-instrument 0.4 (2022-11, on the archived parity-wasm) cannot
//! read what rustc builds today – the `0xFC` prefix ops (bulk memory, non-trapping float-to-int) the precompiled std
//! carries even in an `-C target-cpu=mvp` build, and the padded `call_indirect` table index – so `prepare` reports its
//! error and instruments with this pass. The scheme is wasm-instrument's: an exported mutable `gas_left: i64`, a local
//! function `gas_charge(cost)` that subtracts or – out of gas – sets the global to u64::MAX and traps (`unreachable`),
//! one call per metered segment carrying the segment's instruction count (cost 1 an instruction). A segment starts at
//! the function's entry and after every control instruction a path can fall through (`block`, `loop`, `if`, `else`,
//! `end`, `br_if`); a loop's body is charged on every iteration, so a runaway runs out of gas. (wasm-instrument merges
//! a `block`'s segments where it can – this pass charges a little more often: an upper bound for its overhead.)
//!
//! [`Scheme::Loops`] is the epoch-like alternative (preemption, not metering): the same global, decremented by one
//! inline – no call – at every function entry and every loop head, out of it (below 0) = `unreachable`. Its unit is
//! "loop iterations + calls", deterministic but coarse; what it buys is a runaway stopped at a low cost per check.
use anyhow::{anyhow, bail, Result};
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
use wasm_encoder::{
    CodeSection, ConstExpr, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    GlobalType, Instruction, Module, TypeSection, ValType,
};
use wasmparser::{Operator, Parser, Payload};

fn e<E: std::fmt::Debug>(x: E) -> anyhow::Error {
    anyhow!("{x:?}")
}

/// the instructions after which a metered segment starts (a path falls through to the next instruction)
fn starts_after(op: &Operator) -> bool {
    matches!(
        op,
        Operator::Block { .. }
            | Operator::Loop { .. }
            | Operator::If { .. }
            | Operator::Else
            | Operator::End
            | Operator::BrIf { .. }
    )
}

/// how the module is instrumented
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// wasm-instrument's metering: a `gas_charge(cost)` call per segment
    Metered,
    /// a decrement and a check inline at every function entry and loop head
    Loops,
}

/// the inline check of [`Scheme::Loops`]: `gas_left -= 1; if gas_left < 0 { unreachable }`
fn tick(func: &mut Function, global: u32) {
    for i in [
        Instruction::GlobalGet(global),
        Instruction::I64Const(1),
        Instruction::I64Sub,
        Instruction::GlobalSet(global),
        Instruction::GlobalGet(global),
        Instruction::I64Const(0),
        Instruction::I64LtS,
        Instruction::If(wasm_encoder::BlockType::Empty),
        Instruction::Unreachable,
        Instruction::End,
    ] {
        func.instruction(&i);
    }
}

fn body(
    re: &mut RoundtripReencoder,
    f: wasmparser::FunctionBody,
    charge: u32,
    global: u32,
    scheme: Scheme,
) -> Result<Function> {
    let mut locals = Vec::new();
    for l in f.get_locals_reader()? {
        let (n, t) = l?;
        locals.push((n, re.val_type(t).map_err(e)?));
    }
    let mut func = Function::new(locals);
    let ops: Vec<Operator> = f
        .get_operators_reader()?
        .into_iter()
        .collect::<Result<_, _>>()?;
    if scheme == Scheme::Loops {
        tick(&mut func, global);
        for op in &ops {
            func.instruction(&re.instruction(op.clone()).map_err(e)?);
            if matches!(op, Operator::Loop { .. }) {
                tick(&mut func, global);
            }
        }
        return Ok(func);
    }
    // segment starts and their costs (the final `end` closes the function: nothing starts after it)
    let mut starts = vec![0usize];
    for (i, op) in ops.iter().enumerate() {
        if starts_after(op) && i + 1 < ops.len() {
            starts.push(i + 1);
        }
    }
    let mut next = 0;
    for (i, op) in ops.iter().enumerate() {
        if next < starts.len() && starts[next] == i {
            let end = starts.get(next + 1).copied().unwrap_or(ops.len());
            func.instruction(&Instruction::I64Const((end - i) as i64));
            func.instruction(&Instruction::Call(charge));
            next += 1;
        }
        func.instruction(&re.instruction(op.clone()).map_err(e)?);
    }
    Ok(func)
}

/// `gas_charge(cost: i64)`: wasm-instrument's mutable-global function
fn charge_fn(global: u32) -> Function {
    let mut f = Function::new([]);
    for i in [
        Instruction::GlobalGet(global),
        Instruction::LocalGet(0),
        Instruction::I64GeU,
        Instruction::If(wasm_encoder::BlockType::Empty),
        Instruction::GlobalGet(global),
        Instruction::LocalGet(0),
        Instruction::I64Sub,
        Instruction::GlobalSet(global),
        Instruction::Else,
        Instruction::I64Const(-1),
        Instruction::GlobalSet(global),
        Instruction::Unreachable,
        Instruction::End,
        Instruction::End,
    ] {
        f.instruction(&i);
    }
    f
}

/// the module with gas metering; its `gas_left` export is the budget the host sets before a call
pub fn inject(wasm: &[u8], scheme: Scheme) -> Result<Vec<u8>> {
    inject_from(wasm, scheme, 0)
}

/// [`inject`] with `gas_left` starting at `start` – i64::MAX: the checks run but never stop (#396 measures their cost on
/// a host that does not set the global)
pub fn inject_from(wasm: &[u8], scheme: Scheme, start: i64) -> Result<Vec<u8>> {
    let mut re = RoundtripReencoder;
    let mut out = Module::new();
    let (mut types, mut imported_funcs, mut funcs, mut globals) = (None::<u32>, 0u32, 0u32, 0u32);
    let mut code: Option<(CodeSection, u32, u32)> = None;
    for payload in Parser::new(0).parse_all(wasm) {
        match payload? {
            Payload::Version { .. } | Payload::End(_) => {}
            Payload::TypeSection(r) => {
                let mut s = TypeSection::new();
                re.parse_type_section(&mut s, r).map_err(e)?;
                types = Some(s.len());
                s.ty().function([ValType::I64], []);
                out.section(&s);
            }
            Payload::ImportSection(r) => {
                let mut s = wasm_encoder::ImportSection::new();
                for imp in r.clone() {
                    match imp?.ty {
                        wasmparser::TypeRef::Func(_) => imported_funcs += 1,
                        wasmparser::TypeRef::Global(_) => globals += 1,
                        _ => {}
                    }
                }
                re.parse_import_section(&mut s, r).map_err(e)?;
                out.section(&s);
            }
            Payload::FunctionSection(r) => {
                let mut s = FunctionSection::new();
                funcs = r.count();
                re.parse_function_section(&mut s, r).map_err(e)?;
                s.function(types.ok_or_else(|| anyhow!("no type section"))?);
                out.section(&s);
            }
            Payload::GlobalSection(r) => {
                let mut s = GlobalSection::new();
                globals += r.count();
                re.parse_global_section(&mut s, r).map_err(e)?;
                s.global(
                    GlobalType {
                        val_type: ValType::I64,
                        mutable: true,
                        shared: false,
                    },
                    &ConstExpr::i64_const(start),
                );
                out.section(&s);
            }
            Payload::ExportSection(r) => {
                let mut s = ExportSection::new();
                re.parse_export_section(&mut s, r).map_err(e)?;
                s.export("gas_left", ExportKind::Global, globals);
                out.section(&s);
            }
            Payload::CodeSectionStart { count, .. } => {
                if funcs == 0 {
                    bail!("a code section without functions");
                }
                code = Some((CodeSection::new(), count, imported_funcs + funcs));
            }
            Payload::CodeSectionEntry(f) => {
                let Some((s, left, charge)) = code.as_mut() else {
                    bail!("code before its start")
                };
                s.function(&body(&mut re, f, *charge, globals, scheme)?);
                *left -= 1;
                if *left == 0 {
                    s.function(&charge_fn(globals));
                    out.section(&*s);
                }
            }
            Payload::CustomSection(c) => {
                out.section(&wasm_encoder::CustomSection {
                    name: c.name().into(),
                    data: c.data().into(),
                });
            }
            other => {
                // tables, memory, start, elements, data count, data: unchanged
                let (id, range) = other
                    .as_section()
                    .ok_or_else(|| anyhow!("a section this pass does not know: {other:?}"))?;
                out.section(&wasm_encoder::RawSection {
                    id,
                    data: &wasm[range],
                });
            }
        }
    }
    if code.is_none() || globals == 0 {
        // Rust modules always have both (the stack pointer is a global)
        bail!("a module without code or globals");
    }
    Ok(out.finish())
}
