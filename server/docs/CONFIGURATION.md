# Configuration

Every setting has a default in `config/settings.php`. Per installation, a value comes from (first non-empty wins):

1. the environment variable `QR_<KEY>` (e.g. `QR_DB_SQLITE_PATH` – `composer test` points the servers at a DB copy),
2. the PHP constant `<KEY>` from `config.local.php` (local, gitignored) or `config.php` (production, written by the
   deploy workflow – never edit it on the server),
3. the default.

Relative paths are relative to the server folder. Secrets never go into the repository.

**Runtime settings** (table `settings`, admin app → Settings, `GET/PUT /api/admin/settings`): a few values the owner
changes without a deploy. A stored value wins over the config; without a row the config value below is the default.
Today: `scan_limit_per_hour` (default `SCAN_MAX_PER_IP_HOUR`), `lookup_limit_per_hour` (default
`ACCESS_MAX_PER_IP_HOUR`). `Domain\Settings\RuntimeSettings`.

| Key | Default | Notes |
|---|---|---|
| `DB_SQLITE_PATH` | – | set → SQLite (local, tests); empty → MySQL/MariaDB with the next keys |
| `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASS` | localhost, –, –, –, – | production |
| `AUTH_SECRET` | – | **required**, ≥ 32 chars: HMAC of login codes, IP/device hashes, access tokens. Rotating it invalidates open login mails and access tokens |
| `GENERATE_PASSWORD` | – | legacy `generate/` SPA password (its token = HMAC of it); empty = the SPA is locked |
| `QR_PROXY_SECRET` | – | shared with the qr app (qr.scutoons.com): only requests carrying it in `X-Scutoons-Proxy-Secret` may forward the client IP (`X-Scutoons-Client-Ip`) and device id (`X-Scutoons-Device-Id`), and only they can log landing-page scans (`/api/code/?scan=1`). Empty = trust nobody |
| `QR_URL` | `https://qr.scutoons.com` | the qr app: magic links of login requests without an Origin (it forwards `/?login=` to the viewer) |
| `APP_URL` | `https://scutoons.com` | the viewer: old magic links are forwarded there; "Open SCUTOONS" in mails |
| `ADMIN_URL` | `https://admin.scutoons.com` | the admin app: links in admin notification mails (a new ownership claim) |
| `CDN_URL` | `https://cdn.scutoons.com/` | item icons `<CDN_URL><icon>`; local `http://localhost:8001/` |
| `ALLOWED_ORIGINS` | scutoons.com, www, creator, admin, shop, embassy + dev ports | comma separated; production sets it in the workflow (without localhost). On a local dev server (`php -S`) every port of `shared/dev-ports.json` is added automatically (`Settings::allowedOrigins`). Also the CSRF list of the session cookie: only these origins may change things with it |
| `MAIL_TRANSPORT` | `smtp` | `smtp` \| `log` (dev: `MAIL_LOG_PATH`) \| `mail` (PHP `mail()`) |
| `MAIL_FROM`, `MAIL_FROM_NAME` | `no-reply@scutoons.com`, `SCUTOONS` | |
| `MAIL_LOG_PATH` | `logs/mail.log` | for `log` |
| `MAIL_ADMIN_TO` | – | admin notification recipients (comma separated) – the default of the runtime setting `mail_admin_recipients` (admin app → Emails); empty = nobody |
| `MAIL_ADMIN_LOCALE` | `de` | language of the admin notification mails |
| `MAIL_LOG_DAYS` | 180 | days the sent-mail log (`mail_log`, with recipient addresses) is kept; 0 = forever |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_USER`, `SMTP_PASS`, `SMTP_TIMEOUT` | –, 587, `tls`, –, –, 15 | `SMTP_SECURE`: `tls` (STARTTLS) / `ssl` (465) / `none` |
| `ACCESS_MAX_PER_IP_HOUR` | 120 | direct `/access/` + `/code/` lookups per IP and hour; 0 = no limit. Default of the runtime setting `lookup_limit_per_hour` |
| `SCAN_MAX_PER_IP_HOUR` | 1000 | QR scans per client IP and hour (`/code/?scan=1` from the qr app, own bucket); 0 = no limit. Default of the runtime setting `scan_limit_per_hour` |
| `LOGIN_TTL_MINUTES`, `LOGIN_MAX_ATTEMPTS` | 30, 5 | validity of link + code; wrong codes per mail |
| `LOGIN_MAX_PER_EMAIL_HOUR`, `LOGIN_MAX_PER_IP_HOUR` | 5, 30 | login mails per hour |
| `SESSION_TTL_DAYS` | 365 | sessions end after this long unused; also the `Max-Age` of the session cookie (renewed by `GET /auth/session/`) |
| `SESSION_COOKIE_DOMAIN` | `scutoons.com` | single sign-on cookie `scutoons_sid`: `Domain=` this when the API host is it or a subdomain (api., qr.), else host-only (localhost: every port shares it); empty = always host-only. `Secure` is automatic (only plain http on loopback goes without) |
| `GRANT_MAX_PER_HOUR`, `IMPORT_MAX_PER_HOUR`, `IMPORT_MAX_QTY` | 120, 10, 99 | |
| `CLAIM_MAX_PER_DAY`, `CLAIM_FILES_MAX`, `CLAIM_FILE_MAX_BYTES` | 5, 5, 5242880 | ownership claims per reader and 24 h, evidence files per claim, bytes per file (agents/ITEMS.md "Ownership, claims, gifting") |
| `GIFT_MAX_PER_DAY`, `GIFT_LOOKUPS_PER_DAY`, `GIFT_COOLDOWN_HOURS` | 10, 30, 24 | gifts per reader and 24 h, recipient lookups per 24 h (unknown addresses count too), re-gift cooldown of a received item |
| `PROGRESS_MAX_BYTES`, `CONTENT_MAX_BYTES` | 262144, 2097152 | |
| `ADMIN_MAX_PAGE_SIZE` | 500 | largest `page_size` of the admin lists |
| `VERSION_URLS` | the five production `version.json` URLs | `app=url,…` fetched by `GET /api/admin/versions` (viewer, admin, creator, shop, qr) |
| `ITEMS_PATH`, `EDITIONS_PATH`, `TOONS_PATH` | `config/*.json` | catalog data |
| `PRODUCTS_PATH`, `FEATURED_PRODUCTS_PATH` | `config/products.json`, `config/featured-products.json` | shop catalog + homepage featured products, served as stored |
| `CACHE_DRIVER`, `CACHE_PATH` | `auto`, `storage/cache` | server cache of the library answers (docs/API.md "Library"): `auto` = APCu when enabled, else files below `CACHE_PATH`; `apcu`, `file`, `none` |
| `CONTENT_SHOW_DRAFTS` | `0` | `1` = the library endpoints show `draft` toons/chapters (local development, like the dev viewer); production `0` |
| `PERMISSIONS_PATH` | `config/permissions.json` | what each code type grants (server-only) – see docs/API.md |
| `FILES_PATH` | `storage/files` | batch manifests, QR SVGs, certificates, signatures, ZIPs (was `generate/files/`) |
| `KEYPAIR_PATH` | `storage/keys/keypair.json` | ML-DSA-65 keypair (was `api/keys/keypair.json`) |
| `LOG_PATH` | `logs/error.log` | PHP/API error log; empty = the host's log |

Production values come from the GitHub environment `production` (the full list with comments: root `.env.example`);
`deploy/targets.json` → `api.generate["config.php"]` says which of them become constants in the generated `config.php`
(`GENERATE_PASSWORD`, `DB_*`, `AUTH_SECRET`, `QR_PROXY_SECRET`, `MAIL_*`, `SMTP_*`, `ALLOWED_ORIGINS`); quotes and
backslashes in values are escaped by the generator. The API's version: `composer.json` `version` + the
deploy's `version.json` (git sha) → `GET /api/health/`.
