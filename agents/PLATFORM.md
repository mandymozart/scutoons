# PLATFORM — monorepo, server refactor, shared libraries, creator + admin apps

Branch **`platform`** (from `anger`, 2026-09-30). `anger` stays the release branch for 2026-10-04; `platform` merges
back after the release. Companion docs: [PLAN.md](PLAN.md), [ITEMS.md](ITEMS.md) (API contract), [DESIGN.md](DESIGN.md).

## Target picture

```
scutoons/                         one git repo, npm workspaces (Node 22), Composer for the server
├── package.json                  workspaces: viewer, creator, admin, shared/*
├── viewer/                       SvelteKit (static) → scutoons.com            reader, shop, events, account
├── creator/                      SvelteKit (static) → creator.scutoons.com    role `creator`
├── admin/                        SvelteKit (static) → admin.scutoons.com      role `admin` (replaces generate/)
├── scutoons-server/              PHP 8.3, Slim 4    → qr.scutoons.com (+ later api.scutoons.com)
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

## Creator app (creator.scutoons.com)

Move `viewer/src/routes/(creator)`, `lib/components/creator`, `lib/creator`, `lib/api/content.service.ts` into
`creator/`; rendering from `@scutoons/reader`; role `creator`. The viewer keeps only the draft-preview loader.

## Cross-app auth

Same endpoints, token per origin (localStorage `scutoons_session`) for now; `ALLOWED_ORIGINS` gains
creator/admin origins (+ dev ports: viewer 5173, creator 5180, admin 5190). Later: one `HttpOnly` session cookie on
`.scutoons.com` for single sign-on.

## Deploy

Root `.github/workflows/`: `viewer.yml`, `creator.yml`, `admin.yml` (path filters incl. `shared/**`), `server.yml`
(`scutoons-server/**`: composer install --no-dev, build tokens CSS copy, FTP). Deploy only from `main`.
New FTP targets / docroots / DNS for creator.scutoons.com and admin.scutoons.com are set up by the owner.
The old `qr.scutoons.com` GitHub repo is archived after cutover; its workflow is inert inside `scutoons-server/.github`.

## Phases

| Phase | What | Who | Depends on |
|---|---|---|---|
| P0 | `platform` branch; qr repo imported with history as `scutoons-server/` (git subtree) | coordinator | – ✅ |
| P1a | Monorepo foundation: root workspaces, `shared/{tokens,content,reader,api,ui}` extracted from viewer, viewer consumes them; `creator/` app split out of viewer | agent "frontend-platform" | P0 |
| P1b | Server refactor to Slim 4 structure, contract suites green, `/api/admin/*` with role auth | agent "server-refactor" | P0 (parallel to P1a) |
| P2 | `admin/` app on shared ui + admin API; parity with `generate/` | agent "admin" | P1a ui/tokens, P1b admin API |
| P3 | CI workflows, root README, local dev (one command), cutover checklist, archive old repo | coordinator/agent | P1–P2 |

## Rules while `anger` is being released

- Release fixes land on `anger` (viewer etc.) and, for the server, on the old repo `C:\Development\qr.scutoons.com`
  branch `anger`. `platform` pulls them in: `git merge anger` and
  `git subtree pull --prefix=scutoons-server C:\Development\qr.scutoons.com anger`.
- After cutover the old repo is read-only.

## Local dev (platform branch)

- Server: `cd scutoons-server && php -S localhost:9100 router.php` (port 9100 while the old repo still serves :9000),
  local-only files copied from the old repo (config.local.php, db/local.sqlite, generate/files, keys) — gitignored.
- CDN: `python cdn/server.py 8001`. Viewer 5173, creator 5180, admin 5190.

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
