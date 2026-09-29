# Configuration

Every setting has a default in `config/settings.php`. Per installation, a value comes from (first non-empty wins):

1. the environment variable `QR_<KEY>` (e.g. `QR_DB_SQLITE_PATH` – `composer test` points the servers at a DB copy),
2. the PHP constant `<KEY>` from `config.local.php` (local, gitignored) or `config.php` (production, written by the
   deploy workflow – never edit it on the server),
3. the default.

Relative paths are relative to the server folder. Secrets never go into the repository.

| Key | Default | Notes |
|---|---|---|
| `DB_SQLITE_PATH` | – | set → SQLite (local, tests); empty → MySQL/MariaDB with the next keys |
| `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASS` | localhost, –, –, –, – | production |
| `AUTH_SECRET` | – | **required**, ≥ 32 chars: HMAC of login codes, IP/device hashes, access tokens. Rotating it invalidates open login mails and access tokens |
| `GENERATE_PASSWORD` | – | legacy `generate/` SPA password (its token = HMAC of it); empty = the SPA is locked |
| `QR_HOST` | `qr.scutoons.com` | host of the public pages (`HostRouting`) |
| `API_HOST` | `api.scutoons.com` | host of the API only |
| `QR_HOST_SERVES_API` | `1` | **transition flag**: the full `/api/*` also answers on `QR_HOST` (released viewers call `https://qr.scutoons.com/api/…`). Set `0` only when every app uses `API_HOST` and the stats show no API traffic on the qr host any more |
| `QR_URL` | `https://qr.scutoons.com` | magic links from the landing page |
| `APP_URL` | `https://scutoons.com` | the viewer: old magic links are forwarded there; "Open SCUTOONS" in mails |
| `CDN_URL` | `https://cdn.scutoons.com/` | item icons `<CDN_URL><icon>`; local `http://localhost:8001/` |
| `ALLOWED_ORIGINS` | scutoons.com, www, creator, admin, shop + dev ports | comma separated; production sets it in the workflow (without localhost) |
| `MAIL_TRANSPORT` | `smtp` | `smtp` \| `log` (dev: `MAIL_LOG_PATH`) \| `mail` (PHP `mail()`) |
| `MAIL_FROM`, `MAIL_FROM_NAME` | `no-reply@scutoons.com`, `SCUTOONS` | |
| `MAIL_LOG_PATH` | `logs/mail.log` | for `log` |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `SMTP_USER`, `SMTP_PASS`, `SMTP_TIMEOUT` | –, 587, `tls`, –, –, 15 | `SMTP_SECURE`: `tls` (STARTTLS) / `ssl` (465) / `none` |
| `ACCESS_MAX_PER_IP_HOUR` | 120 | `/access/` + `/code/` lookups per IP and hour |
| `LOGIN_TTL_MINUTES`, `LOGIN_MAX_ATTEMPTS` | 30, 5 | validity of link + code; wrong codes per mail |
| `LOGIN_MAX_PER_EMAIL_HOUR`, `LOGIN_MAX_PER_IP_HOUR` | 5, 30 | login mails per hour |
| `SESSION_TTL_DAYS` | 365 | sessions end after this long unused |
| `GRANT_MAX_PER_HOUR`, `IMPORT_MAX_PER_HOUR`, `IMPORT_MAX_QTY` | 120, 10, 99 | |
| `PROGRESS_MAX_BYTES`, `CONTENT_MAX_BYTES` | 262144, 2097152 | |
| `ADMIN_MAX_PAGE_SIZE` | 500 | largest `page_size` of the admin lists |
| `ITEMS_PATH`, `EDITIONS_PATH`, `TOONS_PATH` | `config/*.json` | catalog data |
| `PRODUCTS_PATH`, `FEATURED_PRODUCTS_PATH` | `config/products.json`, `config/featured-products.json` | shop catalog + homepage featured products, served as stored |
| `PERMISSIONS_PATH` | `config/permissions.json` | what each code type grants (server-only) – see docs/API.md |
| `FILES_PATH` | `storage/files` | batch manifests, QR SVGs, certificates, signatures, ZIPs (was `generate/files/`) |
| `KEYPAIR_PATH` | `storage/keys/keypair.json` | ML-DSA-65 keypair (was `api/keys/keypair.json`) |
| `LOG_PATH` | `logs/error.log` | PHP/API error log; empty = the host's log |

GitHub settings of `.github/workflows/server.yml` (names only; same names as the old qr.scutoons.com workflow):
secrets `AUTH_SECRET`, `GENERATE_PASSWORD`, `DB_USER`, `DB_PASS`, `SMTP_USER`, `SMTP_PASS`, `FTP_USERNAME`,
`FTP_PASSWORD`; variables `DB_HOST`, `DB_NAME`, `MAIL_FROM`, `SMTP_HOST`, `SMTP_PORT`, `SMTP_SECURE`, `FTP_SERVER`,
optional `SERVER_FTP_DIR` (default `./`). Values must not contain a single quote (they are written into PHP strings).
