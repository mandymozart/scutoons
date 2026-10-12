# PERFORMANCE — why the app feels slow (2026-09-30)

Owner question: "Why does the entire app feel so slow? Is it because of the file-based system?"

**Short answer: no, the file-based system isn't the cause.** A content file from SQLite costs 0.2 ms (local dev was SQLite then; MariaDB only since CUTOVER 10.30). Two different
things made the app feel slow:

1. **The local dev stack** (only on this machine). A single-process PHP server without OPcache, a single-threaded
   Python CDN that hung, and Vite serving 450–520 unbundled modules per page. Fixed (below): dev pages now render
   in 0.3–0.6 s instead of 2–8 s.
2. **Media size in production.** Opening chapter 1 downloads **43 MB**: 31 MB of 320 kbps MP3s (both music tracks
   plus samples, fully decoded up front) and 11 MB of 3x JPG slices, all loaded eagerly. On a 4G connection
   (20 Mbit/s, 60 ms RTT) that takes **23–25 s**. The page code is fast: 0.3–0.6 s to first render in a production
   build. The fixes are proposals (below).

## Method

Headless Chrome through CDP (script: `scripts/perf/measure.mjs`, no dependencies). Each run has a cold browser cache
and a new profile. The numbers below are the median of the warm-server runs (runs 2–3 of 3). Viewport 390×844 @3x
for the viewer, 1280×800 for creator/admin/shop. "Fully loaded" means network idle for 1.5 s. Pages that need access
were measured with a device-access entry in localStorage (`scutoons_access`: a client-side gate only, no sign-in).
Production-like numbers come from `vite build` + `vite preview` against the API with 4 workers and OPcache, with and
without 4G throttling (20 Mbit/s down, 60 ms latency). The creator and admin were measured up to their sign-in
screen. Pages behind a sign-in (creator editor, Characters, admin dashboard/codes) were **not** measured, because
that would need signing in as the owner. They use the same API and dev server, so the API and Vite fixes apply to
them unchanged.

## Measurements (dev stack, before → after)

| Page | Fully loaded | First paint (FCP) | Requests | Notes |
|---|---|---|---|---|
| viewer home | 2166 → **509 ms** | 1920 → **488 ms** | 356 → 156 | |
| `/catpope` | 2681 → **565 ms** | 2348 → **572 ms** | 398 → 198 | |
| chapter 1, no access (blocker) | 3176 → **639 ms** | 2956 → **640 ms** | 482 → 281 | |
| chapter 1, with access (94 slices) | 5897 → **2137 ms** | 3096 → **756 ms** | 582 → 381 | 47.7 MB both (media, see proposals) |
| chapter 6 VN, with access | 5217 → **1706 ms** | 3328 → **564 ms** | 499 → 298 | 38.6 MB (music) |
| `/catpope?inventory=open` | 3405 → **544 ms** | 2436 → **556 ms** | 398 → 198 | |
| creator (sign-in screen) | 7981 → **262 ms** | 7804 → **280 ms** | 523 → 154 | before: clean warm A/B 507 ms; 7.9 s with dev-server churn |
| admin (sign-in screen) | 13386 (5.7–59 s) → **264 ms** | 7520 → **276 ms** | 900 → 155 | |
| shop `/de/` | 558 → **262 ms** | 524 → **256 ms** | 177 → 168 | |

| Component | Before | After |
|---|---|---|
| API per request (server time, `health` / content) | 60–70 ms | **15 ms** (OPcache) |
| 6 parallel chapter files (browser) | finished at +111, +203, +305, +425, +502, +581 ms (queued) | all done within **~55 ms** (4 workers) |
| Chapter page API phase (first call → last) | 1.53 s | **~0.35 s** |
| CDN with one idle connection open (browser preconnect) | **hangs** (timeout > 8 s; seen live: every merch image `ERR_CONNECTION_REFUSED`) | 110 ms |
| Idle main-thread script per second (prod build, home / `/catpope`) | 210 / 188 ms/s | **35 / 38 ms/s** |
| `chapterN.json` fetched per chapter open | 2× | 1× |
| CORS preflights for `/items/` + `/editions/` | 2 | 0 |

Production-like build (localhost, no throttling), after the fixes: home 325 ms, `/catpope` 497 ms, chapter 1 without
access 276 ms, chapter 1 with access FCP 584 ms, fully loaded 2.5 s (43 MB). **With 4G throttling:** `/catpope` 1.4–2.4 s,
chapter 1 **22.6–24.7 s / 43 MB**. The same run with MP3s blocked takes **6.1–7.5 s / 12 MB**, and main-thread
CPU drops from 5.3 s to 2.2 s (audio decoding).

## Causes, ranked

### Production (what readers feel)

1. **Music: 31 MB of MP3 per chapter open (72 % of all bytes).** `AdaptiveMusicPlayer` loads both adaptive tracks
   (`s1c1-instrumental-brass-morton.mp3` 12 MB, `s1c2-instrumental.mp3` 16.5 MB, both 320 kbps) through Howler Web
   Audio (`html5: false`, which means a full download plus decode into PCM) as soon as a chapter opens. Samples add
   2.8 MB. On 4G: 24 s → 7 s without them.
2. **Slices: 94 × 1170×1875 JPG, all `loading="eager"`**: 11.3 MB for chapter 1. The browser downloads every slice
   before the reader scrolls.
3. **Animated favicon at 60 fps**: re-parsed 16 SVG paths and re-encoded a PNG data URL on every animation frame on
   every page, using 190 ms of main-thread script per second (about 19 % of a desktop core, more on phones).
   **Fixed.**
4. **Loader waterfall**: the toon layout waited for the slowest of its first 6 files before requesting the chapter
   files. The chapter page fetched `chapterN.json` a second time and loaded the VN scene files only after the whole
   layout. That cost about 3 extra round trips (~180 ms on 4G). **Fixed.**
5. **CORS preflights** for the public item catalog and editions (the `X-Device-Id` header forced an OPTIONS request):
   2 extra round trips per first visit. **Fixed.**
6. **Portraits**: 600×600 PNGs of about 400 KB (for example `catpope-biker_portrait-happy.png` 412 KB → 52 KB as
   WebP q85).

### Dev only (what the owner felt locally)

1. **Vite dev serving hundreds of modules**: 133–240 app modules plus **one module per paraglide message** (viewer
   201, creator 372, admin 377). A long-running dev server also re-transforms after every edit by the parallel
   agents (`?t=` invalidations). This cost 1.5–1.9 s before the first API call on the viewer, and 5–8 s on the
   creator/admin while files changed. **Fixed**: paraglide `locale-modules` in dev (3 files instead of 200–380).
   The servers were restarted.
2. **`php -S`**: one request at a time, no OPcache (60–70 ms bootstrap each), and `PHP_CLI_SERVER_WORKERS` fails on
   Windows ("forking is not supported on this platform"). A chapter page's ~15 API calls queued up. **Fixed**:
   `server/bin/dev-serve.mjs`.
3. **`cdn/server.py`**: a single-threaded `HTTPServer`. One idle socket (a Chrome preconnect) blocked it
   indefinitely. That caused the earlier "CDN hung" reports, and pages waited for images that never came. **Fixed**:
   threaded.
4. `localhost` resolution: the servers listened on `::1` only (PHP) or `127.0.0.1` only (CDN). curl and other
   IPv4-first clients paid ~200 ms per connection. Browsers were mostly unaffected. **Fixed** for API and CDN (both
   listen on both addresses). qr :9001 still listens on `::1` only.

### Checked and refuted (not a cause)

- **The file-based content store and SQLite**: indexed query 0.18 ms, connect 0.45 ms. With OPcache, a content file
  costs the same as `/health/` (~15 ms, all PHP bootstrap). Production MySQL behaves the same.
- **Missing caching headers**: content GETs already send `ETag` + `Cache-Control: no-cache` (304 on revalidate).
  `/products/`, `/items/` and `/editions/` send `public, max-age=300`. Nothing to add there.
- **`/me` / `/access` too often**: 0 calls without a session. `/access` rechecks every 6 h. `/me` runs once per load
  with a session (not measured signed in).
- **Fonts**: all 16 `@font-face` use `font-display: swap`, 15–30 KB each, not blocking.
- **The 4 s store timeout fallback**: never triggered. The store answered every request.

## What was fixed (each measured before/after, above)

| Change | Files |
|---|---|
| Threaded CDN (`ThreadingHTTPServer`, daemon threads, 30 s idle socket timeout), listens on `127.0.0.1` + `::1` only, logs errors only, serves `root/` relative to the script | `cdn/server.py` (since 2026-10-04 `scripts/dev/media-serve.mjs`, serves `media/`) |
| API dev server with concurrency on Windows: 4 `php -S` workers (`[::1]:9200-9203`, OPcache with `revalidate_freq=0`) behind a Node proxy on :9000 (`::1` + `127.0.0.1`, Host/headers/body unchanged, `REMOTE_ADDR` stays `::1`). `npm run dev:api` / `composer serve`. `composer serve:php` = the old single process | `server/bin/dev-serve.mjs`, `package.json`, `server/composer.json`, READMEs |
| Paraglide `locale-modules` for `vite dev` and `npm run check`. Builds keep `message-modules` (tree-shaking; total JS gzip 488 vs 486 KB, so either is fine) | `*/vite.config.ts`, `*/scripts/paraglide.js` |
| Content API: opt-in in-memory cache of published reads (in-flight sharing plus a 60 s TTL; drafts never cached; the creator keeps it off) | `shared/api/src/content.ts`, `viewer/src/lib/api.ts` |
| Toon layout: chapter files start when `season1.json` arrives. Chapter page: starts `chapterN.json` plus its scene files before `parent()` (shared with the layout through the cache) | `viewer/src/routes/(toons)/[toonId]/+layout.ts`, `…/chapter/[chapterNumber]/+page.ts` |
| Item catalog + editions through the header-free public client (no preflight) | `viewer/src/lib/api.ts` |
| Animated favicon: 12 fps (same scroll speed), `Path2D` parsed once | `viewer/src/lib/utils/favicon.ts` |

Gotcha: `npm run build` compiles paraglide as per-message modules into the same `src/lib/paraglide`. A dev server
that is still running then serves ~200–380 message files again until it restarts (it recompiles as locale modules
on start).

## Proposals (P1–P3 done 2026-09-30, see "P1–P3 + video slices — result" below; P4–P7 open)

| # | Proposal | Expected gain (chapter 1, 4G 20 Mbit/60 ms) |
|---|---|---|
| P1 ✅ | **Music**: re-encode the tracks at 128 kbps MP3 (`s1c2` 16.5 → 6.6 MB) or Opus 96 kbps (5.1 MB). Stream them with Howler `html5: true` / `<audio>` + `MediaElementSource` so the crossfade still works, **or** start loading the music only after the first slices are shown / after the first user gesture (browsers block autoplay until then anyway). Load samples lazily when their slice comes near. | −31 MB (−72 % of bytes): 24 s → ~7 s without any other change; 5 s → 2 s CPU |
| P2 ✅ (owner: 1x WebP only, no srcset) | **Slices**: `loading="lazy"` + `decoding="async"` **with `width`/`height` (or aspect-ratio) in the chapter config** (without them the lazy slices collapse to height 0, all intersect at once and `sliceAudio` triggers would fire together, so this needs the dimensions first). Serve WebP q80 through `<picture>`/`srcset` 2x/3x. Measured on all 94 chapter-1 slices: JPG 3x 11.3 MB → WebP 3x 7.0 MB (−39 %) → WebP 2x 4.1 MB (−64 %). | First screen needs ~2 slices (~0.2 MB) instead of 11 MB; whole chapter −4 to −7 MB |
| P3 ✅ | **Portraits / character art**: WebP q85 (412 KB → 52 KB, −87 %); lossless WebP −31 %. | −1 to −3 MB per VN scene |
| P4 | **One request per chapter**: an API endpoint (or a build step) that returns `chapterN.json` with its scene files inlined, plus a season "overview" with the list fields of every chapter (title, thumbnail, release), so the layout doesn't download six full chapter configs (~24 KB each) just to show the list. | −6 to −10 requests; −2 round trips on VN chapters (~120 ms on 4G); −100 KB JSON on `/catpope` |
| P5 | **Production hosting**: confirm OPcache on all-inkl (PHP-FPM). Enable PHP-DI container compilation (`enableCompilation` into `storage/cache`) + `composer dump-autoload -o --classmap-authoritative` in the deploy. HTTP/2 on all subdomains. | a few ms per API call; the API is already ~15 ms with OPcache |
| P6 | **CDN plan** (with the owner, see PLATFORM.md "CDN"): long `Cache-Control: max-age=31536000, immutable` for content-hashed media (P1–P3 produce new names anyway), Brotli/gzip for JSON/SVG (`map-layer-terrain.svg` 330 KB), bunny.net in front of all-inkl. | repeat visits load no media again; SVG −70 % |
| P7 | Inventory: `loadCatalog()` loads `/items/` then `/editions/` one after the other; they are independent (`Promise.all`). This was left alone because the avatar agent was editing `inventory.svelte.ts` at the time. | −1 round trip |

Suggested order: P1 (the single biggest win, a re-encode plus a loader flag), then P2 (needs slice dimensions in the
config / creator), then P3, then P4–P6 with the production cutover.

## P1–P3 + video slices — result (2026-09-30)

Owner decisions, in this order, each measured with `scripts/perf/measure.mjs` on a production build (`vite build` +
`vite preview` :4173 against the local API and CDN), chapter 1 (de), 390×844 @3x, **4G throttling**, cold cache, median
of 3 runs. New metrics in the tool: LCP / CLS (when the init script records them), bytes of cancelled media streams, and
"loaded without media streams" (an `<audio>`/`<video>` keeps its connection open while it buffers, so network idle comes
~10 s late without saying anything about the page).

| Chapter 1, 4G | Before | P1 music | + P2 slices (video blocked) | + video slice 1 (final) |
|---|---|---|---|---|
| Bytes on open | **42.0 MB** | 14.7 MB | 3.9 MB | **6.4 MB** (video 2.7 MB, samples 2.8 MB, music 0.25 MB buffered) |
| Page loaded (without media streams) | 22.3 s | 7.3 s | 3.9 s | **5.1 s** |
| Network idle incl. streams | 22.3 s | 17.3 s | 16.6 s | 16.8 s |
| First contentful paint | 1.23 s | 1.19 s | 1.01 s | 1.17 s |
| Largest contentful paint | 1.58 s | 1.50 s | 1.08 s | 1.24 s |
| Cumulative layout shift | **0.76** | 0.76 | 0 | **0** |
| Main-thread CPU | 5.0 s | 3.2 s | 2.9 s | 2.5 s |
| Requests | 183 | 184 | 94 | 94 |
| Whole chapter scrolled to the end | 42 MB | – | – | **8.4 MB** |

- **P1 music**: `audio/music/*.128.mp3` (128 kbps, 4.9 + 6.6 MB instead of 12.3 + 16.5 MB; the originals stay locally as
  `*.320.mp3`, the live CDN keeps the old names for the released viewer). `AdaptiveMusicPlayer` streams: one `<audio>`
  per track → `MediaElementAudioSourceNode` → gain per track on the music's own AudioContext (Howler replaces its context
  on every cleanup and once for the sample rate, which silenced media elements bound to it). Crossfades stay gain ramps
  (iOS ignores `audio.volume`, so Howler `html5: true` would have broken them). The chapter's audio starts loading after the
  media of the first 2 slices (max. 4 s). Checked with a CDP recorder that logs every gain ramp, sample start and media
  play while scrolling slice by slice: the same targets at the same slices before/after (15: 0.2/0.5, 19: 0/0.5, 26: 0.1/0.8,
  30: 0.2/0, 93: 0/0; bells 4, choir_once 44, choir_loud 89), plus one fix: **slice 1's `calmness 0.2` was lost before**
  (the slice's trigger fired while the tracks were still loading, so the music stayed silent until slice 15); now the
  volumes asked for while loading are applied (faded in) when the music starts. Gesture start (no autoplay) checked too.
- **P2 slices** (owner: 1x only, WebP only, no srcset / JPG fallback): `images/webtoon/<sXcY>/<sXcY>-slice-N.webp`,
  390 px wide (`cdn/tools/make_slice_webp.py`), chapter 1: 94 × 10.3 MB JPG (799×1280 source) → 2.0 MB WebP.
  `Slice.width`/`height` (required, validation error), `<img loading="lazy" decoding="async" width height>`, first 2
  slices eager. CLS 0.76 → 0 (the old eager JPGs had no size either). Text boxes checked de + en (same 5:8 ratio).
- **P3 character images**: WebP q85 next to every portrait / avatar / transition PNG (180 files, 39.3 → 6.1 MB). The
  measure tool can't click through a VN, so a CDP click-through of the sandbox VN chapters counted the character image bytes
  (before = the same run with the PNG paths): sandbox chapter 1 **20.0 → 2.4 MB**, chapter 5 **15.0 → 2.1 MB**.
- **Video slice**: the chapter-1 key visual (17 MB, 1080×2368, 10.6 Mbit/s) → `videos/s1c1-key-visual.900w.mp4`
  (900×1974, H.264 High, CRF 27, `+faststart`, no audio track: 2.7 MB) as slice 1 (poster `images/s1c1-key-visual.webp`),
  carrying the slice-1 music.
- Left for later: the samples (2.8 MB at 320 kbps, Web Audio, loaded in full) are now the biggest audio item – 128 kbps
  and/or loading them when their slice comes near would save ~2 MB; P4–P7. (128 kbps done, see below.)

## Slice sharpness + samples 128 kbps (2026-09-30, later)

Owner: 1x only, but as sharp as possible; samples at 128 kbps.

**Slice sharpness** (`cdn/tools/make_slice_webp.py`). Variants rendered over all 94 chapter-1 slices (799×1280 source →
390×625), compared on a crop sheet (3× zoom, nearest neighbour) of four spots with lettering and line art (slice 3
caption over rain lines, slice 8 "CUCUMBER WATER" logo, slice 20 caption text, slice 40 cat eye + panel border):

| Variant | Chapter 1 total | vs before | Look |
|---|---|---|---|
| A before: Lanczos, q85 | 2.036 MB | – | soft edges on lettering and lines |
| D Lanczos, q90 | 2.613 MB | +28 % | same softness, fewer artefacts |
| **B88 Lanczos + unsharp 0.6/60 %/2, q88 (chosen)** | **2.473 MB** | **+21 %** | crisp letter edges, close to the source |
| B Lanczos + unsharp, q90 | 2.754 MB | +35 % | no visible gain over q88 |
| B92 Lanczos + unsharp, q92 | 3.121 MB | +53 % | over budget |
| C linear-light Lanczos + unsharp, q90 | 2.781 MB | +37 % | thins the dark lettering and lines (gamma-correct averaging lightens thin dark strokes) |

Chosen: B at q88, method 6. Chapter 2 (390×625 JPGs, already 1x): re-encode only, no sharpening (it would lift the
JPEG artefacts), q88: 0.478 → 0.560 MB (+17 %). Both chapters: 2.51 → 3.03 MB (+21 %). `sharp_yuv`: Pillow does not
expose it; the tool uses `cwebp -sharp_yuv` automatically when `cwebp` is on the PATH (or `CWEBP`), which it isn't on
this machine. The new files keep their paths (the `images/webtoon/` folders are not on the live CDN yet → cache-safe).

**Samples**: `audio/samples/*.128.mp3` (libmp3lame 128 kbps CBR, 48 kHz stereo like the originals; originals locally
`*.320.mp3`): 2.87 → 1.15 MB. Only chapter 1 has a sample library (static + store; checked every stored version).
Trigger check with a CDP recorder (headless Chrome, autoplay allowed, scrolled slice by slice, then a click on the
hotzone): before and after identical – bells at slice 5 (14.4 s buffer), choir_once at 45 (12.0 s), choir_loud at 90
(45.4 s), bells (poly) on the click hotzone of slice 63.

Chapter 1 (de), production build against the local API + CDN (`VITE_API_URL` / `VITE_CDN_URL` set at build time, CDN
through a proxy on :8011 that serves the `.320.mp3` bytes for the "before"), 390×844 @3x, 4G, cold cache, median of 3:

| Chapter 1, 4G | Before (320 kbps samples) | After (128 kbps) |
|---|---|---|
| Bytes on open | 6.68 MB | **5.00 MB** |
| Page loaded (without media streams) | 4.8 s | **3.0 s** |
| First contentful paint | 1.00 s | 1.18 s (noise, same page) |
| Largest contentful paint | 1.06 s | 1.25 s (noise) |
| CLS | 0 | 0 |
| Main-thread CPU | 2.6–2.9 s | 2.6–3.0 s |
| Requests | 94 | 94 |

(Both runs already have the sharper slices; the first two slices on open are ~60 KB of the total.)
## Re-measuring

```bash
node scripts/perf/measure.mjs ch1 http://localhost:8000/catpope/chapter/1 --runs=3 --verbose
#   --width=1280 --height=800 --dpr=1   desktop   --throttle   4G (20 Mbit/s, 60 ms)   --block=*.mp3
#   --init=<file.js>   script injected before the page (e.g. a device-access entry in localStorage)   --json=out.json
```
