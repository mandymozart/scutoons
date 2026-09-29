# PLATFORM — monorepo, server refactor, shared libraries, creator + admin apps

Branch **`platform`** (from `anger`, 2026-09-30). `anger` stays the release branch for 2026-10-04; `platform` merges
back after the release. Companion docs: [PLAN.md](PLAN.md), [ITEMS.md](ITEMS.md) (API contract), [DESIGN.md](DESIGN.md).

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
│   ├── config/                   settings, permissions.json, items.json, editions.json
│   ├── migrations/{mysql,sqlite}/
│   ├── bin/console               migrate, roles, import-content, backfill, reports
│   ├── templates/                landing page, mails
│   └── tests/                    PHPUnit + contract suites (api-smoke, content-smoke, claim-race)
├── shared/
│   ├── tokens/   @scutoons/tokens    design tokens (CSS custom properties), fonts, base CSS
│   ├── ui/       @scutoons/ui        Svelte UI kit: Button, IconButton, Field, Panel, Sheet, Tabs, Toast, DataTable, …
│   ├── content/  @scutoons/content   TS types + validation for toon/season/chapter/slice/dialog/audio/VN/items; JSON Schema export
│   ├── reader/   @scutoons/reader    rendering: Slice, Dialogs, Onomatopoeia, group renderers, audio/VN actions
│   └── api/      @scutoons/api       typed client: session, device id, access, items, inventory, content, admin
├── cdn/                          python tools (sync, download, icons, bubble detector), root/ gitignored
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
   Apps and the server's public pages import the same CSS (server gets a built copy at deploy).
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
- `Infrastructure/*`: Database (PDO, MySQL/SQLite, transactions, migrations runner), Mail (SMTP, log), Signing (ML-DSA
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

Sections: Dashboard (scans, unlocks, claims, signups per day/edition) · Codes (table, filters, detail timeline, revoke)
· Batches (port of generate: sign, QR SVG, certificate PDF, sticker sheet, ZIP) · Users & roles · Content (versions,
publish state per toon/chapter) · Catalog (items, editions, permissions map read-only view) · Events log.

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
| `shared/reader` | `Slice`, `Dialogs`, `Dialog`, `Onomatopoeia` (+ variants/effects), actions `sliceAudio` (player injected), `sliceEvents` (VN `run` injected; ex unused `vnEvents`), `getMusicStateForSlice`, `isWithinRect`, type `ActiveOnomatopoeia`; check = svelte-check --fail-on-warnings |
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
- Shared components keep the old DOM/class names (`.slice`, `.dialog`, `.onomatopeia`) so output stays identical;
  files are kebab-case like the apps. Reader onomatopoeia: `$inspect` dropped, props read via `$derived`.
- No viewer markup was swapped for `@scutoons/ui` yet: nothing was a visually 1:1 match (e.g. role tags use a 2px
  border, the kit's Badge a hairline); the creator's new login screen and preview use the kit.
- The creator's static fallback files (`toons.json`, `characters.json`, store fallback) come from the viewer origin →
  production needs CORS on `scutoons.com/api/*.json` for `creator.scutoons.com` (or those files via the API later).

## P1b — result (2026-09-30)

Server folder still `scutoons-server/` (the `git mv` to `server/` was refused by the session's permission guard; to do:
rename + update the path filter / `SERVER_DIR` in `.github/workflows/server.yml`, the root README and package.json –
the code has no folder-name dependency). Docs: `scutoons-server/README.md` + `docs/{ARCHITECTURE,API,CONFIGURATION,
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
- Settings: env `QR_<KEY>` → constant (config.local.php / CI-generated config.php) → `config/settings.php`; SQLite when
  `DB_SQLITE_PATH` is set. Batch output `generate/files/` → `storage/files/`, keypair `api/keys/` → `storage/keys/`.
- Migrations moved to `migrations/<driver>/` with unchanged file names (schema_migrations stays valid);
  `migrations/schema/qr.<driver>.sql` for fresh databases. No schema change in this phase.
- The legacy `generate/*.php` URLs run on Slim (`Legacy\GenerateController` + `GenerateToken`) on the same services as
  the admin API; `generate/get-signature.php` and `generate/files/signatures/*` stay public for the verify page.
- Contract suites boot through `tests/support/legacy.php` (the old static calls as facades); `tests/admin-smoke.php`
  and `tests/public-urls.php` (printed URLs, identical to the old server; `--hosts`) are new; `composer test` runs all.
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
| `lib/components/shop/products/product-slides.svelte` (homepage only, Swiper) | stays in the viewer: `lib/components/home/product-slides.svelte` (not generic → not in `@scutoons/ui`) |
| `static/api/products.json`, `featured-products.json` | server `config/products.json`, `config/featured-products.json` (`PRODUCTS_PATH`, `FEATURED_PRODUCTS_PATH`) |
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

## Cross-app auth

Same endpoints, token per origin (localStorage `scutoons_session`) for now; `ALLOWED_ORIGINS` gains
creator/admin origins (+ dev ports: viewer 8000, creator 10000, admin 11000). Later: one `HttpOnly` session cookie on
`.scutoons.com` for single sign-on.

## Deploy

Root `.github/workflows/`: `viewer.yml`, `creator.yml`, `admin.yml` (path filters incl. `shared/**`), `server.yml`
(`scutoons-server/**` until the rename: PHPUnit + PHPStan, composer install --no-dev, generated config.php, FTP; build
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
| P3 | CI workflows, root README, local dev (one command), cutover checklist, archive old repo | coordinator/agent | P1–P2 |

## Rules while `anger` is being released

- Release fixes land on `anger` (viewer etc.) and, for the server, on the old repo `C:\Development\qr.scutoons.com`
  branch `anger`. `platform` pulls them in: `git merge anger` and
  `git subtree pull --prefix=server C:\Development\qr.scutoons.com anger`.
- After cutover the old repo is read-only.

## Local dev (platform branch)

- Server: `cd scutoons-server && composer serve` (:9000; `composer serve:9100` while the old repo still serves :9000),
  local-only files (config.local.php, db/local.sqlite, storage/files, storage/keys/keypair.json) — gitignored.
  `composer test` = PHPUnit + every contract suite on freshly started servers and a copy of the DB.
- CDN: `python cdn/server.py 8001`. Viewer 8000, creator 10000, admin 11000 (see "Local dev ports").

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
- **Raw record inspector:** key/value table with a JSON toggle on every detail page.
- **CSS split by concern:** color, typography, forms, lists, entities, notifications, columns — values from
  `@scutoons/tokens`.

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
| scutoons.com | `/<toonId>/chapter/<n>` (+ `?label=`) | chapter links |
| scutoons.com | `/<toonId>/credits`, `/<toonId>/download` | links in prints / socials |
| scutoons.com | query param **`code`** on any viewer page (`?code=<CODE>`), plus `&inventory=open` | redirects from the QR landing page |
| scutoons.com | `/de/...`, `/en/...` locale prefixes | shared links |

- New URLs may be **added**; existing ones may only change through server-side code mappings (redirects per code),
  with a migration plan, tests against real printed codes and a rollback. Not now.
- Refactors must include a test that hits every URL above (status + target) before and after.
- Known issue to handle carefully (not changed yet): certificates print the code in upper case, but codes are
  case-sensitive nanoids — verify must accept the printed form (exact match first, then a case-insensitive match only
  when it is unique). Test with real certificate codes before changing anything.

## Local dev ports (2026-09-30, owner — final)

| App | Port | Start |
|---|---|---|
| viewer (main, scutoons.com) | **8000** | `npm run dev:viewer` |
| cdn (static files) | **8001** | `cd cdn && python server.py 8001` |
| server (qr.scutoons.com) | **9000** | `cd server && composer serve` (or `php -S localhost:9000 …`) |
| creator (creator.scutoons.com) | **10000** | `npm run dev:creator` |
| admin (admin.scutoons.com) | **11000** | `npm run dev:admin` |
| shop (shop.scutoons.com) | **12000** | `npm run dev:shop` |

Transition: until the server refactor is verified, the old repo `C:\Development\qr.scutoons.com` may still serve :9000
and the refactored server runs on :9100 for the baseline comparison; after that the platform server owns :9000.
`ALLOWED_ORIGINS` (dev): `http://localhost:8000`, `http://localhost:10000`, `http://localhost:11000`,
`http://localhost:12000` (+ the old
5173/5174/5175/4173 until nothing uses them). The earlier ports (viewer 5173, creator 5180, admin 5190, CDN 8000) are
superseded.

## Domain architecture (2026-09-30, owner — target)

One app per subdomain; on all-inkl every subdomain points to its own build/runtime folder (set in the all-inkl
admin, e.g. `/creator.scutoons.com/`), so each app deploys independently by FTP into its folder.

| Domain | App (repo folder) | Purpose | Runtime |
|---|---|---|---|
| `www.scutoons.com` / `scutoons.com` | `viewer/` | landing page, events, marketing pages **and the reader** (`/<toon>`, `/<toon>/chapter/<n>`, credits, download, account) — printed QR codes redirect here, so the reader stays on this domain | static build |
| `qr.scutoons.com` | `server/` (public pages only) | the low-level public frontend: QR landing `?c=`, verify page, legacy `/api/validate/`, `/api/keys/keys.php` — **nothing else** | PHP (same server code, host-based routing) |
| `api.scutoons.com` | `server/` | the backend for all apps: accounts, access, items, inventory, content, admin API, data | PHP |
| `admin.scutoons.com` | `admin/` | administration: QR generation/batches (port of `generate/` to Svelte), codes, revoke, stats, users & roles, content, catalog, data migrations | static build |
| `creator.scutoons.com` | `creator/` | the creator editor (role `creator`) | static build |
| `shop.scutoons.com` | `shop/` | the shop (moves out of the viewer; `scutoons.com/shop` keeps working as a redirect) | static build |
| `cdn.scutoons.com` | `cdn/` (tools) | all static media — see CDN below | all-inkl origin (+ bunny.net) |

**Server host routing:** one codebase (`server/`), deployed once; `qr.` and `api.` both point to `server/public/`.
The Slim app routes by host:
- `qr.scutoons.com` → landing, verify, `/api/validate/`, `/api/keys/keys.php` (+ assets). Anything else → 404.
- `api.scutoons.com` → `/api/*` (the full API). No landing page.
- **Transition:** until every frontend uses `api.scutoons.com`, the full `/api/*` also stays reachable on
  `qr.scutoons.com` (the released viewer calls `https://qr.scutoons.com/api/...`). Remove only after all apps are
  switched and the stats show no more traffic there. Printed URLs (RULE above) are never removed.
- Dev: one server on :9000 answers both roles (host check relaxed for localhost); frontends use `VITE_API_URL`.

**Shop app:** `shop/` (dev port 12000) with `@scutoons/ui` + tokens; products data from the API (`/api/products/`);
viewer route `/shop` is a redirect to `shop.scutoons.com` (RULE: `/shop` keeps working). ✅ P2b, see "P2b — result".

**Deployment:** one workflow per app (`viewer.yml`, `server.yml`, `admin.yml`, `creator.yml`, `shop.yml`), path
filters incl. `shared/**`, deploy only from `main`, each to its own FTP folder (secrets `<APP>_FTP_*`). The owner
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
  `viewer/static/api/` are deleted (no fallback release: the P2b brief asked for the removal in the same step). Still
  files, not DB tables.

## CDN (careful — owner's live setup)

`cdn.scutoons.com` is served from the all-inkl server and/or bunny.net (owner has set up bunny.net; confirm the exact
current setup before touching anything). Long-term: all static content global via bunny.net (media + app builds).
Rules now:
- **Do not change DNS, pull zones, paths or the FTP sync target.** Asset paths in configs are relative to the CDN base
  and must keep working.
- One config value per app for the CDN base (`VITE_CDN_URL`, server `CDN_URL`) — switching origin/pull zone later is a
  config change, not a code change.
- New uploads get content-hashed or versioned filenames where they may change (cache-safe for a global CDN).
- A separate CDN plan (bunny pull zone ↔ all-inkl origin, cache purge on sync, app builds on CDN) is written with the
  owner before any change.

## Phases (updated)

| Phase | What | Who |
|---|---|---|
| P0 | monorepo + history import | ✅ |
| P1a | workspaces, shared packages, standalone creator | ✅ 2026-09-30 |
| P1b | server refactor (Slim), host routing qr/api, admin API (folder rename `scutoons-server/` → `server/` still pending) | ✅ 2026-09-30 |
| P2a | `admin/` app incl. port of `generate/` (QR generation, batches, certificates) to Svelte | next |
| P2b | `shop/` app split out of the viewer, `/shop` redirect, products from the API | ✅ 2026-09-30 |
| P3 | workflows per app, env per app (`VITE_API_URL` → api.scutoons.com), cutover checklist (subdomains/folders/SSL in all-inkl), remove `generate/` after admin parity | after P2 |
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
whole `/api/*` to `api.scutoons.com` behind one flag (`PROXY_FULL_API`, default on) until stats show no traffic.
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

