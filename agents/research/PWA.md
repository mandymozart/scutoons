Status: phase 1 built 2026-10-16 (installable manifest + icons, Apple tags, app-shell service worker, update prompt, kill switch); phase 2 built 2026-10-16 (media + library cache, §2 "Phase 2"); the Android install prompt (phase 4 part) built 2026-10-16 – phases 3–4 otherwise open. FEATURES 10.74.

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
  sessions, inventory, game state, codes, purchases), audio and video, the vault's signed URLs – and every non-GET request.
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
   `scutoons-*` cache (shell, media, library) and the IndexedDB `scutoons-sw`, and unregisters itself – without a fetch handler, so open pages go straight to the network. Remove the
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
| 3 | **Save chapter to device** (10.74, DATA.md §6): the assets endpoint, `offline-<toon>-<locale>-c<n>` caches, 206 answers for audio ranges, `storage.persist()` + `estimate()`, membership expiry / 410 re-check deletes gated caches; offline reading progress queued for sync | the account page shows saved chapters + space |
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
- iOS: Safari → Add to Home Screen; Safari's Web Inspector (a Mac) for the home-screen app; check the title, the
  icon, the white launch, that a sign-in is needed again (own storage), and the prompt after a deploy.
- The Claude desktop Browser pane can't register service workers ("unknown error when fetching the script") – use
  real Chrome.
