Status: built 2026-10-18 – §2–§4 (the tools installable: manifest, icons, app-shell worker, install / update bars, @scutoons/pwa) and §5 team push after the owner's answers (§6); owner step: PUSH_CRON_KEY + its cronjob (FEATURES 10.108). Reader PWA: research/PWA.md.

# PWA-APPS — creator, admin and embassy as installable apps, push for the team

Owner 2026-10-18: "is it possible to … also turn the other apps into pwas? I would love to allow admins to also use their
phones better with less scrollbar trouble etc. And also have push notifications for them. For example in embassy to
remind them of contacts to make, or admin if reviews are pending. Or creator if drafts are still waiting to be published
after somebody else changed something but myself."

## 1. Scope

| App | Origin | Who (shared/auth roles.ts) | PWA |
|---|---|---|---|
| creator | creator.scutoons.com | creator, admin | yes |
| admin | admin.scutoons.com | admin | yes |
| embassy | embassy.scutoons.com | liaison, ambassador, admin | yes |
| shop | shop.scutoons.com | anyone (a purchase now and then) | no – nothing to come back to daily, the reader app links there |
| qr | qr.scutoons.com | anyone scanning | no – a one-time landing page (PHP), the reader is the app |

Each app has its own origin, so its manifest `scope` / `start_url` `/` and its worker never touch the reader's (or each
other's). An installed tool is its own app with its own icon, storage and session (Android shares Chrome's cookies; an
iPhone home-screen app has its own – sign in once there).

## 2. The plain PWA (built 2026-10-18)

- **Manifest** `static/site.webmanifest` per app: `id` / `start_url` / `scope` `/`, `display: fullscreen` (owner 2026-10-18 – PWA.md §1), white
  `theme_color` / `background_color` (the boot screen's paper), names "SCUTOONS Creator" / "… Admin" / "… Embassy",
  short names "Creator" / "Admin" / "Embassy". Icons: the reader's cat head with a black round badge carrying the
  app's letter (C / A / E) – `app-icon-192/512` (`any`), `app-icon-maskable-192/512`, `app-icon-apple-180` (generated
  from viewer/src/lib/icons/account.svg; the favicon stays). app.html: manifest link, theme-color, the Apple tags.
- **Service worker** (`@scutoons/pwa` tool-worker.ts; each app's `src/service-worker.ts` is one call): index.html
  network first – this build's copy only when the network fails; `/_app/immutable/*` cache first (index.html's files
  precached, the rest on first use, unchanged files carried over between versions); manifest + icons network first.
  **Never** the API, the CDN, other origins or non-GET requests: a tool shows live data or its own "no connection"
  state – no content cache, nothing offline to go stale or leak. Caches `scutoons-<app>-shell-<version>`.
- **Updates**: a new version waits; the update bar "A new version of SCUTOONS Creator is ready · Later · Reload"
  (`UpdateBar`, the page side `ToolPwa`: checks when the tab comes back, at most every 30 min, and hourly). The very
  first install takes the open page at once. Version = package version + git sha + build time (svelte.config.js).
- **Kill switch**: a build with `VITE_SW=off` (per app) – the worker deletes `scutoons-<app>-*` and unregisters.
- **Headers** (`static/.htaccess`): manifest type; `no-cache` for index.html, service-worker.js, version.json, the
  manifest; `/_app/immutable/` a year, immutable.

## 3. Install hints (built)

`@scutoons/pwa` (shared/pwa) now holds what every app shares – moved from the viewer, not copied: `install-rules.ts`
(pure, tested: `npm test -w @scutoons/pwa`) and the `Install` state (one per app: `new Install({ key })`, the reader's
key stays `scutoons_install`). The tools draw `InstallBar` (texts de/en in `labels.ts`, no paraglide in shared):
- from the 2nd visit (a new browser session), 30 days after "Not now", never inside the installed app or a frame;
- Chromium with the browser's event → "Install" (the browser's dialog); iPhone Safari / Firefox on Android → the
  how-to in the bar itself, "OK" = seen;
- `?install` in any URL → the bar at once (testing on a real phone). A Chromium browser without an event right now
  (installed already, or Chrome hasn't offered it on this page) shows "menu ⋮ → Install app" – way `chromium-menu`, only
  with `?install` and on the reader's account page, never as a bar by itself (it may be installed). This also explains
  the Brave badge (a plain shortcut instead of an install, PWA.md §2a).
- The tools have no account page yet; the bar is the only entry (open question 6).

## 3a. One app opens another (owner 2026-10-18: "cross-referential app opening within the same phone if installed")

A link from one SCUTOONS app to another (the reader ↔ creator ↔ admin ↔ embassy) should open the OTHER installed app,
not the page inside the current one.
- **Every cross-app link is a new top-level window without an opener**: `<a href target="_blank" rel="noopener">` or
  `window.open(url, '_blank', 'noopener')` from a command – never `location.href = …` (that loads the other app inside
  this app's window) and never a plain `window.open` (an "auxiliary" window). Fixed 2026-10-18: the reader's account
  link to the creator, every tool's sign-in "Open reader" (shared/auth), the creator's palette "Reader", the admin's
  palette "open creator" / "catalog in the creator", the creator's catalog links to Admin codes / batches
  (`adminLinkTarget()`, @scutoons/catalog). Already right: the chapter preview, credits, catalog, calendar / homepage
  previews, the embassy's store page. Exception: the chapter editor's "save, then preview" opens after its dialogs with
  a plain `window.open` (it needs the return value to say when a popup blocker refused it) – a new tab, not captured.
- **Every manifest: `launch_handler: { client_mode: ["navigate-existing", "auto"] }`** – a captured link reuses the
  app's open window (and navigates it there) instead of stacking a new window per tap (Chrome parses it, 155).
- **What the platforms do with it**: desktop Chrome (Windows / Mac / Linux, 139+) captures such a click into the
  installed app ("navigation capturing"; the person can switch it off in the app's settings). Android: installing an
  app registers its origin with the system, but inside an installed app Chrome keeps a link to another origin in a
  Custom Tab (the overlay with a URL bar) – the owner's phone, 2026-10-20: "didn't work". So **on Android inside an
  installed app a cross-app link goes out as an Android intent** (`@scutoons/pwa` cross-app.ts: `crossAppClick` on the
  link, `openApp(url)` from a command): `intent://<host>/<path>#Intent;scheme=https;action=VIEW;category=BROWSABLE;
  launchFlags=NEW_TASK;S.browser_fallback_url=<the address>;end` – the system hands the address to the app registered
  for it (the other installed SCUTOONS app), else the browser opens it. Outside an installed app (a browser tab,
  desktop, iOS) the plain link. iPhone / iPad: home-screen apps can't open each other – links open in Safari's in-app
  sheet; nothing to do on our side.
- Not a cross-app link: the creator's dock Reader is the reader in a frame on purpose (the draft preview beside the
  editor, DOCKING.md "The Reader panel").

- **Sideways scroll**: `shared/tokens/base.css` clips `overflow-x` on html and body for every app since 2026-10-17
  (a safety net – whatever is wider than the page stays a bug of its own).
- **Safe areas**: no `viewport-fit=cover`, so the browser keeps content clear of the notch and the home bar; the bars
  sit `env(safe-area-inset-bottom)` above the bottom anyway.
- **Scrollbars**: the tools tile regions (AppShell, DOCKING.md); on a phone they stack. Pages with wide tables keep their
  own horizontal scroller (DataTable), never the page. Checked 2026-10-18 at 390 px: nothing wider than the page – but
  only the sign-in screens (the local MariaDB was down); the signed-in pages are still to check (§7).

## 5. Push for the team (built 2026-10-18)

### 5.1 What exists (server, PWA.md §2b)
`push_subscriptions` (endpoint, keys, `user_id` nullable, `locale`, failures) – no app / origin and no kind; `push_sends`
(log, `kind` default `new_chapter`); `/api/push/key|subscribe|unsubscribe` (the session's user is stored);
`PushSender` sends one by one (15 s timeout, 404/410 deletes); the only push is the admin's manual "new chapter".
Scheduling: no queue – cron URLs the host calls (KAS cronjobs: `/api/embassy/cron` daily 07:00, `/api/newsletter/cron`
every 5 min), each guarded by a key. Only the reader has a worker that shows notifications.

### 5.2 What changed (migration `2026-10-18-team-push`, `Domain\Push\TeamPush`)
- `push_subscriptions.app` (`viewer|creator|admin|embassy`, sent by the app with the subscription): a tool's push goes
  only to that tool's subscriptions – a tap opens the right app; the reader's "new chapter" stays `viewer` only.
- `push_prefs (user_id, kind, on)` – the person's own switch per kind, account-wide (all their devices; no row = on); a
  subscription is the device part (dogma §0.6: device → the switch, account → the kinds). `embassy.mail` = the Embassy
  reminder's mail channel.
- `push_notified (user_id, kind, ref, sent_at)` – what a person was told about, so nothing repeats (a review request, a
  proposal, a draft version once; the Embassy reminder once per day).
- Every worker shares the push handlers (`@scutoons/pwa/worker` `registerPush` – moved from the reader; the reader's
  app badge stays the reader's); only paths on the app's own origin open.
- One cron URL `/api/push/cron?key=PUSH_CRON_KEY` every 5 min (a KAS cronjob like the newsletter's) – only the drafts
  need it; reviews and proposals go at once, the Embassy reminder with its daily cron. Sends stay synchronous.

### 5.3 The kinds

| Kind | App | Who gets it | What | When |
|---|---|---|---|---|
| `embassy.due` | embassy | the lead's `owner_id` (liaison / ambassador / admin), not when `do_not_contact` | "3 contacts are due today" → `/` (the weekly portion) | the person's existing reminder choice (`embassy_pace.reminder` daily / Mondays / off) at their reminder time – a second channel of the same reminder (EmbassyReminders), never in addition to its rhythm |
| `admin.proposals` | admin | every admin except the proposal's author | "Neuer Vorschlag · <title> – von <name>" → `/proposals/<id>` | at once, on create |
| `creator.reviews` | creator | the reviewers the request was mailed to (the toon's, the physical rule's), never the author | "Zur Freigabe: <toon> · <name> bittet dich um Freigabe" → the creator | at once, on submit |
| `creator.drafts` | creator | only the toon's reviewers (`content_review_rules` reviewer, the toon or `*`), never about their own drafts | "<toon>: Entwürfe warten · 2 Entwürfe von Nova …" → the toon | the cron: the newest version is a person's draft, newer than the published one, 30 min at rest, at most 7 days old, no open publishing request |

- **Never to the actor**: every kind filters the person who made the change (author, `created_by`, `decided_by`).
- **No quiet hours** (owner): sent right away, also at night. **Batching**: one notification per kind and person per
  run (the drafts: per toon); the notification `tag` = the kind, so a newer one replaces the older on the phone.
- **Opt-in**: each tool's "Benachrichtigungen" (`/notifications`, the shared `NotificationsPage`; the top bar's
  signed-in e-mail links there, on narrow screens a round bell): the switch for this device, the kinds, "App
  installieren". The Embassy reminder: push and / or mail, the same rhythm.
- Admin → Emails → Push: "Team-Nachrichten" – the browsers per tool and a test per kind to the admin's own browsers;
  `push_sends` logs every send (kind, recipients).

## 6. The owner's answers (2026-10-18)

1. Reviews: **both** – the admin proposals in admin to every admin except the author; the content reviews in the
   creator to the toon's reviewers.
2. Embassy: **per channel** – the same rhythm (daily / Mondays / off); the person picks mail and / or push; default push
   on, mail as today.
3. Creator drafts: **only the toon's reviewers** (`content_review_rules` reviewer, the toon or `*`), never the person who
   made the change.
4. Quiet hours: **none** – sent right away, also at night; the batching and the tag replacement stay.
5. Icons: the cat head with the C / A / E badge.
6. Settings: one shared **"Benachrichtigungen" page behind the signed-in-as menu** in all three tools – the push switch,
   the kinds, "App installieren".

## 7. Testing

- Build each tool like production but against the local API (`VITE_API_URL`, `VITE_CDN_URL`), serve `build/` on a
  throw-away 127.0.0.1 port; real Chrome over CDP with a short profile path (GOTCHAS: MAX_PATH): the worker controls
  the page, `Page.getAppManifest` no errors, `Page.getInstallabilityErrors` empty, `beforeinstallprompt` fires.
- On a phone: open `creator.scutoons.com?install` → the bar; Chrome / Brave / Samsung: "Install" or menu ⋮ →
  "App installieren" (not "Zum Startbildschirm hinzufügen" – that is a shortcut with the browser's badge).
- 2026-10-18 (dev API, Chrome 141): all three – controlled on the first visit, no manifest or installability errors,
  `beforeinstallprompt` fired, no console errors; `?install` → the bar (de: "Nicht jetzt · Installieren" with Chrome's
  event, the menu how-to without it, the Share how-to with an iPhone user agent).
- **App-to-app on a phone**: install the reader and the creator from Chrome (⋮ → App installieren); in the reader
  Mein Konto → Creator → "Creator öffnen" – the Creator app should open (not a Custom Tab with a URL bar); in the
  Creator, the palette's "Reader" – the reader app. Android may first ask which app should open it ("SCUTOONS Creator"
  or Chrome – "Immer"): then once. If the browser opens instead, Android doesn't send the address to the installed app:
  tell the agents. Checked 2026-10-20 in Chrome with an Android user agent and an installed display mode (emulated): the
  tap navigates to the intent, no Custom Tab; in a browser tab the plain new tab.
- To do: the signed-in pages of all three at 390 px (`scrollWidth` = `clientWidth`, nothing past the edge outside its own
  scroller) – the test user over the dev API; a throw-away origin needs the app's dev origin sent along (CSRF check).
