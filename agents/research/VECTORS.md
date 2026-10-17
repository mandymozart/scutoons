Status: proposal 2026-10-17 – for the owner to decide (§7); nothing built, nothing uploaded. Today's state: ASSETS.md "Vector library".

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
- **The code set stays in git** (`shared/tokens/vectors/`): only shapes app code imports (today the illumination's five;
  later UI ornaments). Each is listed in `vectors.json` with the same slug as its CDN twin. The other ~420 files leave
  git once the CDN copy is registered (the lab then reads them from the CDN, §5).
- A changed shape gets a **new file name** (30-day cache, PLATFORM "a changed file gets a new name"): `<slug>-2.svg`,
  as the Standard tool does, then one relink (drafts per document, the member publishes). No in-place replace.
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
| `form` | a person (an agent may suggest) | the ONE classification word – below |
| `tile` | a person | `x` / `y` / null – a band repeats along one axis; code needs it to repeat it |
| `mask` | measured | one colour → CSS mask, the page colours it; else the file as it is |
| `colors` | measured | JSON list; colour is never a tag |
| `source` | the pull | JSON `{ file, page, node, frame }` – Figma's node id is the shape's identity across re-exports and renames |
| `source_sha256` | the pull | sha of the raw Figma export (the stored file may be optimised later and differ) |
| `derived` | the pull | `{ from: <asset_id>, rotate: 90 }` – a turned or mirrored copy made by the script, re-made when its source changes |
| `checked_at` / `checked_by` | the review | replaces `suggested: true`; null = a suggestion |

Not modelled: character / subject (a free tag like `scrooge`, `pizza` if wanted), chapter / scene (the usage index –
owner), colour words (measured), licence (own artwork; see §7 for the one exception).

**The `form` word – by shape and how it meets the page** (owner: "by shape and how they follow or engage with the
images or other elements"). Six words instead of today's 20 roles; each says what the shape does next to a picture:

| form | does | today's roles in it |
|---|---|---|
| `edge` | runs along an edge, repeats (`tile` x / y) | band, frieze-unit, teeth |
| `frame` | surrounds or corners something | ring, corner, frame, filigree |
| `figure` | stands on its own beside or over a picture | motif, silhouette, seal, eyes, splash |
| `line` | crosses, connects, underlines | line, brush, vessels |
| `field` | fills an area behind or over | texture, cloud, land |
| `sheet` | a sheet to take parts from | toy-sheet |

How a shape engages in a given place (behind / over, blend, colour, follows a panel's edge) belongs to the **placement**
in the layout document, not to the shape – one shape is used many ways. Free `tags` stay optional search words for what
the form doesn't say (`crescent`, `scrooge`, `pizza`, `medieval`), never colours, never the form again.

## 4. Maintenance

**Source.** Figma is where the owner makes shapes; the bytes come from there, the asset layer holds them. Every shape
row knows its Figma node.
- **Survey** (`vectors-survey.mjs`): lists candidates on a page; candidates whose node is already in the library are
  marked "in the library" – the contact sheet shows only new ones.
- **Pull → sync** (`vectors-pull.mjs` grows): per shape export the node, compare with `source_sha256`:
  same → nothing; changed → upload `<slug>-<n>.svg`, new current version of the same shape, relink (drafts);
  node gone in Figma → reported "gone in Figma", never deleted by the script; derived shapes re-made after their source.
  It writes through the API (`PUT /api/assets/upload` + the shape fields) as the signed-in member – the dev API locally,
  production only when the owner runs it or asks; the MCP's `upload_asset` is the same path.
- **Review** (the owner, in batches): the contact-sheet artifact (claude.ai/artifact/FAwrTDdxJRwvZqh2RQuzus) is the
  prototype of a "To check" view – a grid of unchecked shapes, each with slug, title, form, tile, tags; Enter = checked.
  Renaming is free until a shape is used (a rename is copy + relink afterwards), so check before first use.
- **Duplicates**: byte-identical (same sha) never registered twice – the survey and the upload refuse them; near-copies
  (the pizza sheets) the owner decides in review.
- **Delete**: the asset layer's delete check (published uses and code references block); the code set is listed in
  `AssetUsages::CODE_REFERENCES` like the demon saw, so it is never "unused".

**Browse and pick.** The creator's asset picker gets a "Shapes" kind (an asset with a shape row): toon folder first,
`form` chips, search over title / tags; cards on a checker background, mask shapes shown tinted in the text colour,
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
- **Heavy shapes**: an optional SVG pass at upload (svgo: precision, merged paths) would shrink the pizza sheets; it
  changes the bytes, which is why `source_sha256` keeps the Figma sha for the sync. Owner decision (§7).

## 6. Plan (steps, one at a time)

1. **Owner decides** §7. Nothing else starts before.
2. **Tidy in git** (no DB): `form` replaces `role` per the table, colour and role words leave `tags`, the 6 identical
   pairs merge; the owner checks the suggestions on the contact sheet (forms, names, near-copies, Scrooge).
3. **Migration** `asset_shapes` (idempotent, dry-run on the dev DB, runs with the deploy); `kind=shape` as a filter of
   the list API (an asset with a shape row), the asset sheet shows the fields.
4. **Import** to `catpope/vectors/` through the API, shape rows from `vectors.json`; the code set + its CDN twins as code
   references. Locally first, production when the owner runs it or asks.
5. **Pull becomes sync** (sha compare, new name + relink on change, "gone in Figma", survey marks known nodes).
6. **The lab reads the CDN** (Collage from the API list); the non-code shapes leave git; ASSETS.md "Vector library"
   rewritten to how it is.
7. **Picker** "Shapes" + form chips; the "To check" view in the admin (or creator).
8. Later: shapes in layouts (placement = shape + where + blend + colour), when the layout work starts.

## 7. Open questions for the owner

1. **Where**: library on the CDN per toon (`catpope/vectors/`), only the code set in git – OK?
2. **The six forms** (edge, frame, figure, line, field, sheet) instead of 20 roles – right words, too many, too few?
3. **Owner toon**: the whole "Vector" page belongs to Cat Pope, or are some universe-wide (`scutoons/vectors/`)?
4. **A changed shape**: new file name + relink (recommended, cache-safe) vs replace in place (old shape may show for
   up to 30 days in browsers and offline copies)?
5. **SVG optimisation** at upload (smaller files, bytes differ from Figma) – yes / no?
6. **Uncle Scrooge** (18 shapes tagged `scrooge`, the Dagobert faces): a Disney character – may these appear on public
   pages, or stay in the lab?
7. **Review**: check the 416 suggestions now in one sitting, or only when a shape is first used?
