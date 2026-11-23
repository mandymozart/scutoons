# Models of the editor's stage

The desk under the experience (drawn by `editor/src/lib/stage/mock-stage.ts`): Egon Eiermann's table frame with two tops of
the same 1.8 × 0.8 m and the same surface height – the toolbar's *view* group's *top* (`top` in `editor/src/lib/stage/preview.ts`,
#226) shows one and hides the other. Our own low-poly model, built by `editor/tools/desk-eiermann.py` (Blender,
headless):

| File | What | Source |
|---|---|---|
| `desk-eiermann.glb` | A steel tube frame after Egon Eiermann's table frame (Tischgestell 1, 1953), 72 cm high, with two tops: `TopEiermann`, a screen-printed plywood board (18 mm), and `TopLinoleum`, blue-grey linoleum (2 mm) on an oak-faced plywood board (18 mm) – 256 triangles (the frame 224, the tops 12 and 20) | Our own model; the frame's tube axes were measured from a 3ds Max model of the frame Tilman supplied (`01_E1_1100x660-exzentrisch`), which carries no licence and is not in the repository |

The file carries no textures. The model's origin is the centre of the tops' upper surface. The stage puts its own look
on the plywood top's `top` (Poly Haven's plywood tinted to the phenolic brown) and `edge` (the board's cut sides: the
plywood's plies, drawn in code by `editor/src/lib/stage/plywood.ts`), and on the linoleum top's `linoleum` (the surface
and the layer's thin edge, drawn by `linoleum.ts`), `oak_edge` (the board's cut sides: the same plies tinted to oak) and
`oak` (the oak-veneered underside, drawn by `oak.ts`); it leaves `steel` (the frame) as exported. `steel` is
powder-coated, not bare metal (metallic at most 0.5, rough): a metal shows almost only what it reflects, so thin dark
tubes vanish before a dark studio and under the scene's own lights (textured shading). It stays double-sided – the
tubes are open, their insides show at the ends. `src/lib/stage/__tests__/desk-models.test.ts` checks the file.

UVs: both boards' edges run u along the edge (a repeat every 250 mm) and v through the 18 mm board, one stack of plies –
13 plies per 18 mm on both tops. The linoleum repeats every 400 mm both ways; the oak underneath runs u along the top's
long side (a repeat every 500 mm) and v across it (125 mm). Regenerate it with

```bash
"C:/Program Files/Blender Foundation/Blender 5.1/blender.exe" -b -P editor/tools/desk-eiermann.py
```
