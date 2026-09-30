# Changelog – API (server/, api.scutoons.com)

Versions: `composer.json` `version`; `GET /api/health/` returns `version` + `git_sha` (deploy/README.md "Versions").

## Unreleased

- Library (agents/DATA.md, docs/API.md "Library"): `GET /api/library/toons[/<toon>[/seasons/<n>|/chapters/<n>|/cast|/credits]]`
  and `/api/events/` – the reader's entities composed from the content store (one request per chapter, scenes inlined;
  light season overview), locale fallback, visibility gates on the server, `?draft=1` for creators, ETag/304, server
  cache keyed by version ids (APCu or `storage/cache`). Settings `CACHE_DRIVER`, `CACHE_PATH`, `CONTENT_SHOW_DRAFTS`.
- `import-content` also imports `toons.json`, `events.json` and the map/collectables files; new `export-content`.
  Contract suites `entities-smoke` and `entities-compare` (fresh DB seeded from the static files).

## 1.0.0 – 2026-09-30

- Slim 4 refactor of the plain-PHP qr.scutoons.com server (P1b); admin API `/api/admin/*` (role admin); products
  API (P2b). HTTP contract unchanged (agents/ITEMS.md).
- P3: API only – the QR landing page, verify page and host routing moved to the qr app (`qr/`), which proxies the
  printed API URLs here. Trusted proxy: `X-Scutoons-Client-Ip` / `-Device-Id` honoured only with
  `QR_PROXY_SECRET`; `/api/code/?scan=1` (trusted proxy) logs the scan. `/api/health/` + `version`, `git_sha`,
  `built_at`; new `GET /api/admin/versions` (`VERSION_URLS`). Any non-API path is a plain 404.
