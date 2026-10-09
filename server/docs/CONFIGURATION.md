# Configuration

Every setting has a default in `config/settings.php`. Per installation, a value comes from (first non-empty wins):

1. the environment variable `QR_<KEY>` (e.g. `QR_DB_NAME` – `npm run dev:db` points the API and the console at the local database),
2. **only for the media storage keys** `BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT`, `BUNNY_STORAGE_PASSWORD`,
   `BUNNY_STORAGE_READONLY_PASSWORD`, `BUNNY_PULLZONE_ID`, `BUNNY_API_KEY`, `CDN_PUBLIC_URL` (`Settings::DOTENV_KEYS`): the plain environment variable
   `<KEY>`, then the same key in the **repository-root `.env`** (gitignored; the owner's deploy checklist, `.env.example`).
   Every other line of that file is ignored – it holds the PRODUCTION `DB_*`, `FTP_*`, `AUTH_SECRET`, `SMTP_*`, and a dev
   server must never pick those up (`InfrastructureTest::testRootDotEnvOnlyFeedsTheStorageKeys`). The real environment
   wins over the file; nothing from it is ever printed or logged. Production has no root `.env` (the deploy writes
   `config.php`),
3. the PHP constant `<KEY>` from `config.local.php` (local, gitignored) or `config.php` (production, written by the
   deploy workflow – never edit it on the server),
4. the default.

Relative paths are relative to the server folder. Secrets never go into the repository.

**Runtime settings** (table `settings`, admin app → Settings, `GET/PUT /api/admin/settings`): a few values the owner
changes without a deploy. A stored value wins over the config; without a row the config value below is the default.
Today: `scan_limit_per_hour` (default `SCAN_MAX_PER_IP_HOUR`), `lookup_limit_per_hour` (default
`ACCESS_MAX_PER_IP_HOUR`). `Domain\Settings\RuntimeSettings`.

| Key | Default | Notes |
|---|---|---|
| `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASS` | localhost, –, –, –, – | MariaDB: production's; locally the container (`config.local.php.example`; the dev scripts pass `QR_DB_*` – docs/DATABASE.md "Local MariaDB") |
| `DB_SQL_MODE` | `STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION` | the session `sql_mode` set on every MySQL/MariaDB connection – strict, whatever the host's default (production's is `NO_ENGINE_SUBSTITUTION`: too long values would be cut silently); `server` = keep the server's mode (docs/DATABASE.md) |
| `AUTH_SECRET` | – | **required**, ≥ 32 chars: HMAC of login codes, IP/device hashes, access tokens. Rotating it invalidates open login mails and access tokens |
| `QR_PROXY_SECRET` | – | shared with the qr app (qr.scutoons.com): only requests carrying it in `X-Scutoons-Proxy-Secret` may forward the client IP (`X-Scutoons-Client-Ip`) and device id (`X-Scutoons-Device-Id`), and only they can log landing-page scans (`/api/code/?scan=1`). Empty = trust nobody |
| `EMBASSY_CRON_KEY` | – | key of the Embassy cron URL `GET\|POST /api/embassy/cron?key=…` (inbox sync + the team's reminder mails; also accepted as the HTTP Basic password – docs/API.md "Embassy" → "Pace"). Empty = the URL answers 404 (off). `openssl rand -hex 32` |
| `NEWSLETTER_CRON_KEY` | – | key of the newsletter cron URL `GET\|POST /api/newsletter/cron?key=…` (one batch, agents/NEWSLETTER.md; also the HTTP Basic password). Empty = 404 (off). `openssl rand -hex 32` |
| `QR_URL` | `https://qr.scutoons.com` | the qr app: magic links of login requests without an Origin (it forwards `/?login=` to the viewer) |
| `APP_URL` | `https://scutoons.com` | the viewer: old magic links are forwarded there; "Open SCUTOONS" in mails |
| `ADMIN_URL` | `https://admin.scutoons.com` | the admin app: links in admin notification mails (a new ownership claim); the link of an AI proposal (`/proposals/<id>`) |
| `EMBASSY_URL` | `https://embassy.scutoons.com` | the Embassy app: the Today link in the team's reminder mails (`embassy_portion`); the link of a mail draft (`/orgs/<id>?draft=<id>`). Local: `http://localhost:13000` in config.local.php |
| `CDN_URL` | `https://cdn.scutoons.com/` | item icons `<CDN_URL><icon>`; local `http://localhost:8001/` |
| `ALLOWED_ORIGINS` | scutoons.com, www, creator, admin, shop, embassy + dev ports | comma separated; production sets it in the workflow (without localhost). On a local dev server (`php -S`) every port of `shared/dev-ports.json` is added automatically (`Settings::allowedOrigins`). Also the CSRF list of the session cookie: only these origins may change things with it |
| `MAIL_TRANSPORT` | `smtp` | `smtp` \| `log` (dev: `MAIL_LOG_PATH`) \| `mail` (PHP `mail()`) |
| `MAIL_FROM`, `MAIL_FROM_NAME` | `no-reply@scutoons.com`, `SCUTOONS` | |
| `MAIL_REPLY_TO` | `hello@scutoons.com` | Reply-To of every mail (the texts say "reply to this email"); empty = none |
| `CONTACT_TO` | – | mailbox of the contact form (`POST /api/contact`, scutoons.com/kontakt); empty = `MAIL_REPLY_TO` |
| `MAIL_LOG_PATH` | `logs/mail.log` | for `log` |
| `MAIL_ADMIN_TO` | – | admin notification recipients (comma separated) – the default of the runtime setting `mail_admin_recipients` (admin app → Emails); empty = nobody |
| `MAIL_ADMIN_LOCALE` | `de` | language of the admin notification mails |
| `MAIL_LOG_DAYS` | 180 | days the sent-mail log (`mail_log`, with recipient addresses) is kept; 0 = forever |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_USER`, `SMTP_PASS`, `SMTP_TIMEOUT` | –, 587, `tls`, –, –, 15 | `SMTP_SECURE`: `tls` (STARTTLS) / `ssl` (465) / `none` |
| `NEWSLETTER_PER_HOUR` | 200 | default of the runtime setting `newsletter_per_hour` (admin → Settings): at most this many mails per hour while a newsletter sends – every mail of the last hour counts (`mail_log`), login codes too; 0 = no limit |
| `NEWSLETTER_BATCH`, `NEWSLETTER_DELAY_MS`, `NEWSLETTER_SECONDS`, `NEWSLETTER_PER_CONNECTION` | 20, 1000, 25, 50 | one newsletter batch: at most BATCH mails (the default of the runtime setting `newsletter_batch`, admin → Settings, 1–25), DELAY_MS between two, SECONDS in all (below max_execution_time), a new SMTP connection every PER_CONNECTION mails |
| `NEWSLETTER_ATTEMPTS`, `NEWSLETTER_RETRY_MINUTES` | 3, 15 | a temporary failure (not a 5xx at RCPT = bounce) is tried again after RETRY_MINUTES, failed after ATTEMPTS |
| `API_URL` | `https://api.scutoons.com` | where the newsletter's unsubscribe links point (`/api/newsletter/unsubscribe?t=…`) |
| `EMBASSY_MAIL_FROM`, `EMBASSY_MAIL_FROM_NAME` | `hello@scutoons.com`, `Top Toy · SCUTOONS` | sender (and Reply-To) of the mails people send from the Embassy app (docs/API.md "Embassy" → "Mail"); same transport and SMTP server as above |
| `EMBASSY_SMTP_USER`, `EMBASSY_SMTP_PASS` | – | the SMTP account of that mailbox; empty = `SMTP_USER` / `SMTP_PASS` |
| `EMBASSY_IMAP_HOST`, `EMBASSY_IMAP_PORT` | –, 993 | the Embassy inbox sync (implicit TLS; 143 = plain, local test servers only); empty host = no sync |
| `EMBASSY_IMAP_USER`, `EMBASSY_IMAP_PASS` | – | the IMAP account; empty = the Embassy's SMTP account |
| `EMBASSY_IMAP_FOLDER`, `EMBASSY_IMAP_SENT` | `INBOX`, – | where replies arrive; a folder that gets a copy of every mail sent from the app (e.g. `Sent`), empty = no copy |
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
| `CACHE_DRIVER`, `CACHE_PATH` | `auto`, `storage/cache` | server cache of the library answers (docs/API.md "Library"): `auto` = APCu when enabled, else files below `CACHE_PATH`; `apcu`, `file`, `none` |
| `CONTENT_SHOW_DRAFTS` | `0` | `1` = the library endpoints show `draft` toons/chapters (local development, like the dev viewer); production `0` |
| `FILES_PATH` | `storage/files` | batch manifests, QR SVGs, certificates, signatures, ZIPs (was `generate/files/`) |
| `KEYPAIR_PATH` | `storage/keys/keypair.json` | ML-DSA-65 keypair (was `api/keys/keypair.json`) |
| `LOG_PATH` | `logs/error.log` | PHP/API error log; empty = the host's log |
| `ASSET_STORAGE` | `auto` | the media storage of the asset layer (agents/ASSETS.md): `auto` = the local folder `ASSET_LOCAL_ROOT` when it exists (every dev machine – credentials in the root `.env` never make a dev server write to the production zone), else `bunny` when configured; `local`, `bunny` |
| `ASSET_LOCAL_ROOT` | `../media` | the local media folder (`scripts/dev/media-serve.mjs` serves it on :8001) |
| `CDN_PUBLIC_URL` | – (= `CDN_URL`) | the public media base (purge URLs, links in the asset manager), production `https://cdn.scutoons.com` (bunny pull zone; its own hostname `https://scutoons.b-cdn.net` serves the same files) (a missing `https://` is added); also from the root `.env` |
| `BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT` | –, – (= `storage.bunnycdn.com`) | the bunny.net storage zone: its name and its region hostname (zone → FTP & API Access → Hostname; with or without `https://`). Also from the root `.env`. `php bin/console assets:check-storage` checks them read-only |
| `BUNNY_STORAGE_PASSWORD`, `BUNNY_STORAGE_READONLY_PASSWORD` | –, – | the zone's passwords (FTP & API Access): the read-only one for every list / read / check, the read/write one only for uploads, replace, delete (missing = the zone is read-only for the asset layer). Also from the root `.env`; never printed |
| `BUNNY_PULLZONE_ID`, `BUNNY_API_KEY` | –, – | the NUMERIC pull zone id (not its name; anything else is reported as invalid) + the account API key: purge after a replace, statistics (bandwidth), request logs (per-file traffic). Empty = those are off (uploads still work). Also from the root `.env` |
| `BUNNY_WRITE_PREFIX` | – | only keys below it may be written / deleted on the zone (`_test/` while trying the driver against production); empty = all |
| `ASSET_UPLOAD_MAX_BYTES` | 209715200 | largest upload through the API (also mind PHP's `upload_max_filesize` / `post_max_size` – uploads are a raw `PUT` body, which `post_max_size` doesn't limit); bigger files: `bin/console assets:upload` |
| `FFMPEG_PATH`, `FFPROBE_PATH` | `ffmpeg`, `ffprobe` | the optimization jobs (`bin/console assets:jobs`: video renditions 480p / 720p / 1080p, posters) and probing (duration, size) |

Production values come from the GitHub environment `production` (the full list with comments: root `.env.example`);
`deploy/targets.json` → `api.generate["config.php"]` says which of them become constants in the generated `config.php`
(`DB_*`, `AUTH_SECRET`, `QR_PROXY_SECRET`, `MAIL_*`, `SMTP_*`, `EMBASSY_*`, `BUNNY_*`, `CDN_PUBLIC_URL`, `ALLOWED_ORIGINS`; an unset value is left out); quotes and
backslashes in values are escaped by the generator. The API's version: `composer.json` `version` + the
deploy's `version.json` (git sha) → `GET /api/health/`.
