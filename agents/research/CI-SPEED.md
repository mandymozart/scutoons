Status 2026-11-14: research (#523); proposals 1, 2 (in part) and 3 built (#524, "Built (#524)" under §4's first three rows); 7 in part (rust-lld, line tables: #525, "Measured (#525)" at the end); 10 built (#526, "Design (#526)" + "Built (#526)" at the end); 7's wasmtime half built (#536: the two wasm tests 331 / 76 s → 13 / 9 s, "Measured (#536)"), 4 cut there; 5–6, 8–9 open.

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
   b. *Wasm renders in an unoptimised wasmtime*: `Engine::default()` (conformance/tests/wasm.rs:37, headless/tests/wasm.rs:70) inside a test profile where wasmtime/cranelift
      themselves are opt-level 0 (only 8 of our own crates are raised, Cargo.toml:52-80). (Corrected by #536: `Engine::default()` already compiles at Cranelift's opt level `speed`;
      the cost was Cranelift itself running unoptimised – ~100 s per module – and the wasm rebuilt on every run, "Measured (#536)".) `engine_native_vs_wasm` (:319-362) renders 2 s of audio (common/mod.rs:9) for ~29 scenes x 3 block sizes x 4 renders
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

## Measured (#525)
`cd deck && cargo test --workspace --no-run --timings`, cold (`target/` empty / `cargo clean`), 2026-11-13, worktree of main 5a19c967, 685 units both times.
Change: `[profile.dev] debug = "line-tables-only"` (deck/Cargo.toml) + `[target.x86_64-pc-windows-msvc] linker = "rust-lld.exe"` (deck/.cargo/config.toml).
`rust-lld.exe` is LLD in its lld-link mode (`-flavor link`, LLD 23.1.1), shipped in the rustup toolchain, so no machine depends on `C:\Program Files\LLVM` (LLD 18.1.4).
split-debuginfo is no option: on windows-msvc only "packed" (the PDB) exists, which is what it already did.

| | wall | cargo "Total time" | sum of unit times | test / bin / example units (125, where the link sits) | `target/debug` |
|---|---|---|---|---|---|
| before (MSVC link.exe, full debuginfo) | 615 s | 613.8 s | 6088 s | 3860 s | 29.58 GB |
| after (rust-lld, line-tables-only) | 582 s | 581.3 s | 5270 s | 2688 s | 11.30 GB |
| change | −5 % | −5 % | −13 % | −30 % | −62 % |

**Busy machine, both runs:** another worker's deck:ci built at the same time (its own cargo + up to 13 rustc). Sampled every ~40 s: before, foreign rustc ≥ 5 in 6 of 13
samples (≥ 12 in 5); after, ≥ 5 in 8 of 14 (≥ 12 in 5), its link.exe in 8 of 14 samples (before: 4 of 13). The after run had at least as much competition, so the wall time
saving is a lower bound, but ±10 % noise is plausible: trust the direction and the size (exact, no load effect), not the 33 s. The unit-time sums count
time spent waiting for a CPU too. An idle-machine repeat would sharpen the seconds; the size is final.
Backtraces: a deliberately failing scratch test (not committed) with `RUST_BACKTRACE=1` still prints `at .\tests\<file>.rs:<line>` per frame.

## Design (#526)
Goal (issue): a fresh worktree's first build does not rebuild the registry crates; all worktrees' build output in one place, one command to see and clear it; two sessions at once keep working.
**A plain shared `CARGO_TARGET_DIR` is out – measured wrong, not only slow.** Cargo keys a workspace crate by its path *relative to the workspace root* (`-C metadata`,
the fingerprint, dep-info paths relative to the package root): two worktrees are the same unit to it and freshness is mtime only. Scratch test (two copies of a
one-crate workspace, one target dir): B's edit made before A's build was never compiled – B's `cargo test` ran A's code and `env!("CARGO_MANIFEST_DIR")` named A.
In the deck that means a worker's tests (and a green stamp) on another worker's code, `debug/relay.exe` and `debug/deck.exe` overwritten across worktrees, and the
conformance tests' nested wasm builds run in the other worktree. The lock is not the problem: cargo holds the build-dir lock only while compiling, not while tests run
(measured: a second build in the same dir did not wait for a running 15 s test), so two workers would wait for each other's compile phases only – irrelevant once wrong.
**Built instead: one root, a target dir per worktree inside it, sccache for the registry crates.**
- **Where:** `<git common dir>/deck-target/` (here `C:\Development\scutoons\.git\deck-target\`), next to deck:ci's stamps: every worktree derives it with
  `git rev-parse --git-common-dir`, no per-machine setting in the repo, no tool scans `.git/`, the working trees stay clean. Inside: `<worktree id>/` (the id is
  the basename of `git rev-parse --git-dir`, `.git/worktrees/<id>` – gone when the worktree is removed, which is how a stale build is recognised) and `sccache/`.
- **How a worktree finds it:** an untracked, git-ignored `<worktree>/.cargo/config.toml` (cargo reads it above `deck/.cargo/config.toml`, which stays as is):
  `build.target-dir = <root>/<id>`, `build.rustc-wrapper = <sccache.exe>` when sccache is installed, `[env] SCCACHE_DIR = <root>/sccache` (+ size cap, idle timeout).
  Written by `scripts/deck-target.ts` from the root `postinstall` (every worker runs `npm install`) and again at the start of every deck:ci; never in the main checkout
  (`--git-dir` = `--git-common-dir`) nor in a plain clone (GitHub). So a worker's hand-run `cargo test` lands there too; `CARGO_TARGET_DIR` still overrides (cargo's rule).
- **Why sccache, not shared artifacts:** sccache keys a compile by its inputs' content and absolute paths – the registry sources (`~/.cargo/registry`) are the same
  path for every worktree, so their rlibs hit across worktrees; workspace crates are incremental (sccache never caches those) and have per-worktree paths, so they can
  never be served across worktrees – exactly the line the scratch test says must hold. Not cached by sccache: proc-macro / bin crates and build scripts (linked per
  worktree), the links of the ~100 test executables. Per-worktree disk stays (~11 GB debug after #525 + the wasm dirs) – the gain is one place and a clear command, plus
  the cold registry compile. Two sessions at once: separate target dirs (no build-dir lock between them), one sccache server shared by both (it serves concurrent clients).
- **Sub target dirs:** `live-core-simd`, `engine-wasm`, `conformance-wasm[-simd]`, `wasm-modules`, `conformance/` reports all live inside the worktree's target dir
  (the root's `<id>/`). `target-v3` (the `stage-v3` alias, a relative `--target-dir`) stays beside deck/: owner-only release builds in the main checkout.
- **What follows the location:** Rust integration tests take the target dir from `env!("CARGO_TARGET_TMPDIR")` (= `<target>/tmp`, set by cargo for every integration
  test, whatever chose the dir) instead of `deck/target` (conformance wasm + reports, headless wasm, the module tests' `wasm-modules`); module-bench from its own exe.
  Node: one resolver, `deckTarget()` in shared/live-core/scripts/wasm.ts (`CARGO_TARGET_DIR`, else `cargo metadata`'s `target_directory`, 0.2 s), used by deck-ci.mjs
  (gates, `--target-dir …/engine-wasm`), live-core's and live-deck's wasm scripts, live-core's tests, and the relay tests of live-deck and the viewer (`debug/relay.exe`).
  Owner-only hardware scripts (native/run-*.ps1, os/*.sh) run in the main checkout, whose target stays `deck/target`: unchanged.
- **The main checkout:** keeps `deck/target` (114 GB, the owner's own builds) – not moved, not cleaned. Recommended: leave it where it is (no cold rebuild, the
  run-*.ps1 scripts keep working) and clear it once when it suits him (`--clear --main`): built again after #525 it is ~1/3 of the size.
- **One command:** `npm run deck:target` lists the root (per worktree: size, alive / stale / this one), sccache, the old `deck/target` of each worktree and the main
  checkout's (reported only). `-- --clear` deletes the stale worktree dirs; `--all` the whole root (every worktree's build + sccache; only when no session builds);
  `--legacy` the worktrees' old `deck/target`; `--main` the main checkout's `deck/target` – only that flag ever touches it.
- **Stamps (#524):** the location is not in the key: a step's result does not depend on where its artifacts go (the gates read files the same step just built); sccache
  returns a compile's output for the same inputs, so it is not in the key either.

**Built (#526)** – scripts/deck-target.ts (+ its test in `check:deck-ci`), `npm run deck:target`, the root `postinstall`, deck-ci.mjs, `deckTarget()` in
shared/live-core/scripts/wasm.ts, `CARGO_TARGET_TMPDIR` in the Rust tests; sccache 0.19.0 installed here (`cargo install sccache --locked --no-default-features`, local disk only).
- As designed, plus **`SCCACHE_BASEDIRS` = every worktree's target dir** (in each config; sccache's server reads it when it starts): without it a second worktree hit
  289 of 386 Rust compiles (the registry crates whose build scripts write into OUT_DIR missed), with it 354; 32 miss in every worktree (not traced).
- **Exception: space-web's wasm build runs without sccache** (`RUSTC_WRAPPER=` in deck-ci.mjs and live-deck's build-wasm.ts): web-sys's rustc call (wgpu's features)
  is ~90 KB, cargo hands it over as an @argfile, sccache expands it past Windows' 32 K command line (os error 206). GOTCHAS has the hand-run case.
- Proofs, 2026-11-14, the machine shared with other workers' builds (`cargo test --workspace --no-run` unless named):

| run | wall | sccache Rust hits / misses |
|---|---|---|
| this worktree, empty target + empty cache | 382 s | 0 / 386 |
| a second fresh worktree (`git worktree add`), no base dirs yet | 318 s | 289 / 97 |
| both targets `cargo clean`ed, base dirs set: this worktree / the second | 274 / 282 s | 354 / 32 each |
| core/src/lib.rs touched: both worktrees at once / one alone | 62 s + 62 s / 30 s | – |
| `npm run deck:ci` default tier (first green run of #524 + #525 + #526 together) | 2146 s | tests 1200 s |

  Read: a fresh worktree saves ~100 s of a ~380 s cold test build (~27 %); the rest is what must not be shared – the workspace crates (135 incremental compiles), bins,
  proc-macros, build scripts and the ~100 links. Two builds at once: no wait between them (separate target dirs; only cargo's package-cache lock, seconds), the CPU is shared.
  deck:ci's two failed tries were #526's own (the space-web exception above; build-wasm.ts over its check:core-only budget) – nothing from #524 / #525.
- Disk after the proofs: the root 26.1 GB (this worktree 24.0 GB with all wasm dirs, sccache 2.1 GB of its 20 G cap); the second worktree's 13.7 GB deleted by `-- --clear`
  (21 s, sizes included). Still outside the root, reported, untouched: two worktrees' old `deck/target` (77 GB) and the main checkout's (156.5 GB).

## Measured (#536)
The two slow tests alone: `cargo test -p deck-headless --test wasm` and `-p deck-conformance --test wasm`, one after the other, 2026-11-14, this machine,
seconds = libtest's "finished in" (the test binary was built before). Each step ran twice with a `git status` between; the second run is the measure.
Both tests now print where their time goes (`built in` – cargo's nested wasm build, `compiled in` – Cranelift, `ran in` – the renders).
**Machine:** shared all day – a `cargo build --release` and a `cargo test --workspace` of other sessions ran during the baseline, 11–16 foreign
rustc / cargo during the last step; the seconds are ±20 %, the splits say where the time went.

**Where the time went (baseline, headless split):** native 1.5 s, the two wasm runs 2.5 + 1.3 s – the renders are not the cost. Cranelift's compile of
the 5.4 MB `deck_headless.wasm` 98.6 + 98.9 s, and the nested cargo builds 186 + 227 s – **on every run**, not "once per change" (two causes, below).
`Engine::default()` already compiles at Cranelift's opt level `speed` (wasmtime 38 `Config::new`), so `cranelift_opt_level(Speed)` is a no-op – not added.

| step | headless wasm | conformance wasm | what |
|---|---|---|---|
| baseline, warm | 331 | 75.6 | (cold, empty wasm dirs: 787 / 303) |
| 2 · wasmtime + Cranelift `opt-level = 3` (19 crates, deck/Cargo.toml) | 132 | 15.5 | compile per module 98 s → 7–8 s (live-core 2.4, deck-engine-wasm 2.0); headless still rebuilt 52 + 61 s |
| 3a · fixed label `DECK_BUILD_VERSION=wasm-test` for the nested builds | 149 | 17.6 | conformance's builds fresh (0.8 s); headless still rebuilt 65 + 60 s |
| 3a · headless into its own dirs (`headless-wasm*`) | 20.6 | 12.4 | headless built 0.3 + 0.4 s, compiled 8 + 9 s |
| 3 + 5 · modules compiled once per binary, wasm32 and simd128 built + compiled on two threads | **13.3** | **8.9** | headless 9.3 ‖ 9.2 s compile; conformance 4 compiles instead of 8 |
| the same after `touch core/src/lib.rs` (a deck:ci after a deck change) | 78 | 27.5 | headless built 64 ‖ 65 s, conformance 16–20 s (sccache served the unchanged objects) |

- **Why the wasm was rebuilt on every run:** (1) deck-space's build.rs watches the worktree's git `index` for its build label and runs `git status`,
  which rewrites the index – every run rebuilt deck-space and everything above it, the fat-LTO links too; the nested builds now set a fixed label
  (only drawn, no report holds it). The same rule rebuilds deck-space in the workspace's own builds after any `git status`: #544 (`decide`).
  (2) headless (`-p deck-headless`) and conformance (`-p live-core`) shared `conformance-wasm*`: live-core's unit is the same file for both with other
  dependency info, so each binary re-linked the other's wasm ("info of dependency `deck-space` changed"). Headless has its own dirs now (cost: deps compiled once more per dir, cold).
- **Bit-exact:** every step's runs passed unchanged – headless 41 of 42 scenarios identical in wasm32 and simd128 (01-cathedral-boot's audio-only
  difference, #381, as before), conformance's four tests `exact` on every scene, block size and render; no scene, block, render or second removed.
  Cranelift's own build profile cannot change what it emits (no fast-math; the output depends on `Config`, not on rustc's opt level of Cranelift).
- **Cold build cost of the raised opt-level:** the two test binaries in an empty target dir without sccache, opt 0 / opt 3: 135 / 218 s and (order
  swapped, quieter) 145 / 176 s – ~+30–80 s once; the registry crates then come from sccache in every worktree (Built (#526)). Gain: ~200 s of compile per run.
- **Cut, not built:** step 4 (the tests take deck:ci's simd128 wasm) – after 3a the tests build only after a wasm-relevant change, and then the
  simd128 build runs beside the plain one (which deck:ci never builds), so taking deck:ci's file saves CPU, not wall time; it also needs a guard
  that the file is this run's (a step skipped by its stamp leaves another tree's file). Step 5 for the renders – they take ~1 s. wasmtime's
  `parallel-compilation` (rayon, a new dependency) – both tests are under 60 s without it.
