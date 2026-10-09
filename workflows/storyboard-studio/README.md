# Storyboard Studio

A local tool for the storyboard workflow (owner 2026-10-09: "just start the studio over night. i want to see where
this goes. we might dump it later anyhow if there is easier way"). It shows the layers of one chapter side by side and
lets you change single frames by hand; everything goes back into the repo's storyboard files.

```bash
npm run dev:studio
```

→ http://localhost:14000 (`shared/dev-ports.json` "studio"; `npm run dev -- studio` starts it with the stack; the Claude
desktop app's preview: `studio`). Needs the local dev DB for the script text (`npm run dev:db`), `three` installed
(`npm install`) and, for the character images, the media dev copy (`media/`).

## Columns

1. **Skript** – the script lines of the selected scene (`script:<toon>:de`, local dev DB) and the scene's notes.
2. **Szenen → Shots** – every shot in reading order: its frame, view, review dot (green ok, orange change), variants.
3. **Render** – the selected shot rendered **live** in the page with the frames' own renderer
   (`tools/storyboard-render/core.js`). Drag a figure = its position (x / y); mouse wheel on it = its size (h);
   Alt + wheel = turn it.
4. **Beschreibung** – every field of the shot: what it shows (action, English prompt, view, source, note), format and
   camera (presets + a hand camera: distance, height, pitch, pan, roll, lens), light (key / height / source / shadow /
   rim, a free direction by azimuth / elevation, the accent), figures (cast, persona, pose, facing, framing, mood,
   x / y / h). Each change re-renders.
5. **Varianten** – "Als Variante" keeps the edited shot as a variant (with a note and its frame) without touching the
   shot; "Diese nehmen" swaps it in (the old shot becomes a variant); "Laden" loads it into the editor.

Buttons: **Speichern + Frame** (writes the shot, renders the frame at 2× → `storyboards/_out/<ch>/frames/`), **Urteil**
ok (→ `review` + the dataset, `approve.mjs`) / ändern (an open change, shown as a warning) / **+ Regel** (→
`workflows/scenes/storyboard-taste.md`), **Referenzpaket** for the shot; top bar: check, storyboard page, all frames,
reference packs, chapter draft, the chapter's warnings.

## Files

- the shots: `storyboards/<toon>/<chapter>/scene-*.json` (the studio writes them with 2-space JSON)
- variants: `storyboards/<toon>/<chapter>/variants/<shot>.json`, their frames `storyboards/_out/<chapter>/variants/`
- `server.mjs` (plain Node, 127.0.0.1 only) + `public/` (no build step; three from `node_modules`)

## Not yet

- a 3D view to grab joints directly (poses come from the pose list today), props placed by hand, undo;
- a column for generated images (Luma results) next to the variants – drop them in via `approve.mjs --image` for now;
- the move into the creator, if the team works with it (agents/STORYBOARD.md §7 / phase 5).
