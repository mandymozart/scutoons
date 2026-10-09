# Sketch → 3D (prototype)

A local tool (owner 2026-10-09: "sketch to 3d. low poly models … a workflow that creates 3D from the A-pose for
characters … without an external service"). A drawing, a cut-out or a character sheet becomes a closed **low-poly
mesh**; a character in A-pose also gets a **skeleton with the storyboard figurines' joint names**
(`tools/storyboard-render/figures.js`), so the storyboard's pose table moves it. No ML model, no download, no
service: everything is geometry in the page (three.js).

```bash
npm run dev:sketch3d                         # → http://localhost:14001 (shared/dev-ports.json "sketch3d")
node tools/sketch3d/server.mjs --batch       # every media/characters/<char>/*apose* → tools/sketch3d/out/<char>.glb
node tools/sketch3d/server.mjs --batch catpope,yellowmouse
node tools/sketch3d/test/synthetic.mjs       # the pipeline in Node on a synthetic A-pose, with timings
```

## How it works

1. **Cut-out** (`core/mask.js`): transparency (the A-pose PNG / WebP), a plain background (flood fill from the border
   that stops at edges, so a white dress on white stays; "Toleranz"), or a line sketch (dark lines, gaps closed).
2. **Images / sheet** (`sheet()`): drop one or more images ("+ Bild" adds one, e.g. the side view); a sheet with several
   views side by side is split at empty columns. Each figure gets a role: *Vorne* (inflated + rigged), *Seite* (a true
   90° profile: cuts the depth and gives the trunk joints their depth; facing left is mirrored by itself – the toes
   tell), *–* (ignored: ¾, back). Two figures → the second is the side.
3. **Inflate** (`core/mesh.js`): every inside cell carries a ball as big as its distance to the outline; the surface is
   the hull of those balls, squashed in depth ("Tiefe") → round limbs and head, a flatter torso. *Drehen* turns the
   silhouette instead (bottle, vase, trunk). With a side view every height gets an elliptic cross-section – as wide as
   the front, as deep as the side, limbs round – trimmed to the side outline (front ∩ side alone gave flat slabs).
   Meshed with surface nets, Taubin-smoothed, then reduced to the wanted
   triangle count by quadric edge collapse (`core/decimate.js`; three's SimplifyModifier took minutes) that refuses
   slivers (long thin triangles read as noise).
4. **Rig** (`core/rig.js`): the joints read off the silhouette – the neck (narrowest row under the head), the armpits
   (where arm and body separate), the arm's axis (fitted), the hand tips, the crotch (a gap near the centre line – a
   tail between the legs is fine), the soles. No gap at the knees → a **robe**: one `robe` bone (figures.js swings
   it with the legs), the legs only below the hem. Every vertex is skinned to its own region's bones (arm / leg /
   trunk / robe). Bind pose = the A-pose; rest = limbs straight down, so figures.js's poses apply as they are.
5. **Joints by hand**: drag the dots in the silhouette panel (red = found, blue = set by hand; "Symmetrisch" moves
   the other side along, "Zurücksetzen" drops them). Only the rig reruns. Kept per model name in
   `out/<name>-joints.json` (relative to the figure, so a new resolution keeps them); the batch uses them too.
6. **GLB**: flat-shaded, colourless by default ("Ohne Farbe" – the storyboard renders its own black and white;
   colours per face from the image are optional), `tools/sketch3d/out/<name>.glb` (gitignored).

## Best input (e.g. generated in Luma)

Front A-pose, full body, feet visible, arms clearly apart from the body (and the dress), plain white or transparent
background, no floor shadow. Optionally a **pure side profile** of the same figure at the same scale next to it (on
the same sheet is fine). ¾ and back views are not used.

## Not yet

- the storyboard renderer loading these GLBs instead of the procedural figurines (`core.js`: like the mountain tiger);
- hands / fingers, a tail bone (a tail beside a leg moves with that leg), feet pointing forward (feet are flat
  silhouettes inflated in depth);
- a duo in one image (toptoy) is one panel per figure – rig each separately.
