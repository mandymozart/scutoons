# PERFORMANCE — why the app feels slow (2026-09-30)

Owner question: "Why does the entire app feel so slow? Is it because of the file-based system?"

**Short answer: no, the file-based system isn't the cause.** A content file from SQLite costs 0.2 ms. Two different
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
| Threaded CDN (`ThreadingHTTPServer`, daemon threads, 30 s idle socket timeout), listens on `127.0.0.1` + `::1` only, logs errors only, serves `root/` relative to the script | `cdn/server.py` |
| API dev server with concurrency on Windows: 4 `php -S` workers (`[::1]:9200-9203`, OPcache with `revalidate_freq=0`) behind a Node proxy on :9000 (`::1` + `127.0.0.1`, Host/headers/body unchanged, `REMOTE_ADDR` stays `::1`). `npm run dev:api` / `composer serve`. `composer serve:php` = the old single process | `server/bin/dev-serve.mjs`, `package.json`, `server/composer.json`, READMEs |
| Paraglide `locale-modules` for `vite dev` and `npm run check`. Builds keep `message-modules` (tree-shaking; total JS gzip 488 vs 486 KB, so either is fine) | `*/vite.config.ts`, `*/scripts/paraglide.js` |
| Content API: opt-in in-memory cache of published reads (in-flight sharing plus a 60 s TTL; drafts never cached; the creator keeps it off) | `shared/api/src/content.ts`, `viewer/src/lib/api.ts` |
| Toon layout: chapter files start when `season1.json` arrives. Chapter page: starts `chapterN.json` plus its scene files before `parent()` (shared with the layout through the cache) | `viewer/src/routes/(toons)/[toonId]/+layout.ts`, `…/chapter/[chapterNumber]/+page.ts` |
| Item catalog + editions through the header-free public client (no preflight) | `viewer/src/lib/api.ts` |
| Animated favicon: 12 fps (same scroll speed), `Path2D` parsed once | `viewer/src/lib/utils/favicon.ts` |

Gotcha: `npm run build` compiles paraglide as per-message modules into the same `src/lib/paraglide`. A dev server
that is still running then serves ~200–380 message files again until it restarts (it recompiles as locale modules
on start).

## Proposals (not implemented — bigger changes)

| # | Proposal | Expected gain (chapter 1, 4G 20 Mbit/60 ms) |
|---|---|---|
| P1 | **Music**: re-encode the tracks at 128 kbps MP3 (`s1c2` 16.5 → 6.6 MB) or Opus 96 kbps (5.1 MB). Stream them with Howler `html5: true` / `<audio>` + `MediaElementSource` so the crossfade still works, **or** start loading the music only after the first slices are shown / after the first user gesture (browsers block autoplay until then anyway). Load samples lazily when their slice comes near. | −31 MB (−72 % of bytes): 24 s → ~7 s without any other change; 5 s → 2 s CPU |
| P2 | **Slices**: `loading="lazy"` + `decoding="async"` **with `width`/`height` (or aspect-ratio) in the chapter config** (without them the lazy slices collapse to height 0, all intersect at once and `sliceAudio` triggers would fire together, so this needs the dimensions first). Serve WebP q80 through `<picture>`/`srcset` 2x/3x. Measured on all 94 chapter-1 slices: JPG 3x 11.3 MB → WebP 3x 7.0 MB (−39 %) → WebP 2x 4.1 MB (−64 %). | First screen needs ~2 slices (~0.2 MB) instead of 11 MB; whole chapter −4 to −7 MB |
| P3 | **Portraits / character art**: WebP q85 (412 KB → 52 KB, −87 %); lossless WebP −31 %. | −1 to −3 MB per VN scene |
| P4 | **One request per chapter**: an API endpoint (or a build step) that returns `chapterN.json` with its scene files inlined, plus a season "overview" with the list fields of every chapter (title, thumbnail, release), so the layout doesn't download six full chapter configs (~24 KB each) just to show the list. | −6 to −10 requests; −2 round trips on VN chapters (~120 ms on 4G); −100 KB JSON on `/catpope` |
| P5 | **Production hosting**: confirm OPcache on all-inkl (PHP-FPM). Enable PHP-DI container compilation (`enableCompilation` into `storage/cache`) + `composer dump-autoload -o --classmap-authoritative` in the deploy. HTTP/2 on all subdomains. | a few ms per API call; the API is already ~15 ms with OPcache |
| P6 | **CDN plan** (with the owner, see PLATFORM.md "CDN"): long `Cache-Control: max-age=31536000, immutable` for content-hashed media (P1–P3 produce new names anyway), Brotli/gzip for JSON/SVG (`map-layer-terrain.svg` 330 KB), bunny.net in front of all-inkl. | repeat visits load no media again; SVG −70 % |
| P7 | Inventory: `loadCatalog()` loads `/items/` then `/editions/` one after the other; they are independent (`Promise.all`). This was left alone because the avatar agent was editing `inventory.svelte.ts` at the time. | −1 round trip |

Suggested order: P1 (the single biggest win, a re-encode plus a loader flag), then P2 (needs slice dimensions in the
config / creator), then P3, then P4–P6 with the production cutover.

## Re-measuring

```bash
node scripts/perf/measure.mjs ch1 http://localhost:8000/catpope/chapter/1 --runs=3 --verbose
#   --width=1280 --height=800 --dpr=1   desktop   --throttle   4G (20 Mbit/s, 60 ms)   --block=*.mp3
#   --init=<file.js>   script injected before the page (e.g. a device-access entry in localStorage)   --json=out.json
```
