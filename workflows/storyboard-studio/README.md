# Storyboard Studio

A local tool for the storyboard workflow (owner 2026-10-09: "just start the studio over night. i want to see where
this goes. we might dump it later anyhow if there is easier way"; 2026-10-10: "use our scutoons ui and design
dogmas", "a free moving camera in a 3d view"). Built like the tools: `@scutoons/ui` (AppShell, dock panels, rails,
Segmented, dialogs) and `@scutoons/tokens` (tools.css). Everything goes back into the repo's storyboard files.

```bash
npm run dev:studio
```

→ http://localhost:14000 (`shared/dev-ports.json` "studio"; `npm run dev -- studio` with the stack). A Vite + Svelte app
(no SvelteKit); its API (`api.mjs`) runs inside the dev server. Needs the local dev DB for the script text
(`npm run dev:db`) and the media dev copy (`media/`) for the character images.

## The layout

- **Sidebar – the shots** of the chapter in reading order, by scene: frame, id, format, view, review dot, variants.
  Resizable, folds to a rail.
- **Page – the 3D view**: the shot's scene with the frames' own renderer (`tools/storyboard-render/core.js`), live.
  Drag = orbit, right-drag = pan, wheel = dolly – the camera is written into the shot as a hand camera
  (`camera: { pos, target, fov }`); "Shot camera" goes back to the camera of size + angle. Click a figure → the gizmo
  moves it on the ground (Move) or turns it (Turn); its place goes back as the storyboard's x / y / h / turn. The texts
  in the SCUTOONS presets on top (switchable). **Look switches**: shadows Solid (Sin City, the default) · Hatch
  (crosshatching) · Tone, inner lines, colour, wobble (after Maxime Heckel's "Moebius-style post-processing"; black
  and white unless colour is on).
- **Right dock** (toggles in the top bar; each folds to a rail, drags wider): **Shot** – every field (what it shows,
  format + camera, light incl. a free direction, figures) + Save + frame / Keep as variant / Revert / Approve / Ask for a
  change / Rule (→ storyboard-taste.md); **Script** – the scene's script lines + notes; **Variants** – the saved frame,
  variants (Load / Take this), generated images dropped in (Approve as chosen → the dataset); **Luma** – the full prompt
  from the shot as set (scripts/storyboard/luma.js), the render without texts, the references to attach.
- **Top bar**: chapter, summary, Check, All frames, Reference packs, Chapter draft.

## Files

- the shots: `storyboards/<toon>/<chapter>/scene-*.json` (written as 2-space JSON)
- variants: `storyboards/<toon>/<chapter>/variants/<shot>.json`, frames under `storyboards/_out/<chapter>/` (`frames/`,
  `clean/` without texts, `variants/`, `generated/`)

## Not yet

- joints posed by hand in 3D (poses come from the pose list), props placed by hand, undo;
- the look saved per chapter (the switches are a view setting; `render-shots.mjs --look hatch,inner` for files);
- the move into the creator, if the team works with it (agents/STORYBOARD.md §7 / phase 5).
