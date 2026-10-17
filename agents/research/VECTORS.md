Status: decided 2026-10-17 (owner's answers in §7) – plan §6, steps 2–5 done locally (the API: §5a), next step 6; nothing on production. Today's state: ASSETS.md "Vector library".

# VECTORS – how the shape library lives next to the toons

Owner 2026-10-17: "research and develop a strategy for how to keep vectors stored alongside toons. i am a little
uncertain how to model it, and how to maintain it." Asked what they must be classified by: "in my own i use them
intuitively. just more by shape and how they follow or engage with the images or other elements on the page. so
overclassifying is maybe too dangerous" + **toon / universe**; a link to chapters / scenes **only through usage**.

## 1. What is there today (measured 2026-10-17)

- `shared/tokens/vectors/`: 429 SVGs (428 Figma nodes of Cat Pope → page "Vector" + 1 derived, the turned crescent
  chain), 11.6 MB in git; `vectors.json` one entry each; `vectors.css` (`--vector-<slug>`); `vectors:pull` re-exports
  over the Figma bridge and measures; `vectors-survey.mjs` makes the contact sheet (local `media/`, not git).
- Sizes: median 1.5 KB; the pizza sheets are 0.76–0.99 MB each (3 500 paths, no rasters), trippy-cat-eyes 0.6 MB.
- Classification as it stands: **20 roles** (line 134, motif 58, band 34, brush 31, land 31, texture 24 …) and **178
  tags**; 416 of 429 `suggested: true`. Much of it repeats itself: the colour tags (`red` 114, `white` 48 …) restate the
  measured `colors`, the role is repeated as a tag (`line` 137, `band` 29, `motif` 56 …). That is the over-classification
  the owner fears – words nobody will keep in step.
- Duplicates: 6 byte-identical pairs (cloud-red-3/4, straight-line-4/5, curve-red/-2, curve-red-3/4, curve-red-5/6,
  scaffold-frieze/-2); near-copies (pizza-large ≈ pizza-parts ≈ pizza-parts-2, pizza-medium ≈ pizza-medium-parts).
- `mask`: 239 one-colour shapes (CSS mask, the page gives the colour), 190 as-they-are; `tile` x 22, y 29.
- Users: the score looks only – illumination (5 shapes, `?url` masks), collage (a few `?raw`), riot / "Collage" (the whole
  library by `import.meta.glob`, each file its own lazy chunk, `vectors.json` lazy; entry `@scutoons/reader/score-lab`).
- Precedent on the CDN: `catpope/vectors/vector-demon-saw-1.svg` (the map / inventory frame, an `AssetUsages`
  code reference). The CDN serves SVG with `access-control-allow-origin: *` (checked 2026-10-17) – required for
  `mask-image` from another origin – and `cache-control: max-age=2592000` (30 days): a changed file needs a new name.
- The asset layer already has everything except shape facts: `assets` (slug, folder = toon, kind – SVG counts as
  `image`, title, tags), versions with sha256 / width / height, `asset_usages` (content, catalog, code), the delete check,
  rename / relink, the picker, the admin library; SVG uploads are checked for scripts / handlers.

## 2. Where the vectors live

| Option | For | Against |
|---|---|---|
| A. Git only (today) | simple, reviewable diffs, `?url` imports | not content: the creator can't pick from git, production isn't the source, every shape ships with the repo (12 MB, growing per toon), no usage index |
| B. CDN + asset manager only | content like every other medium: picker, usages, delete check, per toon | app code that needs a shape (a frame the UI draws) would depend on the CDN at build time |
| **C. Both, split by who uses it** | the library is content (CDN, DB); the few shapes app code draws stay in git | two places – but the same split as the fonts (git = source, CDN copy) |

**Recommendation: C.**
- **The library = content on the CDN**, registered in the asset manager, in the folder of the toon that owns it:
  `catpope/vectors/<slug>.svg` (the folder exists already). `scutoons/vectors/` only for shapes of the platform / brand
  that belong to no toon. Another toon uses a Cat Pope shape by reference (`../catpope/vectors/…`, ASSETS "Paths in
  content"), never a copy – so "classified by toon" is simply the folder, no field.
- **The code set stays in git** (`shared/tokens/vectors/`): only shapes app code imports (today the illumination's
  five and the 90s collage's twelve; later UI ornaments). Each is listed in `vectors.json` with the same slug as its CDN twin. The other ~420 files leave
  git once the CDN copy is registered (the lab then reads them from the CDN, §5).
- **Clean slate now** (owner: "we didnt do anythin with anythin yet"): nothing in content uses a shape, so the import
  needs no relink and files may be re-exported over themselves until a shape is first used. **Once a shape is used**, a
  changed one gets a new file name (`<slug>-2.svg`, as the Standard tool does) and one relink – the 30-day cache and
  readers' offline copies would otherwise keep the old shape (PLATFORM "a changed file gets a new name").
- Local `media/catpope/vectors/` is the dev copy like all media (`npm run dev:db -- media --apply`).

## 3. The model

| Option | Verdict |
|---|---|
| Catalog entries | no – the catalog is what readers own or buy (items, products); the owner said "not in the catalog" |
| A separate `vectors` table with its own files | no – it would rebuild the asset layer (versions, usages, delete, CDN) beside it |
| Everything in `assets.tags` (`role:band`, `tile:y`, `figma:2819:3886`) | possible without a migration, but facts code reads hidden in free words |
| **`assets` rows + a small side table `asset_shapes`** (1:1 by `asset_id`) | **recommended** – the asset layer as it is, plus the few facts that only shapes have |

`asset_shapes` – only what a screen edits or code reads (RULES "Keep the configuration small"):

| Column | Typed by | Why |
|---|---|---|
| `asset_id` | – | the asset (slug, folder = toon, title, tags, versions with size / sha256 come from there) |
| `tile` | a person | `x` / `y` / null – a band repeats along one axis; code needs it to repeat it |
| `mask` | measured | one colour → CSS mask, the page colours it; else the file as it is |
| `colors` | measured | JSON list; colour is never a tag |
| `source` | the import | JSON `{ file, page, node, frame }` – provenance only: where the shape came from, so a survey of the same page knows it |
| `derived` | the import | `{ from: <asset_id>, rotate: 90 }` – a turned or mirrored copy of another shape |

Not modelled: character / subject (a free tag like `scrooge`, `pizza` if wanted), chapter / scene (the usage index –
owner), colour words (measured), licence (own artwork; the Scrooge shapes are kept and used – owner, §7.6).

**Roles are tags, not a class** (owner: "consider it a tagging rather than a class. so we can keep the ones we got.
some are clearly more layout … we can use it to create interesting things in the future or decorate text boxes etc.").
The 20 role words of the README (band, frieze-unit, ring, corner, frame, line, brush, cloud, texture …) stay as the
**shared tag words**: they live in `assets.tags`, a shape may carry several, the README keeps their meaning so people
and the picker use the same words. The layout-ish ones (band, frieze-unit, corner, frame, ring, teeth) are what text
boxes and layouts will look for later. Free words beside them (`crescent`, `scrooge`, `medieval`) stay allowed; colour
words leave the tags (the measured `colors` say it).

How a shape engages in a given place (behind / over, blend, colour, follows a panel's edge) belongs to the **placement**
in the layout document, not to the shape – one shape is used many ways.

**Colour is a preset, not a property of the shape** (owner 2026-10-17: "colorations can be carried with once designed
somewhere as a preset"). Shapes stay neutral: no colour words in the tags; the measured `colors` / `mask` stay as facts
about the file (can it be tinted as a mask, or must it show as it is). A **colouration preset** – fill colour or
gradient, outline, blend mode, filter – is designed somewhere (a look like the Collage, later the creator), saved by
name in its own small list, and applied to any shape; a placement names a shape and a preset. Not built yet; where the
list lives (a catalog-like table, a toon's document) is decided when the first screen edits it.

## 4. Maintenance

**The library is the truth, no Figma sync** (owner 2026-10-17: "sync with figma? perhpase only if we ever create
something figma doesnt already have"). The Figma "Vector" page was a one-time source; after the import the library (CDN
+ DB) is edited in the asset manager, and `source` only says where a shape came from.
- **Every Figma access goes over the SCUTOONS Figma bridge** (owner 2026-10-17: "i got the figma bridge so we dont have
  to use api and mcp credits for figma"): survey, exports, placing a shape back – `tools/figma-bridge`, no call limit;
  never Figma's REST API or Figma's MCP (monthly read cap / credits). "The API" in this doc = the SCUTOONS API (uploads).
- **Importing a new Figma page** (another toon's shapes, a new Cat Pope page): `vectors-survey.mjs` lists the
  candidates (those whose node is already in the library marked "in the library"), the owner picks on the contact
  sheet, `vectors-pull.mjs` exports, measures and uploads them – a one-time import each time, not a sync. Uploads go
  through the SCUTOONS API (`PUT /api/assets/upload` + the shape fields) as the signed-in member – the dev API locally,
  production only when the owner runs it or asks; the MCP's `upload_asset` is the same path.
- **A shape made outside Figma** (an upload, an agent's drawing): it lives in the library only. Only if Figma should
  have it too, the bridge can place it there (`createNodeFromSvg`) – the other way round, and only when that happens.
- **A changed shape**: a new upload in the asset manager (before first use: over the old file; after: a new name +
  relink, §2).
- **Review**: today's names are confirmed (owner 2026-10-17). New shapes from a later survey come as suggestions; the
  contact-sheet artifact (claude.ai/artifact/FAwrTDdxJRwvZqh2RQuzus) is the prototype of a "To check" view – a grid of
  unchecked shapes with slug, title, tile, tags; Enter = checked. Renaming is free until a shape is used.
- **Duplicates**: byte-identical (same sha) never registered twice – the survey and the upload refuse them; near-copies
  (the pizza sheets) the owner decides in review.
- **Delete**: the asset layer's delete check (published uses and code references block); the code set is listed in
  `AssetUsages::CODE_REFERENCES` like the demon saw, so it is never "unused".

**Browse and pick.** The creator's asset picker gets a "Shapes" kind (an asset with a shape row): toon folder first,
role-tag chips (band, corner, frame …), search over title / tags; cards on a checker background, mask shapes shown tinted in the text colour,
the size and "edge x / y" on the card. The admin asset sheet shows and edits the shape fields. Picking into a layout
(drag onto a page) waits for the layout feature – the picker is the same either way.

## 5. How apps use them (without bloating the reader)

- **Content** (a layout, an ornament set in a document): a path like every image (`vectors/<slug>.svg` in the toon,
  `toonAsset`), loaded from the CDN only when the page shows it. `mask: true` → `mask-image: url(<cdn>)` +
  `background: <colour from the look>` (CORS is on); `mask: false` → `<img>` / `background-image`. Inline SVG (fetch the
  text, insert) only where a look needs per-path blends – the upload already strips scripts.
- **App code**: single files by `?url` (or `?raw` for inline) from the git code set – one small chunk each.
  `vectors.css` shrinks to the code set (or goes); never imported whole.
- **The lab's Collage**: today it globs all 429 files into the build (lazy chunks, but 12 MB in every app build that
  takes `score-lab`). After the move it asks the API for the shape list (`GET /api/assets?kind=shape&folder=catpope`)
  and loads files from the CDN – nothing in the bundle.
- **Heavy shapes**: every SVG is shrunk at upload (owner: "shrink is fine" – svgo: precision, merged paths; the pizza
  sheets are ~1 MB); the library keeps the shrunk file (Figma is not compared again).

## 5a. The API (the contract – built 2026-10-17, migration `2026-10-17-asset-shapes`)

Table `asset_shapes` (1:1 with `assets`, deleted with it): `tile` x | y | null, `mask`, `colors` (JSON), `source` /
`derived` (JSON). No `checked` fields – the names are confirmed and no screen reviews yet (RULES "keep the configuration
small"); a review flag comes back with a review screen.

- **`GET /api/assets?kind=shape&folder=catpope`** (content editors – creator, admin; `page_size` ≤ 500; `q`, `tag`,
  `sort` as for every asset) → `{ items, pagination, folders, cdn }`; each item = the asset summary (`id`, `slug`,
  `title`, `tags` – role words and free words –, `path` = the SVG's CDN-root path, `width` / `height` from the SVG,
  `size`, `usages`, `renditions`) **+ `shape: { tile, mask, colors }`**. The file's URL = `cdn` + `path`; a page loads
  it only when it shows the shape; `mask: true` → `mask-image` + the colouration preset's colour.
- `GET /api/assets/{id}` → the detail; `asset.shape` also carries `source` (Figma provenance) and `derived`.
- `PUT /api/assets/{id}` `{ shape: { tile, mask, colors, source?, derived? } }` makes an asset a shape (or updates it),
  `{ shape: null }` takes it out of the library – the import does this; the admin asset sheet edits `tile`.
- Types: `@scutoons/api` `AssetShape`, `AssetShapeDetail`, `AssetSummary.shape`; `assets.list({ kind: 'shape', folder })`.

## 6. Plan (steps, one at a time)

1. ~~Owner decides~~ – done 2026-10-17 (§7).
2. ~~Tidy in git~~ – done 2026-10-17 (423 shapes). (no DB): `role` folds into `tags` (a shape may carry several role words), colour words leave the
   tags, `suggested` goes (names confirmed), the 6 identical pairs merge; the README's vocabulary becomes "shared tag
   words".
3. ~~Split Uncle Scrooge~~ – done 2026-10-17: the 18 shapes are two drawings (front face, also as the big "ghost"; the
   profile), drawn as 36–45 loose vectors each; 13 parts ≥ 80 px (and the two Bitcoin eyes) became shapes – eyes
   (`bitcoin`), hat brims, jaw, brow, strokes – `figma.part_of` names the drawing; beak and sideburns were already in
   (red-lash, face-outline-red: tagged `scrooge` now). 436 shapes. Was: (owner: "maybe we can split him up"): the 18 `scrooge` shapes into parts – eyes, hat, beak,
   whiskers … – as their own shapes (the survey's "ungrouped" parts of the Figma groups; the owner picks on a contact
   sheet), tagged `scrooge` + the part.
4. ~~Migration~~ – done 2026-10-17 (§5a). Was: **Migration** `asset_shapes` (idempotent, dry-run on the dev DB, runs with the deploy); `kind=shape` as a filter of
   the list API (an asset with a shape row), the asset sheet shows the fields.
5. ~~Import~~ – done on the LOCAL dev stack 2026-10-18: `npm run vectors:import` (scripts/vectors-import.mjs, svgo with
   viewBox and size kept: 11.6 MB → 6.7 MB, pixel diff of a sample < 0.1 % – anti-aliasing) uploaded all 436 to
   `catpope/vectors/` as the test user and wrote their shape rows; a second run changes nothing. Production: the same
   script once the owner asks (it refuses any API but localhost until then). Was: **Import** to `catpope/vectors/` through the API, shrunk (svgo), shape rows from `vectors.json`, no relink (clean
   slate); the code set (the illumination's five, the 90s collage's twelve) stays in git with its CDN twins as code
   references. Locally first, production when the owner runs it or asks.
6. **The lab reads the CDN**: the Collage look (score-riot) takes the shape list from the API and loads files by CDN
   URL instead of globbing git; the non-code shapes leave git; ASSETS.md "Vector library" rewritten to how it is.
7. ~~Pull becomes sync~~ – dropped (owner: no Figma sync). The survey and pull scripts stay as import tools for new
   pages: the survey marks nodes already in the library, the pull uploads through the API instead of writing to git.
8. **Picker** "Shapes" + role-tag chips; the "To check" view for later surveys.
9. Later: shapes in layouts and on text boxes (placement = shape + where + a colouration preset); the presets as their
   own small named list, designed in a look or the creator, never stored on the shape (§3).

## 7. The owner's answers (2026-10-17)

1. **Where**: "cdn" – the library on the CDN, `catpope/vectors/`; only the code set stays in git.
2. **Classification**: "consider it a tagging rather than a class. so we can keep the ones we got" – the 20 roles stay,
   as tags (§3); no six forms.
3. **Owner toon**: "all these vectors belong to catpope yes".
4. **Changed shape**: "why relink. we didnt do anythin with anythin yet this is clean slate" – no relink for the import;
   new name + relink only once a shape is used (§2).
5. **Optimisation**: "shrink is fine." – svgo at upload.
6. **Uncle Scrooge**: "we keep it and we use it for now. maybe we can split him up for now." – kept, usable; split into
   parts (§6 step 3).
7. **Names**: "names are fine." – the 416 suggestions are confirmed.
8. **Figma sync** (asked on the plan): "sync with figma? perhpase only if we ever create something figma doesnt already
   have" – no sync; Figma was the one-time source, the library is the truth (§4).
