# PLATFORM — monorepo, server refactor, shared libraries, creator + admin apps

Written 2026-09-30 on the former `platform` branch (merged; `main` is the only branch since 2026-10-05). The phase
results (P1a … P3) and the cutover checklist below are history; how agents work: [RULES.md](RULES.md). Companion docs:
[ITEMS.md](ITEMS.md) (API contract), [DESIGN.md](DESIGN.md), [DATA.md](DATA.md), deploy/README.md.

## Target picture

```
scutoons/                         one git repo, npm workspaces (Node 22), Composer for the server
├── package.json                  workspaces: viewer, creator, admin, shop, shared/*
├── viewer/                       SvelteKit (static) → scutoons.com            reader, events, account (/shop → shop)
├── creator/                      SvelteKit (static) → creator.scutoons.com    role `creator`
├── admin/                        SvelteKit (static) → admin.scutoons.com      role `admin` (replaces generate/)
├── shop/                         SvelteKit (static) → shop.scutoons.com       limited editions (P2b)
├── server/              PHP 8.3, Slim 4    → qr.scutoons.com (+ later api.scutoons.com)
│   ├── public/                   web root: front controller, QR landing page, verify page, assets
│   ├── src/                      Http (controllers, middleware) · Domain · Infrastructure
│   ├── config/                   settings, routes (the catalog is in DB tables, agents/CATALOG.md)
│   ├── migrations/mysql/
│   ├── bin/console               migrate, roles, import-content, backfill, reports
│   ├── templates/                landing page, mails
│   └── tests/                    PHPUnit (+ fixtures/catalog, fixtures/content)
├── shared/
│   ├── tokens/   @scutoons/tokens    design tokens (CSS custom properties), fonts, base CSS
│   ├── ui/       @scutoons/ui        Svelte UI kit: Button, IconButton, Field, Panel, Sheet, Tabs, Toast, DataTable, …
│   ├── content/  @scutoons/content   TS types + validation for toon/season/chapter/slice/dialog/audio/VN/items; JSON Schema export
│   ├── reader/   @scutoons/reader    rendering: Slice, Dialogs (+ text box reveal effects), placed items, group renderers, audio/VN actions
│   ├── api/      @scutoons/api       typed client: session, device id, access, items, inventory, content, admin
│   ├── auth/     @scutoons/auth      single sign-on: session factory, login form, app gate, signed-in-as (SSO, 2026-10-01)
│   └── pwa/      @scutoons/pwa       installable apps: install rules + state (all apps), the tools' app-shell worker + install / update bars (research/PWA-APPS.md, 2026-10-18)
├── media/                        local dev media (gitignored; served by scripts/dev/media-serve.mjs)
└── agents/                       plans and memory (this folder)
```

Public pages served by the server stay public: the QR landing page (`/?c=`) and the verify page. Everything else on
the server is API. The three frontends log in against the same account API (email + one-time code); access is by role:
`member` (viewer account), `creator` (creator app), `admin` (admin app).

## Shared libraries — the concept

**Why:** the reader's rendering (slice, text boxes, onomatopoeia) must look identical in the viewer and in the creator's
WYSIWYG canvas; tokens/UI must look identical in viewer, creator, admin and the server's public pages; the content
types are the contract between creator (writes), server (stores/validates) and viewer (reads).

**Rules (dogmas):**
1. **Dependency direction:** apps → shared, never shared → apps. Inside shared:
   `tokens` ← `ui` ← `reader`; `content` has **no** Svelte dependency; `api` depends on `content` only.
2. **Source packages, no build step:** each package is `private`, version `0.0.0`, exports TS/Svelte source via
   `exports` + the `svelte` condition; Vite compiles it inside each app. No publishing, no dist folders.
3. **No app state in shared:** no stores/contexts bound to one app, no routing, no paraglide. Components take props and
   emit callbacks; apps own state (session, access, inventory contexts stay in apps, built on `@scutoons/api`).
4. **i18n:** shared components receive already-translated strings as props (or a small `labels` object); each app keeps
   its own paraglide messages.
5. **Tokens are the single source of style:** `@scutoons/tokens` holds DESIGN.md Part 1 as CSS custom properties
   (`--ink`, `--paper`, `--primary-cyan`, `--space-*`, `--radius-*`, `--border-width-*`, type styles) + font files.
   Apps and the server's public pages import the same CSS (qr gets a generated copy: `qr/bin/sync-tokens.mjs`).
   Fonts only self-hosted from here (no font CDN); `npm run check:fonts` measures it in the browser.
6. **Content types are the contract:** `@scutoons/content` is the only place the chapter schema is defined; it exports
   a JSON Schema that the server uses to validate creator writes.
7. **Each shared package has a README** (purpose, public API, examples) and its own `check` script; `npm run check`
   at the root checks all workspaces.
8. **Moving code into shared is a move, not a copy:** the app's old file is deleted in the same commit.

## Server refactor — the concept

Today: plain PHP, `require_once` bootstrap, one `api/index.php` router, classes in `api/lib`, pages at the root, admin
SPA in `generate/` behind a static password token.

Target: **Slim 4** (PSR-7/15, small, fine on shared hosting) + PHP-DI + Composer autoload (PSR-4 `Scutoons\`),
PHPUnit, PHPStan. Composer `vendor/` is built in CI and uploaded (the host has no shell).

- `Http/Controllers/*` thin; `Http/Middleware/*`: Cors, JsonBody, DeviceId, Session (Bearer), RequireRole, RateLimit,
  ErrorHandler (the `{status:"error", error, message}` shape).
- `Domain/*` (Accounts, Auth, Codes, Access/Permissions, Inventory, Catalog, Avatar, Progress, Content, Events, Roles):
  services + repositories, no HTTP.
- `Infrastructure/*`: Database (PDO, MySQL/MariaDB only since CUTOVER 10.30, transactions, migrations runner), Mail (SMTP, log), Signing (ML-DSA
  keys), Clock.
- Config from env/`config.local.php` → `config/settings.php`; JSON data (permissions, items, editions) in `config/`.
- **Contract first:** the HTTP API (ITEMS.md) does not change during the refactor. The existing suites
  (`api-smoke` 182 checks, `content-smoke`, `claim-race`) are the safety net and must pass after every step.
- Web root becomes `public/`; a root `.htaccess` forwards into it for the current host layout, and denies everything
  else.
- Admin API: `/api/admin/*` with Session + `RequireRole(admin)`; the static `GENERATE_PASSWORD` token stays only until
  the admin app reaches parity with `generate/`, then `generate/` is removed.

## Admin app (admin.scutoons.com)

Login with the account flow, role `admin`. UI patterns from akin (see "Admin UI patterns" below once researched),
styled by `@scutoons/tokens` + `@scutoons/ui` (square, ink/paper, mono, cyan only as fill/glow).

Sections: Dashboard (scans, unlocks, claims, signups per day/edition) · Codes (table, filters, detail timeline, revoke, change the expiry – one or a selection)
· Batches (port of generate: sign, QR SVG, certificate PDF, sticker sheet, ZIP) · Users & roles · Content (versions,
publish state per toon/chapter) · Catalog (items, editions, permissions map read-only view) · Events log · Settings
(runtime settings: QR scan limit + lookup limit per IP and hour, `GET/PUT /api/admin/settings`).

## Creator app (creator.scutoons.com) — standalone (2026-09-30, owner)

Like the server, the creator is its **own standalone app**: own folder, own build, own deploy to
`creator.scutoons.com` (`.github/workflows/creator.yml`), own login screen (role `creator`). It shares only
`shared/*` packages with the viewer — **no creator code in the viewer**. Draft preview happens inside the creator
(a preview page rendering drafts with `@scutoons/reader`), not via a `?preview=draft` mode in the viewer; the viewer's
account page may link to `creator.scutoons.com` for creators, nothing more.

✅ Done in P1a: `viewer/src/routes/(creator)`, `lib/components/creator`, `lib/creator` moved into `creator/`
(`src/routes`, `src/lib/components`, `src/lib/editor`); validation + paths in `@scutoons/content`, content store in
`@scutoons/api`; the viewer's `?preview=draft` loader mode is removed. See "P1a — result" below.

## P1a — result (2026-09-30)

**Package map**

| Where | What |
|---|---|
| `package.json` (root) | private, npm workspaces `viewer`, `creator`, `admin`, `shared/*` (missing folders are fine for npm); scripts `dev:viewer`, `dev:creator`, `dev:admin`, `check` (all workspaces), `build` (all with a build script); one `package-lock.json` + `.npmrc` (`engine-strict`) at the root |
| `shared/tokens` | `tokens.css` (DESIGN.md Part 1 + `--state-error/warning` + legacy `--primary-color`, `--color-acid-*`), `fonts.css` + `fonts/` (Plex Mono 400/700 + italic, Bangers, OFL texts), `base.css` (html/body/headings/p, fluid root size), `index.css`; check = node script |
| `shared/content` | types `content`, `enums`, `audio`, `visual-novel`, `items`, `membership` (+ `isMembershipExpired`, ex `lib/auth`), `template.ts` (escape/sanitize, `isItemPublic(item, includeDrafts)`, `cdnUrl(base, path)`), `paths.ts`, `validate.ts` (ex creator); `schema/{chapter,season,toon,toon-index,group,catalog}.schema.json` via `npm run schema` (ts-json-schema-generator), check = tsc + schema up to date |
| `shared/reader` | `Slice`, `Dialogs`, `Dialog` (+ the text box reveal effects in `src/effects`), `SliceItems`, actions `sliceAudio` (player injected), `sliceEvents` (VN `run` injected; ex unused `vnEvents`), `getMusicStateForSlice`, `isWithinRect`, type `ActiveTextBox`; check = svelte-check --fail-on-warnings |
| `shared/api` | `createApiClient({ baseUrl, getToken, getDeviceId })`, `ApiError`, `createQrApi(client)`, `createContentApi({ client, staticBase })`, `getDeviceId`, `get/setSessionToken`; check = tsc |
| `shared/ui` | `Button`, `IconButton`, `Field`, `Panel`, `Section`, `Sheet` (native dialog), `Tabs`, `Toggle`, `Badge`, `Menu` + `clickOutside`, `createToaster` + `ToastStack`; check = svelte-check --fail-on-warnings |
| `viewer/` | consumes all five; app wiring in `lib/api.ts` (client + endpoints), `lib/config.ts` (`API_URL`, `CDN_SERVER`, `CREATOR_URL`), `lib/actions/audio/index.ts` (`audio` action + `audioPlayer()` adapter), `components/toon/slice.svelte` (thin wrapper around the reader Slice: CDN URL, locale, samples/triggers, audio attachment) |
| `creator/` | SvelteKit static SPA on :10000, own paraglide (`creator.*`, `login.*`; cookie `SCUTOONS_CREATOR_LOCALE`), own session store + login screen (`@scutoons/ui`), role gate, magic link `/?login=`, draft preview `/<toon>/chapter/<n>/preview?locale=&label=` |
| `.github/workflows/` | `viewer.yml` (was `main.yml`, same FTP secrets, now root `npm ci` + path filters), `creator.yml` (`CREATOR_FTP_*` secrets, `VITE_*` repo vars) |

**Decisions**
- Env: `VITE_API_URL` everywhere (still reads the old `VITE_QR_API`), prod fallback `https://qr.scutoons.com/api`;
  `VITE_CDN_URL` (dev 8001); viewer `VITE_CREATOR_URL`; creator `VITE_VIEWER_URL` (+ optional `VITE_STATIC_API`).
- Tokens are imported by the root layout (`import '@scutoons/tokens'`); the viewer keeps `static/css/styles.css`
  (buttons, inputs, utilities) and `animations.css` as they were. Apps need `server.fs.allow: ['../shared']`.
- `check` compiles paraglide first (`scripts/paraglide.js` with the same `paraglide.config.js` as the vite plugin), so
  it works on a fresh clone / CI.
- Shared components keep the old DOM/class names (`.slice`, `.dialog`) so output stays identical;
  files are kebab-case like the apps. (The full-screen reader onomatopoeia was removed 2026-10-03: onomatopoeia are text boxes.)
- No viewer markup was swapped for `@scutoons/ui` yet: nothing was a visually 1:1 match (e.g. role tags use a 2px
  border, the kit's Badge a hairline); the creator's new login screen and preview use the kit.
- The creator's static fallback files (`toons.json`, `characters.json`, store fallback) come from the viewer origin →
  production needs CORS on `scutoons.com/api/*.json` for `creator.scutoons.com` (or those files via the API later).

## P1b — result (2026-09-30)

Server folder renamed to `server/` on 2026-09-30 (owner go; was `server/`). (Former note:
rename + update the path filter / `SERVER_DIR` in `.github/workflows/server.yml`, the root README and package.json –
the code has no folder-name dependency). Docs: `server/README.md` + `docs/{ARCHITECTURE,API,CONFIGURATION,
DATABASE,DEPLOY}.md`; the admin API contract is ITEMS.md "Admin API".

**Structure**: `public/` (front controller, `router.php` for `php -S`, `.htaccess`, verify.html, assets, generate/ SPA
files) · `src/Http/{Controllers,Middleware}` · `src/Domain/*` (services, `ApiError`, `RequestContext`) ·
`src/Infrastructure/*` (Settings, Database + MigrationRunner, Mail, Signing/KeyStore, Storage/FileStore, Templates,
Clock) · `src/Console` + `bin/console` · `config/` (settings, container, routes, items/editions/toons/permissions JSON) ·
`templates/` (landing, mails) · `migrations/{sqlite,mysql,schema}` · `storage/` (private: batch files, keypair) ·
`tests/Unit` + contract suites · `tools/keys/` (key setup pages, not deployed).

**Decisions**
- Slim 4 + PHP-DI 7 (autowiring) + slim/psr7 + symfony/console; PHPUnit 11; PHPStan level **8** (clean).
- Middleware order: NormalizePath → HostRouting → Cors → ErrorHandler → routing → DeviceId → JsonBody → Session →
  route guards (RequireRole, RateLimit, GenerateToken). CORS outside the error handler, so errors carry CORS headers.
- Contract kept byte-for-byte where clients could notice: JSON flags, `no-store`, CORS quirks (validate `*`, toons and
  keys without CORS, items/editions `*` + public cache), 404 "Unknown endpoint." for wrong methods, body parsed before
  the session check, lazy JSON body. Verified by the suites and by a byte/header diff against the old code.
- Host routing: `QR_HOST` = public pages + `/api/validate/` + `/api/keys/keys.php` + legacy generate/;
  `API_HOST` = `/api/*` only; transition flag `QR_HOST_SERVES_API` (default on); other hosts both. Both subdomains
  point at `<server folder>/public/`; the root `.htaccess` keeps the old document root working (forward into public/).
- Settings: env `QR_<KEY>` → constant (config.local.php / CI-generated config.php) → `config/settings.php`; (SQLite via
  `DB_SQLITE_PATH` removed in CUTOVER 10.30). Batch output `generate/files/` → `storage/files/`, keypair `api/keys/` → `storage/keys/`.
- Migrations moved to `migrations/<driver>/` with unchanged file names (schema_migrations stays valid);
  `migrations/schema/qr.<driver>.sql` for fresh databases. No schema change in this phase.
- The legacy `generate/*.php` URLs run on Slim (`Legacy\GenerateController` + `GenerateToken`) on the same services as
  the admin API; `generate/get-signature.php` and `generate/files/signatures/*` stay public for the verify page.
- Contract suites boot through `tests/support/legacy.php` (the old static calls as facades); `tests/admin-smoke.php`
  and `tests/public-urls.php` (printed URLs, identical to the old server; `--hosts`) are new; `composer test` runs all. (All removed in CUTOVER 10.30; `composer test` = PHPUnit.)
- CI `server.yml`: tests on every push/PR (incl. `platform`), deploy only from `main`; same secret/variable names as
  the old workflow (`FTP_SERVER` var, `FTP_USERNAME`/`FTP_PASSWORD` secrets – not `SERVER_FTP_*`), production
  `ALLOWED_ORIGINS` = scutoons.com, www, creator, admin, shop.

## P2b — result (2026-09-30)

**What moved where**

| From (viewer) | To |
|---|---|
| `routes/(shop)/shop/+page.{svelte,ts}` | `shop/src/routes/+page.{svelte,ts}` (loads `GET /api/products/`) |
| `lib/components/shop/products/product-item.svelte` | `shop/src/lib/components/product-item.svelte` |
| `lib/components/shop/products/shop.ts` (types + price/variant helpers) | `@scutoons/content` `products.ts` (`ProductCatalog`, `Product`, `ProductVariant`, `FeaturedProduct`, `LocalizedText`, `isSoldOut`, `pickVariant`, `variantPrice`, `formatPrice`, `editionPieces`, `localized`) |
| `lib/components/shop/products/product-slides.svelte` (homepage only) | stays in the viewer: `lib/components/home/product-slides.svelte` (not generic → not in `@scutoons/ui`) |
| `static/api/products.json`, `featured-products.json` | server `config/products.json`, `config/featured-products.json` (since 2026-10-04 the catalog tables; the files are a test fixture since 10.30) |
| messages `shop.title/description/variants/remaining` | `shop/messages/{de,en}.json` (the homepage keeps `shop.buy/sold_out/sizes/colors/price/limited_edition_of/credits`) |

**API** (public, like `/api/items/`): `GET /api/products/` and `GET /api/products/featured/` return the stored files
byte-for-byte (`Content-Type: application/json; charset=utf-8`, CORS `*` or the allowed origin, `Cache-Control:
public, max-age=300`, other methods 405) — `Catalog\Products` + `ProductsController`; api-smoke 182 → 187 checks,
`AppTest::testProductsAreTheStoredFilesByteForByte`. Client: `@scutoons/api` `createProductsApi(client)` →
`fetchProducts`, `fetchFeaturedProducts`; apps pass a header-free client (`createApiClient({ baseUrl })`) so the
browser sends a simple GET without a CORS preflight.

**Shop app** `shop/` (:12000, `npm run dev:shop`, deploy `.github/workflows/shop.yml`, secrets `SHOP_FTP_*`): static
SPA, paraglide de/en with strategy url → cookie → preferredLanguage → baseLocale and both locales prefixed (`/de/`,
`/en/`); header = the viewer's logo position/size linking to `VITE_WWW_URL` + a de/en switch (`@scutoons/ui`
Button); `app.css` carries the viewer's `img`/`.btn`/`.disabled` rules the page relied on. Env: `VITE_API_URL`,
`VITE_CDN_URL`, `VITE_WWW_URL`.

**Viewer**: `routes/(shop)/shop/[...rest]/+page.ts` redirects `/shop`, `/de/shop`, `/en/shop`, `/shop/…` (VN
`jumpTo` product links) to `VITE_SHOP_URL/<locale>/` (URL prefix, else the reader's locale); menu "Buy access" /
"Limited Editions" and the membership blocker open the shop URL; the homepage keeps the slides (Bandcamp buy links)
and gains a "Limited Editions" link to the shop; it renders without the slides when the API fails.
`VITE_SHOP_URL` (dev `http://localhost:12000`, prod `https://shop.scutoons.com`; `viewer.yml` passes the variable).

**Verified**: the shop on :12000 against the viewer's shop before the move (probe of box + computed styles of the page
header, grid and four tiles incl. sold-out/badge) — identical at 1280px and 375px; de/en; the redirects of all four
URL forms, menu and "Buy access" on a viewer; homepage slides identical (+ the new link). Checks: viewer/creator/shop/
shared 0 errors; builds OK; `composer test` green.

## P2a — result (2026-09-30)

**Admin app** `admin/` (:11000, `npm run dev:admin`, deploy `.github/workflows/admin.yml`, secrets `ADMIN_FTP_*`, repo
vars `VITE_API_URL`, `VITE_CREATOR_URL`, `VITE_VIEWER_URL`): static SPA like the creator (own login + `admin` role
gate, magic link `/?login=`, paraglide de/en with cookie `SCUTOONS_ADMIN_LOCALE`). Shell = sticky top bar (logo,
command palette trigger, account, language, sign out) + sidebar from `src/lib/navigation.ts` (groups → items with a
one-line description; burger < 768px) + the fixed bottom `ActionBar` on every page. `html` font size is pinned to
16px in the admin (dense tool; the text is 14px), so rem tokens don't grow with the reader's fluid 24px root.

| Route | Parity with `generate/` + new |
|---|---|
| `/` | Stats chart → KPIs (totals + period), daily scans/unlocks (line) and claims/signups (bar), claimed/total per edition |
| `/codes`, `/codes/<code>` | Stats table/detail → URL-driven table (search, edition/type/status filters, sort, pager, bulk revoke/unrevoke, CSV), split-view drawer, detail page (facts, revoke with reason, stats, full timeline) |
| `/codes/new` | Generate → all types, toon/redirect prefill, expiry, edition + template fields, ML-DSA-65 signing, archived SVG + signature, downloads SVG / CoA / printable QR / signature |
| `/codes/<code>` → Certificate | Regenerate → rebuilt from the stored signature (`qr_data`), local signature check, legacy regenerate download names |
| `/batches`, `/batches/<file>` | Batch → history (resume, ZIP, print sheet), new batch (edition, count, start number, `409 batch_complete` → "Create anyway"), resumable pipeline sign → db → svg → pdf → sig with pause/abort, manifest saved after every code |
| `/users`, `/users/<id>` | Users → search + role/opt-in filters, detail (owned codes, signup code, opt-in, sessions), role toggles |
| `/events` | new: activity log with event-type/code/edition/toon/date filters |
| `/catalog/<kind>` | the catalog editor (agents/CATALOG.md part 3): every kind, list + form + history / undo |
| `/content` | new: content store per toon/file/locale (newest + published version, state), links into the creator |

**Parity checks** (browser, same data, legacy `generate/` modules on :9000 vs the admin's): certificate PDF of a
single code (regenerate path) and of a batch code, QR SVG and the batch print sheet are **byte-identical** apart from
jsPDF's `/CreationDate` and `/ID`; `signedData` has the legacy key order (single: … template fields as strings,
`svg: null`; batch: numbers, `receiver: ""`); QR URL `https://qr.scutoons.com?c=<CODE>`, footer "Verify at
https://qr.scutoons.com/verify.html with code: <UPPER CASE>" (since 2026-10-03 the exact code: codes are
case-sensitive, matched exactly – see the RULE section). Admin signatures verify with the verify page's bundled
noble copy and use the published public key.

**Decisions**
- Libraries: `jspdf@3.0.3` (the legacy CDN version) and `@noble/post-quantum@^0.5` as npm deps; datalog
  `qrcode-svg` is not on npm → vendored unchanged as `admin/src/lib/qr/qrcode-svg.js`. Printed URLs are constants
  in `src/lib/qr/config.ts` (never env).
- Batch pipeline fix vs legacy: a code whose `sig` step is still open also redoes its PDF, so certificate and stored
  signature always come from the same (new) signature after a resume.
- Legacy `exportSignature` (base64 of the signature, console only; `btoa` threw on non-Latin-1 text and then skipped
  saving the signature) is dropped.
- New server endpoint `GET /api/admin/content` (`Admin\ContentController` + `Domain\Admin\ContentAdmin`, admin-smoke
  +5 checks; ITEMS.md "Admin API" → Content): the creator's `/content/index` needs the `creator` role and one toon.
- `@scutoons/api`: `createAdminApi`, `client.delete`, `client.fetchRaw`. `@scutoons/ui`: `ActionBar`, `DataTable`,
  `Pager`, `SearchField`, `readList`/`listHref`/`listParams`, `SplitView`, `CommandPalette`, `Skeleton`, `Chart`,
  `RecordInspector` (README entries).
- Command palette in every tool (owner 2026-10-02): admin, creator and embassy share `CommandPalette` +
  `PaletteTrigger` (top bar button: search icon, label, `Ctrl K`; icon only < 768px; `compact` = icon + kbd for the
  crowded creator / embassy bars) + `linkCommands` / `languageCommand` / `signOutCommand` (@scutoons/ui). Creator: all
  toons, universe pages, style guide, every toon's seasons / chapters ("Cat Pope · Chapter 3", numbers from the content
  index, loaded on the first open) / script / cast / credits / settings, the current toon first. Embassy: sections,
  lead views (mine, due, per track), every organisation (`orgs` limit 500, on the first open), "Leads matching …".
- Bottom action bar (owner rule): `ActionBar` on every admin page (filters/search + pager in `start`, primary action
  right-most) and every creator page (editor: back to chapters, slice ‹ n/N ›, problems summary · undo/redo, preview,
  history, style guide · note, Save draft, Publish; toon page: new chapter, season save/publish; preview: locale +
  group, back to the editor; toons: sign out, open the reader). < 1024px: `start` + `secondary` go into "More".

**Open**: `generate/` + `GENERATE_PASSWORD` can be removed once the owner has used the admin in production (P3);
the events type filter is a drop-up checklist in the bar; there is no page-size choice in the bar (URL `page_size`
still works); production needs `https://admin.scutoons.com` in `ALLOWED_ORIGINS` (already in server.yml).

## Emails (2026-10-02, owner: "a place to manage emails")

**One sender:** the API (`server/`). Inventory 2026-10-02: the only mails were the two sign-in mails of
`POST /auth/request/` (`LoginService` → the old `LoginMail` + `Infrastructure\Mail\Mailer`); the qr app, the viewer,
creator, admin, shop, embassy (it only *logs* mails people write – `mail_out` / `mail_in` activities – and opens
`mailto:`) and the scripts send none; the legacy `generate/` SPA sends none. So nothing had to move onto the API –
every future mail (ownership claims etc.) is added to the module below, never sent by an app itself.

| Where | What |
|---|---|
| `server/src/Domain/Mail/MailTemplates.php` | **the registry**: id consts + definition (audience `member` \| `admin`, description, vars with types, sample values, body blocks `p` / `button` / `code` with optional `if`, fine print) |
| `server/templates/mail/texts.php` | **every text**: `[locale][template id][key]` (de + en, `subject` required, `{var}` placeholders), `_layout.signature` |
| `server/templates/mail/layout.html.php` | the one HTML frame (ink on paper, mono, 2px frame, SCUTOONS header, fine print) |
| `MailRenderer` | subject + plain text + HTML from the same blocks (pure; also the admin preview) |
| `Mailer` | **the send API**: `send(string $templateId, string $to, string $locale, array $vars = []): int` (log id) – throws `MailFailed` (logged as failed) or `InvalidArgumentException` (unknown template / missing var = a bug); `sendToAdmins(string $templateId, array $vars = []): array{sent, failed}` (every admin recipient, in `MAIL_ADMIN_LOCALE`) |
| `MailLog` | table `mail_log` (migration `2026-10-02-mail-log`, sqlite + mysql): id, template, recipient, locale, status `sent` \| `failed`, transport, error, created_at – never subject / text / vars (a login mail has a one-time code). Kept `MAIL_LOG_DAYS` (180, pruned on write). Missing table → mails still go out, unlogged |
| `MailRecipients` | admin notification addresses: setting `mail_admin_recipients` in the `settings` table (like `featured_products`), default `MAIL_ADMIN_TO` (comma separated, empty) |
| `server/src/Infrastructure/Mail/Transport.php` (+ `Smtp.php`) | delivery only (`MAIL_TRANSPORT` smtp \| log \| mail); the `log` format of `server/logs/mail.log` is unchanged (contract suites, `scripts/dev/test-user.mjs`, `check-next-bar.mjs` read one-time codes there) |
| admin API | `GET /admin/emails/templates`, `GET /admin/emails/templates/{id}/preview?locale=`, `GET /admin/emails/log` (list), `GET/PUT /admin/emails/recipients` (ITEMS.md "Admin API" → Emails) |
| admin app | **Emails** (nav group System): `/emails` = sent-mail log (URL-driven: `q`, `template`, `status`, `recipient`, `from`, `to`, sort, pager; a recipient → filter / find the account), `/emails/templates` = every template rendered de/en with its sample vars (HTML in a sandboxed iframe + plain text), `?locale=`, and the admin recipients editor |

**Registered:** `login_code` (link + 6-digit code; vars `link`, `otp`, `minutes`, `code?`, `signup`), `signup_without_code`
(no account, no usable signup code; vars `url`, `code?`), `signup_admin` (sendToAdmins on every new account in
`LoginService::confirm`: the address, the signup code and its type, a "claim" note for a dispute signup, link
`ADMIN_URL/users/<id>`; who gets it = the admin recipients on the Emails page). Subjects, texts and HTML are byte-identical to before the
refactor (both mails, de/en, with and without code / signup).

**Adding a mail (e.g. the ownership emails):** 1. a const + definition in `MailTemplates` (audience, description, vars,
sample, blocks, fineprint); 2. its keys in **both** locales of `texts.php`; 3. call `Mailer::send(...)` (to a person) or
`Mailer::sendToAdmins(...)` (audience `admin`); catch `MailFailed` where the request must still succeed; 4. a render test
in `server/tests/Unit/Domain/MailTest.php` (its "every template renders in every locale" test and the registry list
assertion cover the new id). The admin page lists it without a frontend change. **Registered 2026-10-02** (agents/ITEMS.md
"Ownership, claims, gifting"): `ownership_claim_received` (claimant), `ownership_claim_admin` (sendToAdmins, link
`ADMIN_URL/claims/<id>`), `ownership_claim_rejected` (claimant), `ownership_transferred_from` / `ownership_transferred_to`
(previous / new owner), `gift_received` (recipient). Still planned: `ownership_transfer_offer` (the owner sends a code to
another person's email). A new block type = a case in `MailRenderer` + the layout.

**Settings:** `MAIL_TRANSPORT`, `MAIL_FROM`, `MAIL_FROM_NAME`, `MAIL_LOG_PATH`, `SMTP_*` (unchanged); new
`MAIL_ADMIN_TO`, `MAIL_ADMIN_LOCALE` (de), `MAIL_LOG_DAYS` (180). The mail log table (`2026-10-02-mail-log`) is applied on production; the admin recipients are set on the Emails page.

## Cross-app auth

~~Same endpoints, token per origin (localStorage `scutoons_session`) for now.~~ Single sign-on since branch `sso`
(2026-10-01) – see "SSO — result" below.

**Framed apps (the creator's dock Reader, BACKLOG 10.49):** the creator frames the viewer's draft preview; the frame
signs in with the same cookie (one site: creator. and the viewer are both scutoons.com, so its API calls are same-site
and carry the `SameSite=Lax` cookie). No session token is ever passed – not in a URL (history, logs, referrers), not
by postMessage. The two windows exchange only "ready" / "show group / slice", each origin-checked and source-checked on
both sides, the viewer only in its draft preview (agents/DOCKING.md "The Reader panel").
Only the creator may frame the viewer (owner 2026-10-07): `viewer/static/.htaccess` sends `Content-Security-Policy:
frame-ancestors 'self' https://creator.scutoons.com` (Apache mod_headers; the dev server doesn't, localhost:10000 frames
freely). Another embedder = one more origin there.

## SSO — result (branch `sso`, 2026-10-01)

**Owner:** "once a member is logged in they will not need to log in again on any of the apps, depending on whether their
roles allow it" · the notice says the account does not allow this, "switch account" = logout + that app's login · one
shared component.

**Mechanism:** the API sets an HttpOnly session cookie `scutoons_sid` on `/auth/verify/` (requests from an
`ALLOWED_ORIGINS` app only): `Path=/api; HttpOnly; SameSite=Lax; Secure; Domain=scutoons.com` in production
(`SESSION_COOKIE_DOMAIN`, only when the API host is under it), host-only on `localhost` (cookies ignore ports, so all dev
apps share it); `Max-Age` = `SESSION_TTL_DAYS`, renewed by `GET /auth/session/`. Same token as the Bearer token (one
`sessions` row); the API accepts the cookie **or** Bearer (Bearer wins), so old builds keep working. Apps call the API
with `credentials: 'include'`; CORS answers allowed origins with `Access-Control-Allow-Credentials: true` (never `*`).
CSRF: a cookie on POST/PUT/DELETE/PATCH counts only with an `Origin` in `ALLOWED_ORIGINS` (`403 csrf_rejected` where a
user is needed; anonymous elsewhere, so the printed `/validate/` is untouched). Logout clears the cookie and ends the
shared session = signed out in every app (`{ all: true }` = every device). Migration: an app that still has a token in
localStorage moves it into the cookie once (`POST /auth/session/` with Bearer), checks it, deletes the copy. If the
browser refuses the cookie, the app keeps the token in localStorage as before (works, just per origin).
Why not a central login page / OAuth-style redirects: every app already has the same API and same site
(`*.scutoons.com`); a first-party cookie on the API is the smallest change, needs no new host, no token passing in URLs.
Details + security notes: server/docs/API.md "Single sign-on".

**Shared component:** `shared/auth` = `@scutoons/auth` (README): `createAuthSession` (the session of every app),
`LoginForm` (email → 6-digit code, resend, change email, errors, signup opt-in), `AuthGate` (loading → sign-in screen →
"Dein Konto (x@y) hat keinen Zugang zu dieser App." + "Mit anderem Konto anmelden" → the app), `SignedInAs` (+ sign
out), de/en texts (`authLabels`), the role matrix `APP_ROLES`. Used by viewer (account page, claim sheet), creator,
admin, embassy (gate) and shop (signed-in display); the three app copies of `login-form.svelte` + session store and the
viewer's login form are deleted. Apps keep their own routing (`/account?return=`, `?login=` magic links via
`takeMagicToken`). Dependency direction: `tokens ← ui ← auth`, `api ← auth`.

| App | Who gets in (`APP_ROLES`) |
|---|---|
| viewer | anyone (member features need membership, as before) |
| shop | anyone (shows "signed in as" + sign out only) |
| creator | `creator` or `admin` – admin **for now** (owner 2026-10-01). One rule on each side: server `Roles::CONTENT_EDITORS` (`/api/entities/*`, library `?draft=1`), client `APP_ROLES.creator`; narrowing later = change those two lists |
| admin | `admin` |
| embassy | `liaison`, `ambassador`, `admin` |

**qr.scutoons.com:** needs no session itself ("Add to my account" → `<viewer>/account?code=`, the viewer has the shared
session). Its `/api/*` proxy forwards only the `scutoons_sid` cookie, so single sign-on also works while
`VITE_API_URL` still points at `qr.scutoons.com/api`.

**Cutover notes (production):**
- Nothing to configure: `SESSION_COOKIE_DOMAIN` defaults to `scutoons.com`; `ALLOWED_ORIGINS` in `deploy/targets.json`
  already lists every app (scutoons.com, www, creator, admin, shop, embassy) – it is now also the CSRF list, so a new
  app origin must be added there before its sign-in works.
- HTTPS on every subdomain is required (`Secure` cookie): api., qr., and the apps (cutover checklist steps 5–7).
- No DB migration (the `sessions` table is unchanged). **Deploy order matters: api first**, then qr, then the
  frontends (as "Deploy all" does). The new frontends send `credentials: 'include'`, and a browser only reads such an
  answer when it carries `Access-Control-Allow-Credentials: true` – the old api does not send it, so new frontends in
  front of the old api fail on every signed-in request. Old frontends in front of the new api are fine.
- Users signed in before: their apps move the stored token into the cookie on the next visit (no new sign-in). Two apps
  with two different stored tokens: the first visit wins the cookie, the other app's token is dropped on its next visit
  (the cookie session is used).
- Safari/iOS: see API.md "Single sign-on" (first-party cookie; 7-day cap only if api. and the apps get different IPs,
  e.g. apps on a CDN; installed PWAs have their own cookie jar).

**Merge / rollback (owner):** `sso` is based on `platform` a83b1cc and additive: new endpoints (`/auth/session/`),
the cookie next to Bearer, one new package. Merge `sso` into `platform` after the 2026-10-04 release (`git merge sso`;
expected conflicts only where the apps' layouts changed meanwhile). Rollback = redeploy the previous api and/or
frontends (Actions → the app's workflow on the earlier commit); nothing to undo in the data (no migration, sessions are
the same rows, a cookie left in a browser is ignored by the old API):
- roll back the frontends first (or together with the api): the old apps use localStorage tokens; the ones the new apps
  had moved into the cookie are gone, so each app asks for one sign-in again. The new api serves old apps unchanged.
- never the api alone: new frontends need its CORS credentials header (see the deploy order above).
Check after merging: `npm run check:sso` (+ `composer test`).

## Deploy

> Superseded by "Hosting layout & deployment" and "P3 — result" (deploy/targets.json, one reusable workflow,
> `deploy-all.yml`, one FTP secret set). Kept for history:

Root `.github/workflows/`: `viewer.yml`, `creator.yml`, `admin.yml` (path filters incl. `shared/**`), `server.yml`
(`server/**`: PHPUnit + PHPStan, composer install --no-dev, generated config.php, FTP; build
tokens CSS copy not needed yet – the landing page inlines its tokens). Deploy only from `main`.
New FTP targets / docroots / DNS for creator.scutoons.com and admin.scutoons.com are set up by the owner.
The old `qr.scutoons.com` GitHub repo is archived after cutover; its workflow is inert inside `server/.github`.

## Phases

| Phase | What | Who | Depends on |
|---|---|---|---|
| P0 | `platform` branch; qr repo imported with history as `server/` (git subtree) | coordinator | – ✅ |
| P1a | Monorepo foundation: root workspaces, `shared/{tokens,content,reader,api,ui}` extracted from viewer, viewer consumes them; `creator/` app split out of viewer | agent "frontend-platform" | P0 ✅ |
| P1b | Server refactor to Slim 4 structure, contract suites green, `/api/admin/*` with role auth | agent "server-refactor" | P0 ✅ |
| P2 | `admin/` app on shared ui + admin API; parity with `generate/` | agent "admin" | P1a ui/tokens, P1b admin API |
| P3 | CI workflows, root README, local dev (one command), cutover checklist, archive old repo | coordinator/agent | P1–P2 ✅ (see "P3 — result") |

## Rules while `anger` is being released

- Release fixes land on `anger` (viewer etc.) and, for the server, on the old repo `C:\Development\qr.scutoons.com`
  branch `anger`. `platform` pulls them in: `git merge anger` and
  `git subtree pull --prefix=server C:\Development\qr.scutoons.com anger`.
- After cutover the old repo is read-only.

## Local dev (history: the platform branch era – today: RULES.md "Dev setup")

- Server: `cd server && composer serve` (:9000),
  local-only files (config.local.php, db/prod-seed.sql, storage/files, storage/keys/keypair.json) — gitignored.
  Database: MariaDB in Docker only (`npm run dev:db`, CUTOVER 10.30); `composer test` = PHPUnit (`npm run dev:db test`).
- CDN: `scripts/dev/media-serve.mjs` serving `media/` (gitignored), started by `npm run dev`. All ports: "Local dev ports" (`shared/dev-ports.json`).

## Admin UI patterns (from akin)

Source: `C:\Developmentkinkin-core\server` (SvelteKit 2, Svelte 5, the owner's own project — patterns are
reimplemented in SCUTOONS style, no copying of akin's look).

**Take over (restyled):**
- **Shell:** sticky top bar + left sidebar + content column; nested scope sidebar per entity (e.g. per toon, per
  edition). < 768px: sidebar → burger overlay.
- **Navigation as data** (`lib/data/navigation.ts` in akin): groups → items `{ title, description, href, icon }`,
  active via `aria-current="page"`, disabled via `aria-disabled`. The one-line description per item fits SCUTOONS'
  literal copy.
- **URL mirrors the hierarchy**, every level a page: `/codes/[code]`, `/editions/[id]`, `/users/[id]`,
  `/content/[toon]/[locale]/[path]`.
- **URL-driven lists:** `?page=&page_size=&sort=&q=&filter…` read in `load`; services return
  `{ items, pagination:{ total, page, pageSize } }`; server clamps page size.
- **Entity detail page:** small uppercase type label, title, meta; content in `<section>`s with header + hint.
- **Forms:** `.field` / label / `.field-hint` / `.has-error` / `.field-error`; sticky bottom action bar.
- **Toasts:** runes store `addToast({ message, type, timeout })` + fixed bottom-right stack.
- **Badges** for status (claimed, revoked, draft, published); **Menu** dropdown with click-outside; **Toggle**.
- **No raw record sections** on admin detail pages (owner 2026-10-04: uninformative). `RecordInspector` was removed
  from `@scutoons/ui`.
- **CSS split by concern:** color, typography, forms, lists, entities, notifications, columns — values from
  `@scutoons/tokens`.

**Bottom action bar (owner, 2026-09-30 — mandatory for admin and creator):** every page has a **fixed footer bar**
that mirrors the top bar — same height, `paper` background, `border-width-regular` `ink` rule on top, full width, always
visible (no fade, no auto-hide, unlike akin). It holds the page's **pagination, filters/search and primary actions**
(primary action right-most, e.g. "Save draft · Publish" in the creator, "Generate batch" / "Revoke" in admin). Page
content gets bottom padding so nothing hides behind it. On narrow screens the bar keeps the primary action visible and
moves secondary items into a "More" menu. Implemented once as `ActionBar` in `@scutoons/ui` (slots via snippets:
`start` = pagination/filters, `end` = actions) and used by both apps; no page-level action buttons elsewhere except
inline row actions.

**Build new (akin has none):** real data table (sortable headers, bulk select + actions, server-side search/filter),
split view / detail drawer for fast triage, command palette (`/` or Ctrl+K) + keyboard shortcuts, loading skeletons,
stats + charts (plain SVG, DESIGN.md rules), inline edit for small fields.

**Drop:** rounded radii, gradients, hover-scale animation, akin's palette and serif/sans fonts, legacy
`createEventDispatcher` / `<slot>` patterns (Svelte 5 snippets + callback props only).

## RULE: printed / public URLs never change (2026-09-30, owner)

Physical items in the real world carry these links (QR stickers, certificates, prints). Until a deliberate, tested
migration via the server's code mappings exists, **these URLs, paths and parameter names must keep working exactly as
they are** — in every refactor, on every host/docroot change, in both old and new server layouts:

| Where | URL / parameter | Used by |
|---|---|---|
| qr.scutoons.com | `https://qr.scutoons.com?c=<CODE>` and `https://qr.scutoons.com/?c=<CODE>` (param **`c`**) | every printed QR code (`QR_BASE_URL = QR_DOMAIN + '?c='`) |
| qr.scutoons.com | `/?c=<CODE>&skip=1` (param **`skip`**) | legacy "continue without claiming" links |
| qr.scutoons.com | `/verify.html` (+ its code input / query) | printed certificates: "Verify at https://qr.scutoons.com/verify.html with code: <CODE>" — the code is printed in **UPPER CASE** |
| qr.scutoons.com | `/api/validate/` (GET `?code=` / `?c=`, POST `{code}` / `{c}`) and its response shape | old viewer builds, external links |
| qr.scutoons.com | `/api/keys/keys.php` (public key) | verify page, third-party signature checks |
| scutoons.com / www.scutoons.com | `/<toonId>` e.g. `/catpope` (toon entry + chapter overview) | `redirect_to` of all codes (`https://scutoons.com/catpope`, `https://www.scutoons.com/catpope`) |
| scutoons.com | `/<toonId>/chapter/<n>` (+ `?label=`) | chapter links – THE address of a chapter, forever (printed QR codes) |
| scutoons.com | `/<toonId>/season/<s>/chapter/<k>`, `/<toonId>/season/<s>` (BACKLOG 10.41) | redirect to `/<toonId>/chapter/<n>` (position k of season s, the prologue not counted) / the toon page at season s (`#season-s`); unknown → not found |
| scutoons.com | `/<toonId>/credits`, `/<toonId>/download` | links in prints / socials |
| scutoons.com | query param **`code`** on any viewer page (`?code=<CODE>`), plus `&inventory=open` | redirects from the QR landing page |
| scutoons.com | `/de/...`, `/en/...` locale prefixes | shared links |

- New URLs may be **added**; existing ones may only change through server-side code mappings (redirects per code),
  with a migration plan, tests against real printed codes and a rollback. Not now.
- Refactors must include a test that hits every URL above (status + target) before and after.
- Known issue to handle carefully (not changed yet): certificates print the code in upper case, but codes are
  case-sensitive nanoids — verify must accept the printed form (exact match first, then a case-insensitive match only
  when it is unique). Test with real certificate codes before changing anything.
  **Done for the verify page 2026-10-02:** `/verify.html` checks through `GET /api/certificates/<CODE>` (exactly that
  rule; tested with all signed dev-DB codes in UPPER CASE); `/verify.html?code=` keeps working (checks right away).
  The landing page `?c=` is unchanged. Details: agents/ITEMS.md "Certificates".
  **Superseded 2026-10-03 (owner: "make it case sensitive … there are never only-capital codes … stick to one rule"):**
  codes are case-sensitive and matched exactly everywhere, also by `/api/certificates/` and the verify page – the
  printed UPPER CASE form of older certificates is no longer accepted when typed (their QR and the signature file
  still verify); new certificates print the exact code. Migration `2026-10-03-code-case` (server/docs/DATABASE.md).

## Local dev ports (2026-09-30, owner — final)

**Single source of truth: `shared/dev-ports.json`** (owner 2026-10-01: "in code, no special place"). Every consumer
reads it: the apps' `vite.config.ts` (`server`/`preview` port + `strictPort`) and their `src/lib/config.ts` dev
fallbacks, `scripts/dev/up.mjs` (`npm run dev`), `server/bin/dev-serve.mjs` (`composer serve`, `npm run dev:api`),
`scripts/dev/php-serve.php` (`qr` `composer serve`, `npm run dev:qr`, server `composer serve:php`), `scripts/dev/media-serve.mjs`
(`npm run dev`), `qr/tests/public-urls.php`'s defaults (`qr composer test`), the Node check scripts (`scripts/dev/ports.mjs`),
`.claude/launch.json` (local, generated: `npm run dev:launch-json`) and the API's local CORS: on a dev server
(`php -S`, never production) `Settings::allowedOrigins()` allows `http://localhost:<port>` + `http://127.0.0.1:<port>`
for every port in the file, so a new app needs no `config.local.php` entry. `npm run check:ports` (part of the root
`npm run check`) fails when a vite config, the dev start-up code or package/composer scripts hard-code a port, when an
`.env.example` / `config.local.php.example` URL doesn't match the file, or when this table drifts from it.
New service: add it to the file, then to `up.mjs` + `launch-json.mjs` (both refuse an unknown name) and this table.
The docker services (`docker/compose.yml`: mariadb, adminer) take the port from the file through `npm run dev:db` /
`up.mjs`; the compose file's defaults and `server/phpunit.xml.dist` must match it (`check:ports` checks).

| App | Port | Start |
|---|---|---|
| viewer (main, scutoons.com) | **8000** | `npm run dev:viewer` |
| cdn (static files, `media/`) | **8001** | `npm run dev` (`scripts/dev/media-serve.mjs`) |
| API (server/, api.scutoons.com) | **9000** | `npm run dev:api` or `cd server && composer serve` |
| qr (qr/, qr.scutoons.com) | **9001** | `npm run dev:qr` or `cd qr && composer serve` (calls the API on :9000) |
| creator (creator.scutoons.com) | **10000** | `npm run dev:creator` |
| admin (admin.scutoons.com) | **11000** | `npm run dev:admin` |
| shop (shop.scutoons.com) | **12000** | `npm run dev:shop` |
| embassy (embassy.scutoons.com) | **13000** | `npm run dev:embassy` (agents/EMBASSY.md) |
| studio (the storyboard studio, a local workflow tool) | **14000** | `npm run dev:studio` (workflows/storyboard-studio, agents/research/STORYBOARD.md); its handheld phone page over Wi-Fi: HTTPS on the LAN at the studio's port + 443 (14443; `STUDIO_LAN=0` = off) |
| mariadb (Docker, the production engine; the only local database) | **13306** | `npm run dev:db` (server/docs/DATABASE.md "Local MariaDB"); `npm run dev` starts it too |
| adminer (Docker, web UI of the MariaDB; optional) | **13380** | `npm run dev:db adminer` |

`ALLOWED_ORIGINS` (dev): nothing to list for the ports above (automatic on the dev server, see above); a local
`config.local.php` only lists extra origins (the old 5173/5174/5175/4173 until nothing uses them). Production lists its
origins explicitly (deploy/targets.json → config.php; never localhost). The earlier ports (viewer 5173, creator 5180,
admin 5190, CDN 8000) and the transition server on :9100 are superseded.

## Domain architecture (2026-09-30, owner — target)

One app per subdomain; on all-inkl every subdomain points to its own build/runtime folder (set in the all-inkl
admin, e.g. `/creator.scutoons.com/`), so each app deploys independently by FTP into its folder.

| Domain | App (repo folder) | Purpose | Runtime |
|---|---|---|---|
| `www.scutoons.com` / `scutoons.com` | `viewer/` | landing page, events, marketing pages **and the reader** (`/<toon>`, `/<toon>/chapter/<n>`, credits, download, account) — printed QR codes redirect here, so the reader stays on this domain | static build |
| `qr.scutoons.com` | `server/` (public pages only) | the low-level public frontend: QR landing `?c=`, verify page, legacy `/api/validate/`, `/api/keys/keys.php`, the verify page's `/api/certificates/*` (2026-10-02) — **nothing else** | PHP (same server code, host-based routing) |
| `api.scutoons.com` | `server/` | the backend for all apps: accounts, access, items, inventory, content, admin API, data | PHP |
| `admin.scutoons.com` | `admin/` | administration: QR generation/batches (port of `generate/` to Svelte), codes, revoke, stats, users & roles, content, catalog, data migrations | static build |
| `creator.scutoons.com` | `creator/` | the creator editor (role `creator`) | static build |
| `shop.scutoons.com` | `shop/` | the shop (moves out of the viewer; `scutoons.com/shop` keeps working as a redirect) | static build |
| `cdn.scutoons.com` | – (media: asset manager, local `media/`) | all static media — see CDN below | all-inkl origin (+ bunny.net) |

**Server host routing:** one codebase (`server/`), deployed once; `qr.` and `api.` both point to `server/public/`.
The Slim app routes by host:
- `qr.scutoons.com` → landing, verify, `/api/validate/`, `/api/keys/keys.php`, `/api/certificates/*` (2026-10-02) (+ assets). Anything else → 404.
- `api.scutoons.com` → `/api/*` (the full API). No landing page.
- **Transition:** until every frontend uses `api.scutoons.com`, the full `/api/*` also stays reachable on
  `qr.scutoons.com` (the released viewer calls `https://qr.scutoons.com/api/...`). Remove only after all apps are
  switched and the stats show no more traffic there. Printed URLs (RULE above) are never removed.
- Dev: one server on :9000 answers both roles (host check relaxed for localhost); frontends use `VITE_API_URL`.

**Shop app:** `shop/` (dev port 12000) with `@scutoons/ui` + tokens; products data from the API (`/api/products/`);
viewer route `/shop` is a redirect to `shop.scutoons.com` (RULE: `/shop` keeps working). ✅ P2b, see "P2b — result".

**Deployment:** one workflow per app (`viewer.yml`, `server.yml`, `admin.yml`, `creator.yml`, `shop.yml`), path
filters incl. `shared/**`, deploy only from `main`, each to its own FTP folder (~~secrets `<APP>_FTP_*`~~ – one FTP
secret set, see "Hosting layout & deployment"; the server/qr host routing above is superseded by the qr app). The owner
creates the subdomains/folders in all-inkl (`api.`, `admin.`, `shop.` are new; `creator.` exists without SSL yet —
enable SSL).

## Data (later phase — needs its own migration plan)

Today some data is static JSON shipped with the viewer (`viewer/static/api/`: toons, events, products,
featured-products, characters, collectables, locations, routes, credits, season/chapter configs) and some comes from
the server (content store, items, editions, codes). Target: **everything served by `api.scutoons.com`**, stored in the
DB. Rules:
- The **toon configuration format stays** — self-contained, interchangeable JSON (import/export per toon); the API
  returns it in that format so the viewer keeps working. The DB is storage, the format is the contract
  (`@scutoons/content` schema).
- Not everything moves at once: first catalog data (products, events), then toon/season/chapter configs (the content
  store already holds them), then items/editions JSON → tables.
- Each step: export → import script → compare (API output == old static file) → switch the frontend → keep the static
  fallback for one release → remove.
- Starts after P1–P3 (refactor + apps) are done.
- ✅ First step done in P2b (2026-09-30): `products.json` + `featured-products.json` moved to the server's `config/`,
  served as stored by `GET /api/products/` and `/api/products/featured/` (byte-for-byte = the old static files,
  checked by api-smoke + a unit test); shop and viewer homepage load them via `@scutoons/api`; the static copies in
  `viewer/static/api/` are deleted (no fallback release: the P2b brief asked for the removal in the same step). Since
  2026-10-04 DB tables (CATALOG.md).
- Toon languages (2026-09-29): the toon index entry (`ToonIndex`, `toons.json`) carries `sourceLanguage` + `languages`
  (source first; optional, defaults via `toonLanguages()`: `languages` → `title` keys → the old `i18n` keys → `de`). The creator edits it as
  the language-neutral content store file `<toon>/_/index.json` (locale `_`), falling back to the static `toons.json`;
  when `toons.json` moves to the API, that store file is its per-toon source.
- Content languages (2026-10-02): `Language` (ISO 639-1) = de, en, jp, **ko** (was the unused country code `kr`), es –
  the enum is the list (schemas, `isLanguage`, the toon page's "Add a translation"); the server takes any 2-letter locale
  and falls back source → other languages (`Context::pick`). The viewer's UI languages stay de/en (paraglide): it asks the
  API in de/en, so a `ko` translation exists for the creator only until the viewer gets a content-language picker. The
  creator works for N languages (DESIGN.md "Creator: content language (i18n) + translate mode"); the sandbox toon has a
  partial `ko` translation (toon / season / chapter titles + descriptions, the dialogue still the German copy) to test it.

### Players, albums, related items (2026-10-02)

- **Albums** live in the item catalog (DB since 2026-10-04, admin → Catalog; `GET /api/items/`), not in the content
  store: they exist through the media items that carry them (owner-edited config like products / editions); agents/ITEMS.md
  "Players, albums, related items" has the model, the access flags (`download` / `listen` = everyone | members | owners)
  and why.
- **`related_items`** (catalog item ids, ordered) on `season:<toon>:<n>` and `chapter:<toon>:<n>`: language-neutral,
  stored once (the language-neutral documents since 2026-10-06, DATA.md §2). The library passes them
  through unchanged.

### Credits (2026-09-29)

Language-neutral content store files (locale `_`), types/validation/schemas in `@scutoons/content` `credits.ts`
(`schema/{credits,people,roles}.schema.json`):

| Store path | Type | What |
|---|---|---|
| `scutoons/_/people.json` | `PeopleRegistry` | every person, **global** (same people on several toons): `{ id, name, sort_name?, aliases?, links?, image? (CDN path), bio? {locale}, merged_into? }` |
| `scutoons/_/roles.json` | `RoleVocabulary` | shared roles `{ id, category (story/art/music/performance/production/translation/other), label {locale} }` |
| `<toon>/_/credits.json` | `ToonCredits` | `{ toon, roles? (custom roles of the toon), sections[] }`; section kinds `toon` (`lead` = "Created by" + `entries`), `season` (number), `chapter` (number), `song` (title {locale}), `performance` (title, venue?, date?), `thanks`; entries `{ person, roles: [{ role, detail? {locale}, language? }] }` |

- `scutoons` is a **pseudo-toon** of the store (`CREDITS_SCOPE`) for platform-wide files – never use it as a toon id.
- Viewer `/<toon>/credits` (printed URL, unchanged): store (published) → `resolveCredits()` in the reader's locale →
  `CreditsView` (@scutoons/reader, also the creator's preview); fallback while a toon has no `credits.json`: the old
  static `<toon>/<locale>/credits.json` (current locale, then `de`).
- Production has them since the Oct 3 content seed; the seed files and `migrate-credits.ts` were deleted in CUTOVER 10.30
  (local dev = production's export, `npm run dev:db -- seed`).
- `Season.credits` is deprecated (creator shows it read-only); the old per-locale `credits.json` files stay as fallback
  until production has the store files, then they can go.

### Characters (2026-09-30)

Language-neutral content store files (locale `_`), types / validation / schemas / fallback rule in `@scutoons/content`
`characters.ts` (`schema/{characters,moods,asset-requests,cast}.schema.json`, unit tests `test/characters.test.ts`):

| Store path | Type | What |
|---|---|---|
| `scutoons/_/characters.json` | `CharacterRegistry` | every character, **global**: Character → Persona → Look → Mood × View. Character: `id`, `name`/`description` {locale}, `color`, `own_toon?`, `voiced_by?` (people.json), `tags?`, legacy `image`, `artwork?`, `personas` [{ id (`regular` always + e.g. `human`, `cat`, `mountaintiger`, `biker`), label, introduced?, description?, color?, `looks` [{ id, label, default (exactly one), images: mood → view → CDN path, apose?: { path?, slots?: [{ id, slot, label? }] } (the reader's avatar body in this look, ITEMS.md "Reader avatar bodies") }], transitions? [{ from, to, image, label? }] }] |
| `scutoons/_/moods.json` | `MoodVocabulary` | open-ended mood vocabulary `{ id, label, standard?, order? }` (17 standard + dizzy, tripping); only `neutral` is required |
| `scutoons/_/asset-requests.json` | `AssetRequests` | wanted images `{ id, character, persona, look, mood, view, path (convention), note?, requested_by, requested_at, status open/done }` |
| `<toon>/_/cast.json` | `ToonCast` | `{ seasons: { "<n>": [{ character, personas, role? }] } }` – casting per season, in order |

- Terms: **Persona** = another identity/role (changes who they are), **Look** = appearance right now (could come off in
  the next panel), **Mood** = expression, **View** = framing (data field `variant`; UI says View/Ansicht). Only the
  `portrait` is authored; `avatar` = generated 192px downscale, `full` unused.
- Former characters are Cat Pope personas: plf → `human`, cat → `cat`, mountain-tiger → `mountaintiger`
  (`LEGACY_CHARACTER_IDS`: old refs still resolve).
- Paths are stored explicitly (CDN root). Convention for new files: `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.webp` (delivered as PNG under the same name; the creator's asset import writes the WebP (`toWebp`, `@scutoons/media`), the 192 px avatar comes from the owner's Luma generation workflows (repo `cdn/tools/` removed 2026-10-04), the registry points to the WebP; transitions likewise `.webp`),
  A-pose (per look) `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_apose.webp`.
- **Transitions are named after their states (owner 2026-09-30):**
  `characters/<character>[-<persona>]/<character>[-<persona>]_transition-<from>-to-<to>.png` (look ids, e.g.
  `yellowmouse_transition-helmet-to-default.png`, `catpope-biker_transition-helmet-to-default.png`). No other names
  (the old `yellowmouse_keyvisual-helmet-off.png` was renamed); `conventionTransitionPath()`; the migration script picks
  every such file up as `persona.transitions`. An asset request for a transition has `view: "transition"`
  (`TRANSITION_VIEW`, `look` = from, `path` = the file).
- Fallback (`resolveExpression`, used by the viewer VN and the creator): per step the view, then the same mood's
  portrait; steps persona+look+mood → persona+look+neutral → persona default look+mood → +neutral → regular default
  look+mood → +neutral → legacy `image` → **missing-image substitute** `characters/_missing/missing_<view>.png`
  (avatar 192², portrait 600², full 600×1200, apose 768×1152) – also
  when the resolved file is known to be missing (`options.isMissing`: viewer VN = an `<img>` error, creator = its CDN
  check). The A-pose never falls back to expressions (only to its substitute). The creator still flags the real file ✗.
- Viewer: the toon layout loads `fetchCharacters()` (@scutoons/api: registry + the toon's cast of every season from the
  store, published, `resolveCast()` → the legacy `characters.json` shape + `personas`) → static
  `<toon>/<locale>/characters.json` → `[]`. VN refs `{ character, persona, look?, mood?, variant? }` (old form + old ids
  accepted); scene key = character + persona; a look change with a transition shows the transition image for 800 ms.
- Creator: `/characters` (list with persona chips, detail: personas → looks → mood grid + the look's A-pose, transitions, artwork,
  wanted images; tabs Moods and Wanted images; glossary), `/<toon>/cast` (per season), the visual picker in every VN
  character field (picking someone outside the season cast adds them, with undo).
- `avatarBodies(registry)` → `[{ character, look, label, path, slots, default }]` for the reader's avatar picker (follow-up).
- Removed on the owner's request (keep it small): shared groups / affiliations (`groups.json`), the cast group
  (`lead/support/…`, nothing read it), the `mannequin` field. A local store version of `scutoons/_/groups.json` may
  exist – nothing reads it.
- Production has them since the Oct 3 content seed; the seed files and `migrate-characters.ts` were deleted in CUTOVER
  10.30 (local dev = production's export). The
  `characters/` files must be on the CDN first (bunny; missing ones via admin → Assets), or Cat Pope's avatar in the reader
  breaks. (The old chapter 6 VN test bed with the only character refs, `chapter6/scene-200.json`, was removed 2026-09-30;
  production never had it in the store – nothing to migrate there.)

## CDN (careful — owner's live setup)

`cdn.scutoons.com` is served from the all-inkl server and/or bunny.net (owner has set up bunny.net; confirm the exact
current setup before touching anything). Long-term: all static content global via bunny.net (media + app builds).
Rules now:
- **Do not change DNS, pull zones, paths or the FTP sync target.** Asset paths in configs are relative to the CDN base
  and must keep working.
- One config value per app for the CDN base (`VITE_CDN_URL`, server `CDN_URL`) — switching origin/pull zone later is a
  config change, not a code change.
- New uploads get content-hashed or versioned filenames where they may change (cache-safe for a global CDN).
- Media conventions (2026-09-30, PERFORMANCE.md P1–P3): slices `<toon>/images/webtoon/<sXcY>/<sXcY>-slice-N.webp` (1x, 390 px
  wide, standard 390 × 625, every slice carries width/height); character images WebP next to the delivered PNGs; music
  `audio/music/<name>.128.mp3` (128 kbps, streamed); slice videos H.264 MP4 ≤ 3 MB with the size in the name
  (`videos/s1c1-key-visual.900w.mp4`). A changed file gets a new name – the old one stays on the CDN for older builds.
- A separate CDN plan (bunny pull zone ↔ all-inkl origin, cache purge on sync, app builds on CDN) is written with the
  owner before any change.
- **Asset manager (2026-10-02, agents/ASSETS.md):** the in-between layer – a registry of assets / versions / renditions /
  uses over the storage drivers (local `media/`, bunny.net Storage + pull zone purge / statistics); content keeps the
  CDN paths. bunny.net state, the domain switch (`cdn.scutoons.com` → bunny when the new system is deployed) and the
  owner's setup steps: agents/ASSETS.md "Connecting bunny.net".

## Phases (updated)

| Phase | What | Who |
|---|---|---|
| P0 | monorepo + history import | ✅ |
| P1a | workspaces, shared packages, standalone creator | ✅ 2026-09-30 |
| P1b | server refactor (Slim), host routing qr/api, admin API (folder renamed to `server/` 2026-09-30) | ✅ 2026-09-30 |
| P2a | `admin/` app incl. port of `generate/` (QR generation, batches, certificates) to Svelte; bottom ActionBar in admin + creator | ✅ 2026-09-30 |
| P2b | `shop/` app split out of the viewer, `/shop` redirect, products from the API | ✅ 2026-09-30 |
| P3 | `qr/` app (qr.scutoons.com) + server API-only, `deploy/targets.json` + one reusable deploy workflow + `deploy-all`, versions (`version.json`, `/api/health/`, `/api/admin/versions`, CHANGELOGs), cutover checklist | ✅ 2026-09-30 (see "P3 — result"; open: switch `VITE_API_URL`, remove `generate/` after admin parity) |
| P4 | data into the DB via the API (own migration plan) | later |
| P5 | CDN plan with bunny.net (with owner) | later |

## Hosting layout & deployment (2026-09-30, owner — supersedes "Server host routing")

**One API, everything else is a frontend.** `qr.scutoons.com` is no longer served by the server code; it becomes its
own small app `qr/`: **plain PHP, server-side rendered, no JS framework**, which calls `api.scutoons.com` from the
server side (landing `?c=`, verify page, redirects). Reasons: instant page on scan, works without JS, no CORS, can
cache lookups and show a friendly fallback if the API is briefly down. (Security gain is small — the API is public for
the other frontends anyway; its own defences stay: rate limits, no owner data, one owner per code, revoke.)

Printed-URL RULE still holds on `qr.scutoons.com`: `?c=` (with/without `/`), `skip=1`, `/verify.html`,
`/api/validate/`, `/api/keys/keys.php` keep answering identically — `qr/` serves the pages and **proxies** the two
legacy endpoints to the API. Transition: the released viewer calls `https://qr.scutoons.com/api/*` → `qr/` proxies the
whole `/api/*` to `api.scutoons.com` behind one flag (`PROXY_FULL_API`) – **off since 2026-10-05** (CUTOVER 10.5).
The legacy `generate/` SPA is not moved: it retires when the admin app reaches parity (at cutover at the latest).

**Host (all-inkl): one FTP account, its root is the deploy root; each subdomain points into it:**

```
<ftp root>/
├── viewer/dist/      ← scutoons.com, www.scutoons.com
├── admin/dist/       ← admin.scutoons.com
├── creator/dist/     ← creator.scutoons.com
├── shop/dist/        ← shop.scutoons.com
├── qr/               ← qr.scutoons.com      (PHP; docroot qr/public/, root .htaccess forwards if docroot is qr/)
├── api/              ← api.scutoons.com     (server/; docroot api/public/ recommended, root .htaccess forwards otherwise)
│   ├── storage/      private (batches, keys) — never overwritten by deploys
│   └── config.php    generated by the workflow
└── cdn/              ← cdn.scutoons.com     (or bunny.net — config, see CDN)
```

Repo → host mapping: `viewer/`→`viewer/dist`, `admin/`→`admin/dist`, `creator/`→`creator/dist`, `shop/`→`shop/dist`,
`qr/`→`qr/`, `server/`→`api/`. **One source of truth for targets:** `deploy/targets.json` (app → protocol, host path,
build command, include/exclude) read by all workflows — switching an app or the media to bunny.net/Cloudflare later is
a change there (+ secrets), not in every workflow.

**Workflows:** one per app (path filters incl. `shared/**`, deploy only from `main`) **plus `deploy-all.yml`**
(manual dispatch + optional on tag) that builds and deploys every app in order: api (+ migrations reminder) → qr →
frontends. One FTP secret set for the host (`FTP_SERVER`, `FTP_USERNAME`, `FTP_PASSWORD`); per-app secrets only if a
target moves elsewhere.

**Versions / out-of-sync detection:** every app and shared package has a semver `version` (package.json /
composer.json). Each build writes `version.json` (`app, version, git sha, built_at, shared package versions`) into its
output; the API's `/api/health/` returns its own. The admin dashboard shows all deployed versions side by side and
flags mismatches (e.g. a frontend built against an older `@scutoons/api` than the API expects). Each app keeps a
`CHANGELOG.md`.

## P3 — result (2026-09-30)

**Structure**

| Where | What |
|---|---|
| `qr/` | plain PHP app for qr.scutoons.com, no Composer deps (`src/` App, Landing, Api, Proxy, Http, Cache, Config, View, Response, Client; `public/` docroot with `index.php`, `router.php`, `.htaccess`, `verify.html`, `assets/js/`; `templates/landing/` moved from the server unchanged + `unavailable.php`; root `.htaccess` forwards into `public/` if the docroot is `qr/`; `config.local.php.example`, README, CHANGELOG, `composer.json` = version + `serve`/`test` scripts) |
| `qr/tests/public-urls.php` | the printed-URL suite (moved from the server): every printed URL through the qr app; proxies byte-identical to the API; `--full-api=off`; `--down` (fallback page); `--limited` (429 page) |
| `server/` | API only: `HostRouting`, `LandingController`, `templates/landing`, `public/verify.html`, `assets/js/verify-crypto.js` removed (noble stays for `generate/`); non-API paths → plain 404 (`PageNotFoundController`); settings `QR_HOST`/`API_HOST`/`QR_HOST_SERVES_API` gone, new `QR_PROXY_SECRET`, `VERSION_URLS`; `composer.json` `version` 1.0.0 |
| `deploy/` | `targets.json` (the one description of every target), `deploy.mjs` (build → stage `.deploy/<app>/` → version.json → generated config.php → plan; `--dry-run`, `--github`, `--setup`), `version.mjs`, README |
| `.github/workflows/` | `deploy-app.yml` (reusable, environment `production`, secrets/vars as JSON), `viewer/shop/creator/admin.yml` (callers), `qr.yml` (php -l + offline smoke test, deploy), `server.yml` (PHPUnit + PHPStan, deploy), `deploy-all.yml` (manual / tag `deploy-*`: migrations reminder → api → qr → frontends, matrix from `targets.json`) |
| versions | apps `package.json` 0.1.0 (qr, api `composer.json` 1.0.0), shared packages 0.0.0 → 0.1.0; `npm run build` writes `version.json` (`node ../deploy/version.mjs build`); `CHANGELOG.md` in viewer, creator, shop, admin, qr, server |

**Decisions**
- qr → API: `GET /api/code/?c=&scan=1` (new: `scan=1` logs the `scan` event, only for the trusted proxy, also for
  expired/revoked codes – as the old landing page did) + `GET /api/items/` (names, icons). The lookup now counts against
  the API's per-IP lookup limit (`ACCESS_MAX_PER_IP_HOUR`, shared with `/access/`) → new 429 page; the old landing page
  had no limit.
- **Trusted proxy:** qr sends `X-Scutoons-Proxy-Secret`, `X-Scutoons-Client-Ip` (REMOTE_ADDR), `X-Scutoons-Device-Id`
  (the browser's `X-Device-Id`); the API's `DeviceId` middleware uses them only when the secret equals its
  `QR_PROXY_SECRET` (hash_equals; forwarded IP must be a valid IP). Clients' own `X-Scutoons-*` headers are dropped
  by the proxy. Behind Cloudflare later, REMOTE_ADDR would be Cloudflare's → use `CF-Connecting-IP` then.
- Cache: APCu if enabled, else JSON files in `qr/cache/`. A scan always asks the API (counting + rate limit); the
  cache serves `skip=1` (fresh 60 s) and the fallback page's Continue (kept 1 day); catalog 5 min.
- Proxy: method, query, body and headers pass through (minus hop-by-hop, Host, Accept-Encoding, Cookie); the answer's
  status, body and headers (minus hop-by-hop, lengths, Date/Server/X-Powered-By) come back unchanged; PHP's default
  content type is suppressed. API unreachable → 502 `api_unavailable`. The verify page's
  `generate/get-signature.php` + `generate/files/signatures/SIGNATURE-*.json` are proxied too (they must stay in the API
  when `generate/` retires); other `/generate/*` → 302 to `API_URL/generate/…`. Since 2026-10-02 the verify page
  checks through `/api/certificates/*` (always proxied, also with `PROXY_FULL_API=0`); get-signature.php is only its
  "Download Signature File" button.
- `/?login=` (old magic links) → `WWW_URL` (default `https://scutoons.com`, as `APP_URL` before). qr's `API_URL` in
  production is fixed to `https://api.scutoons.com` in `targets.json` (never from `VITE_API_URL`, which points at qr
  during the transition → loop).
- Deploy: `deploy.mjs` stages exactly the upload (PHP targets build `vendor/` inside the staging copy, the dev vendor is
  never touched), `SamKirkland/FTP-Deploy-Action` uploads it; `protect` globs become its `exclude` (neither uploaded
  nor deleted: `api/storage/`, `db/`, `logs/`, `config.local.php`, `.htpasswd`; `qr/cache/`). Generated values with
  an unset `${VAR}` are left out (the app default applies). The creator reads `VITE_WWW_URL` (old `VITE_VIEWER_URL`
  still read); the admin build gets `VITE_VIEWER_URL=${VITE_WWW_URL}` via `buildEnv`; `VITE_STATIC_API` dropped
  (default `<VITE_WWW_URL>/api` is right).
- `.env.example`: `MAIL_FROM` must be a plain address (the server builds `From: SCUTOONS <MAIL_FROM>`; the earlier
  example value `SCUTOONS <no-reply@…>` would have broken the header and the SMTP envelope).

**Verified**
- Landing HTML of 11 states (vinyl, dago choice, beetles, no edition, promo, unknown, enter form; de variants)
  **byte-identical** to the server's before the move.
- Printed-URL suite: qr on :9191 → API on :9190 (DB copy) with `--out`, diffed against the baseline recorded from the
  server before the change (identical to the P1b baseline of the old plain-PHP server): identical, plus 3 lines for a
  revoked code that exists in the local DB now (410 as expected). Live :9001 → :9000: 79/79.
- `composer test` (server): PHPUnit 46 tests; api-smoke 187/187, content-smoke 62/62, claim-race claim + signup PASS,
  admin-smoke 65/65, public-urls via qr 89/89, with `PROXY_FULL_API=0` 78/78. PHPStan level 8 clean. `npm run check`:
  0 errors in every workspace.
- Browser at 375px: vinyl (de), dago choice, promo, revoked, unknown, enter form, API down (fallback + Continue),
  rate-limited; `/verify.html` verifies a real signed code through the proxy.
- `npm run deploy:dry-run` (all builds): api 676 files / 2.9 MB → `api/`, qr 29 / 109 KB → `qr/`, viewer 155 / 2.1 MB
  → `viewer/dist/`, shop 42 / 353 KB, creator 54 / 485 KB, admin 98 / 1.3 MB; every output has `version.json` with the
  shared package versions.

**Open**
- ~~Switch `VITE_API_URL` to `https://api.scutoons.com/api`, then `PROXY_FULL_API` → 0~~ done (CUTOVER 10.5, 2026-10-05).
- Admin dashboard widget for `GET /api/admin/versions` (admin app).
- Remove `generate/` + `GENERATE_PASSWORD` after admin parity; keep `generate/get-signature.php` and the signature
  files route in the API (the verify page needs them) – e.g. move them to `/api/…` paths and change qr's proxy map.
- The printed-URL suite reads codes from SQLite; against production it needs a SQLite copy of the MySQL data, or a few
  real codes by hand (checklist step 6).

## Cutover checklist (all-inkl)

For the owner, in this order. Nothing public changes until step 5; every step until 8 is undone by pointing a
subdomain back at its old folder (the old folders stay untouched until step 10).

**0. Before** (a quiet hour; the `anger` release is out)
- Do **not** merge `platform` into `main` yet: every push to `main` deploys the touched apps – that is step 4.
- In the **old** repo `mandymozart/qr.scutoons.com`: disable Actions (or archive the repo) so it never uploads again.
- Backups: FTP download of the current qr folder (at least `api/keys/keypair.json`, `generate/files/`,
  `generate/.htpasswd`, `config.php`) and of the viewer folder; phpMyAdmin export of the database.
- GitHub → repo `scutoons` → Settings → Environments → **`production`** (restrict to branch `main`): every secret and
  variable of the root `.env.example` (`AUTH_SECRET` and `QR_PROXY_SECRET`: new random strings, ≥ 32 chars).
  Keep `VITE_API_URL=https://qr.scutoons.com/api` for now.

**1. Folders** (KAS → FTP / file manager, in the root of the FTP account from `FTP_USERNAME`)
- Create `viewer/dist/`, `admin/dist/`, `creator/dist/`, `shop/dist/`, `qr/public/`, `api/public/`,
  `api/storage/keys/`, `api/storage/files/` (the API creates `api/storage/files/claims/` itself on the first ownership-claim
  evidence upload – private like the rest of `storage/`, never served; include it in backups).

**2. Private data → `api/storage/`** (before the first API deploy, so `/api/keys/keys.php` never 404s)
- Upload `api/keys/keypair.json` (old qr folder) as `api/storage/keys/keypair.json`.
- Copy the old `generate/files/*` into `api/storage/files/` (batch output; the verify page's signature fallback).
- **Check the copy before going on** (codes and certificates in circulation must stay valid, owner 2026-10-01): per folder
  (`batch/`, `certificates/`, `signatures/`, and any other subfolder of the old `generate/files/`) the file count and total
  size on the server equal the local backup from step 0; `api/storage/keys/keypair.json` is byte-identical to the backup
  (same size + SHA-256). After step 5, verify 3 real codes from different batches: `/verify.html` with the printed
  UPPER CASE code, `/api/validate/?c=CODE`, and an admin certificate "Regenerate" that matches the printed one.

**3. Database** (phpMyAdmin, same MySQL database)
- Compare `server/migrations/mysql/*.sql` with the `schema_migrations` table and run the missing files in file-name order
  (`2026-09-29-accounts-inventory`, `2026-09-30-code-owner-guard`, `2026-09-30-content-versions`,
  `2026-09-30-entity-keys` (after content-versions: the content store by entity key, agents/DATA.md), `2026-09-30-member-characters`,
  `2026-09-30-revoke-events`, `2026-09-30-roles-signup`, `2026-09-30-sensitive-content` (`users.hide_sensitive`, "Hide
  sensitive content", default on – agents/ITEMS.md), `2026-10-02-mail-log` (the sent-mail log), `2026-10-02-ownership`
  (ownership claims, claim files, the transfer history – agents/ITEMS.md "Ownership, claims, gifting"; until it ran, gifts
  and claims answer 503 and `/me/ received` is empty, everything else works), `2026-10-02-ownership-signup`
  (`login_requests.dispute`: signing up to file a claim), `2026-10-01-media-quality` (`users.media_quality`, the reader's
  "Medienqualität", default `auto` – ships with the `media-quality` branch after the release: run it then), `2026-09-30-settings` – runtime settings table for the
  admin's Settings page; without it the API uses the config defaults: scans 1000/h, lookups 120/h), then the seeds if not done: edition numbers
  (`php bin/console backfill-edition-numbers --sql` locally → run the output) and the content store
  (`shared/content/seed/content-seed.mysql.sql` = `php bin/console import-content shared/content/seed --sql`, every toon's
  content incl. characters + credits; seed removed in 10.30). server/docs/DATABASE.md, agents/DATA.md §4.

- Embassy stores (2026-10-02, branch `stores`): `2026-10-02-embassy-stores` (additive: `embassy_orgs.listed`,
  `independent`, `address`, `opening`, `carries`) – run it right after `2026-10-01-embassy` (it needs that table; both
  are idempotent and may run with the release or at the Embassy go-live). Until it ran, `GET /api/stockists/` answers an
  empty list and the viewer simply shows no "Local stores" link and the empty "Where to buy" page; the Embassy app
  (held back) must not be used before it ran. agents/EMBASSY.md "Stores".

- Content seed note (2026-10-02, players + related items): `content-seed.mysql.sql` carries `related_items` in
  `season:catpope:1:{de,en}` and the record-player pickup in `chapter:catpope:2:{de,en}`. The statements skip stored keys
  – a database that already has them needs those 4 entities imported separately (`import-content … --toon=catpope
  --only=season,chapter --update` against it, or a one-off INSERT of the new published versions).
- CDN (2026-10-02): upload the player icons `scutoons/items/{record_player,tape_player,cd_player}.png` (admin →
  Assets, unless already on bunny) before the
  API serves the new catalog – otherwise the items show their initials.
- Record player repair (2026-10-02): `content-seed.mysql.sql` now has the broken-record-player pickup in
  `scene:catpope:2:scene-14:{de,en}` and the repair-coupon pickup (instead of the record player) in
  `chapter:catpope:2:{de,en}` – a database that already has these 4 keys needs `import-content … --toon=catpope
  --only=chapter,scene --update` against it. CDN: upload `scutoons/items/{broken_record_player,repair_coupon}.png`
  (admin → Assets, unless already on bunny). agents/ITEMS.md "Record
  player repair".

- Asset manager (2026-10-02, branch `assets`, agents/ASSETS.md): run `2026-10-02-assets.mysql.sql` (any time; without it
  only the asset manager answers 503). GitHub environment `production`: the optional bunny.net keys (`.env.example`
  "bunny.net (asset manager)" – `deploy/targets.json` writes them into the API's `config.php`). After the deploy: admin
  → Assets → Rescan (registers the storage's files, indexes the uses). The bunny zone is reached at
  `https://cdn.scutoons.com` (custom hostname on pull zone 6731366 + CNAME + SSL, CUTOVER 10.3; before that the interim
  `https://scutoons.b-cdn.net`, which stays valid) – `CDN_PUBLIC_URL` / `VITE_CDN_URL` in `deploy/targets.json`; content
  keeps its CDN-relative paths (absolute b-cdn URLs: migration `2026-10-05-cdn-domain`). Before that switch: `php bin/console assets:sync --source-root=../media
  --apply --overwrite --purge-zone` (the initialisation – done 2026-10-02, CUTOVER 1.5; agents/ASSETS.md "Connecting bunny.net").

**4. Deploy into the new folders** (nothing points at them yet)
- Merge `platform` into `main` (the push deploys the touched apps into the new folders).
- Actions → "Deploy all" → Run workflow with **dry run** → read the plan (files → folders, protected paths).
- Run it again without dry run (or push the tag `deploy-<date>`). Order: api → qr → frontends.

**5. api.scutoons.com** (new subdomain)
- KAS → Domains → Subdomain `api` → target directory `/api/public/` → SSL: enable Let's Encrypt (+ "force HTTPS").
- Upload `generate/.htpasswd` to `api/public/generate/.htpasswd` and set `AuthUserFile` in
  `api/public/generate/.htaccess` to its absolute path (KAS shows `/www/htdocs/<account>/…`).
- Check: `https://api.scutoons.com/api/health/` → `status ok, db ok, version 1.0.0, git_sha <main>`;
  `/api/items/`, `/api/keys/keys.php` (same `publicKey` as the old qr site), `/?c=x` → 404, `/generate/` asks for
  the password.

**6. qr.scutoons.com → the qr app** (printed URLs – check carefully)
- KAS → subdomain `qr` → target directory `/qr/public/` (SSL stays on; re-issue if KAS asks).
- Check with real codes: a membership code `/?c=CODE` and `?c=CODE` (no slash) → item page with Continue + Add to my
  account; a promo code → Create account; a page code → redirect; `/?c=CODE&skip=1` → redirect; an unknown code → 404
  page; `/verify.html` with a real certificate code (by its exact code – since 2026-10-03 codes are case-sensitive, see the RULE section); `/api/validate/?c=CODE` (JSON,
  `Access-Control-Allow-Origin: *`); `/api/keys/keys.php`; `/version.json`.
- Admin (or `generate/` Stats): the scan shows up with a device/IP hash – if every scan has the same IP hash,
  `QR_PROXY_SECRET` differs between api and qr.
- Rollback: target back to the old qr folder.

**7. Frontends**
- `admin.scutoons.com` (new) → `/admin/dist/`, SSL; `shop.scutoons.com` (new) → `/shop/dist/`, SSL;
  `creator.scutoons.com` (exists) → `/creator/dist/`, **enable SSL**; `scutoons.com` + `www.scutoons.com` →
  `/viewer/dist/` (SSL on both).
- Check each `https://<host>/version.json`, the viewer home + `/catpope` + a chapter, `/shop` → shop, the creator and
  admin logins.

**8. API URL switch**
- Variable `VITE_API_URL` → `https://api.scutoons.com/api`; run "Deploy all" (the frontends rebuild).
- Check: viewer `?code=CODE` (device access), sign in + claim, progress sync; creator save/publish; admin dashboard and
  "versions" (`/api/admin/versions` lists every app).

**9. Admin role + generate/**
- Grant `admin` to the owner's account (legacy `generate/` → Users, or `php bin/console roles grant`).
- Admin app → Emails → set the admin notification recipients (or `MAIL_ADMIN_TO`): new ownership claims are mailed
  there – with none set, nobody is told about a claim (it still shows on the Claims page). `ADMIN_URL` (default
  `https://admin.scutoons.com`) is the link in that mail.
- When the admin app has parity: remove `generate/` and `GENERATE_PASSWORD` (separate change).

**10. Later**
- ~~`PROXY_FULL_API` → `0` in `deploy/targets.json`, deploy qr~~ done 2026-10-05 (CUTOVER 10.5).
- After one release without problems: delete the old qr folder and the old viewer folder (keep the backups).

**Rollback**
- Pages/apps: point the subdomain back at the old folder (instant; old folders are untouched until step 10).
- API: the old qr folder still has its own keypair and batch files (copied, not moved) and talks to the same DB; the
  migrations only add tables/columns.
- Code: re-run the deploy of an earlier commit (Actions → the app's workflow on that commit) or `deploy-*` tag.
