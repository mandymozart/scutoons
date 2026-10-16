Status: L2, L3, L6, L9, L10 built 2026-10-16 (viewer lib/data-budget.svelte.ts + lib/lookahead.ts); L1 viewer headers + L11 phase 1 (app shell) + L11 media (PWA phase 2: CDN images + library JSON) built with research/PWA.md – L1 bunny edge rule, L4, L5, L7, L8, L12 open. FEATURES 10.101.

# LOADING — viewer loading times, lazy and predictive loading (2026-10-16)

Owner 2026-10-16: "a strategy to bring down viewer loading times and review lazy and preloading strategies. I assume we
can predictively load more than we do right now." Measured on a local `vite build` of `main` (brotli q11) and with
curl against production (scutoons.com, api.scutoons.com, cdn.scutoons.com); media sizes from the local `media/` copy
(production names). Earlier report: research/PERFORMANCE.md (P1–P3 done); offline / save to device: DATA.md §6 (10.74).

## Measurements – `/catpope/chapter/1`, cold cache, guest

| What | Requests | Compressed | Notes |
|---|---|---|---|
| HTML (SPA fallback, `ssr = false`, adapter-static) | 1 | 0.7 KB | empty body; only the 9 entry chunks are `modulepreload`ed |
| Entry JS (start + app) | 9 | 39 KB br | level 2 of the waterfall |
| Route JS: root layout + toon layout + chapter page | 49 | 221 KB br | level 3: found only after `kit.start` ran |
| **All JS before the first paint** | **58** | **260 KB br** (877 KB raw) | production's brotli ≈ 8 % larger |
| CSS (16 route chunks + `/css/styles.css`, `animations.css`) | 18 | 25 KB br | |
| API before the render (parallel, from the loads) | 4 | 14 KB br | toons 0.3 · toon 2.4 · cast 3.3 · chapter 8.3 (40 KB raw); ~0.3 s each |
| API beside it (not blocking) | 3–4 | 7 KB | `/auth/session`, `/items/` 6.3 KB, `/editions/`, `/library/map` (idle) |
| Fonts (Plex latin 400 + 700, swap) | 2 | 30 KB | in the CSS → found at level 3 |
| **Before the first paint** | **~81** | **~300 KB** | 3 origins: www, api., cdn. – no preconnect |
| First 2 slices (`EAGER_SLICES = 2`) | 2 (+1) | 59 KB | key visual poster 49 KB + slice 1 9.5 KB; **+ the key-visual loop video 2.7 MB, `preload="auto"`** (eager) |
| **First slice readable** | **~85** | **~390 KB** (+2.7 MB video already streaming) | |
| After the first 2 slices (or 4 s): audio | 5 | **12.6 MB** | both music tracks `preload="auto"` (4.9 + 6.6 MB, 128k, no variants) + 3 samples fully (Howler XHR, 1.1 MB) |
| The rest of the chapter, lazily | 93 | 2.4 MB | 95 slices, 1x WebP, median 24 KB, p90 44 KB, max 68 KB; no 2x/3x files exist yet |

Whole chapter 1 read ≈ 17.7 MB: **music 65 %**, video 15 %, slice images 14 %, app + JSON 2 %. The prologue (ch0) is
one player video: 53 / 184 / 254 MB (low / medium / high), `preload="metadata"`. Toon page: the hero trailer loop
`preload="auto"` (480p 2.7 MB / 720p 5.2 MB / 1080p 12.5 MB; `low` until Automatic is known).

Caching (production headers):
- `/_app/immutable/*`: **no `Cache-Control`** (only ETag / Last-Modified) → heuristic freshness, then 58 revalidations.
  `index.html`: none either – a heuristically cached old index can point at chunks a deploy removed.
- cdn.scutoons.com: `public, max-age=2592000` (30 days), no `immutable`; edge `cdn-cache: MISS` on slice 1 (cold edge
  per region). No `Timing-Allow-Origin` → Resource Timing sees 0 bytes for CDN files.
- API: `public, max-age=60` + ETag (`/items/` 300); the library client also keeps answers 60 s in memory (`cacheMs`).
- No service worker.

## Findings – today's behaviour

- **Waterfall of 5 levels**: HTML → entry → route JS (+ CSS + fonts) → the loads' API calls (they live in the route
  nodes, so they start only after 260 KB of JS) → render → slice images on a third origin. Nothing overlaps levels 3–4.
- **Heavy features ride the first load** (rendered size before minify; ≈ 100 KB br of the 260): the map overlay
  (+ map-terrain, viewport, content map.ts), inventory, claim sheet, item detail, camera overlay, invites panel, avatar,
  score view + Swiper (score-actions), the player overlay; `feather-icons` whole (all icons, ~17 KB br, `shared/ui`
  icon.svelte); Swiper ~20 KB br (horizontal renderer, product slides, score view); Howler ~8 KB br; paraglide both
  locales (~15 KB br wasted); the URLPattern polyfill (~5 KB br). Already lazy, good: Verovio (1.8 MB br), tfjs
  (136 KB br), jsQR, alien shapes, minigames.
- **SvelteKit preload**: `data-sveltekit-preload-data="hover"` on `<body>` (app.html): hover / touchstart on a link runs
  its load (chapter JSON) + code. The next bar navigates by `goto()` after a 10 s countdown – nothing is preloaded in
  those 10 s. No `preloadData` / `preloadCode` / `requestIdleCallback` anywhere except the map data (idle).
- **Images**: `loading="lazy"` + `decoding="async"` + width/height; the first 2 eager with `fetchpriority="high"`.
  The browser's lazy margin (Chrome ~1250–2500 px, Safari less) is 2–4 slices: a fast flick outruns it. No look-ahead
  of our own. Quality variants: the code picks `imageVariants` by media quality, but no slice has any (1x only).
- **Video**: an eager loop video gets `preload="auto"` (2.7 MB on ch1, and `afterFirstSlices` waits for its
  `loadeddata`, so the audio gate depends on it); lazy loops `preload="none"`; players `metadata`; the poster is the
  slice image. Variants by media quality; Automatic waits for the session check (`pending`) and, without the Network
  Information API (all iPhones, Firefox), **downloads a 512 KB probe with `cache: no-store` on every page load**.
- **Audio** (the biggest bytes): every track of the chapter streams with `preload="auto"` at once, also tracks that
  start much later (ch1 "tension" 6.6 MB); all samples load fully up front. Music has no quality variants yet on ch1.
- **VN**: scenes inline in the chapter JSON (good); portraits load when the script shows them (pop-in risk).
- **Map**: data at idle (small); terrain tiles only when opened (good). Fonts: `font-display: swap`, no preload.
- **Next chapter / toon page → chapter 1**: nothing is fetched ahead (only by hover).

## Measures, ranked by effect ÷ effort

Expected gains are estimates for a phone on 4G (≈ 20 Mbit/s, 60 ms RTT) / slow 4G–3G (≈ 2 Mbit/s, 150 ms).

| # | Measure | Effort | Expected gain |
|---|---|---|---|
| L1 | **Cache headers** (viewer `.htaccess`): `/_app/immutable/` `public, max-age=31536000, immutable`; `index.html` + `version.json` `no-cache`; bunny edge rule `max-age=31536000, immutable` (names are immutable by rule) + `Timing-Allow-Origin: *` | XS | repeat visit: 0 instead of ~60 revalidations (−0.2–0.6 s); no stale index after a deploy; TAO unlocks L6/L9 |
| L2 | **Audio on demand**: only the default track `preload="auto"`, the others `preload="none"` until the first slice that switches to them is ≤ 5 slices ahead; samples loaded when their slice is near (IntersectionObserver, 2 screens); music `variants` for every track (96k low) | S | ch1 open −7.7 MB (−60 % of what loads in the first minute); 3G: the slices no longer fight the music |
| L3 | **Eager video lighter**: the first loop video `preload="metadata"` and its `src` set after the first 2 slice images are `complete` (poster shows meanwhile); `afterFirstSlices` waits for the poster, not `loadeddata`; key visual with 480p / 720p variants | S | first slice readable at ~390 KB instead of competing with 2.7 MB (3G: −1–3 s to slice 1) |
| L4 | **Predict the next chapter**: when the next bar appears (10 s countdown) or at 80 % of the chapter: `preloadData(nextHref)` (JSON + code) + the first 3 slice images (exact same URLs, `new Image()`, `fetchpriority` low) + its poster; never its music / video on cellular | S | the next chapter opens from memory: −0.4–1 s, the first slices instant |
| L5 | **Toon page → chapter**: on idle, `preloadCode` of the chapter route + `preloadData` of the start button's chapter + its first 2 slices (~60 KB); home → toon: `preloadCode` of the toon route; `data-sveltekit-preload-code="viewport"` on chapter lists | S | "Lesen" opens in ~1 RTT: −0.5–1.5 s |
| L6 | **In-chapter look-ahead** with a budget (one `prefetch.ts` policy): after the first screen is loaded, at idle keep N slices ahead decoded (fast: 10 slices / 1.5 MB, slow: 3, saveData: 0 = native lazy only); unmetered Wi-Fi (Android `connection.type`): the whole chapter's images (ch1: 2.4 MB) | M | no grey slices on flicks; whole-chapter cost stays at the images' 2.4 MB |
| L7 | **Start the API with the entry**: a client `init` hook (`hooks.client.ts`) parses the URL and calls `fetchToons` / `fetchToon` / `fetchCast` / `fetchChapter` – the loads reuse the in-memory promises (`cacheMs`); `<link rel="preconnect">` to api. and cdn. in app.html (after the boot-screen work there) | S | API overlaps the 221 KB route JS: −0.3–0.6 s to the first paint; −2 handshakes (−0.2–0.4 s on 3G) |
| L8 | **Split heavy features out of the first load**: dynamic `import()` for map overlay, inventory, claim sheet, item detail, camera, invites, avatar, score view (+ Swiper), player overlay (only a chapter with a player video), Howler (only with samples); feather → a generated map of the used icons; paraglide one locale; URLPattern polyfill only when missing | M | −90–110 KB br (−35 %) and ~−300 KB raw JS to parse – most felt on low-end Android (−0.3–1 s main thread) |
| L9 | **Automatic without the probe**: decide from the first slice images' Resource Timing (needs TAO, L1) and remember the level for the session (`sessionStorage`); variants don't wait for `/auth/session` when the device setting is not `auto` | S | −512 KB per page load on iPhone / Firefox; videos start earlier |
| L10 | **Network-aware limits + data budget**: one policy (`saveData`, `effectiveType`, `downlink`, else measured) → `none / light / normal / generous`; speculative bytes per session capped (cellular 5 MB, Wi-Fi 50 MB, saveData 0); the hero trailer only on `normal+` (poster otherwise); a muted loop video only when on screen | S (with L6) | phones on data / saveData never pay for guesses; predictable "Mittel ≈ 25 MB" stays true |
| L11 | **Service worker** (10.74, DATA.md §6): app shell precache, slices / posters cache-first LRU; repeat visits and the next chapter from disk | L | repeat visit first paint ≈ 0 network; offline later |
| L12 | Portraits ahead in VN groups: the next 3 script steps' character images at idle | S | no portrait pop-in |

Suggested order: L1 → L2 → L3 (bytes and headers, small) → L4 + L5 + L7 (predictive, the owner's question) → L10 + L6
(one policy file) → L8 → L9 → L11.

## Rules for every prefetch (so it never loads twice)
- Prefetch exactly the URL the element will request (the media-quality level's file) and in the same mode: images
  without `crossorigin` (`new Image()` / `<link rel=preload as=image>`), audio with `crossorigin=anonymous`.
- Only at idle (`requestIdleCallback`, fallback `setTimeout`), never during the first screen, never on `saveData` /
  `2g`, never video or music on cellular; abort when the reader leaves the chapter.
- Count the bytes (Resource Timing, needs TAO) against the session budget; the account page's "Mittel etwa 25 MB"
  text must stay true.
- Check: a `check:*` browser script that opens ch1 with throttling and asserts requests / bytes before slice 1 and that
  nothing is downloaded twice (`viewer/scripts`, like research/PERFORMANCE.md "Re-measuring").

## Built 2026-10-16 (owner: "do the data measures … tune it in")
- **One policy** `lib/data-budget.svelte.ts`: tier `none` (saveData, 2G) / `light` (3G, < 1.5 Mbit/s, quality Low) /
  `normal` (unknown, 4G) / `generous` (Android Wi-Fi / Ethernet). Per tier: images ahead (0 / 1.5 / 4 / 10 screens),
  audio ahead (0.5 / 2 / 4 / 10 screens), all audio at once (generous only), first loop video after the first images
  (normal+; else when on screen), session budget for guesses (0 / 3 / 12 / 60 MB, sessionStorage `scutoons_prefetch_bytes`).
- **Look-ahead** `lib/lookahead.ts`: one IntersectionObserver per distance; at idle after the chapter's first screen
  (`afterFirstSlices`, videos count with their poster); the exact `<img>` URL, no crossorigin, `fetchPriority` low.
- **Audio on demand**: the default track + what the first slices raise load at the start; other tracks dormant (no
  `src`) until a slice that raises them comes near or asks for a volume – then they join the default track in step;
  samples likewise, a trigger before the file is there plays it on arrival (≤ 1 s late, else dropped).
- **Short loops** (`…-loop.<rate>.mp3`): decoded once, looped sample-exactly in the music's own AudioContext
  (AudioBufferSourceNode → the track gain → the music master: the effect chain still applies). Ch1 now: calmness
  29.9 s + tension 8 bars 26.9 s, 0.3–1.2 MB per level instead of 4.9 + 6.6 MB (draft 656).
- **Probe once per session** (`scutoons_media_auto`); the hero trailer preloads only on normal+.
