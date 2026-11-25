Status 2026-11-25: research for the owner (nothing built), proposal #921; the capability question is #922, the concept sketches #920 (SPORES-EFFECTS.html). Read from the code at d7458b49 and from the pinned crate sources (wgpu / naga 30.0.1). Open questions at the end.

# RENDER-TIERS – lean, full and showpiece graphics on one core

Owner 2026-11-25: low-end decks on cheap hardware (Raspberry Pi) · a laser output that "renders only geometries", sharing its source with the toon outline · "super impressive effect loaded high performance decks", specific to one hardware (a latest NVIDIA RTX) · "my api supports engine swapping and decides on build time what features to compile".

## 1 · What exists

- The GPU layer is wgpu 30.0.1, Vulkan alone on Linux, GL only when Vulkan finds nothing, a software adapter last (`deck/native/src/gpu.rs`). No raw Vulkan in the repo.
- Build-time choice exists: modules are cargo features, `deck/os/profiles.toml` has the classes `full` and `lean`, `deck-scene-features` turns a set of scenes into the features a build needs, and a scene asking for a missing module is refused by its name (deck/native/README.md).
- A quality governor exists at run time (`quality.rs`, low / medium / high).
- Shaders are data: the `shader` node takes a built-in effect or a `wgsl` file defining `fn effect(uv)`, with four knobs a–d and hot reload; the post stack is WGSL in `deck/space/src/shaders/post/` and runs in the browser too.
- Lasers are a decided deck output (2026-10-29 [outputs], #178): ILDA DACs, 20–30k points a second.
- Sound and picture on one machine: RENDER-AUDIO-BUDGET.md (the frame runs on winit's one thread today; X1 numbers still to be measured).

## 2 · The proposal: one core, pipelines behind a frame description

Not a second engine. The core writes each frame once, without a GPU call: instances, cameras, lights, lines, levels, all read from the show clock. Pipelines read it; each is a cargo feature and a profile class. This keeps A1 (a platform is an adapter) and A4 (one engine, one clock). `deck/outlet` (`frame.rs`, `plan.rs`, `compose.rs`) may already be part of this seam – not read for this note.

| Tier | For | Pipeline | Leaves out |
|---|---|---|---|
| lean | Raspberry Pi 4 / 5, RK3588, phones (wasm) | plain forward, few lights, low internal size, one or two post passes | G-buffer, prepass, compute |
| full | X1, PCs, the conductor | clustered forward + a thin prepass (depth, normal, object id) → toon edges, SSAO; MSAA and transparency stay | PBR G-buffer |
| showpiece | one named machine (an RTX) | full + effect packs: ray queries (shadows, reflections), mesh shaders, compute particles, volumes, feedback | nothing – may be native-only |

Why not deferred: the look is toon (#850: albedo-only now), which needs edges and a few lights, not a material G-buffer; forward keeps MSAA and see-through. Clustered lights cover "many lights" when a scene needs them.

Rule for content: a higher tier adds to the picture and never changes what happens (place, time, main colours). Every effect names its fallback or is marked required; a required one is refused by name, as modules are.

A4's golden render can hold for the base tier on every deck; full and showpiece need their own goldens. That is the owner's decision (#921).

## 3 · Toon edges and the laser

- On a screen the edge is screen-space: an edge-detect pass over the thin prepass (depth + normal + id). Cheap, and the id buffer gives clean outlines between things. Owner 2026-11-25: "the toon edge detection prepass makes sense".
- A laser needs ordered paths with blanking, about 700–1000 points a frame at 30 Hz – pixels are no use. So the two share a source, not a pass: the core extracts silhouette, crease and border edges as lines (one face towards the camera, its neighbour away), the laser adapter orders them, a screen may draw them too for the laser look.
- The hard part is hiding the lines behind things: a small depth test (a read-back of a low-size depth buffer, or rays on the CPU). Low-poly helps.
- Laser safety (audience scanning) belongs to the adapter's limits, not to a scene.

## 4 · The audio clock and a strong GPU

The clock does not cap the GPU. A frame reads the clock for the moment it will be on screen; audio never waits for the picture. More GPU = more frames of the same moment.

What disturbs the sound is the processor: a shader compiled or a large texture uploaded in the middle of a show, and the frame sharing a thread with input. So: pipelines built while a scene loads (wgpu has `PIPELINE_CACHE`), no allocation per frame, the renderer on its own thread, one core kept free for audio on a Pi. Ranked list: RENDER-AUDIO-BUDGET.md §3.

## 5 · What the layer gives on an RTX (checked in the sources of wgpu-types / wgpu / naga 30.0.1)

- `EXPERIMENTAL_RAY_QUERY` (Vulkan, native only), `EXPERIMENTAL_RAY_TRACING_PIPELINES`, `EXPERIMENTAL_RAY_HIT_VERTEX_RETURN`; naga's IR has ray pipeline functions. All marked experimental, "expected to be subject to breaking changes".
- `EXPERIMENTAL_MESH_SHADER` (+ `_MULTIVIEW`, `_POINTS`), native only; naga has the Mesh stage and a task payload.
- Binding arrays (bindless-style): `TEXTURE_BINDING_ARRAY`, `BUFFER_BINDING_ARRAY`, `STORAGE_RESOURCE_BINDING_ARRAY`, `PARTIALLY_BOUND_BINDING_ARRAY`, non-uniform indexing.
- `PASSTHROUGH_SHADERS` with `Device::create_shader_module_passthrough` (unsafe): SPIR-V handed over as it is, unchecked.
- `Device::as_hal` / `Adapter::as_hal` (unsafe) give the raw Vulkan device; `create_device_from_hal`, `create_texture_from_hal`, `create_buffer_from_hal` go the other way – an own Vulkan effect can live beside wgpu on one device.
- Not in wgpu: DLSS and other NVIDIA SDKs (no mention in the sources; they need the raw device), vendor extensions in general.
- naga's SPIR-V writer: `ADJUST_COORDINATE_SPACE` flips the Y of the position output and is on by default, with `CLAMP_FRAG_DEPTH`.

Reading: the experimental features move with every wgpu release, and a wgpu bump is its own step here. An own Vulkan layer is not needed to start a showpiece; it pays where one effect needs what wgpu lacks, through `as_hal`, behind a showpiece-only feature.

## 6 · Shaders: WGSL in the Library, SPIR-V at the top

Decided 2026-11-25 (decisions/deck.md [modules]):
- WGSL is the standard for Library shaders: the one language every tier runs, phones included, and the core can check it on save (naga). Vulkan takes only SPIR-V, and naga turns WGSL into it, so an own layer stays compatible; `@group` / `@binding` map to descriptor set and binding.
- A high-end layer takes SPIR-V. An effect standard WGSL cannot say is written in GLSL or Slang and compiled to SPIR-V.
- Showpiece shaders are shared in the Library too – a conductor flashes high-end decks on demand – with a flag for the tier they need.

Follows: a `shader:` document kind (knobs declared, version, the tier flag, a lean form or "required"), named by scenes as module presets will be (#808); and the fixed uniform block of SPORES-EFFECTS sketch 5 (bands, envelopes, beat, time, seed).

Open in it: who compiles a GLSL / Slang shader (the compile service, as for modules?), and whether SPIR-V itself may be a Library payload – it cannot be checked the way WGSL is.

## 7 · Capabilities (#922)

Today the way is one: the conductor flashes, the deck drops what it has no hardware or build for. `hello` carries only `device`. With tiers and flagged shaders a conductor should know before the flash. Proposed: the deck declares (tier, features, outlets, a few GPU facts), the conductor sees it in the Flash dialog, the deck still resolves every fallback itself by the core's one rule. Options and the recommendation: #922.

## Gaps (RULES L1: filled one at a time, when a scene or a show needs it – each in its own issue)

| Gap | Today | Convention when it is filled |
|---|---|---|
| the renderer shares winit's input thread | one thread (RENDER-AUDIO-BUDGET §1) | its own thread; reads the clock (L4) |
| tiers | `full` / `lean` are module sets, not pipelines; the governor has low / medium / high | lean · full · showpiece as features and profile classes (L2, L3) |
| the frame description | not looked for in `deck/outlet` yet | open (#921) – ask first (L7) |
| the prepass and toon edges | none on the deck; XR has a `toon` plugin draft | depth + normal + id, an edge pass (L5); full first |
| lines for the laser | none; the `laser` adapter does not exist (#178) | edges as geometry from the core, the adapter orders them (L5) |
| shaders in the Library | a `wgsl` file beside a scene | a `shader:` kind, WGSL, knobs declared, tier flag, a lean form or "required" (L6) |
| the sound block | four knobs a–d per shader node; FFT and `follower` / `env` exist (#746) | one named uniform block: bands, envelopes, beat, time, seed (L6) |
| SPIR-V for the high end | wgpu compiles WGSL only | open who compiles GLSL / Slang; passthrough exists in wgpu (§5) |
| showpiece effects | none | a feature per effect, native only, on the named machine (L2) |
| capabilities | `hello.device` only | open (#922) – ask first (L7) |
| goldens above the base tier | one golden contract (A4) | open (#921) |

## 8 · Order to build and learn

1. Measure the X1 (RENDER-AUDIO-BUDGET §4); move the frame off the input thread.
2. The frame description as the seam; today's renderer behind it.
3. The thin prepass and toon edges on full.
4. The lean pipeline, measured on a Pi.
5. `shader:` in the Library with the tier flag and the uniform block.
6. Line extraction in the core, then the laser adapter (#178).
7. Capabilities declared (#922).
8. The first showpiece effect on the RTX (ray-query shadows or compute particles), then clustered lights.

## Open (owner / next session)

- Pipelines behind one frame description, or a second engine for the showpiece (#921).
- Goldens per tier (#921).
- The own Vulkan layer: beside wgpu for single effects, or the tier's base (#921).
- The showpiece's machine and its first effect.
- One shader language for deck and XR (WGSL), or OSCT's plugin shaders stay their own kind (#920).
- Capabilities: 1 / 2 / 3 (#922).
