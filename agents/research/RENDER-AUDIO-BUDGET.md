Status 2026-11-23: research for the owner (nothing built). Read from the code at ca0634d7a plus one CPU micro-bench on the dev PC (§1.3); the X1 numbers after the #816 knob cap are still to be measured (§4). Open questions at the end.

# RENDER-AUDIO-BUDGET – graphics and sound on one machine

Owner 2026-11-23: "figure out graphic optimizations for our engine, because we are having conflicts and audio glitches. Since both have to run together we want to be efficient."

The measurement that started it (X1 Carbon Gen 7: i7-8565U 4C/8T 15 W, UHD 620 Mesa/Vulkan, linux-rt, PipeWire 48 kHz, deck/HARDWARE.md):
crows-rave at 14 fps, main thread 99 %, the audio thread (SCHED_FIFO 70) ~30 %, `audio glitch: late – interval 6.8 ms (period 2.67 ms), callback 0.01 ms`,
quality "high (by the GPU, the governor off)". The 14 fps figure was taken **before** 73c2a61d6 capped the scene graph at 4 knobs a node (its commit
message cites it). For comparison: the X1 ran "AV madness" at 60 fps with 1.25 ms CPU per frame on 2026-10-29 (HARDWARE.md).

## 1 · Where the frame time goes today

### 1.1 The frame, in order (deck-native, one thread: winit's event loop)
| Step | Where | CPU / GPU | Notes |
|---|---|---|---|
| fps cap check | native/src/main.rs:1139–1146 | – | `Governor::due`; an early frame sleeps (`host::wait`, ControlFlow::WaitUntil). Every preset's cap is 60 (space/src/quality.rs:330–331), so the cap never bites below 60 fps. |
| governor sample | main.rs:1151–1158 | CPU, tiny | the frame interval + the audio glitch count into `Governor::frame` (quality.rs:684). |
| controls face | main.rs:1167 | CPU, small | reads the engine's snapshot ring. |
| **space update** | main.rs:1212 → space/src/space/mod.rs:903 | **CPU** | camera, director, then `Being::update` for the scene's own being **and every cast member's being** (mod.rs:948–955), hover picking, session. |
| rig frame, outputs, HUD lines | main.rs:1213, 1172–1180, 1265–1295 | CPU, small | HUD text is rastered on the CPU every frame in Full mode (main.rs:1293; 320 px wide – small) and uploaded every frame (space/src/stage.rs:787). Off by default. |
| acquire | native/src/render.rs:176 | wait | `AutoVsync` (FIFO), frame latency 2 (native/src/gpu.rs:94–95). This wait is excluded from `cpu_ms`. |
| encode the space | stage.rs:809 → space/src/space/gpu.rs:780 | **CPU** writes + GPU | see 1.2. |
| post, composite, overlays | space/gpu.rs:899–901 → post_gpu.rs:194; stage.rs:776–806 | GPU | bloom = 5 small passes + composite; FXAA one pass; the space texture onto the screen; egui only "when it changed" (stage.rs:771). Offscreen targets are Rgba8Unorm (stage.rs:31). |
| submit + present | render.rs:200, 207 | CPU (driver) | |

`cpu_ms` in the exit line = time from the frame's start to the render call + the render's own work after the acquire (main.rs:1328–1331, render.rs:177/201). It does not say which part.

### 1.2 Per object (space/src/space/gpu.rs, being.rs)
- **Being update – O(n²) in the knobs.** `Being::step_buttons` (being.rs:1269) is linear per knob (tether, three rounds of `clear_of`, fold), but
  ends in `spread` (being.rs:1470): 12 rounds × every folded knob × every knob – O(12·n²) whenever the camera is close enough to fold knobs.
  Every knob also reads its value through `Controls::input(node, "name")`, a linear string search over the kind's ports
  (engine/src/control.rs:163 → space/src/graph.rs:258) – 90 ports for analog.
- **Tubes on the CPU.** Every hose, tendril, neck cable and knob tether is a Bézier swept into `(SEGMENTS+1) × SIDES` = 19 × 8 = 152 vertices
  (being.rs:52–53, capacity at being.rs:1631), rebuilt every frame by `write_tubes` (being.rs:1778) and uploaded whole (gpu.rs:787).
- **Spheres.** Nuclei, knobs, value rings, selection rings, bubbles, shells and the core are all instances of one 14 × 9 sphere = 252 triangles
  (gpu.rs:387), one instanced draw for the solid ones and one for the translucent ones (gpu.rs:859–863, 888–892) – few draw calls, but a 3 cm
  bubble costs the triangles of a head-sized shell, and the translucent ones overdraw.
- **Labels.** At most 24 labels (space/label.rs:20), glyph quads into a preallocated vector (gpu.rs:565); only the named / hovered knob shows one – cheap.
- **Figures.** Skinned on the GPU: per figure per frame `Figure::pose` (figure.rs:147; 65 joints) and one 8.2 KB palette upload
  (`FigureUniform` 128 joints × 64 B, gpu.rs:68–73, written at gpu.rs:675); one indexed draw per texture (gpu.rs:846–856). 4 figures ≈ 33 KB/frame
  and ~40k triangles – not a CPU cost of note.
- **Set (room).** One vertex/index buffer, one draw per material (gpu.rs:836–843); lighting per vertex over up to 8 lights (shaders/space.wgsl:63),
  so the fragment side is a texture fetch + fog. Its triangle count for the sand dunes is not in the repo (not determined).
- **Shadows.** There is no shadow pass. "Glow" is the beings' cores as up to `rung.lights` point lights (gpu.rs:801–802) plus bloom.
- **Resolution.** The space renders at `scene render_scale × rung size` (stage.rs:516–548); High = 0.9–1.0 of the display, Low = 0.5–0.6
  (quality.rs:353–365). The X1C7 panel is 1080p or 1440p depending on the model – which one the owner's has is not recorded.

### 1.3 Measured: the being's CPU cost (dev PC, Ryzen 5 3600X, release, single thread)
A scratch bench outside the repo (deck-space as a path dependency, `CARGO_TARGET_DIR=.claude/research-target`) loads
deck/native/scenes/sandbox/crows-rave.toml, registers its five modules as deck-space's tests do, and times `Space::update` and the CPU writes
(`Space::write` + `write_labels`) over 1140 frames (no set, no figures, no GPU):

| crows-rave as … | beings | knobs | spheres | tube verts (upload) | update mean / p99 | writes mean / p99 |
|---|---|---|---|---|---|---|
| shipped (≤ 4 knobs a node, #816) | 4 | 102 | 661 | 33 440 (1.3 MB) | 0.91 / 1.29 ms | 0.91 / 1.44 ms |
| every control a knob (before #816) | 4 | 900 | 2 257 | 154 736 (6.0 MB) | **116.8 / 206 ms** | 5.1 / 13.2 ms |
| one knob only | 4 | 19 | 495 | 20 824 (0.8 MB) | 0.04 / 0.10 ms | 0.62 / 1.96 ms |

Readings: 9× the knobs cost 128× the update – the quadratic `spread`, not the knob count itself; the 14 fps / 99 % on the X1 is explained by it.
After the cap the being costs ~1.8 ms a frame on the PC (the X1's 15 W U-series core is slower – not measured); of the writes, the tubes are the
larger part (one knob still writes 21k tube vertices: 52 hoses + tendrils + 4 necks).

## 2 · Audio and graphics on one machine

### 2.1 How the threads are set today
| Thread | Priority | Where |
|---|---|---|
| main (winit, update, encode, submit) | SCHED_OTHER, nice 0 | – |
| `audio-render` (renders 256-frame blocks into the ring) | SCHED_FIFO 70 | native/src/audio/ring.rs:93–101, priority.rs:59–68 |
| PipeWire data thread (our `process` callback, copies from the ring) | whatever libpipewire's client module-rt gets from rtkit | backend.rs:991 (`RT_PROCESS`); deck/os ships a config only for the daemon (rootfs/etc/pipewire/pipewire.conf.d/10-deck-rt.conf: rt.prio 88) |
| events sequencer / dispatcher | nice −10 | audio/sequencer.rs:28, dispatch.rs:134 |
| `module-epoch` ticker | SCHED_OTHER, wakes every 1 ms | module-host/src/wasmtime_backend.rs:42–47 |

- No thread is pinned (no `sched_setaffinity` anywhere in deck/), no core is isolated, deck/os sets no kernel command line for it
  (`isolcpus`, `nohz_full`, `rcu_nocbs`), no IRQ-thread priorities (rtirq-style) and no CPU frequency / C-state policy (`cpu_dma_latency`).
- Nothing in the audio path locks or allocates against the frame thread by design: SPSC rings (rtrb), a seqlock for the stamp, atomics for the
  glitch counters (ring.rs:1–12, glitch.rs:1–10); tests/no_alloc.rs guards it (it fails on the dev PC per HANDOVER – deck:ci passes).
- The render thread wakes at least every 1 ms even when the ring is full (`park_timeout(1 ms)`, ring.rs:101): ~1000 FIFO wake-ups/s.
- **The control half runs on the audio thread**, once per 128-frame quantum (engine/src/control.rs:1–12, 663–673), and that includes every `fft`
  node's analysis: a 1024-point FFT with a Hann window (core/src/analysis.rs:8, 51–54, 82–91). crows-rave has 7 fft nodes → 7 × 375 = 2 625 FFTs/s
  on the FIFO thread, for values the picture reads ~60 times a second.
- crows-rave also renders 3 `voice` + 3 `walk` nodes whose gain is 0 (the default character rig standing in, crows-rave.toml header).
- Wasm modules compile with NaN canonicalization on (wasmtime_backend.rs:31) for bit-equal decks (ABI-V2 §6). MODULE-BENCH.md measured it costs
  30–45 % of wasmtime's p50 on analog. Per-module time on the X1 is not measured.

### 2.2 What "late" means here, and what it does not
`late` = our callback's interval exceeded 1.5 × the period it covered + slack (core/src/monitor.rs:110–113); `callback 0.01 ms` says our own
callback was cheap and the ring was not empty (otherwise it would read `underrun`). So the deck's audio work was not the problem – the PipeWire
data thread (or the driver node ahead of it) was **woken ~4 ms late**. Candidates the code cannot settle (each checkable on the X1, §4.3):
1. our PipeWire data thread did not get RT (rtkit refused, or the client config has no module-rt) – then a busy SCHED_OTHER main thread and
   Mesa's threads compete with it;
2. on PREEMPT_RT every IRQ is a FIFO-50 thread – i915's and the sound card's (SOF on the X1, or xhci for the RME) share a priority, so GPU
   interrupt work can delay the audio IRQ thread;
3. the i7-8565U's CPU and UHD 620 share one package power limit: a GPU at full load at High lowers the cores' clocks; deep C-state exits add wake-up latency;
4. SMT: the FIFO audio thread on the hyperthread sibling of the 99 % main thread runs at roughly half speed.

### 2.3 Frame pacing and the governor
- The governor already treats an audio glitch as an over-budget window (quality.rs:713–725) – but only with `auto` on (quality.rs:721); the X1
  ran "the governor off", so it did nothing. Where `auto = false` came from (the X1's quality file or `--quality …,fixed`) is not determined.
- Its rungs change GPU work only: render size, FXAA, bloom, lights (stage.rs:516–527, gpu.rs:741–744). The fps cap is the one knob that
  also cuts CPU work (fewer `Space::update` + encode per second) – and every preset sits at 60 (quality.rs:330–331).
- Integrated GPUs default to High (quality.rs:108–111: "the X1's UHD 620").

### 2.4 What a low-latency deck should do (the plan this note proposes)
1. Audio health drives the picture: a glitch steps the **fps cap** first (60 → 30), then the render scale – and this "audio guard" stays on
   even when the owner fixes the level (the level is a ceiling for looks, not a licence to glitch).
2. The audio core gets its own core: pin `audio-render` and the PipeWire data thread to one physical core (both its hyperthreads, e.g. CPU 3 + 7
   – check `lscpu -e`), everything else (main, Mesa, loaders, epoch) to the other three; on deck/os optionally `isolcpus=3,7 nohz_full=3,7 rcu_nocbs=3,7`.
3. The sound card's IRQ thread above i915's (rtirq-style chrt on boot), and a `cpu_dma_latency` request while a show plays.
4. Nothing on the audio thread that the audio does not need: analysis at the picture's rate, silent nodes skipped.

## 3 · Ranked optimizations
Gain = on crows-rave-like scenes on the X1; "PC" numbers from §1.3. Effort S < 1 day, M 1–3 days, L > 3 days.

| # | What | Where | Gain | Effort | Risk |
|---|---|---|---|---|---|
| 1 | **Re-measure the X1 after #816** before anything else (§4.2) – the 14 fps is pre-fix | – | decides the rest | S | none |
| 2 | **Make `spread` linear-ish**: only folded knobs against each other (not all n), a uniform grid / sort by screen x, and skip it when nothing is folded | being.rs:1470–1500 | removes the O(n²) cliff (117 ms → ~linear at 900 knobs); ~most of today's 0.9 ms update | S | low – unit tests of the fold exist; visual check close up |
| 3 | **Audio guard in the governor**: a glitch steps the fps cap down (60→30) before scale; active also with `auto` off; low preset's default cap 30 | quality.rs:684–760, 330–365; main.rs:1155 | halves main-thread CPU + GPU load when audio suffers | S–M | low – a visible fps drop under stress, by design |
| 4 | **FFT at the picture's rate**: run `Analyzer::update` every Nth quantum (≈ 60–90 Hz) with the summed dt, or move it to the frame thread's face | control.rs:671–673 | 7 FFTs × 375/s → ~7 × 60/s on the FIFO thread (≈ −84 % of that work) | S | low – values the picture reads lag ≤ 1 frame; check the golden-render conformance |
| 5 | **Pin + isolate the audio core**, raise the sound IRQ thread, `cpu_dma_latency` while playing | priority.rs (affinity next to `raise`), deck/os rootfs (kernel line, a boot unit) | targets the "late" wake-ups directly | M | medium – wrong core map halves render throughput; test with `cyclictest` + the glitch counter |
| 6 | **Cache port indices**: resolve `Button.name` → port index once (at build), read `input_at` per frame | being.rs:1299 (`vals.input(b.node, b.name, …)`), graph.rs:258 | removes per-knob string scans (analog: 90 ports) | S | low |
| 7 | **Tubes on the GPU**: upload 4 control points + radii per tube (~10 KB instead of 1.3 MB), sweep in the vertex shader from `vertex_index` | being.rs:1778, gpu.rs:787, space.wgsl:331 | −0.6…0.9 ms CPU/frame (PC), −1.3 MB/frame upload | M | medium – the conformance render must stay equal |
| 8 | **Sphere LOD / impostors**: bubbles, rings and nuclei below N px as a low-poly sphere (6×4 = 48 tris) or a camera-facing quad with a ray-sphere fragment | gpu.rs:387, 859–892; space.wgsl:205–257 | ~5× fewer sphere triangles (661 × 252 = 167k today) | M | low–medium – looks |
| 9 | **X1 default level Medium** (0.7–0.8 scale) or High's size as a height (720/900 px) on 1440p panels | quality.rs:108–111 or the X1's quality file | GPU fill ~−35…50 %; also frees package power for the CPU | S | low – owner's call on looks |
| 10 | **Skip silent nodes**: a voice/walk whose gain is 0 and whose output only feeds silence renders nothing | engine render (a "silent" flag per block) | crows-rave: 6 nodes of audio work | M | medium – envelopes/state must stay continuous |
| 11 | **Ring thread without the 1 ms poll**: park until unparked, with a longer safety timeout (e.g. 10 ms) | ring.rs:101 | ~900 FIFO wake-ups/s less | S | low – the comment's lost-wake-up case stays covered |
| 12 | **Update only what changed / what is seen**: a cast being outside the frustum or a card skips `step_buttons`'s tether/fold work; HUD texture uploaded only when its lines changed | mod.rs:948–955, stage.rs:787 | scales with cast size | M | low |
| 13 | **NaN canonicalization off** for the audio modules | wasmtime_backend.rs:31 | −30…45 % wasm time (MODULE-BENCH.md) | S code, **owner decision** | breaks bit-equality across decks (ABI-V2 §6) – only with a decision |
| 14 | **A render thread** (encode/submit off the event loop) | native main.rs / render.rs | overlaps update and GPU submission | L | high – a structural change of the deck host |

## 4 · How to measure

### 4.1 Add to the deck's own stats (core/src/monitor.rs + main.rs)
- Split `cpu_ms` into **update** (main.rs:1151–1240), **ui** (HUD, menu), **encode** (stage.frame), **submit/present**, and the **acquire wait**
  (render.rs:176) – a long acquire = GPU- or vsync-bound, a long update = CPU-bound.
- **GPU time** per pass (space, post, composite) through wgpu timestamp queries when the adapter has `TIMESTAMP_QUERY` (Mesa anv does).
- **Scene counters** once a second: beings, knobs, spheres, tube vertices, glyphs, draw calls, bytes uploaded per frame.
- **Audio**: per-module render time (top 3) next to the block load p50/p99 already there; the callback interval's histogram (how late, not only
  that it was); the live PipeWire quantum.
- **Machine** (Linux): CPU MHz of the main and the audio core, `thermal_throttle/package_throttle_count`, which CPU each thread last ran on
  (`/proc/self/task/*/stat` field 39) – in the exit line.

### 4.2 A repeatable bench (PC and X1)
- **Native, with GPU and audio** – the flags exist (main.rs:212–290): `cargo run --release -p deck-native --features modules -- --scene crows-rave
  --frames 3600 --quality high,fixed --audio pipewire --audio-buffer 256 --no-hud`, then the same with `low,fixed`; compare the exit line
  (`frames avg_fps frame_ms p99 cpu_ms … glitches=… (late … long … underrun …)`). Missing: a `--no-vsync` (present mode Immediate) to see the
  headroom above 60 fps, and a fixed camera distance (the fold – hence `spread` – depends on it).
- **Headless, no GPU** – deck/headless already runs `Space::update` on a virtual clock (headless/src/stage.rs:64–92) and the engine into a
  virtual sink: a `bench` step (N frames: update + `write` + `write_labels` timed, counts reported; M quanta of the engine timed per node) would
  give the CPU side of both, identical on the PC and the X1, in CI if wanted. The §1.3 numbers are that bench done by hand.
- Baselines to keep: crows-rave (heavy), cathedral (the 2026-10-29 60 fps / 1.25 ms reference), one knob-heavy rig.

### 4.3 One-off checks on the X1 (no code)
`ps -eLo tid,cls,rtprio,ni,psr,pcpu,comm | grep -i -e deck -e pw -e data` (does the PipeWire data thread have RT?), `pw-top` (ERR column, quantum),
`ps -eLo …` for `irq/*i915*` vs `irq/*snd*|*sof*|*xhci*` priorities, `cyclictest -m -p 80 -t -a` under load, `turbostat` (package power, MHz, C-states) while crows-rave plays at High vs Low.

## Open (owner / next session)
- The X1's numbers after #816 (fps, cpu_ms, glitches at quantum 256) – §4.2.
- Why the governor was off on the X1 (the quality file or `,fixed`) – and whether the audio guard (§3 #3) may override a fixed level.
- The X1's panel resolution (1080p / 1440p) – decides how much #9 gains.
- Whether the PipeWire data thread runs RT on the X1, and the IRQ priorities – §4.3.
- Bit-equality vs 30–45 % wasm speed (§3 #13) – a decision, not a fix.
- Not determined from the code: the sand-dunes set's triangle count, per-module audio cost of analog on the X1, GPU time per pass on the UHD 620.
