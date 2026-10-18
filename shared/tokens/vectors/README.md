# The vector library – the code set

**Since 2026-10-18 the library lives in the asset manager:** all 436 shapes as `catpope/vectors/<slug>.svg` on the
CDN with their shape rows, listed by `GET /api/assets?kind=shape` (agents/research/VECTORS.md §5a) – that is where
pages and the creator find them. This folder keeps only the **code set**: the 13 shapes app code imports by file
(`?url` / `?raw` – the score lab looks), each the same slug as its CDN twin. The other entries are in git history.

Shapes from our own artwork (owner 2026-10-17: "der Anfang von einer … Vektoren-Shapes-Datenbank … für solche Arbeiten
und auch fürs Layouting"), sorted so they can move into the asset database later (agents/ASSETS.md). Not in the catalog
yet. Today: the Cat Pope Figma file, page "Vector".

- `<slug>.svg` – the shape, as Figma exports it (or `derived` from another entry).
- `vectors.json` – one entry per shape = one future `assets` row: `slug`, `title`, `tags`, `tile`, the source
  (`figma`: file / page / node / frame – where it came from; `copies` = other nodes with the same bytes), `used_by`;
  measured by the script: `path`, `width`, `height`, `colors`, `mask`, `bytes`, `sha256`. The names are the
  owner's (confirmed 2026-10-17). The same bytes twice are one shape – `vectors:pull` reports twins.
- `../vectors.css` (`@scutoons/tokens/vectors.css`) – `--vector-<slug>: url(…)` for every shape, written by the script.

## Add or refresh

1. The Figma bridge running (`node tools/figma-bridge/relay.mjs`, the plugin open on the file – tools/figma-bridge).
2. A new entry in `vectors.json` with `slug`, `title`, `tags`, `tile` and `figma.node` (the node id from Figma).
3. `npm run vectors:pull` (all) / `npm run vectors:pull -- <slug> …` (only these) / `-- --local` (no Figma: derive and
   measure what is on disk).

## Into the asset manager

`npm run vectors:import [-- --dry-run] [slug …]` shrinks every SVG with svgo (viewBox and size kept) and uploads it
through the SCUTOONS API to `catpope/vectors/<slug>.svg`, then writes its shape row (tile, mask, colours, Figma
provenance, derived) – `GET /api/assets?kind=shape` lists them (agents/research/VECTORS.md §5a). Idempotent; local dev
API only (the test user, `npm run dev:test-user`) until the owner asks for production.

## Survey a whole page

`node scripts/vectors-survey.mjs [--page Vector]` reads a Figma page over the bridge and lists every candidate – named
things, every group and the parts inside it big enough on their own ("ungrouped" on export, the file never changes),
big single vectors – without copies, with a thumbnail each, into `media/scutoons/vectors-survey/<page>/` (local, not in
git). Candidates already here (by Figma node) carry `library: <slug>`. The owner picks from that contact sheet; the
picks become entries here.

## Tags

Tags, not classes (owner 2026-10-17: "consider it a tagging rather than a class"): a shape carries as many as fit, lower
case. These shared words say what a shape does next to a picture or text – use them, so people and pickers find the
same things; the layout-ish ones (`band`, `frieze-unit`, `corner`, `frame`, `ring`, `teeth`) are what text boxes
and layouts will look for.

| Tag | What it is | Used like |
|---|---|---|
| `band` | a strip that repeats along one axis (`tile`: `x` / `y`) | a frame edge, a divider |
| `frieze-unit` | one piece of a frieze, repeated side by side | a foot or head band |
| `teeth` | a zigzag edge (zickzack barrier teeth, the saw) | a barrier, a cut edge |
| `ring` | a closed round shape | corners, around a portrait |
| `corner` | a piece made for a corner | a frame's corners, a text box |
| `frame` | a whole frame | around a picture or text |
| `filigree` | an open, symmetric ornament | a headpiece, a corner, a background |
| `motif` | one figure or sign | a vignette, a stamp |
| `silhouette` | a filled outline | a cut-out, a backdrop |
| `seal` | a round, self-contained emblem | a colophon, a badge |
| `eyes` | eyes on their own (trippy cat eyes) | a stare, a pattern |
| `splash` | a splash or splat | an accent, a hit |
| `line` | a line or arc | crossing, underlining, connecting |
| `brush` | a brush stroke | a gesture over a picture |
| `vessels` | branching lines like veins or blood vessels | a background, a map |
| `cloud` | a cloud or smoke shape (the saw clouds) | a sky, a backdrop |
| `land` | a piece of land or ground | a horizon, a backdrop |
| `texture` | a surface to fill with (dagotex) | a fill, a backdrop |
| `toy-sheet` | a punch-out sheet whose parts break out (pizza-large / -medium / -small) | a source of many parts |

Other words are free: form (`crescent`, `dots`, `arch`), subject (`scrooge`, `catpope`, `pizza`), style (`medieval`,
`gradient`). **No colour words** – `colors` is measured.

`mask: true` = one colour (near-blacks count as one): use it as a CSS mask and the page gives the colour
(`mask-image: var(--vector-wreath); background: var(--ornament)`). `mask: false` = several colours or white parts: an
image as it is (a mask would fill the white too).
