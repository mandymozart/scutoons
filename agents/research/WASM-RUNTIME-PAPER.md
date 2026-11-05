Status 2026-11-05: research (#396) – measured on the dev PC; the owner chose option C (§4) on 2026-11-05 (decisions/deck.md [modules]); the build is a ticket of its own.

# WASM-RUNTIME-PAPER – why the deck's wasmtime is slower than V8, and what the LAC-26 paper says

## 1. The paper (Jaffe, "Leveraging WebAssembly as a Universal Distributable for Embedded Audio Applications", LAC-26, hal-05697689, CC BY 4.0)
- **Question:** can one wasm file be the distributable for third-party audio DSP on embedded devices (and desktop / web from the same
  file)? Target: Daisy Seed (STM32H7, Cortex-M7, 400 MHz), 128-sample blocks, mean of 100 calls. Modules from C/C++ (Emscripten): a FAUST
  spring reverb, a Gen~ phaser (4 SVFs + LFO), the Klon Centaur as wave digital filters, a NAM (neural amp) inference.
- **Runtimes, throughput vs native** (WAMR AOT / wasm2c→gcc / wasmi): reverb 0.88 / 0.86 / 0.08 · phaser 0.55 / 0.62 / 0.003 · Klon 0.23 /
  0.27 / unstable · NAM 0.07 / 0.29 / unstable. The native baseline is gcc `-Ofast` (fast-math: not IEEE, not bit-exact).
- **Findings:** an interpreter is not viable for real-time DSP; AOT (code made before playing) is, best for float-heavy code with predictable
  control flow and a memory footprint known at compile time; dynamic allocation hurts. Gains left: host-imported math intrinsics, better
  compiler pipelines, a specialised audio ABI; the AOT compiler belongs to each target's runtime / flash tool, not to the plugin author.
  Tooling: a browser IDE that compiles in the page (wasm-clang, wamrc compiled to wasm).
- **Not in the paper:** determinism (float bits, NaN payloads across CPUs), SIMD, JIT engines (V8, wasmtime / Cranelift), desktop CPUs, x86 vs
  Arm. Its "cross-platform" is portability of one file, not equal bits.
- **For us:** it confirms #306 – wasmi is out (ours: 50–77× behind V8), AOT is the path; the deck already is AOT (Cranelift + the `.cwasm`
  cache, `module-host/src/wasmtime_backend.rs`). Its "host-imported math" would break our bit-equality unless the host's math is the SDK's
  libm everywhere (ABI-V2 §6: libm compiled into the module is what makes native ≡ wasm). Our bounded memory (`MEMORY_LIMIT`, the freeze after
  `init`, "allocate in init") is the paper's "footprint known at compile time".

## 2. Our measurement (`cargo run --release -p module-bench -- variants`, results/variants/pc.md; PC: Zen 2, Windows 10)
Mean µs per sample (p50), median of 3 interleaved rounds, 2000 calls; every row's output bits equal the deck's AND Chrome's (FNV of every sample).

| variant (wasmtime 38 unless noted) | fm16 | analog ×2 | walk | vs deck | determinism |
|---|---|---|---|---|---|
| deck: Cranelift NaN canonicalization + epoch | 100.8 (86.1) | 779.9 (729.3) | 9.8 | 1.00× | the rule as is |
| canonicalization off | 46.0 (39.4) | 345.4 (302.2) | 4.5 | 0.46× / 0.44× | NaN bits = the CPU's |
| **NaN pass in the wasm** + canonicalization off | 51.7 (47.4) | 402.2 (379.4) | 5.0 | **0.51× / 0.52×** | canonical where observable |
| NaN pass + deck config (the pass's own cost) | 98.0 (86.7) | 826.7 (788.5) | 10.2 | 0.97× / 1.06× | – |
| epoch off (no budget – not shippable) | 86.2 (74.6) | 760.3 (719.4) | 11.2 | 0.85× / 0.97× | – |
| canonicalization + epoch off | 30.9 (30.2) | 300.4 (276.8) | 4.0 | 0.31× / 0.39× | – |
| spectre mitigations off / opt SpeedAndSize | 93.6 / 98.1 | 798.7 / 760.1 | 10.6 / 9.8 | ≈ 1× (noise) | – |
| modules with simd128 (deck config / canon. off) | 89.8 / 47.7 | 717.8 / 290.5 | 10.3 / 4.8 | 0.89× / 0.37× | lanes: see §3 |
| Chrome 152 (#306) / Node V8, plain | 39.1 / 42–46 | 291.4 / 274–307 | 6.9 / 5.3 | 0.39× / 0.37× | V8 never canonicalizes |
| **native Rust, no wasm** (`--only …`, results/variants/pc-native.md) | 35.6 (28.7) | 285.9 (247.0) | – | 0.37× / 0.35× | the same bits |
| Node V8 + NaN pass | 43.8 (40.3) | 326–333 (300) | 5.7–7.8 | p50 +0 % / +17 % | canonical where observable |

- **wasm itself costs ≈ nothing here:** native Rust (the module crates for this CPU, `deck_module::testing::Rig`) is as fast as Chrome and
  as wasmtime with both checks off (fm16 36 / 39 / 31, analog 286 / 291 / 300) – the deck's 2.7–2.8× over native is its two settings.
- **The gap is NaN canonicalization:** Cranelift's `enable_nan_canonicalization` adds a compare + select after every float operation; off,
  wasmtime is at V8's level (fm16 46 vs 39–46, analog 345 vs 274–307). The #306 probe's 30–45 % is 54–59 % here (means and p50).
- **The epoch check** costs 11–33 % on top (fm16 46 → 31, analog 345 → 300). It is the budget (the strict / show restart), so it stays; Chrome's comparable row
  is the loop counter (#306: 34 / 328), i.e. a budgeted V8 and a budgeted wasmtime without canonicalization are even. **The loop counter
  instead of the epoch on wasmtime** (`loops`, `nan-pass+loops`; results/variants/pc-loops.md): as dear as the epoch (fm16 104 vs 100, analog
  816 vs 825) – Cranelift keeps the counter global in memory –, 3–6 % cheaper on top of the pass (54 / 437 vs 58 / 452). Not worth a second
  mechanism on the deck: the epoch stays there, the counter is the browser's.
- **What the pass costs on wasmtime is its store points:** without them (`nan-light`: reinterpret + copysign only) analog is as fast as
  canonicalization off (727 vs 763, 1004 vs 1060 µs), fm16 unchanged – measured under load (another process on ~2 cores; ratios only, the
  native row 566 / 850 instead of 286). The light form needs the host to canonicalize the float outputs it reads and gives up int loads of
  stored floats (a module hashing its own state) – a weaker guarantee, so C keeps the store points unless the X1 / S22 run short.
- **Nothing else matters:** spectre flags, opt level, memory (the default 4 GiB reservation + guard pages already elide bounds checks), call
  overhead (one write in / one read out: walk's 4–10 µs whole). Instance reuse is what the host already does. No speed-only, bit-equal
  setting gives a measured gain – so deck-module-host's default is unchanged (only `deck_config()` / `from_config()` for the harness).
- **SIMD:** the modules are built without simd128 (harness and build service alike); V8 does not vectorize scalar wasm. With LLVM's
  auto-vectorizer (+simd128) analog gains 8–16 % (wasmtime) / 6 % (V8), fm16 nothing; same bits on these fixtures.

## 3. Determinism: where a NaN's bits can be seen (the unit test in `deck/module-bench/src/nan.rs`)
- Which results are NaN is IEEE-exact everywhere; only sign and payload are the CPU's: x86 makes 0xFFC00000 (sign set), Arm 0x7FC00000. On
  this PC, 0/0 with canonicalization off gives 0xFFC00000 for `to_bits`, for a stored-then-int-loaded value, and copysign gives −1.
- **V8 never canonicalizes.** So today the browser is NOT bit-exact for a module that makes a NaN and stores it or reads its bits: the S22
  (Arm) and the PC (x86) can differ. The deck's Cranelift setting makes decks agree with each other, not with the browser.
- A NaN's bits are only observable where a float becomes bits: `f32/f64.store` (state, outputs), `i32/i64.reinterpret` (`to_bits`, libm),
  and copysign's sign operand. Everything else keeps a NaN a NaN or reads only its NaN-ness (compare, `trunc_sat` → 0, `trunc` traps).
  **The NaN pass** (a wasm → wasm rewrite like the loop counter: `t == t ? t : canonical` at those points; fm16 31, analog 299 points) gives
  the canonical bits on any engine and CPU, since none of the rewritten points depends on the engine; the unit test proves it for wasmtime
  on x86 (canonicalization on = off + pass ≠ off alone); an Arm run (S22 / a Pi) is still to do.
- **"Canonicalize only the outputs" (host side) is not enough:** a module can branch on a NaN's sign (`is_sign_negative`, `to_bits` in libm,
  copysign) or keep it in its state; **"the SDK math never makes NaNs"** cannot cover member arithmetic (0/0, ∞ − ∞).
- Limits of the pass: refuses SIMD (lanes are reinterpreted without an instruction – a SIMD version would canonicalize every float lane op);
  float params / results across the host boundary are not covered (the module ABI has none). Needs review before it is a build-service step.

## 4. Options and recommendation (not a decision)
- **A – keep as is:** decks ≈ 2× slower per block than with C; browser NaN bits still the CPU's.
- **B – canonicalization off, nothing else:** −54 % on decks; NaN bits become the CPU's on decks too (the rule weakened, x86 deck ≠ Arm deck).
- **C – the NaN pass in the build service + canonicalization off on decks:** −48–49 % on decks, +0–17 % in V8, and the rule holds on every
  runtime incl. the browser (today it does not). Cost: one more wasm pass beside the loop counter, a review of its completeness.
- **D – C + simd128 for modules:** another 8–16 % on analog-like code; needs a SIMD-aware pass (per-lane canonicalization) first.
- **Recommendation: C – decided by the owner 2026-11-05** ("nan pass option C is also great."): the pass moves into the build service
  (#407; #344's pipeline), ABI-V2 §6 and `deck_config` change with it, conformance hashes guard it. D later if the X1 / S22 run short. Epoch stays.
