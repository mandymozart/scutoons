Status 2026-11-14: research (#523); proposals 1, 2 (in part) and 3 built (#524, "Built (#524)" under §4's first three rows); 4–10 open.

# Where deck:ci's time goes, and how to cut it

Read-only research (no build, no test, no Docker – another worker was running deck:ci). Sources: `scripts/deck-ci.mjs`, `deck/Cargo.toml`,
`deck/.cargo/config.toml`, the test sources, `cargo metadata` / `cargo tree`, `git log`, two run logs of 2026-11-13 (run A `deckci.log` 3473 s,
run B `deck-ci.log` 3653 s; the 2925 s run is not in them). **The logs hold one line per step and nothing finer** (1.2 KB each): there is no per-crate,
per-test or build-vs-run split anywhere in them. Everything below the step level is read from code and marked as an estimate; section 5 lists the commands that measure it.
Machine: Ryzen 5 3600X (6 cores / 12 threads), 64 GB, Windows, rustc 1.99.1; no sccache, no cargo-nextest installed; `lld-link` is in `C:\Program Files\LLVM\bin`.

## 1. What deck:ci runs (steps strictly one after another: `spawnSync` loop, deck-ci.mjs:124-126)
| # | step | A s | B s | what it really is |
|---|---|---|---|---|
| 1-3 | core-only · deck-deps · protocol schema | 12 | 10 | node scripts, trivial |
| 4 | kind catalog current | 76 | 66 | `cargo run -p deck-space --features schema` – dev build of deck-space in a feature set of its own (:65) |
| 5 | live-core wasm32 simd128 (+ gz gate) | 112 | 145 | release, fat LTO, own target dir `live-core-simd` (:68) |
| 6 | live-core tests (TS) | 3 | 3 | |
| 7 | deck-engine wasm32 simd128 (+ gate) | 144 | 137 | release, fat LTO, own target dir `engine-wasm` (:78) |
| 8 | space-web wasm32 | 73 | 144 | profile `wasm-web` = release (fat LTO, 1 CGU), opt-level s (:87) |
| 9-11 | deck-outlet · deck-show · deck-ui wasm32 | 87 | 94 | three `cargo build` of the dev profile for wasm32 (:95-99) |
| 12-13 | relay binary (debug) · relay-web wasm | 54 | 108 | |
| 14-15 | live-deck tests · clock tool (TS) | 11 | 11 | |
| 16 | **release build** (`cargo build --release`, workspace) | 574 | 616 | fat LTO, 1 CGU for every binary (Cargo.toml:40-45) |
| 17 | headless scenarios | 76 | 83 | `cargo run -p deck-headless --bin deck-scenarios`, 42 scenarios |
| 18 | **tests** (`cargo test --workspace`) | **1958** | **1952** | 56 % / 53 % of the run |
| 19-20 | module tests · engine module tests | 181 | 180 | deck-native `--features modules`; deck-engine `--lib module` |
| 21-23 | clippy · module clippy · engine module clippy | 108 | 103 | three separate `cargo clippy` |
Shares of run A: tests 56 %, release 16.5 %, steps 4-15 (wasm + checks) 16 %, module tests 5 %, clippy 3 %, headless 2 %.
Noise from parallel runs: space-web 73 s vs 144 s, relay binary 0 s vs 30 s – a single step varies up to 2x, the total only 5 %.
Build vs run is only estimable for step 18: 45 packages = 57 integration-test binaries + the lib/bin test binaries (up to 45 MB each, full debuginfo, a PDB each),
linked and then run **one binary at a time** (cargo's rule), plus a doc-test pass per lib crate. The 596 s "finished in" for deck-native is libtest's per-**binary** total.

## 2. The slow pieces, ranked
1. **Step 18, the tests (~33 min).** Unmeasured split; four causes found in code, in order of likely weight:
   a. *Nested wasm release builds inside the tests* – six fat-LTO wasm builds per run: `conformance/tests/wasm.rs:16-30` called at :196-197, :240-241, :283-284, :321-322
      (live-core plain + simd, deck-engine-wasm plain + simd) and `headless/tests/wasm.rs:9-25` (deck-headless plain + simd), each into `target/conformance-wasm[-simd]`.
      The simd ones are the same crate, same RUSTFLAGS, same profile as steps 5 and 7 (112-145 s each there) – built a second time into another directory.
      Plus the module wasm builds: `native/tests/modules.rs:23-42` (6 crates), `module-host/tests/common/mod.rs:15-35`, `modules/analog/tests/wasm.rs:15-35`, `modules/walk/tests/wasm.rs:10-25`,
      `modules/stepseq/tests/stepseq.rs:145-160`, all `--release` (fat LTO) into `target/wasm-modules`, with different `-p` sets. Estimate 6-12 min whenever a deck source changed (usual).
   b. *Wasm renders in an unoptimised wasmtime*: `Engine::default()` (conformance/tests/wasm.rs:37, headless/tests/wasm.rs:70; Cranelift opt-level none) inside a test profile where wasmtime/cranelift
      themselves are opt-level 0 (only 8 of our own crates are raised, Cargo.toml:52-80). `engine_native_vs_wasm` (:319-362) renders 2 s of audio (common/mod.rs:9) for ~29 scenes x 3 block sizes x 4 renders
      (1 native + 3 wasm) + 6 routing cases x 3 x 4. headless/tests/wasm.rs compiles a module twice and re-runs 42 scenarios x 3 targets.
   c. *Link time*: no `[profile.dev] debug` setting, so full debuginfo + PDB on MSVC for ~100 test executables (the 114 GB `deck/target/debug` of the shared checkout is the symptom).
   d. *The 596 s binary is not identified.* By reading, nothing in deck-native's default tests has that much work (real waits are deadlines: PATIENCE 30-60 s, cathedral_relay 3 x <=60 s, far_events 20 s;
      sleeps total seconds; no_alloc is 6 serial tests). Suspects: a test whose loop runs at opt-level 0 in a crate Cargo.toml does not raise (deck-kit, deck-outlet, deck-link, deck-show, deck-events are opt-level 0),
      or the lib unit-test binary (94 tests). One measured run settles it (5.1).
2. **Step 16, the release build (~10 min).** Nothing later uses its output (live-deck's tests spawn the *debug* relay, :101). It can only fail on release-only code (cfg(debug_assertions), LTO/linker, abort).
   The pre-push hook already skips it (`--no-release`, scripts/git-hooks/pre-push), so the project already lives without it on every push.
3. **Steps 4-15 (~8-10 min), mostly single-thread LTO** while 11 of 12 threads idle; steps share nothing in time. `live-core-simd`, `engine-wasm` and `conformance-wasm-simd` are three target dirs with the same flags: deck-core / deck-engine / deck-space compile three times.
4. **Steps 19-20 (3 min):** nested wasm module builds again (a) and a relink of deck-native with `modules`; deck-engine rebuilt with `modules`.
5. **Clippy x3 (1.8 min), kind catalog (1.2 min), headless (1.3 min):** small; see clutter.

## 3. Clutter
- **Run twice:** the 42 headless scenarios – step 17 and `headless/tests/scenarios.rs` in step 18 (and once more native in `headless/tests/wasm.rs`). Step 17 only adds one printed line per scenario.
- **Built twice, different flags:** live-core and deck-engine simd wasm (steps 5, 7 vs the conformance tests); deck-space dev (`--features schema`, step 4) vs the workspace feature union (step 18) vs clippy; deck-native and deck-engine
  with/without `modules` (steps 19-20 and 22-23, each its own build and its own clippy pass; `cargo ... --workspace --features deck-native/modules,deck-engine/modules` would do both in one).
- **Subsumed:** deck-outlet and deck-ui wasm32 build checks (steps 9, 11): both are dependencies of deck-space (`cargo tree`), which steps 5, 7, 8 build for wasm32 (other profile; whether every feature path matches is unchecked).
- **Cannot fail from the usual change:** of the last 150 deck / shared-live commits, 29 touched only leaf crates
  (touch counts over all 150: native 46, editor 42, space-web 28, relay 16, shared/live-deck 30) that have <=4 dependents (`cargo metadata`) – for them steps 4-15, 16, conformance and headless cannot change result. The hub `deck/space` (15 dependents, touched by 56 of 150) and core / catalog / protocol (24-32 dependents) are the rest.
  A TS-only change in shared/live-deck also matches WATCH (deck-ci.mjs:24) and runs `cargo test --workspace`.
- **Worker AND orchestrator:** RULES F4/A13 – the worker runs deck:ci; ORCHESTRATION.md:46 – the orchestrator runs it again before every merge; the pre-push hook runs it a third time (`--no-release`) when the push touches deck/.
  After a fast-forward merge all three runs see the same tree: two to three runs of ~58 min prove one tree once.

## 4. Proposals, ranked by saving / effort (the run is ~58 min; "min" = minutes of one run)
| # | proposal | saving | sure | risk | size |
|---|---|---|---|---|---|
| 1 | **Green stamps keyed by content.** After a green step, store `git rev-parse HEAD:deck`, `HEAD:shared/live*`, `HEAD:scripts` (tree hashes) + step name; a later run (orchestrator, pre-push) with equal hashes skips that step. | 45-58 min per redundant run, 1-2 per merge | high | none for equal trees; a non-FF merge has other hashes and runs again | S |
| 2 | **Delete the duplicates:** step 17 (the test covers it), merge the 3 clippy runs and the 2 module test steps via `--features deck-native/modules,deck-engine/events,deck-engine/modules`, fold outlet + ui into one wasm `cargo build -p ... -p ...`. | 3-4 min | high | module tests join the workspace run (more in one binary set, feature union may show new clippy lints) | S |
| 3 | **Release build out of the merge path:** `cargo check --release --workspace` per merge (~1-2 min), the real fat-LTO build nightly and before a deck release / owner push. | 8-9 min | high | LTO / linker / size-only failures show up a day later (the pre-push hook already accepts this) | S |

**Built (#524)** – scripts/deck-ci.mjs + scripts/deck-ci-stamps.ts (its unit test: `npm run check:deck-ci`):
- **1, as proposed, finer:** the key is the blob hashes of every file under WATCH at HEAD (deck/, shared/live*, the deck-ci script, the stamps,
  the pre-push hook, the deck-deps files; an npm step adds package.json / package-lock.json, the clock step tools/clock-measure) + step + tier +
  `rustc -V` / node / RUSTFLAGS; stamps in `<git common dir>/deck-ci-stamps/` (every worktree), pruned after 30 days. A dirty or untracked WATCH path:
  every step runs, nothing is stamped; `--fresh` / `DECK_CI_FRESH=1` ignores them; `--plan` prints run / skip without running. Never stamped: core only
  (it scans viewer / creator / admin / tools and compares with main's baseline). The pre-push hook runs the default tier, so it reuses the stamps.
- **2, in part:** the headless scenarios step is gone (headless/tests/scenarios.rs runs the same files through the same `run` / `summary` in the
  tests step); one clippy over `--features deck-native/modules,deck-engine/events,deck-engine/modules` (green as it was, no new lint; the
  `cfg(not(feature = "modules"))` lines – native/src/rig.rs, engine/src/walk.rs – are now compiled by the release check and the tests, no longer linted);
  deck-outlet + deck-ui in one wasm32 build. **Not joined: the module tests** – with deck-native/modules in `cargo test --workspace`,
  conformance's control_rate fails (`unknown kind 'module:stepseq'`: `deck_native::modules::carried_part` keeps what the stage built); joining
  them needs a test change, so both module test steps stay.
- **3:** the default tier runs `cargo check --release --workspace`; `deck:ci -- --full` builds release with fat LTO (end of every shift, before a
  deck release; RULES A13, ORCHESTRATION "Checks"). `--no-release` is accepted as the default tier (hooks installed before #524).

| step (s) | before A / B | after: run 1, cold worktree | after: run 2 | re-run, same tree |
|---|---|---|---|---|
| core only · deck-deps · protocol schema | 12 / 10 | 10 | 11 | 3 (core only) |
| kind catalog current | 76 / 66 | 99 | 13 | skipped |
| live-core wasm32 + simd128 · its tests | 115 / 148 | 146 | 128 | skipped |
| deck-engine wasm32 + simd128 | 144 / 137 | 156 | 127 | skipped |
| space-web wasm32 | 73 / 144 | 175 | 91 | skipped |
| outlet · show · ui wasm32 (3 builds → outlet + ui, show) | 87 / 94 | 58 + 92 | 0 + 5 | skipped |
| relay binary · relay-web | 54 / 108 | 52 + 134 | 4 + 60 | skipped |
| live-deck tests · clock tool | 11 / 11 | 27 | 12 | skipped |
| release build → release check | 574 / 616 | 131 | 15 | skipped |
| headless scenarios | 76 / 83 | – (gone) | – | – |
| tests | 1958 / 1952 | failed (the union, above) | 2123 | skipped |
| module tests · engine module tests | 181 / 180 | – | 137 + 80 | skipped |
| clippy ×3 → ×1 | 108 / 103 | – | 10 (warm: a cold union clippy before took 67) | skipped |
| **total** | **3473 / 3653** | – | **2822** (steps 1–15 warm from run 1) | **3** (18 of 19 skipped) |

Read: the cuts remove ~10–11 min of steps from a first run (release −440 s, headless −80, clippy ~−40, one wasm build), but this first run in a
fresh worktree spent 2123 s in the tests (cold test binaries; A / B ran in worked-in trees) – a cold first run lands ~ at the old total
(1080 s for steps 1–15 + 2407 s for tests, modules, clippy). The win is the re-run: the orchestrator's run and the pre-push hook on a tree a
worker proved green take 3 s instead of ~58 min. Full tier (`--full`, run 3 on the same tree, everything warm; the tier is in the key, so it reuses no default stamp): green in 2516 s –
release build 663, tests 1375, module tests 36 + 9, clippy 9, the rest as run 2. On a warm tree the default tier is therefore ~1870 s (31 min:
2516 − 663 + 15) against 3473 / 3653 before.

| # | proposal | saving | sure | risk | size |
|---|---|---|---|---|---|
| 4 | **Conformance tests reuse the wasm deck:ci already built** (env `DECK_WASM_DIR` or a stamp file; own build only when unset, e.g. run alone); the plain variants on a `[profile.ci-wasm]` = release with `lto = "off"`, 16 CGUs; module wasm: one `-p` set for all five test crates so `wasm-modules` builds once. | 6-12 min | medium (the nested builds are certain, their cost is derived from steps 5 / 7) | simd: none (same artifact); plain: LLVM does not change float results without fast-math, bit-exactness is the test | M |
| 5 | **Fast tier by changed paths, full tier nightly / before the owner's push.** Fast = guards + `cargo test` of the changed crates plus their reverse-dependency closure (from `cargo metadata`) + workspace clippy; wasm / conformance / headless only if a hub crate (core, catalog, protocol, space, engine, live-core, outlet, ui, kit, events) or Cargo.toml/lock/profile changed. | 35-50 min on ~20-35 % of merges (29 / 150 leaf-only, 54 / 150 outside the space-core hub) = 8-15 min average | medium | a cross-crate effect the graph misses (build.rs, include_str! of files outside the crate, scripts) - make those paths "hub"; nightly full tier catches the rest a day later | M |
| 6 | **Build lanes in parallel:** run the independent builds (steps 5, 7, 8, 12-13 and, if kept, 16) concurrently, 3 lanes, each already has its own target dir; the tests step stays alone. | 5-9 min with the machine idle, 2-4 under a second session (B1) | medium | none for correctness; RAM (LTO) - the memory guard of ORCHESTRATION applies | M |
| 7 | **Dev profile for link and wasmtime:** `[profile.dev] debug = "line-tables-only"` (or `split-debuginfo = "packed"`), `linker = "lld-link"` in deck/.cargo/config.toml, `[profile.dev.package.wasmtime*] / cranelift*  opt-level = 3`, wasmtime `Config::cranelift_opt_level(Speed)` in the tests. | 3-10 min (link of ~100 exes + the wasm tests); first cold build gets slower | low-medium, measure first (5.2) | debugger experience (line tables still give backtraces); lld-link vs MSVC behaviour differences | S |
| 8 | **cargo-nextest** for the unit/integration tests (+ `cargo test --doc` apart): binaries run in parallel; many tests wait on sockets and sleeps. | 3-10 min of the run share | low (the run / build split is unknown) | per-test processes re-run every OnceLock'd nested cargo build (20 tests in modules.rs) - do 4 first; timing-sensitive tests (GOTCHAS: midi mock) flake more under load | M |
| 9 | **The slowest tests themselves**: once 5.1 names them - a stepped clock instead of real waits, fewer scenes x block sizes in the wasm sweep (the full sweep nightly). | up to 10-20 min (the 596 s binary alone is 10) | unknown until measured | less coverage per run if the sweep shrinks | M |
| 10 | **Shared warm cache for cold worktrees** (all workers build cold today): one `CARGO_TARGET_DIR` for the one-worker-at-a-time rule, or sccache for the registry crates (wgpu, wasmtime, egui, naga, image). | cold build of the registry crates, my guess 5-12 min per fresh worktree | low | tests and deck-ci.mjs hard-code `deck/target` (conformance, headless, modules tests, `--target-dir target/engine-wasm`) – they must honour the variable; sccache needs `CARGO_INCREMENTAL=0` and keys path crates by absolute path | L |
**First:** 1, 2, 3 (together ~50 % off a re-run, ~12 min off a first run, a day of work), then 4, then the one measured run of 5.1-5.2 to decide 7-9, then 5 and 6.
**Not first / not at all:** 10 (largest effort, path-sensitive, gain unmeasured); nextest before the slow tests are named; dropping clippy (cheap, 108 s) or the conformance suite (it is the native = wasm contract, A4);
GitHub runners (owner 2026-10-31 [deck-ci-local]); a Defender exclusion for the target dirs is a security setting – the owner's to decide (often 20-40 % on Windows builds, unmeasured here).
Rough landing: 1+2+3+4+6 take a first run from ~58 to ~30-35 min and a re-run to ~0; 5 takes a leaf-only merge to ~10-15 min.

## 5. What I could not measure (and the command)
1. Per-binary run time and the 596 s binary: `cd deck && cargo test --workspace --no-fail-fast 2>&1 | tee tests.log` then `grep -E "Running|has been running|finished in" tests.log` (libtest prints both lines, stable).
2. Build vs link vs run per crate: `cargo test --workspace --no-run --timings` (HTML in `target/cargo-timings`); the debuginfo effect: the same with `CARGO_PROFILE_DEV_DEBUG=line-tables-only` in a scratch `CARGO_TARGET_DIR`.
3. Cost of the nested wasm builds: touch `core/src/lib.rs`, then `RUSTFLAGS="-C target-feature=+simd128" cargo build --release --target wasm32-unknown-unknown -p live-core --target-dir target/conformance-wasm-simd` (and -p deck-engine-wasm, -p deck-headless --lib).
4. Cold-worktree cost: `cargo build --workspace --timings` and `cargo build --release` in an empty target dir, machine otherwise idle.
5. Whether another session's load explains B vs A (3653 vs 3473 s): needs the two sessions' overlap, not in the logs.

## 6. Open questions for the owner
1. Is a nightly full tier (release build, full conformance sweep, all wasm) acceptable, with a path-chosen fast tier before every merge?
2. May the release build leave the per-merge path (a `cargo check --release` stays)? The pre-push hook already skips it.
3. May the native = wasm conformance suite run only when core / engine / space / live-core / protocol change, and nightly?
4. May I install tools here (cargo-nextest, optionally sccache) and change the dev profile / linker for deck/? Excluding the target dirs from Windows Defender is yours.
5. Should one deck:ci use up to 3 build lanes when it is the only session building (B1 allows two sessions, memory guard)?
