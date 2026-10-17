# The vector library

Shapes from our own artwork (owner 2026-10-17: "der Anfang von einer … Vektoren-Shapes-Datenbank … für solche Arbeiten
und auch fürs Layouting"), sorted so they can move into the asset database later (agents/ASSETS.md). Not in the catalog
yet. Today: the Cat Pope Figma file, page "Vector".

- `<slug>.svg` – the shape, as Figma exports it (or `derived` from another entry).
- `vectors.json` – one entry per shape = one future `assets` row: `slug`, `title`, `role`, `tile`, `tags`, the source
  (`figma`: file / page / node / frame, so it can be exported again), `used_by`; measured by the script: `path`,
  `width`, `height`, `colors`, `mask`, `bytes`, `sha256`.
- `../vectors.css` (`@scutoons/tokens/vectors.css`) – `--vector-<slug>: url(…)` for every shape, written by the script.

## Add or refresh

1. The Figma bridge running (`node tools/figma-bridge/relay.mjs`, the plugin open on the file – tools/figma-bridge).
2. A new entry in `vectors.json` with `slug`, `title`, `role`, `tile`, `tags` and `figma.node` (the node id from Figma).
3. `npm run vectors:pull` (all) / `npm run vectors:pull -- <slug> …` (only these) / `-- --local` (no Figma: derive and
   measure what is on disk).

## Vocabulary

| `role` | What it is | Used like |
|---|---|---|
| `band` | a strip that repeats along one axis (`tile`: `x` / `y`) | a frame edge, a divider |
| `frieze-unit` | one piece of a frieze, repeated side by side | a foot or head band |
| `ring` | a closed round shape | corners, around a portrait |
| `motif` | one figure or sign | a vignette, a stamp |
| `filigree` | an open, symmetric ornament | a headpiece, a corner, a background |
| `silhouette` | a filled outline | a cut-out, a backdrop |
| `seal` | a round, self-contained emblem | a colophon, a badge |

`tags`: free words, lower case – colour (`red`, `black`), form (`crescent`, `dots`, `arch`), subject (`catpope`, `face`),
use (`border`, `corner`, `medallion`, `headpiece`).

`mask: true` = one colour (near-blacks count as one): use it as a CSS mask and the page gives the colour
(`mask-image: var(--vector-wreath); background: var(--ornament)`). `mask: false` = several colours or white parts: an
image as it is (a mask would fill the white too).
