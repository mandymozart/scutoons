# ASSETS — the media asset layer, storage drivers, bunny.net, the asset selector

Built 2026-10-02 (merged; `main` only since 2026-10-05); how it is now, with the bunny.net set-up as history. Companion docs: PLATFORM.md
"CDN" + cutover, DATA.md "Language-independent media", ITEMS.md "Players, albums, related items", server
docs/CONFIGURATION.md + docs/API.md "Assets".

Owner (2026-10-02): "An asset selector will be very much needed, like the character selector … an extension to the CDN:
we need to be able to exchange files on the CDN … link my production bunny.net to an asset manager: uploading content,
relinking content, and perhaps some optimization tasks. Some sanity checks to see if deleting a media causes issues
anywhere. Perhaps we need an in-between layer that stores media assets and versions and handles links to bunny.net – this
way we could even migrate in the future or run various systems behind it. Long-term compatibility is key. And bandwidth."

## Decisions

1. **Content keeps referencing stable CDN paths** (`videos/s1-trailer-loop.720p.mp4` in the toon's folder, catalog paths
   from the CDN root). The registry maps logical assets ↔ files; readers never call the layer. Replacing a file keeps its
   path (new version + purge); a rename copies and relinks.
2. **No `asset:<id>` references now.** Reasons: the reader and the seed / interchange JSON stay self-contained and work
   without the registry (static export, PWA preload, other hosts); every existing tool (the validation's
   media checks, the language-independence audit) keeps working; a path is human-readable in the creator's raw JSON.
   *Compatible later:* if ever wanted, `asset:<id>[@quality]` would be resolved by the server's library at compose time
   (the entity store keeps the reference, the reader gets paths) – the usage scanner already maps both directions, so
   introducing it is a migration, not a rewrite. Not needed for anything the owner asked.
3. **The registry is an index, not the source of truth for bytes.** The storage is. `assets:scan` (re-)registers what is
   on the storage at any time; out-of-band uploads (the bunny dashboard) are picked up by a rescan.
4. **Two places:** creators pick and upload in the creator (the asset picker); admins manage storage, delete, rename,
   relink, run jobs and see bandwidth in the admin (Assets page). The API checks the same split (content editors vs admin).
5. **Grouping by name conventions** (PLAN.md "RULE: one file per quality level"): files that differ only by a marker or
   by their image format are one asset (see "Grouping").
6. **Delete is guarded:** published content and the catalog always block; draft-only uses block unless an admin forces.
7. **Writes to bunny.net are opt-in per environment:** `ASSET_STORAGE=auto` uses the local folder whenever it exists
   (every dev machine), so credentials in the root `.env` never turn a dev server into a writer on the production zone;
   `BUNNY_WRITE_PREFIX` can fence writes further (`_test/`).

## Data model (migration `2026-10-02-assets`, MySQL)

| Table | What | Key columns |
|---|---|---|
| `assets` | one logical medium | `id` (uuid7), `slug` (the path without marker + extension, unique: `catpope/videos/s1-trailer-loop`), `kind` image / video / audio / archive / document / model / other, `folder` (top-level CDN folder = toon id, `characters`, `scutoons` …), `title`, `tags` (JSON), `status` active / deleted, created/updated/by |
| `asset_versions` | one upload of it | `number`, `path` (the version's source file), `storage`, `size`, `sha256`, `mime`, `width`, `height`, `duration`, `status` current / archived / purged, `archive_path` (old bytes after an in-place replace), `note`, created/by |
| `asset_renditions` | every file of a version (incl. the source) | `label` (original, 480p, 720p, 1080p, 1280w, 2x, 3x, webp, 128k, poster), `quality` (MediaQuality low / medium / high), `path`, `size`, `sha256`, `mime`, `width`, `height`, `duration`, `bitrate`, `origin` imported / uploaded / generated |
| `asset_usages` | the usage index | `path` (CDN-root), `source` content / catalog / code, `ref` (entity key; catalog entry `<kind>:<id>` – `item:camera`, `product:dago_hoodie` – since the catalog is in tables, 2026-10-04 agents/CATALOG.md; code file), `field` (JSON pointer), `state` published / draft / config, `version_id`, `toon` (the folder it resolved against), `raw` (the string as written) |
| `asset_jobs` | optimization jobs | `kind` video-renditions / image-webp / poster, `status` queued / running / done / failed, `params`, `log` |

Without the migration the asset endpoints answer 503 `not_available`; nothing else changes (the content save hook is
silent). Domain: `server/src/Domain/Assets` (AssetPaths, AssetRegistry, AssetImporter, AssetUsages, AssetService,
AssetJobs, MediaProbe); storage: `server/src/Infrastructure/AssetStorage`.

### Grouping (`AssetPaths`)

| Files | Asset (slug) | Labels / quality |
|---|---|---|
| `videos/x.{480p,720p,1080p}.mp4` | `…/videos/x` | 480p low, 720p medium, 1080p high |
| `videos/x.{1280w,1920w}.mp4`, `x.mp4` | same slug | widths; a plain file is `original` |
| `…/x.{96,128,320}.mp3` | `…/x` | 96k low, 128k medium, 320k high (≤ 96 low, ≤ 160 medium) – the content's audio levels (below) |
| `images/{1x,2x,3x}/y.jpg`, `y@2x.png` | `…/images/y` | 1x low, 2x medium, 3x high |
| `images/z.png` + `images/z.webp` | `…/images/z` | the PNG/JPEG is `original`, the WebP twin `webp` |
| `videos/x.poster.webp` | `…/videos/x` | poster (a video's frame) |

The version's source = a plain file in a delivered format, else the biggest marker. Dot files / folders (`.versions/`),
Thumbs.db, half-written uploads are never registered.

### Usages (`AssetUsages`)

Every string with a media extension in: the content store (per entity its **published** version and, when newer, its
**working draft**; `asset-requests` are wishes, not uses) and the catalog rows (items incl. albums, products, editions,
toons, featured – source `catalog`, agents/CATALOG.md; the files until 2026-10-04). Content paths resolve against the document's toon folder – the nearest
enclosing `toon` field wins (map locations) – and fall back to the CDN root when only that exists (registry style).
History versions are not indexed (the delete check says so: restoring an old version may bring a reference back). Rebuilt
by `assets:scan`, refreshed per entity on every creator save / publish / restore (EntitiesController). Not indexed: paths
built in app code beyond `AssetUsages::CODE_REFERENCES` (source `code`, state `config`: `scutoons/models/` – the camera's
MobileNet – and the map / inventory frame `catpope/vectors/vector-demon-saw-1.svg`; AssetsTest checks each entry still
appears in its code file). Add a path there whenever app code builds a CDN path itself.

## Paths in content (BACKLOG 10.39, 2026-10-07)

A toon document stores media paths relative to its toon's folder (`s1/c1/s1c1-slice-3.webp`). Another toon's file is
referenced, never copied: `../catpope/s1/c2/s1c2-slice-53.webp` (the asset picker writes that – `toonRelativePath` –
and so does a paste across toons, `rebaseToonAsset`); a leading `/` starts at the CDN root (`/characters/x.webp`, the
VN character views the reader gets). ONE resolver, `@scutoons/content` **`toonAsset(toon, path)`** → the path from the
CDN root (`.` / `..` resolved in code, not by the browser's URL normalisation); the apps' `toonUrl(toon, path)` (viewer
and creator `utils/cdn.ts`) = `asset(toonAsset(…))` – every former `` `${toon}/${path}` `` join uses it (slices, their
levels and videos, keyvisuals, thumbnails, covers, hero, credits, download, audio, the media slots, `rootPath`).
Stored `slice.image` values are never rewritten (they are part of the pickup key `picked:<slice image>#<id>`).
Dev data 2026-10-07 (all versions): 272 out-of-folder values, all `../` (slice images 32 + 162 scene, thumbnails 19 +
3, season squares 38, toon avatars 3, tile images 18 – e.g. toptoy / yellowmouse / sandbox chapters using catpope's
slices); 0 with a leading `/`. Writing `../` for pastes keeps ONE form in the data (owner 2026-10-07: keep `../` –
the BACKLOG draft's `/<toon>/…` is not written; a leading `/` still resolves, from the CDN root).

## Operations (`AssetService`, API + CLI)

| Operation | What happens | Who |
|---|---|---|
| upload | a NEW path (409 `exists` otherwise); becomes a rendition of the asset with the same slug or a new asset; magic bytes must match the extension; SVG without script / handlers; ≤ `ASSET_UPLOAD_MAX_BYTES` (raw PUT body, streamed to disk) | content editors (the picker), CLI `assets:jobs --upload` |
| replace | new bytes at the same path: old bytes copied to `.versions/<asset>/<n>/<file>`, new version, renditions move along (the source's derived files reported stale), CDN purge | admin (by id); content editors by path: `PUT /assets/replace?path=` (owner 2026-10-03 – the picker asks "Replace?" when an upload's path exists, the slice import updates slices it already has) |
| relink | every use of path A → existing path B: content gets a new **draft** per entity (`publish: true` publishes when the newest version was the published one), catalog entries are updated in their table (validated, catalog_history; a refused one is listed as manual) | admin |
| rename | copies the current files to the new slug (markers / extensions kept), new version, relink; old files stay with the archived version until nothing uses them | admin |
| delete | the delete check (uses by state); published / catalog uses block, draft-only uses need `force`; deletes all versions' files + purges; the row stays `deleted` for history | admin |
| missing / orphans | uses whose file the registry doesn't know · current files nothing references | admin page, API |
| cleanup | admin → Assets → Unused (owner 2026-10-03: "clean up the CDN"): per asset the files nothing references (index rebuilt first) – whole unused assets, unused renditions of used assets (never their source), files left behind by a rename, replace archives under `.versions/`; select rows → one in-page confirm → `POST /admin/assets/cleanup`, which rebuilds the index again and skips anything referenced by now | admin |
| jobs | queued in the admin / CLI, run by `php bin/console assets:jobs` where ffmpeg is (the owner's machine): download the source, transcode, upload next to it, register | admin, CLI |

Video renditions = the trailer-loop recipe: H.264 `-preset slow -crf 24 / 24 / 22` for 480p / 720p / 1080p, `yuv420p`,
AAC 128k, `+faststart`; only levels up to the source's height; existing files kept unless `params.overwrite` (then replaced
in place + purged). Image WebP q85 (+ `params.widths` → `<slug>.<w>w.webp`). Poster: WebP frame at `params.at` s.
Tested for real on a copy (`s1c1-key-visual` → 480p 460 KB + poster in ~4 s).

## Storage drivers (`AssetStorage` interface)

`name, describe, list(prefix), stat, download, upload, delete, copy, publicUrl, purge`. Keys = CDN-root paths.

- **LocalStorage** – the local CDN folder (`ASSET_LOCAL_ROOT`, default `../media` = the repo-root `media/`, gitignored), atomic writes (temp + rename), purge
  no-op, public URL = `CDN_URL`. Dev storage and the test double.
- **BunnyStorage** – bunny.net Edge Storage, built against the official reference: **bunny.net/docs/api-reference/storage
  = "bunny.net Edge Storage API", OpenAPI 1.0.0 (`/docs/api-reference/storage/openapi.json`), fetched 2026-10-02**, plus
  bunny.net/docs/storage/http (region endpoints) and api-reference/core (purge, pull zone, statistics). `AccessKey` = the
  zone's **read-only** password for every GET, the read/write password only for PUT / DELETE; `PUT` with `Checksum`
  (SHA-256, upper-case hex) → 201; list = `GET …/{dir}/` (recursive walk; `StorageObject` fields; `Checksum` /
  `ContentType` used when present); a 404 on the zone root = wrong zone / endpoint, on a sub-folder = empty; never deletes
  directories. Core API with the account key: `POST /purge?url=`, `POST /pullzone/{id}/purgeCache`, `GET /pullzone/{id}`,
  `GET /statistics`; Logging API `logging.bunnycdn.com/{MM-DD-YY}/{id}.log` for per-file traffic (later). Errors never
  contain a secret (401/404 come with a hint). `CurlHttpClient` verifies TLS (Windows dev machines without a CA bundle in
  php.ini use the OS certificate store); tests run against `FakeBunny` (the documented wire format).
- **FTP** – not a driver: the old FTP CDN folder `/cdn.scutoons.com/` stays as a backup until CUTOVER 10.7 (the name moves to bunny in 10.3; the repo's FTP tooling `cdn/` was removed 2026-10-04).
- **AssetStorages** – the configured set; `ASSET_STORAGE` = auto | local | bunny (auto: the local folder when it exists,
  else bunny). Migration between storages = `assets:sync` (below) + switching the setting.

## Configuration (server docs/CONFIGURATION.md)

Read from env / `config.php` (production, written by the deploy from the GitHub environment) and – **locally only these
seven keys** – from the repository-root `.env` (the owner's deploy checklist; DB / FTP / AUTH / SMTP lines are ignored,
unit-tested): `BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT` (region hostname, with or without `https://`),
`BUNNY_STORAGE_PASSWORD` (read/write), `BUNNY_STORAGE_READONLY_PASSWORD`, `BUNNY_PULLZONE_ID` (numeric), `BUNNY_API_KEY`
(account key – purge, statistics; optional), `CDN_PUBLIC_URL` (missing `https://` added). Plus `ASSET_STORAGE`,
`ASSET_LOCAL_ROOT`, `BUNNY_WRITE_PREFIX`, `ASSET_UPLOAD_MAX_BYTES`, `FFMPEG_PATH`, `FFPROBE_PATH`. Never printed:
`assets:check-storage` and the admin's Storage tab show set / missing / invalid per key, values only for non-secrets.

## Connecting bunny.net (owner)

1. Repository-root `.env` (local) / GitHub environment `production` (deploy): the seven keys above
   (`.env.example` "bunny.net (asset manager)"; `deploy/targets.json` writes them into the API's `config.php`).
2. `cd server && php bin/console assets:check-storage` – read-only: key status, zone listing (objects / bytes per folder),
   comparison with `media/` (only on bunny / only local / size), a HEAD through `CDN_PUBLIC_URL`, the pull zone (origin,
   hostnames) and its 30-day statistics. `--write-test` puts / reads back / deletes one file below `_test/`.
3. `php bin/console assets:sync --source-root=<media> --hash` – the upload plan (dry run). `--apply [--overwrite]
   [--purge-zone] --log=../logs/cutover/<name>.json` performs it (verified uploads, retries, never deletes).
4. `npm run dev:db` (local; production: `2026-10-02-assets` is applied), then `assets:scan`.

**State 2026-10-02 (read-only checks with the owner's keys):** zone `scutoons` (storage zone #1599226) at
`storage.bunnycdn.com`, only `catpope/`: 330 files / 324,078,057 bytes = an older snapshot. vs the main checkout's
`cdn/root` (824 files): 194 identical, **628 to upload (663.6 MB)** – catpope 213 (all 9 videos = 526 MB, the WebP slices,
items, merch, .128/.320 audio), characters 391, scutoons/items 17, models 5, toptoy + yellowmouse avatars – **2 differ**
(`audio/CatPope-Awakening-Music.zip`, `images/s1c2-thumbnail.jpg`), **134 only on bunny** (old `images/1x|3x/**`, the
6 album mp3s, unsuffixed audio, `videos/output.mp4`). Pull zone 6731366 "scutoons" = OriginType 2 (the storage zone),
hostname `scutoons.b-cdn.net` only, logging on, cache override 30 days; last 30 days ≈ no traffic (readers use
cdn.scutoons.com / all-inkl). HEAD `scutoons.b-cdn.net/catpope/avatar.png` → 200, same size as the zone.
Reference check of the 134 bunny-only files (a text search over EVERY stored version, the working tree and branch
`main`): 124 appear somewhere, 10 nowhere (the 6 old album tracks, `images/1x/de/s1c2-slice-{15,21,22}.jpg`,
`videos/output.mp4`). **Correction (owner, 2026-10-02):** that count is history, not current use – the usage index
(published + newest draft) shows `images/1x/de/s1c2-slice-16…20.jpg` only in the old, never-published draft entity
`scene:catpope:2:scene-19:{de,en}`; the current chapter 2 (scenes 14–18 + 20) uses none of them; the other hits are
`main`'s static viewer JSON (pre-content-store) and old versions. The 3x key visuals of chapters 3–5
(`images/3x/s1c{3,4,5}-key-visual.jpg`) are referenced by published content but exist nowhere – the owner regenerates
them in Figma.
**The initial upload ran 2026-10-02** (CUTOVER 1.5, PLATFORM.md "CDN"); history: it first needed the owner to lift the
local write fence (`BUNNY_WRITE_PREFIX`).

**Owner decision (2026-10-02): the domain switch.** Until the new asset system is deployed, bunny is reached at the
interim `https://scutoons.b-cdn.net` (`CDN_PUBLIC_URL`). When it is deployed the owner rewires `cdn.scutoons.com` to bunny
(custom hostname on pull zone 6731366 + DNS CNAME → `scutoons.b-cdn.net` + SSL); then `CDN_PUBLIC_URL` =
`https://cdn.scutoons.com` and every stored / printed URL stays valid (PLAN.md RULE "printed / public URLs never change").
Content keeps CDN-relative paths, so nothing in content changes at the switch. Production readers stay on
cdn.scutoons.com (all-inkl) until then; `VITE_CDN_URL` / `CDN_URL` unchanged.
**2026-10-05 (branch `10.3-cdn-domain`; CUTOVER 10.3 has the owner runbook + rollback):** `deploy/targets.json` =
`https://cdn.scutoons.com` everywhere. `scutoons.b-cdn.net` stays a hostname of the zone and stays valid:
`AssetPaths::PUBLIC_BASES` (both names) resolves absolute references to CDN-root paths (usage index, relayout);
migration `2026-10-05-cdn-domain` rewrites stored absolute b-cdn URLs (content versions, catalog + its history, toons,
settings, usage index) to cdn.scutoons.com. The asset registry stores CDN-root paths only – nothing to rewrite there.
No local → bunny sync at the switch: bunny is ahead of the dev copy (production uploads go straight to the zone).

## Relayout DONE (2026-10-02, owner: "I want to risk it; I keep the old CDN until the switch")

_(Since CUTOVER 10.30 `assets:relayout` rewrites code paths only – content and catalog live in the DB; the seed JSON /
seed SQL below no longer exist.)_

Run in the `assets` worktree after merging `platform` (c6a06bc): `assets:relayout --apply
--source-root=C:/Development/scutoons/cdn/root` – **214 files copied** (712 MB, verified, 0 failed) next to the old
ones in the dev CDN folder; **552 references in 99 tracked files** rewritten (seed JSON of every toon + `scutoons/_`,
`server/config/items.json` (album tracks, zip, cover), code: the camera model `MOBILENET_MODEL` →
`scutoons/models/…`, the inventory / map frame → `catpope/vectors/vector-demon-saw-1.svg`, tests); seed SQL patched
(content-seed 24 + characters-seed 2 statements, timestamps kept); the worktree DB updated (89 entities). Then by hand:
creator defaults (new chapters / seasons / slices in `s<n>/c<n>/`, `s<n>/`), the picker's upload suggestion from the file
name (`s1c2-…` → `s1/c2/`), map place images `<toon>/map/places/`, `slice-sizes.ts`, `make_slice_webp.py`, make-sandbox,
placeholders, the style guide, no fallback to the never-existing `images/default-thumbnail.jpg`, the video checks find
videos by extension. Verified against a CDN with ONLY the new layout (hard-link copy without `<toon>/images|videos|audio/`
and `models/`; its request log has no 404).
**Still to do (coordinator / owner):** `assets:relayout --delete-old` after the merge into platform is green (removes the
214 old paths + the deletions below from the dev folder), then the bunny sync of the new layout (CUTOVER Phase 1).
The new platform uses `https://scutoons.b-cdn.net` (`deploy/targets.json`, owner decision for the beta); the legacy system
keeps cdn.scutoons.com (all-inkl) until the domain moves to bunny. **Final delete list (104):** the 94 lettered masters
`catpope/images/webtoon/de/S1C1 - Slice N.jpg`, `catpope/images/collectables/cucumber.png`,
`catpope/images/s1s{1..6}-thumbnail.jpg`, `catpope/images/vectors/{vector-blood,vector-bloog-glot,vector-frame-right}.svg`.
**Kept (correction):** `vector-demon-saw-1.svg` is used by the viewer's inventory and map overlay (app code – the usage
index only covers content + catalog; the layout plan now counts exact paths in app code as uses) → `catpope/vectors/`.

## Folder convention (owner 2026-10-02)

The folders ARE the toon hierarchy; videos and audio belong to their season / chapter like images:

| Folder | What lives there |
|---|---|
| `scutoons/…` | the universe: `scutoons/items/` (cross-toon item icons), `scutoons/models/<model>/` (SHARED base models, e.g. `mobilenet_v2_100_224/` – items and models belong together), platform media |
| `characters/<id>/…` | the character registry (unchanged) |
| `<toon>/` | toon level: `avatar.*`, and topic folders `items/` (item icons + a camera item's own recognition head `<item-id>.head.json`), `merch/`, `map/`, `characters/` (the toon's own drawings) |
| `<toon>/s<n>/` | season: covers, trailer (+ poster), the album / music zip, `supporters/` |
| `<toon>/s<n>/c<n>/` | chapter: slices, key visual, thumbnail, chapter videos, music, samples (`c0` = the prologue) |

- **No kind folders** (`images/`, `images/webtoon/s1c2/`, `videos/`, `audio/`) where the season / chapter folder says it.
- **File names keep their ordering prefix**: `s1c2-slice-14.webp`, `s1c1-instrumental-brass-morton.128.mp3`.
- **Models follow the item hierarchy** (owner 2026-10-02: "items and models belong together"): the shared base model
  stays universe-wide in `scutoons/models/<model>/` next to `scutoons/items/`; a camera-SPECIFIC recognition head lives
  next to its item in the item's toon folder as `<toon>/items/<item-id>.head.json` (e.g. the onion camera's `onion-v1`,
  planned on branch `items/onion-camera` at `models/onion-v1/head.json` → `toptoy/items/onion_camera.head.json`). The
  scanner / picker treat `<name>.head.json` as a model file of that item (to build with the grouping: same slug as the
  item's icon `<item-id>.png`, label `head`, kind document).
- **Variants are suffixes, never folders**: `.480p` `.720p` `.1080p`, `.1280w`, `.poster`, density `.2x` `.3x` (1x = no
  suffix; the owner OKed density). The scanner already groups exactly these (see "Grouping").
- A locale segment (`/de/`) is a smell (DATA.md "Language-independent media"); the plan keeps it where it exists, flagged.
- **Uploads suggest the place** (next UI step): the picker knows its field – season fields → `<toon>/s<n>/`, chapter
  fields → `<toon>/s<n>/c<n>/` with the `s<n>c<n>-` prefix, the kind from the field (trailer, slice, key visual …) –
  and proposes `<toon>/s1/c2/s1c2-slice-<next>.webp`; the path stays editable. Until the migration, new files may
  already go to the new places (both conventions coexist; the registry does not care).

### Migration plan (dry run: `php bin/console assets:layout --json=../logs/cutover/layout-plan.json`)

`AssetLayout` maps every file of the storage: the name's `s<n>c<n>-` / `s<n>-` prefix places it; topic folders stay
topics; otherwise the uses decide (one chapter → that chapter's folder, one season → the season; chapters → season via the
seasons' `sequence`); `models/` → `scutoons/models/` (shared base models; item-specific heads go to `<toon>/items/<item-id>.head.json`); unused files without a place and the lettered masters are
listed for deletion (owner decisions below). It reports conflicts (two files → one
path), the uses each move rewrites, and paths built in app code.

**Result on the local cdn/root (824 files, 2026-10-02):** 318 move, 506 stay; **567 uses in 103 documents** would be
rewritten; 0 conflicts. By rule: chapter (from the name) 161, chapter masters 94, season (from the name) 12, from the uses 7
(the album zip → `catpope/s1/`, the 3 supporters → `catpope/s1/supporters/`, the prologue videos → `catpope/s1/c0/`),
toon topics: characters 27 (`catpope/images/characters/…` → `catpope/characters/…`), map 1; unchanged: items 21,
merch 73, `characters/` 391, `scutoons/` 17, toon level 4; universe: models 5 – **needs a code change**
(`viewer/src/lib/components/camera/object-detector.ts` `MOBILENET_MODEL`). **11 unplaced, all unused** (owner: delete?):
`images/collectables/cucumber.png` (collectables were removed), `images/s1s{1..6}-thumbnail.jpg`, 4 `images/vectors/*.svg`.
Examples: `images/webtoon/s1c2/s1c2-slice-14.webp → s1/c2/s1c2-slice-14.webp`, `videos/s1-trailer-loop.720p.mp4 →
s1/s1-trailer-loop.720p.mp4`, `images/3x/s1c3-key-visual.jpg → s1/c3/s1c3-key-visual.3x.jpg`.

**How the move runs (later, timed with the bunny switch):** per file `rename`/`relink` (copy to the new path, every use →
the new path: content as new drafts, the catalog files rewritten, then publish), on the bunny zone; the old paths stay on
**cdn.scutoons.com (all-inkl)** until the domain moves to bunny, so readers on old builds keep working; after the switch
and one release, the old paths are deleted through the orphan list. Order: (1) bunny initialisation with today's paths,
(2) `assets:layout` re-run + owner review of the mapping, (3) a batch relink command (to build: applies the JSON mapping,
one draft per document, `--publish`), (4) the viewer code change for the model path, (5) domain switch, (6) prune.

### Owner decisions on the mapping (2026-10-02 – applied by the batch relink, not yet)

- The folder convention is **approved** as proposed.
- The **94 lettered masters** (`images/webtoon/de/S1C1 - Slice N.jpg`): **delete**, no `masters/` folder.
- The **11 unused files without a place** (`images/collectables/cucumber.png`, `images/s1s{1..6}-thumbnail.jpg`,
  `images/vectors/{vector-blood,vector-bloog-glot,vector-demon-saw-1,vector-frame-right}.svg`): **delete**.
- `models/` → `scutoons/models/` together with the one-line viewer change (`MOBILENET_MODEL` in object-detector.ts) in
  the same step as the move.
- `scene:catpope:2:scene-19` (the stale draft): **leave it** – the owner cleans up when preparing chapter 2's final
  release on production.
- Deletions happen **with the move** (local `media/` and bunny), after the owner reviews the final mapping run;
  `assets:layout` lists them (`action: delete`, `summary.delete_list`). A file that is used is never planned for deletion.

Mapping after these decisions (local cdn/root, 824 files): **213 move, 506 stay, 105 delete**; 567 uses in 103
documents to rewrite; 0 conflicts.

### Backup (owner 2026-10-02)

No second bunny zone (too expensive). The backup target, later, is the **old all-inkl FTP CDN** (cheap, no global reach
needed): an `FtpStorage` driver behind the same `AssetStorage` interface (list / get / put) and a
scheduled `assets:sync --from=bunny --to=ftp` (copies only, never deletes). Not needed now.

## API (server docs/API.md "Assets")

Content editors: `GET /api/assets` (q, folder, kind, tag, state used|unused, page, sort slug|updated|size|usages|kind),
`GET /api/assets/lookup?path=`, `GET|PUT /api/assets/{id}`, `PUT /api/assets/upload?path=&title=&tags=` (raw body),
`GET /api/assets/missing`, `GET /api/assets/orphans`. Admin: `GET /api/admin/assets/storage[?stats=1]`, `POST …/scan`,
`POST …/relink`, `PUT …/{id}/file?path=&note=` (raw body), `POST …/{id}/rename`, `GET …/{id}/delete-check`,
`DELETE …/{id}[?force=1]`, `POST …/{id}/jobs`, `GET …/jobs`. Client: `@scutoons/api` `createAssetsApi`, helpers
`qualityVariants`, `toonRelativePath`, `rootPath`.

## UI

- **Creator – the asset picker** (`@scutoons/media` `AssetPickerSheet`, `shared/media/src/asset-picker-sheet.svelte` since 2026-10-04, like the character
  picker): search, folder (the toon first), kind, unused-only; a grid of thumbnails with ONE line of name (no overlap –
  the owner's screenshot of the old "130 known images" grid), size / dimensions / "unused"; the chosen asset's files
  (renditions with quality) – click one to use exactly it, "Use" takes the source; upload into the toon's folder (path
  editable, 409 when taken). Every image field (a `MediaSlot` from @scutoons/media; the old path input `image-picker.svelte` is gone since COMPONENTS.md step 14c) opens it via
  "Choose …". **Video fields** (slice video "Hintergrund", the season trailer) have "Choose video …": the asset's
  480p / 720p / 1080p become `variants` low / medium / high, 720p the default (`src` / `trailer.loop`). The season trailer is
  one slot since 2026-10-03 (`media-variants-slot.svelte`, DESIGN.md "Creator: season page"): the preview picks the whole
  asset, chips per level replace / clear one file.
- **Admin – Assets** (`/assets`, nav Data → Assets): Library (grid, filters in the bar, Rescan), Missing files, Unused
  (the cleanup: select assets, "Delete N files (size)", confirm),
  Storage (drivers, bunny settings status, totals per kind, 30-day bandwidth chart). The asset sheet: preview, files per
  version, uses (with links into the creator), title / tags, replace, rename, relink, optimize (jobs), delete behind the
  check.
- **The media standard** (BACKLOG 10.64, `@scutoons/media` media-standard.ts, owner 2026-10-11/12): images WebP (PNG /
  JPEG converted, transparency kept; WebP / SVG / GIF as they are; a delivered WebP keeps its bytes unless an importer's maximum
  scales it down), NO size limit (owner 2026-10-12: "ein Large Limit gibt es im Moment nicht"); densities as suffixes `name.webp` / `.2x.webp` / `.3x.webp`, each made from the
  file delivered for it, else the smallest bigger one (`densityPlan`, never scaled up), widths per density (slices 390 ×
  n, place images 256 / 512 / 1024); video MP4 / WebM (no conversion – no ffmpeg on the host). EVERY importer goes
  through it: slice import (`standardDensities`), character import + catalog item / recipe images (`standardImage`), the
  map's import (`<id>` painted, `<id>-plaster` the plaster model, `.2x` / `.3x` / `@2x` in the name; 1x / 2x / 3x
  written to `image.src` / `variants` / `plaster` / `plasterVariants`), the asset picker's upload (converts by default,
  untickable; always with `webp`).
- **Admin – Assets → Standard** (the optimisation tool, 2026-10-12): `media-audit.ts` audits the files content / the
  catalog REFERENCE (the list's per-file `usages`, AssetRegistry) – an unused PNG master beside its used WebP is
  "Unused"'s business, not this tab's. Per file: `convert` (encode → a NEW file `standardName(path)`, `-2` … when taken
  – a CDN path is never overwritten, PLATFORM.md), `twin` (the WebP is already a rendition – relink only), none (video).
  "Convert N files …" (`fixFindings`, I/O injected – node tests): read from the CDN (bunny: CORS `*`), `standardImage`
  at full size, upload, then ONE `POST /admin/assets/relink { map }` (`AssetService::relinkAll`): one new draft per
  touched document, catalog rows directly; the report links the drafts into the creator (`$lib/creator-links.ts`) and
  lists the by-hand ones. Tested on the dev seed: 2 thumbnails 103 KB → 19 KB, 8 uses, 5 drafts (toon:catpope one draft
  for both). Dev seed 2026-10-12: 85 referenced files (28 MB), 71 convert, 14 relink.

## Slice 1 and the trailer (owner: "can use the same asset and its resolution alternatives")

Possible now: any video field can pick any video asset; the picker fills the quality variants from its renditions
(`roar-little-kitty` = the prologue's 480p / 720p / 1080p; `s1-trailer-loop` = its own 480p / 720p / 1080p + the old
1280w / 1920w). **Not changed:** the season trailer keeps `s1-trailer-loop` (a 24 s excerpt, 2.7 / 5.2 / 12 MB). Using the
prologue film (7:55; 55–267 MB per level) as the desktop background loop would download that much on every toon-page
visit – **owner decision**. Alternative that keeps one source: an optimization job "excerpt" (cut a loop from the
prologue master and register it as renditions of a loop asset) – not built.

## 3D models (owner 2026-10-13)

Kind `model`: **binary glTF (`.glb`) only** – one self-contained file (upload magic `glTF`, MIME
`model/gltf-binary`, counted as used content like images). A `.gltf` is refused: it points to separate `.bin` /
texture files that a move or rename would break; GLB ↔ glTF converts losslessly (e.g. gltf-transform) when a model must
be edited. Used by the characters' personas / looks (`model`, DATA.md); picker and `FileSlot kind="model"`. **Preview**
(owner 2026-10-14): `ModelPreview` (@scutoons/media – a live view, turns slowly, drag to turn; dashed + "Placeholder"
for a stand-in) in the character page and the admin asset sheet, `ModelThumb` stills on the picker / admin cards (one
shared renderer); three.js loads only when a model is shown (`model-view.js`). **Mannequins** (the fallback of a
persona without a model, by its body preset `legs`: two / more – `MANNEQUINS`, code references in AssetUsages):
`scutoons/3d/mannequin.glb` (Xbot, Mixamo, via the three.js examples), `scutoons/3d/mannequin-more-legs.glb` (open).

## Albums and downloads (owner: the season "Downloads" fields are redundant)

The season's music = the **album of its related items** (`seasonMusic` in `@scutoons/content` albums.ts): the first
related item that carries an album gives the zip (`album.download`) and the record image (`album.cover`). The toon page's
record row, `/<toon>/download` and the chapter next-bar use it; a season without such an item still reads its old
`download.music` / `download.image` (compatibility – no content migration needed: catpope's album already has the same
zip + cover, `season1.json` lists `awakening_vinyl`). The creator's season page no longer has the Downloads section (a
hint under Related items says where the music comes from). `download.complete` (unused, null) is no longer edited.

## Tests

Server: `AssetStorageTest` (keys, local round trip, bunny against FakeBunny: URLs, read-only vs read/write password,
checksum, recursive list, purge, statistics, pull zone, write fence, errors without secrets, config normalisation),
`InfrastructureTest::testRootDotEnvOnlyFeedsTheStorageKeys`, `AssetsTest` (grouping, idempotent import incl. out-of-band
changes, usage index incl. drafts / map toons / catalog, delete check, upload refusals, replace in place + archive,
relink, rename incl. catalog rewrite, jobs), `AssetsApiTest` (roles, list / lookup / upload, the save hook re-indexes,
delete refused, storage status without secrets, scan). Content: `albums.test.ts` season music. Browser (own ports 9310 /
9311 / 10310 / 11310 / 8310, test user): creator picker on the season page (cover, trailer video → variants, upload +
delete), admin Assets (library, detail, delete check, storage + bandwidth), viewer toon page (record image from the album).

## Fonts (CUTOVER 10.1c, 2026-10-05)

`scutoons/fonts/` on the CDN = a copy of `shared/tokens/fonts/` (the source; same file names): IBM Plex Mono 400/700 by
subset (cyrillic-ext, cyrillic, vietnamese, latin-ext, latin) + italic 400/700 (latin, latin-ext), Bangers 400 (latin,
latin-ext) – 16 woff2 – and both SIL OFL 1.1 texts (`IBMPlexMono-OFL.txt`, `Bangers-OFL.txt`); 18 files, 212 KB.
Staged in the local `media/scutoons/fonts/` (byte-identical); the owner uploads them by hand in the bunny dashboard
(owner 2026-10-05: no local bunny sync), Rescan on the admin Assets page registers them afterwards.
Registered as app code in `AssetUsages::CODE_REFERENCES` (+ AssetLayout): never "unused", a delete is blocked.

**Served from the CDN, the local copy as fallback** (decision 2026-10-05, evidence below):
- Apps (viewer, creator, admin, shop, embassy): `cdnFonts()` from `@scutoons/tokens/vite` makes every `src`
  `url(<VITE_CDN_URL>/scutoons/fonts/<file>), url(<bundled, hashed>)`. A failed CDN source (404, CORS, outage) falls
  through to the bundled file (measured: 404 → bundled 200, text stays Plex). Without `VITE_CDN_URL` nothing changes.
- qr landing page: `templates/landing/fonts-cdn.css` (generated by `qr/bin/sync-tokens.mjs`, `{{CDN}}` = `CDN_URL`)
  inlined after the stylesheet – the later `@font-face` wins (measured: only the CDN files are requested). verify.html
  keeps the local fonts (static HTML, no config).
- Why: the HTTP cache is keyed by the top-level site; every app is `*.scutoons.com`, so one CDN copy is shared by the
  reader's path qr landing → viewer → shop (and creator / admin), where the bundled copies (one per origin, hashed names)
  are downloaded again per app. The CDN connection is open anyway (media). Typical page: Plex latin 400 + 700 = 29.6 KB.
- Costs (accepted): one more origin for fonts (bunny sends `Access-Control-Allow-Origin: *` for woff2 – checked on
  `scutoons.b-cdn.net`, also on a 404; the pull zone's 30-day cache override applies, file names are not hashed →
  **a changed font gets a new file name** or a purge); dev without the media copy logs one 404 per face before the fallback (and `check:fonts` reports it).
- `font-display: swap` everywhere (unchanged). No preload (none before either): the apps are SPAs whose CSS comes with the
  JS; a preload in `app.html` would need the CDN base in the static template – possible later, works with both options.
- Check: `npm run check:fonts` counts CDN-served Plex like local (it matches the file stem); a missing upload shows as
  `font … → 404` there.

## Open / next

- Owner: decide the bunny-only files (the old `1x/de` slices are used only by the
  never-published draft `scene:catpope:2:scene-19`; deleting that draft entity frees them).
- Owner decision: trailer from the prologue film (see above).
- Per-file bandwidth from the Logging API (aggregate the daily logs into a table; per asset in the detail).
- Image renditions for slices (`IMAGE_QUALITY_DENSITIES` 1x/2x/3x as `{ src, variants }`) once the reader reads them – the
  picker can fill them like video variants.
- When production writes to bunny: `ASSET_STORAGE=bunny` in the deploy's config (the shared host has no local media folder);
  the picker's uploads then go straight to the zone.

## Audio quality levels (2026-10-04, owner)

Music tracks and sample-library sounds carry `variants { low, medium, high }` = 96k / 128k / 320k MP3 next to `path` (the
128k default; `AUDIO_QUALITY_KBPS` / `AUDIO_QUALITY_LABELS` in @scutoons/content). The reader loads the file for its media
quality level when the chapter's audio loads (`viewer/src/lib/contexts/audio.svelte.ts`, `pickQualityVariant`); `path` while
the level isn't known. The creator shows them as a `MediaVariantsSlot` with `kind="audio"` (chips 96k / 128k / 320k).
**Lower levels are made in the browser** (`shared/media/src/audio-levels.ts`, `wasm-media-encoders` = LAME as WebAssembly,
MIT wrapper, loaded only when encoding): an upload in the picker is decoded, its rate estimated (size / duration), the
original stored as `<name>.<rate>.<ext>` and every level clearly below it encoded (CBR) and uploaded – a 320k upload gives
96k + 128k, a 128k upload 96k; a lossless source (WAV / FLAC / AIFF) gives all three and isn't uploaded itself. An existing
audio asset gets a "Create 96k …" button in the picker's detail (its best file fetched from the CDN – CORS – and encoded).
No server job: the shared host has no ffmpeg, and this works in production. Existing content keeps `path` only until a
track / sample is picked again (then all levels fill in).
