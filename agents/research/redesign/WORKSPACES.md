Status: collection 2026-11-20 for #673 (owner: "workspaces also include background stages scenes, or canvases … lets collect everything we got") – nothing decided; the owner picks from §4.

# Workspaces – everything we have, by background

A workspace is identified by its **background** – the stage, scene or canvas the panels float over (owner 2026-11-20).
Sources: a read-only sweep of creator, shared, deck, viewer (admin, embassy, shop have no canvas); OSCT's current XR model
(owner: "we can copy from there"); deck decisions [workspaces] / [design]. Route map of the creator: INVENTORY.md.

## 1. Today (26 surfaces)

| # | Surface | App | Background | What is done there | Overlaps |
|---|---|---|---|---|---|
| 1 | Storyboard viewport | creator | 3D stage (three.js) | shot camera, figures, texts, look pass | deck Scene + DOP (#12, #14) |
| 2 | Storyboard variants canvas | creator | 2D canvas, bands of generations, lineage lines | generations, review marks, selection A–D | – (the Generations surface) |
| 3 | Storyboard compare | creator | image / video compare, one clock | 2–4 generations side by side | – |
| 4 | Storyboard timeline | creator + shared/ui | timeline | picked shots, trims | deck media trim (#16), DOP moves (#14) |
| 5 | Handheld stage | shared, creator `/h`, viewer cameras | phone pose / camera | takes, live camera | already shared; closest to an AR feed |
| 6 | Chapter editor | creator | 2D layer canvas (slice / strip) | slices: images, text boxes, tails, collage, media, audio; VN events | #7, #9, #8 (same kind of canvas) |
| 7 | Cover lettering | creator | 2D layer canvas | cover lettering per format / language | #6 |
| 8 | Character stage | creator | 2D image stage + mood grid | images, moods, looks | deck Asset `character:` (#13) |
| 9 | Score / collage stage | creator | 2D page canvas | an item's collage pieces and looks | #6 |
| 10 | Map builder | creator | map (hex terrain, fog, routes) | places, regions, routes, start view | reader map (#21) shares the terrain |
| 11 | Flow graph | creator | node graph (Svelte Flow) | story jumps, groups, chapters | #15, #18 (other node graphs) |
| 12 | Deck Scene / Build | deck native | 3D stage (wgpu, deck-space) | holders, slots, video, parallax, events; Edit / Play | #1 |
| 13 | Deck Asset (character, item, emotion) | deck native | 3D stage, isolated | being + rig, modules as tracks, 3D tubes | #8, #18 |
| 14 | Deck DOP | deck native | 3D stage, camera corridors | camera, lens, shots, moves | #1, #4, #5 |
| 15 | Deck Show | deck native | grid-matrix (rows band) + node view | conductors, roles, rows × cells; author and launch | #11 (node graph) |
| 16 | Deck Media | deck native | player / timeline | play, trim a medium | #4 |
| 17 | Deck Module | deck native | 3D stage + external code editor | module build, problems, parameters | #18 |
| 18 | Sound workspace | creator (wasm deck inside) | 3D stage (deck-space wasm) + panels | rig / module: code, parameters, nodes, assignment | #13, #17 – rigs edited in two apps |
| 19 | Deck Home | deck native | 3D idle scene | new / recent / library | – |
| 20 | Browser deck `/live/deck` | viewer | 3D stage (deck-space wasm) | plays a deck | same engine as #12, #18 |
| 21 | Reader map | viewer | map | read only | #10 |
| 22 | Reader VN / slices | viewer, shared | 2D | read only (preview of #6) | #6 |
| 23 | Lab vote | viewer (dev) | phone UI | audience vote | #15 |
| 24 | Lab room / cave | viewer (dev) | 3D stage (three.js) | diorama draft | #1 |
| 25 | Model preview | shared | 3D turntable (three.js) | .glb preview | – |
| 26 | Deck native output | deck native | 3D stage output | performs | #12 |

## 2. Planned – XR (copied from OSCT's current model)

OSCT (mandymozart/osct #307, #319, #314, #316, #321, #169, #255): an **experience** is an ordered list of **scenes**; a
scene has **surfaces** (one: cover, wall, poster; two: a spread; three: a fold-out; many: a room), each with a printed
label, a format / size in mm, a place in the scene and an optional reference image; **targets** lie on a surface, entries
belong to a scene and optionally a surface; everything links by id; groups (front matter, chapters, rooms) are one stop
in the reader. Targets get their place from the print PDF (#321); the PDF export makes books, posters, wallpaper with
AR targets (#169). OSCT's editor stage: three.js in a sandboxed frame (tree · stage · inspector · log). Its newer work
(scenes and surfaces, schema 3.0) postdates research/XR.md – read OSCT first.

| # | Surface | Background | What | Would share |
|---|---|---|---|---|
| P1 | **XR experience editor** (Book, Concert reading, Exhibition, **Poster**) | surfaces on a stage set (a poster, a spread, a room) + image targets + simulated camera | scenes › surfaces › targets › entries; SCUTOONS story events on found / tap | OSCT's stage (three.js frame) – or the deck stage if the frame can host it (#242) |
| P2 | Reader XR mode | AR camera feed + image targets | scan, found, reward, entry sheet | OSCT engine |
| P3 | 3D scene types: pure 3D / AR / VR (#241, #243) | 3D stage | a `scene:` per type | deck-space (#12; wasm #18, #20) |
| P4 | AR scenes (#247) | AR camera (WebXR immersive-ar) | a deck-space object on a plane / image / geo anchor | deck-space wasm, handheld (#5) |
| P5 | VR scenes (#248) | headset (WebXR, OpenXR later) | the cathedral in a headset | deck-space |
| P6 | Songlines (#394) | map + AR feed | stations as verses on the U-Bahn | map (#10, #21) + P4 |

Short term: **posters printed in December, rolled out in January (#674)** – P1's Poster type and P2.

**OSCT's pieces to copy (read 2026-11-20):**
- **Stage** (`osct/editor/src/lib/stage/`, ~6.7k lines): the engine's own `ArScene` (three.js 0.174) in an iframe sandboxed
  without same-origin, behind a typed postMessage protocol (`frame-protocol.ts`); picking, gizmo, snapping, sky; a simulated
  image tracker (`mock-tracker.ts`: found / lost / tap without a camera); surface-mm ↔ stage math (`surfaces.ts`,
  `surface-gizmo.ts`, `snap-scene.ts`). Surfaces are boards on a 3D tabletop; targets lie on them.
- **Schema 3.0:** scene = id, label, group, `surfaces[]`; surface = `widthMm`, `heightMm`, a required `place` {x, y, z mm;
  rx, ry, rz °} in the scene's coordinates (origin on the table, +y up, +z to the reader; a wall is rx 90); a poster is one
  surface; target `place {surface, x, y, width, height}` mm; reference image per surface (editor only).
- **Surface mode** (S tool, shipped): outlines, select, Inspector (label, size, place + gizmo, reference), Tree level,
  + surface (page / wall / custom), reorder, arrange in a row, floor and vertical grid.
- **Scene overview** (filmstrip, #307 phase 3): order, add, delete scenes; groups. The experience **type becomes a layout
  of surfaces**, not a stage of its own – this replaces XR.md's "stage set per type".
- **Print** (#321 targets placed from the print PDF; #169 PDF export of books, posters, wallpaper with targets): the
  poster's production path – nothing like it in SCUTOONS yet.
- **Kit:** OSCT already has `FloatingPanel`, floats, `ToolGroup` / `ToolMenu`, `GridSnapMenu`, `ScrubNumber` – the same
  rules as deck-kit's floats; a source for #656.
- **three.js:** OSCT 0.174, creator 0.186 – not a blocker (own sandboxed frame and bundle; plugins get `api.three` from the engine); at most a second copy in the reader.
- XR.md §8.1 and XR-SHARED-CORE §3 named "surfaces without pages" as the one conflict; schema 3.0 resolves it, so the
  copy fits. Both docs predate 3.0.

## 3. Same background, different apps

- **3D stage – three engines today:** deck-space (wgpu native #12–14, #17, #19, #26; the same as wasm in #18, #20),
  three.js (storyboard #1, lab #24, model preview #25, OSCT's stage P1 / P2). The storyboard stage repeats the deck's
  scene, camera and shots. The open fork: OSCT's three.js frame vs the deck stage for XR (#242).
- **2D layer canvas – four:** chapter #6, cover #7, collage #9, character #8 – each with its own pointer, zoom, snap;
  `lib/editor/canvas-geometry.ts` could be the common base.
- **Node graph – three:** flow #11 (Svelte Flow), rig nodes #18 (own SVG), show nodes #15 (egui). The deck already
  wants one port / wire grammar (rows band, node panel, node view, tubes).
- **Timeline – three:** storyboard #4 (shared/ui Timeline), deck media trim #16, DOP moves #14.
- **Map – two + planned:** builder #10 and reader #21 share the terrain; Songlines adds AR. (The deck calls its 3D Scene
  "the map" – a name clash only.)
- **Matrix – one:** the Show band #15.
- **Generations – one:** variants canvas + compare #2, #3. Data: the `storyboard_generations` ledger (parents[] = lineage,
  marks, approved, cost), files in the member's local folder and the work storage, MCP tools for every step. Today only
  the storyboard uses it; characters' moods and looks, items, covers, slices and map places could pull approved results,
  and `wanted-images.svelte` is already the to-do list of missing media.

## 4. Candidate workspaces (for the owner to pick – not decided)

One workspace per **background kind**, opened items as tabs inside it, panels defined once:

| Workspace | Background | Holds today | Planned |
|---|---|---|---|
| **Scene** | 3D stage (deck-space) | deck Scene / DOP, storyboard viewport (moves here), sound workspace's stage | 3D / AR / VR scenes (P3–P5) |
| **Asset** | 3D or 2D stage, isolated | character (#8 + #13), item + collage (#9), rig / module (#17, #18), emotion, medium | – |
| **Experience** (XR) | surfaces on a 3D tabletop (OSCT stage, copied) | – | Poster, Book, Exhibition, Concert reading as layouts of surfaces (P1); scene filmstrip; print PDF; posters first |
| **Chapter** | 2D layer canvas | chapter editor (#6), cover (#7) | – |
| **Storyboard** | timeline + shot list over the Scene stage | storyboard (#1, #4) | – |
| **Generations** | variants canvas + compare | #2, #3 | the library every other workspace pulls approved media from |
| **Flow** | node graph | flow (#11) | the same graph grammar as rig and show nodes |
| **Map** | map | map builder (#10) | Songlines (P6) |
| **Show** | matrix + node view | deck Show (#15) | – |

Questions: Storyboard as its own workspace or a Scene workspace's mode? Chapter and cover one workspace? Is Generations a
workspace, a panel, or both? XR on OSCT's three.js stage or on deck-space? The word "workspace" is already the storyboard's
name for the member's local folder – rename that.
