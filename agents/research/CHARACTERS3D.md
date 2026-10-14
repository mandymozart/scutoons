Status 2026-10-13: learnings from a dropped experiment (owner: "we keep the learning … we just need better tooling").

# CHARACTERS3D — 3D models of the cast: what the sketch → 3D experiment taught us, where to go

tools/sketch3d (2026-10-09…13; removed – the code: `git show f535aeb8:tools/sketch3d/<file>`, e.g. core/rig.js): front A-pose (+ 90° side profile) → silhouette →
inflated low-poly mesh → auto-rig with figures.js's joints → GLB. Local, no service, no ML. The models weren't good
enough (owner: "kind of stupid actually and didnt work as i hoped"); the cast's models come from artists
(FEATURES 10.95). The ideas below are worth reusing.

## What worked – reuse
- **figures.js's joint names as the contract.** A model rigged with `pelvis, spine, chest, neck, head, sh/el/wr L|R,
  hip/kn/an L|R` (+ `robe`) is posed by the storyboard's pose table as it is (`pose()` in figures.js; the pose names are
  the keys of its `HUMAN` table). L = the figure's left = +x, it faces +z, metres, feet on y = 0. → the spec for artists' models.
- **Bind pose = A-pose, rest = limbs down.** The shoulder / hip bones carry the A angle as their rest rotation (z), so
  the pose table's angles (0 = hanging down) apply unchanged. The pelvis height is scaled to the figure's own
  (`p.y` drops of the pose table assume 0.98 m) – core/rig.js `skinnedFigure`, public/app.js `applyPose`.
- **A `robe` bone for dresses / cassocks** (no gap between the legs): figures.js already swings and shortens it with
  the legs; the feet below the hem stay legs. Works for any long garment.
- **Skin by region, not by distance alone:** arm vertices only on arm bones (+ chest), legs on leg bones – otherwise
  hands hanging next to the thighs stretch with the legs. A shoulder cap / cape belongs to the arm only near its axis.
- **Auto-detect as a first guess + drag to correct** (symmetric by default, red = found, blue = set by hand), stored
  relative to the figure's height so a new resolution keeps it. Better than any detector for capes, sleeves, tails.
- **Inputs:** an edge-aware background cut-out (flood from the border stops at shaded edges – a white dress on white
  survives), sheets split into figures at empty columns with a role each (front / side / ignore), a side view facing
  left flipped by the toes' direction. Useful for any image → silhouette step (core/mask.js).
- **Geometry helpers:** quadric edge-collapse decimation that refuses slivers (core/decimate.js – three's
  SimplifyModifier is O(n²), minutes); surface nets + Taubin smoothing; with a side view an elliptic cross-section per
  height (front width × side depth), smoothed vertically – front ∩ side alone gives flat slabs.
- **Headless batch** in Edge / Chrome posting results to a local server (server.mjs `--batch`), like render-shots.mjs.

## What didn't – and why
- A silhouette (even front + side) can't carry a face, ears, hands, feet, cloth folds or a ¾ form: the result reads as
  a blob with the right outline. Inflation depth is a guess; ¾ and back views have no place in it.
- Joint detection from an outline breaks on capes, wide sleeves, tails, dresses, duos (toptoy).
- Colours projected from the front smear along the sides (the storyboard doesn't need colour anyway).
- A hybrid (Claude desktop / mobile doing the workflow, a deployed web page doing the 3D) – not wanted: no Claude API.

## Where models live (2026-10-13)
A persona's `model` / a look's `model` in the character registry (a .glb asset, `personaModel` – the look's wins); the
storyboard's catpope (regular) and the stand-in tiger (CC BY 3.0, Poly by Google – credit in the asset title) are the
first. The renderer still reads tools/storyboard-render/models/ until FEATURES 10.95.

## Better tooling (directions, not decided)
- **Artists model in Blender** (or similar) to a one-page spec: GLB, low-poly, A-pose, figures.js joint names (or a
  standard rig such as Mixamo / AccuRIG with a name mapping in the renderer), scale / facing / feet as above, a `robe`
  bone for long garments, weights painted by the artist.
- **Image → 3D models** (local on the 8 GB GPU: TripoSR, Hunyuan3D-2 mini, InstantMesh) – not tried; at most a
  starting mesh for an artist, then retopology + rig.
- The renderer side (FEATURES 10.95): load a character's GLB like the mountain tiger (tiger-gltf.js) and pose it with
  figures.js – the joint contract above makes that a small step.
