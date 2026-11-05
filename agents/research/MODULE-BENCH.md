Status 2026-11-04: research – the #306 harness is built (deck/module-bench) and ran on the dev PC (native, Node, Chrome); the X1 and the S22 are the owner's step (§2). No decision recorded: the owner decides the runtime for #290 from §1 + the device numbers.

# MODULE-BENCH – which runtime runs modules where (#306)

## 1. How modules run in the browser for development (≤ 60 lines; PC numbers, X1 / S22 to be filled)
Every runtime ran the same Rust suite through deck-module-host's `Backend` (io staging, strict / show, restart unchanged), so a row
differs only by the wasm engine. Per 128-frame call, mean µs (budget 2667 µs); PC = the dev PC (AMD Zen 2, family 23 model 113, 12 threads, Windows 10), Chrome 152 cross-origin isolated (5 µs clock):

| runtime (PC) | fm16 | analog ×2 (16 voices) | walk | fm16 p99 % | runaway stopped after | show restart | to be filled: S22 fm16 p99 % · X1 fm16 mean |
|---|---|---|---|---|---|---|---|
| wasmtime native (the deck) | 103 | 882 | 11 | 6.2 % | 1.6 ms (epoch) | 0.12 ms | – · X1 |
| wasmi native | 396 | 2196 | 31 | 23 % | 12.6 ms (fuel) | 0.40 ms | – · X1 |
| Chrome WebAssembly direct | 39 | 291 | 7 | 3.2 % | 5.2 ms (watchdog) | 37 ms (new worker) | S22 · X1 |
| Chrome + gas metering | 76 (1.9×) | 849 (2.9×) | 11 | 5.6 % | 19 ms (gas) | 0.14 ms | S22 · X1 |
| Chrome + loop counter | 34 (0.9×) | 328 (1.1×) | 6 | 3.0 % | 1.7 ms | 0.14 ms | S22 · X1 |
| Chrome, wasmi compiled to wasm | 3005 (77×) | 14605 (50×) | 216 | 168 % | 16 ms (fuel) | 0.18 ms | S22 · X1 |

- **Bit-equal everywhere:** every output sample of fm16, walk and analog ×2 has the same bits on all 10 PC runs (wasmtime, wasmi, Node and
  Chrome in all four modes). Determinism does not decide between the runtimes.
- **Native is not faster per block on the PC:** wasmtime is 2.6–3× slower than Chrome's V8 (fm16 103 vs 39 µs, analog 882 vs 291 µs). A probe with
  the deck's `cranelift_nan_canonicalization` off (not committed) cut wasmtime's p50 by 30–45 % (fm16 60, analog 483 µs) with the same bits –
  still ~2× V8. "The native editor runs faster" holds for builds (a module compiles in 58 ms cold, `cargo build` of a module in seconds), not
  for playing. → X1: the same comparison on the deck's own machine.
- **Gas metering fails the 1.5× gate** (Chrome 1.9× / 2.9×, Node 4.8×); **the loop counter passes** (≈ 1.0×): a decrement + check at every
  function entry and loop head, inserted by the build service, stops a runaway in-thread (1.7 ms) and allows the show's restart (0.14 ms).
  Its unit (loop iterations) is deterministic but coarse; time budgets via units are approximate (gas / fuel stopped a `loop {}` at 6–19 ms
  for a 2 ms budget calibrated on fm16).
- **wasmi in the browser fails twice:** 50–77× slower than V8 (analog ×2 = 726 % of the budget on the PC) and **+294 KB gz** (gate 150; live-core
  is 498 KB gz today). Native wasmi is only 2.5–4× behind wasmtime – it is the interpreter compiled to wasm that collapses.
- **The browser cannot refuse `memory.grow`:** the web backend sees the growth after the call and faults the instance (1 MB grew, strict
  silenced it); wasmtime / wasmi refuse (-1). The build service can declare the memory's maximum (`MEMORY_LIMIT`) so V8 refuses past the cap.
- **wasm-instrument is unmaintained** (0.4.0, 2022-11; parity-wasm archived) and cannot read any SDK module (opcode 0xFC from the precompiled
  std, even in an MVP build); the harness re-does its mutable-global scheme on wasmparser + wasm-encoder (`deck/module-bench/src/gas.rs`).

**The options** (what #290 and the build service need; the numbers that decide)
- **A – browser WebAssembly directly, preempted by instrumentation.** #290: the `web` backend in live-core (this harness's `web` crate: 12
  imports, no JS logic) + setting / reading the counter global + the show restart; build service: the instrumented copy (loop counter;
  metering only if a cost unit is wanted in the browser) and the memory max. Decides: S22 fm16 / analog p99 with the loop counter ≤ 25 %.
- **B – wasmi in the live-core worker with fuel.** One VM, fuel everywhere – but out on the PC numbers already (50–77×, +294 KB gz).
- **C – no browser execution: the editor as a native app on the host computer** (wasmtime, local `cargo` builds, linked to the creator's
  content over the API). #290's browser backend is not needed for development; a native editor app is new work (Rust, deck-native's stack).
  Decides: X1 wasmtime vs X1 browser, the owner's wish for local builds.
- **C′ – C, and live/deck plays only final builds** (owner 2026-11-04: compiled, published, strict-tested by the build service). The browser
  then needs only the `web` backend on its own WebAssembly – no Play / audition, no browser strict mode, no wasmi, no gas metering. Is
  preemption still needed for trusted builds? The tests prove the tested inputs, not every input (a data-dependent loop, a NaN state); without
  it the only stop is terminating the worker – in live-core that is the whole engine (all sound; ≥ 37 ms for a new worker on the PC + the
  engine's start), against the show rule "restart or accept, never cut". The loop counter costs ≈ nothing and gives the deck's in-thread
  restart (0.14 ms), so keep it, added by the build service. #290 shrinks to: web backend + counter + show restart + the memory max check.
  Runtimes that still matter: wasmtime native (decks, native editor, build service) and the browser's WebAssembly with the loop counter;
  wasmi and metering drop out (a deterministic cost number, if wanted, is the build service's CI number from wasmi / wasmtime fuel natively).
- **D – hybrid:** audition in the browser via A (B is out), project builds on the build service or locally. Needs A's #290 plus C's builds.

**Recommendation (not a decision):** C′ with the loop counter – the numbers take B and metering out, the native editor gives the local
builds the owner wants, and the browser keeps the smallest runtime surface (one backend, no interpreter, no second strict mode). If the S22
shows web + loop counter fm16 p99 ≤ 25 % and analog ×2 within budget, D's browser audition is a later add-on on the same backend, not a
second design. Separately: the deck's NaN canonicalization is the whole wasmtime-vs-V8 gap – #396, **research/WASM-RUNTIME-PAPER.md**
(the variants, a NaN pass that keeps the bits on every runtime at about half the deck's cost, the LAC-26 paper).

## 2. What the owner runs (X1, S22) – about 10 minutes
1. On the PC, once: `cd deck` → `cargo run --release -p module-bench -- prepare --exe` (≈ 10 min; builds the modules, the web suite, the X1's
   binary), then from the repo root: `node --experimental-strip-types deck/module-bench/www/serve.ts --lan` (it prints the URLs and the
   firewall rule for ports 8100–8101 – once, elevated). Ctrl+C stops it at the end.
2. **S22 (Chrome):** open `https://<PC-IP>:8100/?label=s22` → certificate warning → Advanced → proceed → **Run** (≈ 2 min, screen on, page in
   front). The line "cross-origin isolated: yes" should show. At the end the log says "sent to the PC: results/s22-chrome.json" (else: Download
   and send the file).
3. **X1** (boot Windows: F12 → Windows Boot Manager): in Edge open `https://<PC-IP>:8100/?label=x1` → Run. Then in PowerShell:
   `curl.exe -o module-bench.exe http://<PC-IP>:8101/dist/module-bench.exe` and
   `.\module-bench.exe run --label x1 --post "http://<PC-IP>:8101/results?label=x1-native"` (unsigned: SmartScreen → Run anyway).
4. On the PC: `cd deck` → `cargo run --release -p module-bench -- compare`. Look at: the S22's `web` / `web-loops` fm16 p99 % (gate ≤ 25 %) and
   analog ×2 p99 %, the S22 watchdog restart, x1 `wasmtime` vs `x1-edge` `web` means, and "Bit equality". Commit `deck/module-bench/results/`.

## 3. The harness (deck/module-bench; a tool crate, `tools` in scripts/deck-deps.json)
- `src/lib.rs` the suite (generic over `Backend`), `src/report.rs` the table + gates, `src/wasmi_backend.rs` wasmi 2 behind `Backend` (fuel as
  the budget, calibrated on fm16), `src/gas.rs` the metering and loop-counter passes, `src/main.rs` prepare / run / compare; `web/` the same
  suite as a raw wasm with the `web` backend (+ wasmi with `--features wasmi`); `modules/fm16` (16 two-operator true-FM voices) and
  `modules/stress` (`loop {}`, 1 MB `memory.grow` a call); `www/` bench.ts (the imports, the worker runs, the watchdog), page.ts, node.ts,
  serve.ts (LAB ports 8100–8101; `.ts` served type-stripped, COOP / COEP, `POST /results`).
- PC quick runs: `cargo run --release -p module-bench -- run` (native) · `node --experimental-strip-types deck/module-bench/www/node.ts` (V8 in
  Node) · `node --experimental-strip-types deck/module-bench/www/serve.ts` + http://127.0.0.1:8100/?label=pc (this browser).
- #396: `-- variants` (wasmtime settings × module builds → results/variants/), `src/nan.rs` (the NaN pass, `.nan.wasm`), `.simd.wasm`
  (+simd128); `node.ts --runtimes web,web-nan,web-simd` runs those in V8.
- Measured: 2000 calls of 128 frames (250 blocks of 1024 for fm16), p50 / p99 / max / mean per sample, compile + instantiate, fuel / gas /
  counter units per sample, an FNV hash of every output sample, peak linear memory; the runaway (strict 2 ms budget → the stop; a show's
  rebuild); the growth after the freeze. Sizes: `www/dist/sizes.json` (live_core.wasm when `npm run build:wasm -w shared/live-core` ran).
- Results: `results/pc-native.*`, `pc-node.*`, `pc-chrome.json` (2026-11-04). The old deck/lab numbers (fm16 native 34.5 µs, Chromium 49.8 µs)
  were another FM kernel without the module host – not comparable.
