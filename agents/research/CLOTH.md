Status: research 2026-10-31 (owner: "research fabric simulation methods with the physics and most performant for our self-made engine – we will need that"); issue #207.

# Fabric / cloth simulation for deck-space (native + wasm)

**Where it runs:** our own engine – deck-space (Rust, wgpu) on the decks (X1: UHD 620) and in the browser (the S22: **WebGL2,
no WebGPU – and WebGL2 has no compute shaders**). So the method must have a **fast CPU path** that runs in wasm; a GPU
compute path is a bonus where WebGPU / native Vulkan exist. Uses we see: banners and curtains in the sets, capes / limbs
/ veils on the beings (#188's "funky elements"), later characters' wear, all driven by parameters and audio like the rest.

## The methods
| Method | Idea | Real-time | Stiffness | Parallel (GPU) | Fit for us |
|---|---|---|---|---|---|
| Mass–spring, explicit | springs between particles, integrate forces | cheap per step but needs tiny steps, stiff springs explode | poor | easy | no – unstable when stiff |
| Verlet + distance constraints (Jakobsen) | positions, project lengths a few times | very cheap | iteration-dependent | Jacobi works | the old game classic; fine for small things |
| **PBD / XPBD** (Müller 2007 / Macklin 2016) | project constraints on positions; **XPBD adds compliance** so stiffness doesn't depend on iterations / step | **standard in games** | good, tunable (compliance per constraint) | graph colouring or Jacobi | **yes – phase 1** |
| **XPBD with small steps** (Macklin et al. 2019) | many substeps × 1 iteration instead of few steps × many iterations | best quality per cost for cloth | good | same | **yes – how we run XPBD** |
| Projective Dynamics (Bouaziz 2014) | local / global solve, prefactored matrix | fast for fixed topology | very good | the global solve is the bottleneck | heavier to build; no |
| **VBD – Vertex Block Descent** (Chen et al., 2024) | per-vertex Newton-style block updates, few colours | rated high in recent comparisons, stable at large steps and mass ratios | very good | **fewer colour groups than XPBD → better GPU parallelism** | **phase 3** for large / stiff cloth on the GPU |
| IPC / GIPC | guaranteed no intersections | offline / heavy | excellent | GPU research | no |

## Recommendation
1. **Phase 1 – XPBD, small steps, on the CPU** (native + wasm, one code):
   - particles in **structure-of-arrays** (positions, previous, inverse mass) like our batch evaluator (#183), SIMD where
     it pays; **no allocation per frame** (no_alloc)
   - constraints: **stretch** (edge distances), **bending** (distance across a shared edge – cheap – or dihedral),
     **long-range attachments / tethers** to pinned points (stop over-stretch without more iterations)
   - **collisions with simple colliders**: spheres / capsules (the being's body and limbs), planes (floor), the set's
     floor height; friction as a position correction
   - **forces**: gravity, damping, **wind** (a noise field), and **audio-reactive** pushes (the bands, the beat)
   - ~8–20 substeps × 1 iteration at 60 fps; budget: a 32 × 32 cloth (~1k particles, ~6k constraints) well under
     **1 ms CPU** – to be measured on the X1 and the S22
   - rendering: normals per frame, double-sided, our being / set shading; tear = remove constraints (later)
2. **Phase 2 – the same XPBD as a wgpu compute pass** for big cloth (a banner wall, 10k+ particles): constraint
   colouring or Jacobi with averaging; native (Vulkan / Metal / DX12) and WebGPU only – WebGL2 phones keep the CPU path.
3. **Phase 3 – VBD on the GPU** if large or stiff cloth needs it (fewer colours, bigger steps), and **self-collision**
   via a spatial hash (bin particles per step, test neighbours; pairs closer than 2 × thickness collide) – only for cloth
   that folds onto itself; it is the usual bottleneck, so off by default.

## In the engine
- A node kind `cloth` (a #185 module feature `cloth`): inputs = stiffness (compliance), damping, wind, gravity, pins,
  the colliders it follows; outputs = its mesh for the space node and a "motion" level for the hoses / feelings.
- Its parameters are knobs on the being like any other; a preset per character / mood (#191).
- Tests: a hanging square settles and stays within a stretch bound; energy doesn't grow; no tunnelling through a sphere
  at the max speed; no_alloc; the 32 × 32 budget as a bench.

## Sources
[VBD vs PBD in Unity – Applied Sciences 2024](https://doi.org/10.3390/app142311072) ·
[Vertex Block Descent – Chen et al. 2024](https://www.alphaxiv.org/abs/2403.06321) ·
[Real-time cloth on WebGPU – arXiv 2025](https://arxiv.org/html/2507.11794) ·
[MGPBD: multigrid XPBD – arXiv 2025](https://arxiv.org/pdf/2505.13390) ·
[XR cloth: Unity vs PBD on GPU – Applied Sciences 2025](https://www.mdpi.com/2076-3417/15/12/6611) ·
[80.lv: GPU cloth with collisions (AVBD, XPBD self-collision cost)](https://80.lv/articles/impressive-cloth-simulations-with-collisions-on-gpu-in-unity) ·
[WebGPU XPBD small-steps cloth (TypeScript)](https://github.com/ccincotti3/webgpu_cloth_simulator) ·
[rusting_engine XPBD compute + CPU reference](https://docs.rs/crate/rusting_engine/2.4.1/source/src/shaders/compute/cloth.comp) ·
[Berkeley CS184: cloth self-collision with spatial hashing](https://cs184.eecs.berkeley.edu/sp25/hw/hw4/part4) ·
[Unity: no compute on WebGL2](https://docs.unity.com/en-us/engine/6000.6/manual/platform-specific/webgl/develop/graphics/web-graphics-apis-intro.md)
