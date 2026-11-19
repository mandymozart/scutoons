# The sandbox toon – real data for demos and tests (#642, #643)

**Rule:** anything a session needs for a demo, a test or "let me try this" lives in the toon `sandbox`, as real
drafts – never in the other toons, never as a special case in code. Nothing here is ever published by a worker;
the owner publishes in the editor / creator. (Created in production 2026-11-19 through the admin.)

## What is in it (all drafts, 2026-11-19)

| what | key / id | notes |
|---|---|---|
| the toon | `toon:sandbox` | de + en, source de, status draft |
| character | `character:sandgirl` | regular / default look, **no images yet** (validator warns: neutral portrait needed); colour `#d9b26f`, tag `sandbox` |
| place | `place:sandbox-dunes` | map place, toons `["sandbox"]`, hidden, x 1300 / y 400, **no image yet** |
| story | `chapter:sandbox:1` "Dünen / Dunes" | group `dunes`, one placeholder slice (a generated sand gradient, `sandbox/s1/c1/`); unlocks the place, joins Sandgirl, a message, a choice (sets `sandbox_way`, adds to `sandbox_laps`), a `divert` to label `end`, leaves |
| scenes | `scene:sandy-dunes` · `sandy-dunes-sandgirl` · `sandy-dunes-stage` | toons `["sandbox"]`. Empty stage / Sandgirl alone (`rig:character.sandgirl`, not authored yet) / two placeholder holders (`left`, `right`, any character, join) |
| shows | `show:demo-far-nodes` · `demo-new-scene` · `demo-values-and-scenes` · `demo-vote-and-question` | the four editor demos from `deck/demos/*.show.json`, ids prefixed `demo-`, toon `sandbox` |

## Gaps to fill when a test needs them
- **No dunes `set`**: the scenes have a `space` node without `set` (there is no sand 3D set yet; sets are `.glb` files
  like `cathedral`). Add `"set": "<id>"` to the `space` node (or a `location:<id>`) once one exists.
- **No images / rig** for Sandgirl; no map image for the place.
- The chapter's events have no ids yet: open it once in the creator and save – it assigns them.
- The story has no season document; the toon says `chapters: 0` until the creator counts it.

## How to work in it
- Via the SCUTOONS MCP: `list_shows` / `read_show` / `save_show_draft` (toon `sandbox`), `list_scenes` / `read_scene` /
  `save_scene_draft`, `save_draft` for `character:`, `place:`, `chapter:sandbox:N`. Never use the owner's token outside the MCP.
- Name new demo things `demo-<name>` (shows) or `sandy-…` / `sandbox-…` (scenes, places) so they are recognisable.
- Platform-wide kinds (`character:`, `place:`, `scene:`) have no toon in their key: the sandbox ones carry `toons: ["sandbox"]`
  / the tag `sandbox`.
- Cleanup is never automatic: delete a demo only on the owner's word.

Refs #642 #643
