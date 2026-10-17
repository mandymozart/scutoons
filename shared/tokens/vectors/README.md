# The vector library

Shapes from our own artwork (owner 2026-10-17: "der Anfang von einer … Vektoren-Shapes-Datenbank … für solche Arbeiten
und auch fürs Layouting"), sorted so they can move into the asset database later (agents/ASSETS.md). Not in the catalog
yet. Today: the Cat Pope Figma file, page "Vector".

- `<slug>.svg` – the shape, as Figma exports it (or `derived` from another entry).
- `vectors.json` – one entry per shape = one future `assets` row: `slug`, `title`, `role`, `tile`, `tags`, the source
  (`figma`: file / page / node / frame, so it can be exported again), `used_by`; measured by the script: `path`,
  `width`, `height`, `colors`, `mask`, `bytes`, `sha256`.
  `suggested: true` = name, role and tags were proposed from a look at the shape (the contact sheet, 2026-10-17) and
  not yet confirmed by the owner; drop the flag when a person has checked the entry.
- `../vectors.css` (`@scutoons/tokens/vectors.css`) – `--vector-<slug>: url(…)` for every shape, written by the script.

## Add or refresh

1. The Figma bridge running (`node tools/figma-bridge/relay.mjs`, the plugin open on the file – tools/figma-bridge).
2. A new entry in `vectors.json` with `slug`, `title`, `role`, `tile`, `tags` and `figma.node` (the node id from Figma).
3. `npm run vectors:pull` (all) / `npm run vectors:pull -- <slug> …` (only these) / `-- --local` (no Figma: derive and
   measure what is on disk).

## Survey a whole page

`node scripts/vectors-survey.mjs [--page Vector]` reads a Figma page over the bridge and lists every candidate – named
things, every group and the parts inside it big enough on their own ("ungrouped" on export, the file never changes),
big single vectors – without copies, with a thumbnail each, into `media/scutoons/vectors-survey/<page>/` (local, not in
git). The owner picks from that contact sheet; the picks become entries here.

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
| `cloud` | a cloud or smoke shape (the saw clouds) | a sky, a backdrop |
| `eyes` | eyes on their own (trippy cat eyes) | a stare, a pattern |
| `teeth` | a zigzag edge (zickzack barrier teeth, the saw) | a barrier, a cut edge |
| `vessels` | branching lines like veins or blood vessels | a background, a map |
| `toy-sheet` | a punch-out sheet whose parts break out (pizza-large / -medium / -small) | a source of many parts |
| `texture` | a surface to fill with (dagotex) | a fill, a backdrop |
| `other` | anything else, until it has a word | – |

`tags`: free words, lower case – colour (`red`, `black`), form (`crescent`, `dots`, `arch`), subject (`catpope`, `face`),
use (`border`, `corner`, `medallion`, `headpiece`).

`mask: true` = one colour (near-blacks count as one): use it as a CSS mask and the page gives the colour
(`mask-image: var(--vector-wreath); background: var(--ornament)`). `mask: false` = several colours or white parts: an
image as it is (a mask would fill the white too).
