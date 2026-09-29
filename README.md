# SCUTOONS

One git repo, npm workspaces (Node 22) for the frontends and shared libraries, Composer for the server.
Plan and rules: [`agents/PLATFORM.md`](agents/PLATFORM.md).

```
scutoons/
├── package.json        npm workspaces: viewer, creator, admin, shop, shared/*
├── viewer/             SvelteKit (static) → scutoons.com           reader, events, account (/shop → shop app)
├── creator/            SvelteKit (static) → creator.scutoons.com   editor, role `creator`
├── admin/              SvelteKit (static) → admin.scutoons.com     role `admin` (phase P2, not built yet)
├── shop/               SvelteKit (static) → shop.scutoons.com      limited editions (products from the API)
├── server/             PHP (qr.scutoons.com API + QR landing page), own Composer setup
├── shared/
│   ├── tokens/         @scutoons/tokens    design tokens (CSS custom properties), fonts, base CSS
│   ├── content/        @scutoons/content   content/items types, pure helpers, validation, JSON Schema
│   ├── reader/         @scutoons/reader    Slice, Dialogs, Onomatopoeia, pure slice actions
│   ├── api/            @scutoons/api       typed API client (session, device id, accounts/items API, content store)
│   └── ui/             @scutoons/ui        Svelte UI kit (Button, Field, Sheet, Tabs, Toasts, …)
├── cdn/                python tools (sync, download, icons, bubble detector); cdn/root/ is gitignored
└── agents/             plans and memory
```

Shared packages are source packages (no build step): each app's Vite compiles them. Dependency direction is
apps → shared, never the other way; see each package's README.

## Local development

```sh
npm install                 # once, at the repo root (one lockfile for all workspaces)
npm run dev:viewer          # viewer   http://localhost:8000
npm run dev:creator         # creator  http://localhost:10000
npm run dev:shop            # shop     http://localhost:12000
npm run check               # svelte-check / tsc in every workspace
npm run build               # builds every workspace that has a build script (viewer, creator, shop, …)
```

Local dev ports (agents/PLATFORM.md → "Local dev ports", final):

| Service | Port | Start |
|---|---|---|
| viewer | 8000 | `npm run dev:viewer` |
| CDN | 8001 | `cd cdn && python server.py 8001` (serves `cdn/root/`) |
| API (server) | 9000 | `cd scutoons-server && composer serve` (= `php -S localhost:9000 -t public public/router.php`; the folder becomes `server/`; until cutover the old repo `C:\Development\qr.scutoons.com` serves :9000 – then use `composer serve:9100`) |
| creator | 10000 | `npm run dev:creator` |
| admin | 11000 | `npm run dev:admin` (phase P2) |
| shop | 12000 | `npm run dev:shop` |

Dev settings live in each app's `.env.development` (gitignored, see `.env.example`): `VITE_QR_API` (API base, e.g.
`http://localhost:9000/api`), `VITE_CDN_URL` (default `http://localhost:8001` in dev). The creator also has
`VITE_VIEWER_URL` ("Open the reader" link, the static fallback files), the viewer `VITE_CREATOR_URL` (link on the account
page) and `VITE_SHOP_URL` (menu, "Buy access", `/shop` redirect), the shop `VITE_WWW_URL` (logo link back).

Every frontend origin must be in the API's `ALLOWED_ORIGINS` (`config.local.php`): `http://localhost:8000`,
`http://localhost:10000`, `http://localhost:11000`, `http://localhost:12000`. Login mails are written to the server's `logs/mail.log`
locally. Sessions are per origin for now (localStorage `scutoons_session`), so the creator and the viewer each need
their own sign-in.
