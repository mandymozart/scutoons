//! NaN canonicalization at the points where a NaN's bits can be seen (#396, research/WASM-RUNTIME-PAPER.md) – an
//! EXPERIMENT, not the deck's rule. Which values are NaN is IEEE-exact on every engine; only a NaN's sign and payload
//! are the CPU's (x86 makes 0xFFC00000, Arm 0x7FC00000). wasmtime's `cranelift_nan_canonicalization` fixes them after
//! every float operation; a NaN's bits are only observable where a float becomes bits, so this pass canonicalizes
//! there instead and leaves the arithmetic alone:
//! - `f32.store` / `f64.store` (memory is the module's state and its outputs; a later int load sees the bits),
//! - `i32.reinterpret_f32` / `i64.reinterpret_f64` (`to_bits`, libm's classification),
//! - the sign operand of `copysign` (its sign bit lands in a non-NaN result).
//!
//! Everything else keeps a NaN a NaN (arithmetic, `abs` / `neg`, min / max, promote / demote) or reads only its NaN-ness
//! (comparisons, `trunc_sat` → 0, `trunc` traps). Not covered, refused or out of scope: SIMD (a `v128` op is refused –
//! its lanes are reinterpreted without an instruction), float params / results of imports and exports (the module ABI
//! has none), float globals read by the host (none).
//! The canonical NaNs are wasmtime's: 0x7FC00000 and 0x7FF8000000000000. One scratch local per type and function.
use anyhow::{anyhow, bail, Result};
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
use wasm_encoder::{CodeSection, Function, Ieee32, Ieee64, Instruction, Module, ValType};
use wasmparser::{Operator, Parser, Payload};

fn e<E: std::fmt::Debug>(x: E) -> anyhow::Error {
    anyhow!("{x:?}")
}

/// the top of the stack, canonicalized: `t = top; t == t ? t : NaN`
fn canon(func: &mut Function, f64: bool, local: u32) {
    let nan = if f64 {
        Instruction::F64Const(Ieee64::new(0x7FF8_0000_0000_0000))
    } else {
        Instruction::F32Const(Ieee32::new(0x7FC0_0000))
    };
    let eq = if f64 {
        Instruction::F64Eq
    } else {
        Instruction::F32Eq
    };
    for i in [
        Instruction::LocalTee(local),
        nan,
        Instruction::LocalGet(local),
        Instruction::LocalGet(local),
        eq,
        Instruction::Select,
    ] {
        func.instruction(&i);
    }
}

/// `Some(true)` for an f64 observation point, `Some(false)` for f32
fn observes(op: &Operator) -> Option<bool> {
    match op {
        Operator::F32Store { .. } | Operator::I32ReinterpretF32 | Operator::F32Copysign => {
            Some(false)
        }
        Operator::F64Store { .. } | Operator::I64ReinterpretF64 | Operator::F64Copysign => {
            Some(true)
        }
        _ => None,
    }
}

fn body(
    re: &mut RoundtripReencoder,
    f: wasmparser::FunctionBody,
    params: u32,
    stores: bool,
) -> Result<(Function, usize)> {
    let mut locals = Vec::new();
    let mut count = params;
    for l in f.get_locals_reader()? {
        let (n, t) = l?;
        count += n;
        locals.push((n, re.val_type(t).map_err(e)?));
    }
    let (l32, l64) = (count, count + 1);
    locals.push((1, ValType::F32));
    locals.push((1, ValType::F64));
    let mut func = Function::new(locals);
    let mut points = 0;
    for op in f.get_operators_reader()? {
        let op = op?;
        let store = matches!(op, Operator::F32Store { .. } | Operator::F64Store { .. });
        if let Some(wide) = observes(&op).filter(|_| stores || !store) {
            canon(&mut func, wide, if wide { l64 } else { l32 });
            points += 1;
        }
        let name = format!("{op:?}");
        if ["V128", "I8x16", "I16x8", "I32x4", "I64x2", "F32x4", "F64x2"]
            .iter()
            .any(|p| name.starts_with(p))
        {
            bail!("a SIMD op ({name}): this pass does not canonicalize lanes");
        }
        func.instruction(&re.instruction(op).map_err(e)?);
    }
    Ok((func, points))
}

/// the module with NaNs canonicalized at every observation point; also the number of points
pub fn canonicalize(wasm: &[u8]) -> Result<(Vec<u8>, usize)> {
    canonicalize_at(wasm, true)
}

/// `stores: false` – the light variant (#396 measures its cost): reinterpret and copysign only; float stores keep the
/// CPU's NaN, so the host would canonicalize the float outputs it reads, and an int load of a float the module stored
/// (a hash of its state, a byte-wise compare) is no longer covered
pub fn canonicalize_at(wasm: &[u8], stores: bool) -> Result<(Vec<u8>, usize)> {
    let mut re = RoundtripReencoder;
    let mut out = Module::new();
    let mut params: Vec<u32> = Vec::new();
    let mut funcs: Vec<u32> = Vec::new();
    let (mut code, mut next, mut points) = (CodeSection::new(), 0usize, 0usize);
    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload?;
        match &payload {
            Payload::TypeSection(r) => {
                for t in r.clone().into_iter_err_on_gc_types() {
                    params.push(t?.params().len() as u32);
                }
            }
            Payload::FunctionSection(r) => {
                for t in r.clone() {
                    funcs.push(t?);
                }
            }
            _ => {}
        }
        match payload {
            Payload::Version { .. } | Payload::End(_) | Payload::CodeSectionStart { .. } => {}
            Payload::CodeSectionEntry(f) => {
                let ty = *funcs
                    .get(next)
                    .ok_or_else(|| anyhow!("more bodies than functions"))?;
                let (func, n) = body(&mut re, f, params[ty as usize], stores)?;
                code.function(&func);
                points += n;
                next += 1;
                if next == funcs.len() {
                    out.section(&code);
                }
            }
            Payload::CustomSection(c) => {
                out.section(&wasm_encoder::CustomSection {
                    name: c.name().into(),
                    data: c.data().into(),
                });
            }
            other => {
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
    Ok((out.finish(), points))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::{
        ExportKind, ExportSection, FunctionSection, MemArg, MemorySection, MemoryType, TypeSection,
    };

    /// `nan(a, b) -> i32`: `(a / b).to_bits()`; `stored(a, b) -> i32`: `a / b` stored at 0 and read back as an int;
    /// `sign(a, b) -> f32`: `copysign(1, a / b)`. 0 / 0 makes x86's default NaN, 0xFFC00000.
    fn module() -> Vec<u8> {
        let mut m = Module::new();
        let mut t = TypeSection::new();
        t.ty()
            .function([ValType::F32, ValType::F32], [ValType::I32]);
        t.ty()
            .function([ValType::F32, ValType::F32], [ValType::F32]);
        m.section(&t);
        let mut f = FunctionSection::new();
        f.function(0);
        f.function(0);
        f.function(1);
        m.section(&f);
        let mut mem = MemorySection::new();
        mem.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        m.section(&mem);
        let mut x = ExportSection::new();
        x.export("nan", ExportKind::Func, 0);
        x.export("stored", ExportKind::Func, 1);
        x.export("sign", ExportKind::Func, 2);
        m.section(&x);
        let arg = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut c = CodeSection::new();
        let div = [
            Instruction::LocalGet(0),
            Instruction::LocalGet(1),
            Instruction::F32Div,
        ];
        let mut a = Function::new([]);
        for i in div
            .iter()
            .chain(&[Instruction::I32ReinterpretF32, Instruction::End])
        {
            a.instruction(i);
        }
        let mut b = Function::new([]);
        b.instruction(&Instruction::I32Const(0));
        for i in div.iter().chain(&[
            Instruction::F32Store(arg),
            Instruction::I32Const(0),
            Instruction::I32Load(arg),
            Instruction::End,
        ]) {
            b.instruction(i);
        }
        let mut s = Function::new([]);
        s.instruction(&Instruction::F32Const(Ieee32::new(1f32.to_bits())));
        for i in div
            .iter()
            .chain(&[Instruction::F32Copysign, Instruction::End])
        {
            s.instruction(i);
        }
        c.function(&a);
        c.function(&b);
        c.function(&s);
        m.section(&c);
        m.finish()
    }

    /// (nan, stored, sign) of 0 / 0 on wasmtime with Cranelift's canonicalization `on` or off
    fn bits(wasm: &[u8], on: bool) -> (u32, u32, u32) {
        let mut c = wasmtime::Config::new();
        c.cranelift_nan_canonicalization(on);
        let engine = wasmtime::Engine::new(&c).unwrap();
        let module = wasmtime::Module::new(&engine, wasm).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        let i = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        let int = |s: &mut wasmtime::Store<()>, n| {
            i.get_typed_func::<(f32, f32), i32>(&mut *s, n)
                .unwrap()
                .call(s, (0.0, 0.0))
                .unwrap() as u32
        };
        let (n, st) = (int(&mut store, "nan"), int(&mut store, "stored"));
        let sign = i
            .get_typed_func::<(f32, f32), f32>(&mut store, "sign")
            .unwrap()
            .call(&mut store, (0.0, 0.0))
            .unwrap();
        (n, st, sign.to_bits())
    }

    #[test]
    fn the_pass_gives_the_canonical_bits_without_cranelifts_canonicalization() {
        let plain = module();
        let (canon, points) = canonicalize(&plain).unwrap();
        assert_eq!(points, 3);
        let deck = bits(&plain, true);
        assert_eq!(deck, (0x7FC0_0000, 0x7FC0_0000, 1f32.to_bits()));
        assert_eq!(bits(&canon, false), deck);
        assert_eq!(bits(&canon, true), deck);
        if cfg!(target_arch = "x86_64") {
            // without either, the CPU's NaN shows: x86's default NaN has the sign bit set
            assert_eq!(
                bits(&plain, false),
                (0xFFC0_0000, 0xFFC0_0000, (-1f32).to_bits())
            );
        }
    }
}
