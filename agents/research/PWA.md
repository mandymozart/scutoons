Status: phase 1 built 2026-10-16 (installable manifest + icons, Apple tags, app-shell service worker, update prompt, kill switch) – phases 2–4 open. FEATURES 10.74.

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
- **Never touched** (no `respondWith`): other origins – api. (data, auth, sessions), cdn. (slices, posters, audio,
  video), the vault's signed URLs – and every non-GET request.
- **Offline**: the shell starts; the root layout's toons request fails → `+error.svelte` says "You're offline …" with
  "Try again" (instead of "Upsi! Not available").

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
   installs at once on every device that had one (browsers re-check it on load and our hourly check), deletes the
   caches and unregisters itself – without a fetch handler, so open pages go straight to the network. Remove the
   variable + redeploy to switch it on again. (Tested: worker + caches gone, the page uncontrolled after a reload.)
2. **Without a build** (CI broken): upload this as `viewer/dist/service-worker.js` by FTP –
   `self.oninstall=()=>self.skipWaiting();self.onactivate=e=>e.waitUntil(caches.keys().then(k=>Promise.all(k.filter(n=>n.startsWith('scutoons-shell-')).map(n=>caches.delete(n)))).then(()=>self.registration.unregister()));`
   – the next deploy overwrites it, so set `VITE_SW=off` too.
3. **One device**: Chrome DevTools → Application → Service workers → Unregister (+ Storage → Clear site data);
   Android: site settings → Clear & reset; iPhone: Settings → Apps → Safari → Advanced → Website Data → scutoons.com
   (an installed home-screen app: delete it and add it again).
4. **Rollback of phase 1 itself**: revert the commit **and** keep `VITE_SW=off` for that deploy (a plain revert
   leaves registered workers behind – a worker only goes away by a newer service-worker.js or the user).

## 3. Phase 2+ (open)

| Phase | What | Notes |
|---|---|---|
| 2 | **Media cache**: slices / posters / portraits (cdn., immutable names) cache first in `scutoons-media` with an LRU cap (≈ 150 MB, DATA.md §6; less on `light`); library JSON stale-while-revalidate with the ETag; next-chapter prefetch (LOADING.md L4) fills it | needs `Access-Control-Allow-Origin` on the bunny zone or opaque responses (opaque = ~7 MB quota padding each in Chrome – avoid); never the vault's signed URLs, never `/auth`, `/access`, `/me`; audio/video stay network (range requests) |
| 3 | **Save chapter to device** (10.74, DATA.md §6): the assets endpoint, `offline-<toon>-<locale>-c<n>` caches, 206 answers for audio ranges, `storage.persist()` + `estimate()`, membership expiry / 410 re-check deletes gated caches; offline reading progress queued for sync | the account page shows saved chapters + space |
| 4 | Polish: install hint (Android `beforeinstallprompt`, an iOS "Share → Add to Home Screen" note on the account page), manifest `screenshots` + `shortcuts` (Continue reading), badging, Web Push (needs a server part, iOS only installed), Wake Lock while reading / playing | each one an owner decision |

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
  and caches gone. Chrome: no installability errors. Lighthouse 12 has no PWA category any more (removed 2024) – it
  runs performance / a11y / best practices only.
- iOS: Safari → Add to Home Screen; Safari's Web Inspector (a Mac) for the home-screen app; check the title, the
  icon, the white launch, that a sign-in is needed again (own storage), and the prompt after a deploy.
- The Claude desktop Browser pane can't register service workers ("unknown error when fetching the script") – use
  real Chrome.
