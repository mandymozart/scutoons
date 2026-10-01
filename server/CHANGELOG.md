# Changelog – API (server/, api.scutoons.com)

Versions: `composer.json` `version`; `GET /api/health/` returns `version` + `git_sha` (deploy/README.md "Versions").

## Unreleased

- "Media quality" (agents/DESIGN.md "Media quality", branch `media-quality`, after the 2026-10-04 release): migration
  `2026-10-01-media-quality` (`users.media_quality`, default `auto`); `GET /api/me/` `user.media_quality`;
  `PUT /api/me/ { media_quality: auto|low|medium|high }` (alone or with the other settings; anything else → `400
  invalid_body`). Contract: api-smoke "Media quality"; PHPUnit `MediaQualityTest`.

- "Hide sensitive content" (agents/ITEMS.md "Sensitive content"): migration `2026-09-30-sensitive-content`
  (`users.hide_sensitive`, default 1); `GET /api/me/` `user.hide_sensitive`; `PUT /api/me/ { hide_sensitive }` (with or
  without `marketing_opt_in`, answer adds `avatar`) – turning it on moves a worn sensitive A-pose look to the character's
  first non-sensitive look; `PUT /api/avatar/` rejects a sensitive look while it is on (`422 sensitive_hidden`).

- Entity keys (agents/DATA.md): migration `2026-09-30-entity-keys` (table `entity_versions`, every `content_versions` row
  copied with the same id and a key such as `chapter:catpope:2:de`); `/api/entities/<key>` (creator: read, save draft,
  versions, publish, unpublish, restore, index) replaces `/api/content/*`; library, admin content view and avatar unlocks
  on keys; `/api/library/characters`; `import-content` / `export-content` work on the interchange folder
  `shared/content/seed` (`--only` takes entity kinds, `--sql` writes `entity_versions`).

- Library (agents/DATA.md, docs/API.md "Library"): `GET /api/library/toons[/<toon>[/seasons/<n>|/chapters/<n>|/cast|/credits]]`
  and `/api/events/` – the reader's entities composed from the content store (one request per chapter, scenes inlined;
  light season overview), locale fallback, visibility gates on the server, `?draft=1` for creators, ETag/304, server
  cache keyed by version ids (APCu or `storage/cache`). Settings `CACHE_DRIVER`, `CACHE_PATH`, `CONTENT_SHOW_DRAFTS`.
- `import-content` also imports `toons.json`, `events.json` and the map files (locations, routes); new `export-content`.
  Contract suites `entities-smoke` and `entities-compare` (fresh DB seeded from the static files).

## 1.0.0 – 2026-09-30

- Slim 4 refactor of the plain-PHP qr.scutoons.com server (P1b); admin API `/api/admin/*` (role admin); products
  API (P2b). HTTP contract unchanged (agents/ITEMS.md).
- P3: API only – the QR landing page, verify page and host routing moved to the qr app (`qr/`), which proxies the
  printed API URLs here. Trusted proxy: `X-Scutoons-Client-Ip` / `-Device-Id` honoured only with
  `QR_PROXY_SECRET`; `/api/code/?scan=1` (trusted proxy) logs the scan. `/api/health/` + `version`, `git_sha`,
  `built_at`; new `GET /api/admin/versions` (`VERSION_URLS`). Any non-API path is a plain 404.
