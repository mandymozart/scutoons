# Changelog – API (server/, api.scutoons.com)

Versions: `composer.json` `version`; `GET /api/health/` returns `version` + `git_sha` (deploy/README.md "Versions").

## Unreleased

- Web search in the Ask panels (owner 2026-10-06): `agent_usage.web_searches` (migration
  `2026-10-06-agent-web-search`, additive; without it the count is dropped). `POST /api/agent-usage` takes
  `web_searches?` (`422 invalid_web_searches`), `GET /api/admin/agent-usage` sums it per row and in `totals`;
  `cost_usd` stays the whole cost (tokens + searches).
- Naked A-pose removed (agents/CUTOVER.md 10.31, owner 2026-10-05): `AVATAR_DEFAULT_UNLOCKS` default
  `{"catpope":["swim"]}`; a look `naked` (or one still marked `sensitive: true`) in the registry is ignored
  (`CharacterUnlocks::RETIRED_LOOKS`: no grant – `invalid_look` –, no unlock from rows / the setting, a stored body reads
  as the character's default look). The cover rule is gone: `PUT /api/avatar/` answers `{ status, avatar }` (no
  `restored`), no `422 sensitive_hidden`, `PUT /api/me/ { hide_sensitive }` no longer touches the avatar (the setting
  stays). Migration `2026-10-05-no-naked` (naked bodies → swim, naked unlock rows deleted, a stored settings override
  cleaned).
- Production is the source of truth; MariaDB only (agents/CUTOVER.md 10.30, owner 2026-10-05): `bin/console db:prod-seed
  [--from=<dump>] [--force]` loads production's content + catalog + asset tables (15 tables, from the gitignored
  `db/prod-seed.sql` phpMyAdmin export or a full dump) into the local database in one transaction; the file's SHA-256 in
  setting `dev_prod_seed` makes a re-run a no-op. SQLite removed: `migrations/sqlite/`, `migrations/schema/qr.sqlite.sql`,
  `Database::sqlite()` / `isSqlite()` / `forUpdate()`, settings `DB_DRIVER` / `DB_SQLITE_PATH`, the `--db=` options of
  migrate / import-content / export-content / content:check, `phpunit.mysql.xml` (`phpunit.xml.dist` targets MariaDB:
  `TEST_DB_*`, default 127.0.0.1:13306 `scutoons_test`). Catalog files → test fixture `tests/fixtures/catalog/`
  (`CatalogSeed` test-only); removed `catalog:seed`, `catalog:export-sql`, settings `ITEMS_PATH` / `EDITIONS_PATH` /
  `TOONS_PATH` / `PRODUCTS_PATH` / `FEATURED_PRODUCTS_PATH` / `PERMISSIONS_PATH`, the file fallback (`fallback` field,
  `503 migration_pending`), the featured default (`GET /api/admin/featured` = `{ items, updated_at, updated_by, max }`;
  PUT `null` → `400 invalid_featured`, `[]` = nothing featured) and `GET|PUT /api/admin/catalog/respawn` (respawn is an
  item field). Contract suites + `composer test:contract` removed (`composer test` = PHPUnit); CI = one MariaDB job
  (PHPUnit + PHPStan). `assets:relayout` rewrites code paths only.

- Embassy mail + the Ask panel per app (agents/EMBASSY.md "Automation", owner 2026-10-04): `POST /api/embassy/orgs/{id}/mail`
  sends a person's mail from the shared mailbox (`EMBASSY_MAIL_FROM`, own SMTP account; recipient must belong to the
  organisation, `409 do_not_contact`, opt-out line always added, threaded, `mail_log` template `embassy`, `mail_out`
  activity with subject / addresses / Message-ID, copy in `EMBASSY_IMAP_SENT`); the inbox sync reads replies back by IMAP
  (`POST /api/embassy/inbox/sync`, `GET /api/embassy/inbox`, `bin/console embassy:inbox`; matched by thread or sender,
  unknown senders never stored, "stop" → do-not-contact). `/api/agent-chats` + `POST /api/agent-usage` take an `app`
  (`creator` | `embassy` | `admin`) with that app's role; the admin usage overview groups by it. Migration
  `2026-10-04-ask-embassy-mail` (additive; everything degrades without it). Settings `EMBASSY_*`. PHPUnit
  `EmbassyMailTest`, `MimeMessageTest`, `AgentChatsApiTest`, `AgentUsageApiTest`, `EmbassyApiTest`.

- Review before publish (docs/API.md "Reviews", agents/CUTOVER.md 10.28, 2026-10-04): review rules per toon set by an
  admin (`reviewer` / `needs_review`, `*` = every toon; `/api/admin/review-rules`, `/api/admin/users/{id}/review-rules/…`);
  `needs_review` accounts get `403 review_required` on `/entities/<key>/publish|unpublish` of toon-level entities and
  submit through `POST /api/reviews` instead (pinned version, a newer submit annuls the open one, `409 no_reviewer`);
  reviewers `POST /api/reviews/publish|return`. Three mails (`review_requested`, `review_published`, `review_returned`),
  setting `CREATOR_URL`. Migration `2026-10-04-reviews` (two new tables; without rules nothing changes). PHPUnit `ReviewsTest`.

- Embassy stores + the public "Where to buy" list (agents/EMBASSY.md "Stores", 2026-10-02): organisation type `store`;
  `store` + `record_shop` are stockists with `listed`, `independent`, `address`, `opening`, `carries` (product ids,
  validated against config/products.json; migration `2026-10-02-embassy-stores`, additive). Public `GET /api/stockists/`
  (`?product=`, `?country=`): listed stockists with public fields only, independent vendors first, cached 5 min + ETag,
  CORS `*`, 600 / IP / hour; empty before the migration. `bin/console sample-stores` (local fake stores, SQLite only).
  PHPUnit `EmbassyTest`, `EmbassyApiTest`; api-smoke "Stockists".

- Ownership, claims, gifting (agents/ITEMS.md, 2026-10-02): transfer core (A → NULL → B under the owner guard,
  entries + avatar slots move), history `item_transfers`; reader `POST/GET /claims/`, `/claims/{id}/files`,
  `/claims/{id}/withdraw`, `POST /inventory/gift/`, `/inventory/received/seen/`, `/me/ received`; `/auth/request/
  dispute` (an account to file a claim); admin `/admin/claims*`, `/admin/codes/{code}/history|transfer`. Catalog
  `giftable` (default: not starters / basic dress / membership or access items). Six mails (claim received / admin /
  rejected, transferred from / to, gift received). Migrations `2026-10-02-ownership`, `2026-10-02-ownership-signup`
  (read paths work before them, writes 503). Settings `ADMIN_URL`, `CLAIM_*`, `GIFT_*`. PHPUnit `OwnershipTest`,
  `OwnershipApiTest`, `LoginServiceTest` (dispute); api-smoke + admin-smoke "Ownership".

- Certificate verification (agents/ITEMS.md "Certificates", 2026-10-02): `GET /api/certificates/{code}` (or `?code=`)
  and `POST /api/certificates/verify` (an uploaded `SIGNATURE-<code>.json`) verify server-side against the PUBLISHED
  public key and the signature stored for the code (before, the verify page trusted the key embedded in the file);
  case-insensitive when unique; public status `registered` / `revoked`, never the owner; CORS `*`; per-IP lookup limit.
  Plain-PHP ML-DSA-65 verifier (`Infrastructure/Signing/MlDsa65.php`, `Shake.php`). PHPUnit `MlDsa65Test`,
  `CertificatesApiTest`; api-smoke "Certificates".

- Email module (agents/PLATFORM.md "Emails"): `Domain\Mail` – registry `MailTemplates` (`login_code`,
  `signup_without_code`), texts per locale in `templates/mail/texts.php`, one HTML layout, `Mailer::send($id, $to,
  $locale, $vars)` / `sendToAdmins()`, sent-mail log (migration `2026-10-02-mail-log`, table `mail_log`), admin
  recipients (setting `mail_admin_recipients`, default `MAIL_ADMIN_TO`); `Infrastructure\Mail\Mailer` → `Transport`,
  `LoginMail` and the two mail templates removed. The sign-in mails are byte-identical; `logs/mail.log` format unchanged.
  Admin API `/admin/emails/{templates,templates/{id}/preview,log,recipients}`. Settings `MAIL_ADMIN_TO`,
  `MAIL_ADMIN_LOCALE`, `MAIL_LOG_DAYS`. Tests: PHPUnit `MailTest`, `EmailsApiTest`; admin-smoke "Emails".

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
