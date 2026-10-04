# SCUTOONS

One git repo, npm workspaces (Node 22) for the frontends and shared libraries, PHP for the API (Composer) and the qr app.
Plan and rules: [`agents/PLATFORM.md`](agents/PLATFORM.md).

```
scutoons/
├── package.json        npm workspaces: viewer, creator, admin, shop, shared/*
├── viewer/             SvelteKit (static) → scutoons.com           reader, events, account (/shop → shop app)
├── creator/            SvelteKit (static) → creator.scutoons.com   editor, role `creator`
├── admin/              SvelteKit (static) → admin.scutoons.com     role `admin`: codes, batches, stats, users, content
├── shop/               SvelteKit (static) → shop.scutoons.com      limited editions (products from the API)
├── embassy/            SvelteKit (static) → embassy.scutoons.com   partner network: leads, calls, mails (roles liaison / ambassador)
├── server/             PHP, Slim 4 (Composer) → api.scutoons.com    the API of every app (+ legacy generate/ until admin parity)
├── qr/                 plain PHP → qr.scutoons.com                 printed URLs: QR landing ?c=, verify page; proxies the API
├── deploy/             targets.json (where each app goes), deploy.mjs, version.mjs – see deploy/README.md
├── shared/
│   ├── tokens/         @scutoons/tokens    design tokens (CSS custom properties), fonts, base CSS
│   ├── content/        @scutoons/content   content/items types, pure helpers, validation, JSON Schema
│   ├── reader/         @scutoons/reader    Slice, Dialogs, Onomatopoeia, pure slice actions
│   ├── api/            @scutoons/api       typed API client (session, device id, accounts/items API, content store)
│   └── ui/             @scutoons/ui        Svelte UI kit (Button, Field, Sheet, Tabs, Toasts, …)
├── media/              local dev media (gitignored; the dev CDN serves it, production media lives on bunny)
└── agents/             plans and memory
```

Shared packages are source packages (no build step): each app's Vite compiles them. Dependency direction is
apps → shared, never the other way; see each package's README.

## Local development

```sh
npm install                 # once, at the repo root (one lockfile for all workspaces)
npm run dev:viewer          # viewer   http://localhost:8000
npm run dev:creator         # creator  http://localhost:10000
npm run dev:admin           # admin    http://localhost:11000
npm run dev:shop            # shop     http://localhost:12000
npm run dev:embassy         # embassy  http://localhost:13000
npm run dev:api             # API      http://localhost:9000   (= cd server && composer serve: 4 php -S workers + OPcache)
npm run dev:qr              # qr app   http://localhost:9001   (= cd qr && composer serve; needs the API)
npm run dev:db              # optional: the API on MariaDB in Docker instead of SQLite (server/docs/DATABASE.md)
npm run check               # svelte-check / tsc in every workspace
npm run build               # builds every workspace that has a build script (+ version.json in each output)
npm run deploy:dry-run      # builds + stages every app, lists what would be uploaded where (deploy/README.md)
```

Local dev ports — single source of truth: `shared/dev-ports.json` (agents/PLATFORM.md → "Local dev ports"; `npm run check:ports`):

| Service | Port | Start |
|---|---|---|
| viewer | 8000 | `npm run dev:viewer` |
| CDN | 8001 | `node scripts/dev/media-serve.mjs` (serves `media/`) |
| API (server) | 9000 | `npm run dev:api` or `cd server && composer serve` |
| qr (qr.scutoons.com) | 9001 | `npm run dev:qr` or `cd qr && composer serve` – landing `/?c=`, `/verify.html`; config `qr/config.local.php` (`API_URL` :9000, same `QR_PROXY_SECRET` as `server/config.local.php`) |
| creator | 10000 | `npm run dev:creator` |
| admin | 11000 | `npm run dev:admin` |
| shop | 12000 | `npm run dev:shop` |
| embassy | 13000 | `npm run dev:embassy` |
| MariaDB (Docker, optional) | 13306 | `npm run dev:db` – the production engine instead of SQLite ([server/docs/DATABASE.md](server/docs/DATABASE.md) "Local MariaDB") |
| Adminer (Docker, optional) | 13380 | `npm run dev:db adminer` |

Dev settings live in each app's `.env.development` (gitignored, see `.env.example`): `VITE_API_URL` (API base, e.g.
`http://localhost:9000/api`), `VITE_CDN_URL` (default `http://localhost:8001` in dev). The creator also has
`VITE_WWW_URL` ("Open the reader" link, the static fallback files; old name `VITE_VIEWER_URL`), the viewer `VITE_CREATOR_URL` (link on the account
page) and `VITE_SHOP_URL` (menu, "Buy access", `/shop` redirect), the shop `VITE_WWW_URL` (logo link back), the admin
`VITE_CREATOR_URL` + `VITE_VIEWER_URL` (links; see `admin/README.md`).

Every frontend origin must be in the API's `ALLOWED_ORIGINS` (`config.local.php`): `http://localhost:8000`,
`http://localhost:10000`, `http://localhost:11000`, `http://localhost:12000`, `http://localhost:13000`. Login mails are written to the server's `logs/mail.log`
locally. Single sign-on: the API keeps the session in an HttpOnly cookie on `localhost` (ports don't matter), so signing
in on one app signs you in on all of them (`@scutoons/auth`, `npm run check:sso`).

## Deploy

Every app deploys independently into its own folder of one all-inkl FTP account (`viewer/dist/`, `admin/dist/`,
`creator/dist/`, `shop/dist/`, `qr/`, `api/`), described once in [`deploy/targets.json`](deploy/targets.json):

- one workflow per app (`.github/workflows/{viewer,shop,creator,admin,qr,server}.yml`, deploy only from `main`) calling
  the reusable `deploy-app.yml`; `deploy-all.yml` (manual or tag `deploy-*`) deploys api → qr → frontends;
- secrets and variables: GitHub environment `production`, the checklist is [`.env.example`](.env.example);
- every build writes `version.json` (app, version, git sha, shared package versions); the API reports its own in
  `/api/health/`, the admin API lists all (`/api/admin/versions`); each app has a `CHANGELOG.md`.

Details, dry runs and how to move a target to bunny.net / Cloudflare: [`deploy/README.md`](deploy/README.md).
The cutover to this layout: `agents/PLATFORM.md` → "Cutover checklist (all-inkl)".
