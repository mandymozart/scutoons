Status: phase 1 built 2026-10-16 (installable manifest + icons, Apple tags, app-shell service worker, update prompt, kill switch); phase 2 built 2026-10-16 (media + library cache, §2 "Phase 2"); phase 3 built 2026-10-16 (save a chapter to the device, §2 "Phase 3"); the Android install prompt (phase 4 part) built 2026-10-16 – phase 4 otherwise open. FEATURES 10.74.

# PWA — SCUTOONS as an installable web app (2026-10-16)

Owner 2026-10-16: "research what needs to be done to turn scutoons into a PWA and then start the process" (asked while
looking at iPhone haptics). Builds on DATA.md §6 (the save-to-device concept) and research/LOADING.md (L1 headers, L11
service worker, the data budget `lib/data-budget.svelte.ts`). Scope: the reader (viewer/, scutoons.com) only.

## 1. Installability – what was missing, what phase 1 did

| Need | Before | Now (phase 1) |
|---|---|---|
| Manifest `name` / `short_name` | empty strings | "SCUTOONS" |
| `id`, `start_url`, `scope` | missing | all `/` – the unprefixed path follows the locale cookie / browser language (paraglide), so one manifest serves de + en; `/en/…` stays in scope |
| `display` | standalone | standalone (iOS knows only standalone; `fullscreen` would hide the Android status bar – not wanted for reading) |
| `theme_color` / `background_color` | #fff | #fff = `--paper` (tokens.css), the boot screen's white – no colour jump on launch |
| Icons | 192 + 512 `any` (a crop of the vertical logo) | + `icon-maskable-192/512.png` (the logo strip at 72 % width on white – survives circle / squircle masks); `favicon.svg` was the Svelte logo → the same SCUTOONS crop |
| Apple | `apple-touch-icon` only | + `apple-mobile-web-app-title`, `-capable`, `mobile-web-app-capable`, status bar `default`, `theme-color` (app.html; the boot screen untouched) |
| Splash (iOS `apple-touch-startup-image`) | – | deliberately none: one image per device size (~20 files) for a white screen we already have |
| Service worker | none | `src/service-worker.ts` (§2) – Chrome needs none for install any more, but offline / updates do |
| MIME + headers | – | `.htaccess`: `AddType application/manifest+json .webmanifest`; `no-cache` for index.html, service-worker.js, version.json, the manifest; `/_app/immutable/` 1 year immutable (= LOADING.md L1, viewer part) |
| Description, screenshots | – | description = the slogan; `screenshots` (Chrome's richer install sheet) – phase 4 |

Chrome reports no installability errors (CDP `Page.getInstallabilityErrors`, `Page.getAppManifest`: no errors).

## 2. Service worker

### Phase 1 (built) – the app shell, nothing that can strand a reader
- **Versioned cache** `scutoons-shell-<kit version>`; the version = `<package version>-<git sha 12>-<build time>`
  (svelte.config.js, the sha from deploy/version.mjs like version.json), one per build.
- **index.html network first** (navigation preload on); the cached copy – this build's own, fetched at install – only
  when the network fails. An online reader always gets the newest index.html.
- **`/_app/immutable/*` cache first** (hashed names never change). Precached at install: index.html's files, the root
  layout and error nodes, the CSS (≈ 25 KB br). Everything else is cached the first time the app uses it – no guessed
  bytes (the data budget's rule); the bundled fonts are not precached (they come from the CDN). Data saver / 2G
  (`navigator.connection` in the worker): only index.html's files. On the first install the page hands over the app
  files it already loaded (Resource Timing → `CACHE_LOADED`, fetched `force-cache` = from the HTTP cache), so the shell
  works offline right away.
- **Carry-over**: a new version copies every file it still has from the old cache – a deploy downloads only changed
  chunks. A mid-deploy index.html that names files of another build makes the install fail (FTP uploads file by file);
  the next update check tries again.
- **Never touched** (no `respondWith`): other origins except phase 2's two (below) – the rest of api. (auth,
  sessions, inventory, game state, codes, purchases; phase 3 only adds `/items/` + `/editions/` as a saved chapter's
  offline copy), audio and video (except a saved chapter's, phase 3), the vault's signed URLs – and every non-GET request.
- **Offline**: the shell starts; with phase 2 the library JSON and the images the reader saw come from their caches. A
  page whose data isn't cached (the root layout's toons request fails) → `+error.svelte` says "You're offline …" with
  "Try again" (instead of "Upsi! Not available").

### Phase 2 (built 2026-10-16) – media and library cache
Code: `lib/sw/cache-rules.ts` (what qualifies, caps – pure, test/sw-cache-rules.test.ts), `lib/sw/runtime-cache.ts`
(the handlers), `lib/sw/media-index.ts` (IndexedDB `scutoons-sw`: per file { url, size, used, fetched } + the tier).
- **Media** `scutoons-media-v1`, **cache first**: GET images (webp/png/jpg/avif/gif/svg) on the public CDN origin
  (`VITE_CDN_URL`), whole files only – never a URL with a query (signed URLs), a `Range` / `no-store` request (the
  media-quality probe), `Authorization`, `crossorigin=use-credentials`, audio, video, fonts. Slices, posters, portraits,
  item icons, map tiles.
- **CORS** (checked 2026-10-16 with curl): cdn.scutoons.com (bunny pull zone) answers `Access-Control-Allow-Origin: *`
  on every file, with or without an `Origin` header (also on 404s); the local dev CDN too. An `<img>` without
  `crossorigin` asks `no-cors` (= an opaque answer, ~7 MB of quota padding each in Chrome), so the worker fetches the
  same URL again with `mode: cors, credentials: omit` and stores that readable, sized answer; opaque answers are never
  stored. No owner step for the bunny zone needed. If the CDN ever stops sending the header, the cors fetch fails and
  the worker falls back to the browser's own request – nothing cached, nothing broken.
- **LRU cap per data tier** (the page posts `DATA_TIER` from `data-budget.svelte.ts` on load, on every change and when a
  worker takes over; the worker stores it and otherwise guesses from `navigator.connection`; data saver always wins):
  normal / generous 150 MB, light 50 MB, **none: nothing new is stored** (hits still served). Never more than 25 % of
  `storage.estimate().quota`. Over the cap → the least recently used go, down to 90 % of the cap. A file older than
  30 days (the CDN's max-age) is fetched again (the old copy when offline).
- **First visit**: the images the page showed before the first worker took over are handed over (`CACHE_MEDIA`,
  Resource Timing, fetched `force-cache` = from the HTTP cache) – a chapter read on the first visit works offline.
  The look-ahead's `new Image()` prefetches pass through the worker → the same cache (no extra bytes).
- **Library** `scutoons-library-v1`: GET `<api>/library/…` from the header-free public client only – no credentials,
  no `Authorization`, no `X-Device-Id`, never `draft`, and only answers that say `Cache-Control: public` (drafts answer
  `private, no-store`). Younger than 10 min → the cached copy at once, revalidated behind; older → the network first,
  the cached copy after 3 s or offline (a publish shows on the next load at the latest). Revalidation = the browser's
  HTTP cache with the ETag (an `If-None-Match` of ours would need a CORS preflight the API doesn't allow). A 4xx
  (chapter gone, `coming_soon`) deletes the stored copy. At most 200 answers. Not cached: `/events/`, `/items/`, …
- **Updates**: both caches outlive builds (immutable names / revalidated JSON); activate deletes other *versions* of them
  (bump the `-v1` in runtime-cache.ts when the stored form changes) and reconciles the bookkeeping with the cache.
- Not cached: audio and video (range requests) – phase 3 saves chapters explicitly.

### Phase 3 (built 2026-10-16) – save a chapter to the device
Owner 2026-10-16: "Go 2 and 3, push in between". Code: server `Domain/Library/ChapterFiles.php` (the list),
`lib/sw/saved-rules.ts` (pure, test/sw-saved-rules.test.ts), `lib/offline/saved-chapters.svelte.ts` (the page: download,
progress, cancel, remove, access), `lib/sw/saved-cache.ts` (the worker: answers), `components/toon/save-chapter.svelte`
(the row's button), `components/account/saved-chapters-setting.svelte`.
- **Where**: a round button on every chapter row (toon page + the chapter menu, `chapter-item.svelte`) – ↓ save, a ring
  filling while it saves (a tap asks "Stop saving?"), ✓ saved (a tap asks "Remove … from this device? Frees about
  17 MB"); failed → ! (a tap tries again). Only with a service worker controlling the page (production, not the
  creator's Reader frame) and only for chapters the reader may read: published (never a draft or coming soon), a free
  preview (`isPreview`) or the reader's access (device code / account). Account page → "Saved chapters": each save
  (toon · chapter · title, MB, remove), "Remove all", the space (`storage.estimate()`: "SCUTOONS uses 76 MB of the
  10316 MB this browser allows"); nothing when nothing is saved.
- **The size first** – the list is one server request, `GET /library/toons/<t>/chapters/<n>/files?locale=`
  (server/docs/API.md "Library"): every image / audio / video of the chapter as the library answers it, the portraits +
  avatars of the personas its VN puts on stage (+ the regular persona, the fallback anchor; no `full` view, no
  A-pose), the icons of the items it names; sizes from the asset registry (`asset_renditions.size`); a media with
  quality variants yields per level the file `pickQualityVariant` plays (`levels` + one `group` per media). The dialog
  says "Save Chapter 1 to this device? About 17 MB. Then it reads without a connection."; videos ≥ 10 MB and ≥ 30 % of
  it → a choice "Everything · about 53 MB" / "Without videos · about < 1 MB"; on mobile data (Android reports
  `connection.type`) from 20 MB "You're on mobile data.", with data saver "Data saver is on." – the reader decides,
  nothing starts without the tap.
- **Level**: the reader's media quality (`effectiveQuality`: Automatic unmeasured → low). The manifest keeps
  **stand-ins**: every other level's file of a saved media → the saved file, so offline (Automatic's probe fails → low)
  or after a changed setting the chapter still plays from the device. The quality probe (`cache: no-store`) is never
  answered from the device – it measures the network.
- **Storage**: one cache per chapter `scutoons-chapter-<toon>-<n>-v1` (never touched by the media LRU); the page
  downloads (4 at a time, counted as the bytes arrive; images the media cache holds are copied, not fetched again;
  images fetched `reload` past the worker – no second copy in the media cache; audio / video / fonts may come from the
  HTTP cache), then a manifest `/__scutoons/saved-chapter.json` last (level, bytes, URLs, docs, stand-ins, version =
  the chapter's ETag, `gated`, `accessUntil`) – a cache without one is an unfinished save and is deleted. With it: the
  chapter / toon / cast / toons-list / credits JSON (exactly the URLs `lib/api.ts`'s public client asks), `/items/` +
  `/editions/` (the inventory's catalog), the CDN fonts the page loaded (lettering), and the app code of the toon,
  chapter and account pages (`preloadCode` through the worker → the shell cache; again on every start with saves, so an
  app update's new chunks are there too). `navigator.storage.persist()` on the first save. A CDN 404 is skipped.
- **The worker** (`saved-cache.ts`): an index URL → cache from the manifests (at start, on activate and on the page's
  `SAVED_CHANGED`). CDN requests: the saved copy first (images before the media cache; audio / video / fonts – before
  the index is read, any of those is routed through the worker, which falls back to the network), a `Range` request →
  **206** sliced from the whole cached file (`Content-Range`, 416 past the end; the music's `<audio>` streams, seeking,
  the key-visual `<video>`). Library JSON: phase 2's handler, the saved copy when its cache has none / offline;
  `/items/` + `/editions/` (header-free client only): network first, the saved copy when it fails.
- **Access**: a gated save stores the access end it was saved with (the latest of the reader's valid grants – device
  code `expires_at`, account `access.expires_at`; null = none). Offline, `MembershipBlocker` shows a saved chapter
  until that end (offline the session can't say – a member signed in only by cookie reads too). On every start the
  root layout checks: the access counts as **known** when `/api/health` answers and the session loaded without an error;
  known and no access → every gated save is deleted; known with access → the saves get the current end (a renewal
  reaches offline reading); unknown (offline) → the saved end decides. Free chapters (`isPreview`) are never deleted.
  Drafts, the creator preview and the vault's signed album URLs are never saved (not in the list, not readable).
- **Kill switch**: `VITE_SW=off` deletes every `scutoons-*` cache – the saved chapters too (page side as well:
  `pwa.svelte.ts` `removeServiceWorker`).
- Sizes (local dev DB = the production seed, 2026-10-16): ch1 low 17.0 MB · medium 17.2 · high 18.9 (2.6 MB of it the
  key-visual video; no "without videos" offer); ch0 (the prologue music video) low 53 MB · medium 184 · high 254 (all
  but < 1 MB video → the choice); ch2 (coming soon, measured from its draft) ≈ 39 MB low · 41.5 medium / high, ≈ 19 MB
  of it video and ≈ 14 MB the VN portraits of six characters (catpope with four personas).
- Open: a saved chapter whose version (ETag) changed is not refreshed yet (download only the changed paths); a whole
  season; reading progress made offline is not queued for sync (members: the next online read saves it); iOS: the
  home-screen app has its own storage – saves made in Safari don't carry over.

### Update flow
New worker installs → waits (no `skipWaiting`) → `lib/pwa.svelte.ts` shows the prompt (`update-prompt.svelte`: "A new
version of SCUTOONS is ready." · Later · Reload). Reload → `SKIP_WAITING` → `controllerchange` → `location.reload()`
(3 s fallback). If the page already runs the waiting worker's build (a reload fetched the new index.html), it takes
over silently (`VERSION` message). An SPA never navigates, so the page calls `registration.update()` when it becomes
visible (at most every 30 min) and hourly; `updateViaCache: 'none'`. The first install claims the open page (no prompt).
Not registered in dev, nor in the creator's Reader frame.

### Kill switch (if anything goes wrong)
1. **Everyone**: set the GitHub variable `VITE_SW=off` (environment production) and redeploy the viewer (Deploy
   workflow, apps `viewer`). That build registers no worker and unregisters existing ones; its `service-worker.js`
   installs at once on every device that had one (browsers re-check it on load and our hourly check), deletes every
   `scutoons-*` cache (shell, media, library, the saved chapters) and the IndexedDB `scutoons-sw`, and unregisters itself – without a fetch handler, so open pages go straight to the network. Remove the
   variable + redeploy to switch it on again. (Tested: worker + caches gone, the page uncontrolled after a reload.)
2. **Without a build** (CI broken): upload this as `viewer/dist/service-worker.js` by FTP –
   `self.oninstall=()=>self.skipWaiting();self.onactivate=e=>e.waitUntil(caches.keys().then(k=>Promise.all(k.filter(n=>n.startsWith('scutoons-')).map(n=>caches.delete(n)))).then(()=>indexedDB.deleteDatabase('scutoons-sw')).then(()=>self.registration.unregister()));`
   – the next deploy overwrites it, so set `VITE_SW=off` too.
3. **One device**: Chrome DevTools → Application → Service workers → Unregister (+ Storage → Clear site data);
   Android: site settings → Clear & reset; iPhone: Settings → Apps → Safari → Advanced → Website Data → scutoons.com
   (an installed home-screen app: delete it and add it again).
4. **Rollback of phase 1 itself**: revert the commit **and** keep `VITE_SW=off` for that deploy (a plain revert
   leaves registered workers behind – a worker only goes away by a newer service-worker.js or the user).

## 2a. Install prompt (Android / Chromium, built 2026-10-16)
Owner 2026-10-16: "a prompt to install for Android first". `lib/install.svelte.ts` keeps `beforeinstallprompt`
(`preventDefault` – no Chrome mini bar), `components/install-prompt.svelte` is the bar (the update prompt's look; a
waiting update goes first): "Install SCUTOONS as an app" · Not now · Install → Chrome's own install dialog.
**When** (pure rules `lib/install-rules.ts`, test/install-rules.test.ts):
- never on the first visit; from the **2nd visit** (a new browser session – sessionStorage marks one) or after the reader
  **finished a chapter** (the chapter's end bar showed with a next chapter / members / coming soon / season end – not
  a mere next group; next-bar.svelte → `install.chapterFinished`);
- checked on the start and on each navigation – never live over the chapter's end bar: a finished chapter offers it on
  the next page (the next chapter, the toon page …); an event arriving late on a start shows at once;
- not inside an installed app (display-mode standalone / fullscreen / minimal-ui, `navigator.standalone`, a TWA
  referrer), not in the creator's Reader frame, not where the browser sends no event (iOS, Firefox);
- **Not now** (or dismissing Chrome's dialog) → away for 30 days; **appinstalled** → never again on this browser.
  Stored in localStorage `scutoons_install` `{ visits, chapters, snoozedAt, installed }` (try/catch; blocked storage =
  this visit only).
- Account page → settings: "App · Install app" whenever the browser can install (also while the bar is snoozed).
- Dev: no real event on the dev server – `__scFakeInstallPrompt('dismissed' | 'accepted')` in the console (dev builds only).
- **iOS: nothing yet** – later item: a "Share → Add to Home Screen" how-to on the account page (Safari, not standalone).

## 3. Phases 2–4

| Phase | What | Notes |
|---|---|---|
| 2 | **Built** – media cache + library JSON (§2 "Phase 2"); the next-chapter prefetch (LOADING.md L4) will fill it by itself | bunny sends `ACAO: *` – no owner step |
| 3 | **Built** – save a chapter to the device (§2 "Phase 3"): the files endpoint, one cache per chapter, 206 for ranges, `storage.persist()` + `estimate()`, gated saves deleted when the access ended | open: refresh on a new version, a season, offline progress sync |
| 4 | Polish: install hint (Android built – §2a; open: the iOS "Share → Add to Home Screen" how-to on the account page), manifest `screenshots` + `shortcuts` (Continue reading), badging, Web Push (needs a server part, iOS only installed), Wake Lock while reading / playing | each one an owner decision |

Data budget: phase 2/3 obey `data-budget.svelte.ts` – tier `none` caches only what the reader looked at, nothing is
prefetched for the cache; saving a chapter is an explicit tap (the bytes shown first).

## 4. iOS vs Android for SCUTOONS (2026)

| Capability | iPhone / iPad (Safari, home-screen web app) | Android (Chrome, installed PWA / WebAPK) |
|---|---|---|
| Install | manual: Share → Add to Home Screen (since iOS 26 such sites open as web apps by default); no prompt API | `beforeinstallprompt` + Chrome's install UI; a real WebAPK in the launcher |
| Vibration / haptics (`navigator.vibrate`) | **no** – not in Safari, not in home-screen apps. Only hack: iOS 18+ ticks when an `<input type=checkbox switch>` toggles from a tap (fragile, no patterns) | yes (after a user gesture) |
| Audio: autoplay | only after a tap (both platforms); the ring/silent switch mutes Web Audio unless `navigator.audioSession.type = 'playback'` (Safari 16.4+) – our music runs in an AudioContext | after a tap / media engagement |
| Audio in the background / locked | `<audio>` with Media Session can go on; **Web Audio (our loops, effects) is suspended** when the app leaves the screen | goes on (Media Session notification) |
| Push notifications | iOS 16.4+, **only installed**, permission from a tap; Declarative Web Push 18.4+ | yes, browser and installed |
| Badging | 16.4+ installed | installed |
| Storage | Cache + IndexedDB up to ~60 % of disk (Safari 17+); `storage.persist()` 17+; Safari tabs: script storage deleted after 7 days without a visit (ITP) – home-screen apps exempt; **a home-screen app has its own storage and cookie jar** → device access (localStorage), progress and sign-in don't carry over from Safari: the reader signs in / scans again | up to ~60 % of disk; best-effort eviction unless persisted (installed apps usually granted); shares Chrome's storage and cookies |
| Display / fullscreen | standalone only; Fullscreen API on iPad, not iPhone (only `<video>` fullscreen); status bar via the Apple meta | standalone, fullscreen, minimal-ui, `display_override` |
| Orientation lock | ignored (manifest and `screen.orientation.lock`) | manifest `orientation` / lock in fullscreen |
| Share target (receive shares) | no | yes (manifest `share_target`) |
| Web Share (send) | yes | yes |
| Background sync / fetch | no | yes |
| Camera (QR / cucumber scan) | yes in home-screen apps (getUserMedia) | yes |
| Wake Lock (screen on while reading) | 16.4+ (home-screen apps since 18.4) | yes |

## 5. Native wrapper – when it is worth it
- **TWA (Android, Trusted Web Activity)**: the PWA itself in a Play Store package (Bubblewrap / PWABuilder +
  `/.well-known/assetlinks.json`). Cheap, no second codebase, same updates. Worth it as soon as a **Play Store listing**
  matters (search, trust, reviews). Play's billing rules apply to digital goods sold inside the app – our sales
  (codes, shop) happen outside; check before listing.
- **Capacitor (iOS + Android)**: the same viewer in a native shell with plugins – Taptic haptics, background audio
  (AVAudioSession, so the music and loops go on locked), APNs push for every iOS user (not only installers), an App
  Store listing. Costs: Apple review (4.2 "minimum functionality", 3.1.1 in-app purchase – memberships / codes would
  need the "reader app" route or IAP), two native builds, store releases for shell changes. Worth it when iPhone
  haptics, locked-screen music or iOS reach become product requirements – not before phase 2/3 have shown readers
  use the installed app.
- Recommendation: PWA phases 2–3 first; TWA when the Play Store is wanted (days of work); Capacitor only as an owner
  decision with the App Store business questions answered.

## 6. Testing
- Build like production but against the local API: `VITE_API_URL=…` (GOTCHAS: a plain build bakes production URLs),
  serve `viewer/build` with the .htaccess headers (SW scripts must not be cached).
- Chrome DevTools → Application: Manifest (no errors, the maskable preview), Service workers (status, "Update on
  reload" off when testing the prompt), Cache storage (`scutoons-shell-<version>`), Network → Offline.
- Scripted (2026-10-16, headless Chrome over CDP): install + control on the first visit (93 files cached), a rebuild +
  `update()` → the new worker waits, the prompt shows; Reload → the new version, the old cache gone; offline
  navigation → the shell (+ the offline message when the API isn't in the HTTP cache); `VITE_SW=off` build → worker
  and caches gone. Chrome: no installability errors.
- Phase 2, scripted (2026-10-16, same setup + proxies in front of the dev API / CDN that can go offline; a test device
  access in localStorage – ch1 needs one locally): media cache fills (incl. the images shown before the first takeover),
  no opaque entries, bookkeeping = cache; private requests (`/auth/session`, `/me/`, `/inventory/` with Authorization,
  a `draft=1` read with credentials, `/items/`, a `?token=` URL, an mp4 range, a no-store probe) never cached; a reload
  → every CDN image and library answer through the worker; offline with the HTTP cache cleared → the chapter renders
  from the library cache, every cached slice shows; 60 MB of fillers + tier `light` → evicted to ≤ 50 MB, the least
  recently used first, the read slices kept; tier `none` stores nothing new; an update keeps media + library, deletes
  an old `scutoons-media-v0`; the three.js chunk stays out of the shell until a page uses it; `VITE_SW=off` → every
  `scutoons-*` cache and the database gone. Lighthouse 12 has no PWA category any more (removed 2024) – it
  runs performance / a11y / best practices only.
- Phase 3, scripted (2026-10-16, the same setup; the API proxy answers CORS for the test origin so `/api/health`
  counts): the toon page shows ↓ on chapter 0 and 1, none on the coming-soon 2; saving chapter 1 says "Etwa 17 MB"
  first, saves 108 files (+ 7 JSON, 3 fonts, 17.0 MB, 6 stand-ins); then the media + library caches deleted, the HTTP
  cache cleared, the device access removed and the network cut: the chapter opens (no blocker), all 95 slices show, the
  chapter's music and samples and the key-visual video come from the worker (3 × 206), a 5-minute track plays and seeks
  to 70 %, the key-visual video plays (900 × 1974), another level's sample is answered by the saved one, the quality
  probe is not; remove from the row → the cache gone; the prologue offers "Everything 53 MB / Without videos < 1 MB";
  the account page lists it with the space, "Remove all" empties; an expired device access → the gated chapter 1 is
  deleted on the next start, the free prologue kept; `VITE_SW=off` → the saved chapters gone with every other cache.
- iOS: Safari → Add to Home Screen; Safari's Web Inspector (a Mac) for the home-screen app; check the title, the
  icon, the white launch, that a sign-in is needed again (own storage), and the prompt after a deploy.
- The Claude desktop Browser pane can't register service workers ("unknown error when fetching the script") – use
  real Chrome.
