# SCUTOONS

One git repo, npm workspaces (Node 22) for the frontends and shared libraries, Composer for the server.
Plan and rules: [`agents/PLATFORM.md`](agents/PLATFORM.md).

```
scutoons/
├── package.json        npm workspaces: viewer, creator, admin, shared/*
├── viewer/             SvelteKit (static) → scutoons.com           reader, shop, events, account
├── creator/            SvelteKit (static) → creator.scutoons.com   editor, role `creator`
├── admin/              SvelteKit (static) → admin.scutoons.com     role `admin` (phase P2, not built yet)
├── scutoons-server/    PHP (qr.scutoons.com API + QR landing page), own Composer setup
├── shared/
│   ├── tokens/         @scutoons/tokens    design tokens (CSS custom properties), fonts, base CSS
│   ├── content/        @scutoons/content   content/items types, pure helpers, validation, JSON Schema
│   ├── reader/         @scutoons/reader    Slice, Dialogs, Onomatopoeia, pure slice actions
│   ├── api/            @scutoons/api       typed API client (session, device id, QR API, content store)
│   └── ui/             @scutoons/ui        Svelte UI kit (Button, Field, Sheet, Tabs, Toasts, …)
├── cdn/                python tools (sync, download, icons, bubble detector); cdn/root/ is gitignored
└── agents/             plans and memory
```

Shared packages are source packages (no build step): each app's Vite compiles them. Dependency direction is
apps → shared, never the other way; see each package's README.

## Local development

```sh
npm install                 # once, at the repo root (one lockfile for all workspaces)
npm run dev:viewer          # viewer   http://localhost:5173
npm run dev:creator         # creator  http://localhost:5180
npm run check               # svelte-check / tsc in every workspace
npm run build               # builds every workspace that has a build script (viewer, creator)
```

| Service | Port | Start |
|---|---|---|
| viewer | 5173 | `npm run dev:viewer` |
| creator | 5180 | `npm run dev:creator` |
| admin | 5190 | (phase P2) |
| API (scutoons-server) | 9100 | `cd scutoons-server && php -S localhost:9100 router.php` |
| API (old repo, until cutover) | 9000 | `cd C:\Development\qr.scutoons.com && php -S localhost:9000 router.php` |
| CDN | 8000 | `python cdn/server.py 8000` (serves `cdn/root/`) |

Dev settings live in each app's `.env.development` (gitignored, see `.env.example`): `VITE_QR_API` (API base, e.g.
`http://localhost:9000/api`), `VITE_CDN_URL` (default `http://localhost:8000` in dev). The creator also has
`VITE_VIEWER_URL` (draft preview links, static fallback files), the viewer `VITE_CREATOR_URL` (link on the account
page).

Every frontend origin must be in the API's `ALLOWED_ORIGINS` (`config.local.php`): `http://localhost:5173`,
`http://localhost:5180`, later `http://localhost:5190`. Login mails are written to the server's `logs/mail.log`
locally. Sessions are per origin for now (localStorage `scutoons_session`), so the creator and the viewer each need
their own sign-in.
