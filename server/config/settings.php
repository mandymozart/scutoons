<?php
declare(strict_types=1);

/**
 * Defaults of every setting (Scutoons\Infrastructure\Config\Settings). Override per installation with a constant in
 * config.local.php (local) / config.php (production, written by the deploy workflow) or an environment variable
 * QR_<KEY>. Relative paths are relative to the server folder. Documented in docs/CONFIGURATION.md.
 *
 * @return array<string, string>
 */
return [
    // ── Database: MariaDB (production; locally the Docker container – npm run dev:db sets QR_DB_* for the API and the
    // console, docs/DATABASE.md "Local MariaDB")
    'DB_HOST' => 'localhost',
    'DB_PORT' => '',
    'DB_NAME' => '',
    'DB_USER' => '',
    'DB_PASS' => '',
    // The session sql_mode the API sets on every MySQL/MariaDB connection, so behaviour does not depend on the host's
    // default (production: NO_ENGINE_SUBSTITUTION = not strict: a too long value is cut, a bad one coerced, silently).
    // Strict = such a write fails with an error instead. `server` = keep the server's mode.
    'DB_SQL_MODE' => 'STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION',
    // The connection collation (SET NAMES utf8mb4 COLLATE …): the tables' own, so string literals and user variables
    // in queries and migrations compare without "Illegal mix of collations" (#1267). `server` = keep the server's.
    'DB_COLLATION' => 'utf8mb4_general_ci',

    // ── Secrets ────────────────────────────────────────────────────────────────────────────────────────
    // Long random string (>= 32 chars): login codes, IP/device hashes, access tokens. openssl rand -hex 32
    'AUTH_SECRET' => '',
    // Password of the legacy generate/ SPA (static admin token, until the admin app replaces it)
    // Shared secret of the qr app (qr.scutoons.com, calls the API server-side): only requests carrying it in
    // X-Scutoons-Proxy-Secret may forward the client IP / device id (Http\Middleware\DeviceId). Empty = trust nobody.
    // Same value as QR_PROXY_SECRET in the qr app's config. openssl rand -hex 32
    'QR_PROXY_SECRET' => '',
    // Key of the Embassy cron URL (GET|POST /api/embassy/cron?key=… – inbox sync + reminder mails; the host's cron job
    // calls it daily, docs/API.md "Embassy" → "Pace"). Also accepted as the HTTP Basic password. Empty = the URL answers
    // 404 (off). openssl rand -hex 32
    'EMBASSY_CRON_KEY' => '',

    // ── URLs ───────────────────────────────────────────────────────────────────────────────────────────
    // The qr app (magic links of login requests without an Origin; it forwards /?login= to the viewer) and the viewer
    'QR_URL' => 'https://qr.scutoons.com',
    'APP_URL' => 'https://scutoons.com',
    // The admin app (links in the admin notification mails, e.g. a new ownership claim)
    'ADMIN_URL' => 'https://admin.scutoons.com',
    // The creator app (links in the review mails, Domain\Content\Reviews)
    'CREATOR_URL' => 'https://creator.scutoons.com',
    // The Embassy app (the Today link in the reminder mails, Domain\Embassy\EmbassyReminders). Local: http://localhost:13000
    'EMBASSY_URL' => 'https://embassy.scutoons.com',
    // Item icons: <CDN_URL><icon path>. Local CDN: http://localhost:8001/
    'CDN_URL' => 'https://cdn.scutoons.com/',
    // Browser origins that may call the API with a Bearer token or the session cookie (comma separated): viewer, creator,
    // admin, shop, embassy + dev ports. Also the CSRF check of the cookie (docs/API.md "Single sign-on")
    'ALLOWED_ORIGINS' => 'https://scutoons.com,https://www.scutoons.com,https://creator.scutoons.com,https://admin.scutoons.com,https://shop.scutoons.com,https://embassy.scutoons.com,'
        . 'http://localhost:8000,http://localhost:10000,http://localhost:11000,http://localhost:12000,http://localhost:13000,'
        . 'http://localhost:5173,http://localhost:5174,http://localhost:5175,http://localhost:4173',

    // ── Mail ───────────────────────────────────────────────────────────────────────────────────────────
    // smtp (production) | log (development: mails appended to MAIL_LOG_PATH) | mail (PHP mail())
    'MAIL_TRANSPORT' => 'smtp',
    'MAIL_FROM' => 'no-reply@scutoons.com',
    'MAIL_FROM_NAME' => 'SCUTOONS',
    // where replies go (Reply-To; the texts say "reply to this email") – empty = no Reply-To header
    'MAIL_REPLY_TO' => 'hello@scutoons.com',
    // the contact form's mailbox (Domain\Contact\ContactForm); empty = MAIL_REPLY_TO
    'CONTACT_TO' => '',
    'MAIL_LOG_PATH' => 'logs/mail.log',
    // The email module (agents/PLATFORM.md "Emails"): admin notification recipients (comma separated; the default of
    // the runtime setting mail_admin_recipients, admin app → Emails), their language, and how long the sent-mail log
    // (table mail_log, recipient addresses) is kept – 0 = forever
    'MAIL_ADMIN_TO' => '',
    'MAIL_ADMIN_LOCALE' => 'de',
    'MAIL_LOG_DAYS' => '180',
    'SMTP_HOST' => '',
    'SMTP_PORT' => '587',
    // tls = STARTTLS (587), ssl = implicit TLS (465), none = plain (local test servers only)
    'SMTP_SECURE' => 'tls',
    'SMTP_USER' => '',
    'SMTP_PASS' => '',
    'SMTP_TIMEOUT' => '15',

    // ── Newsletter (agents/NEWSLETTER.md; Domain\Newsletter; admin app → Emails → Newsletters) ────────────────
    // Mass mails go out in small batches – from the host's cron (GET|POST /api/newsletter/cron?key=NEWSLETTER_CRON_KEY,
    // e.g. every 5 minutes; empty key = the URL answers 404) or "Send now" in the admin – through the SMTP account above,
    // one connection per batch. PER_HOUR = the default of the runtime setting newsletter_per_hour (admin → Settings):
    // every mail sent in the last hour counts (mail_log). One batch: at most BATCH mails (the default of the runtime
    // setting newsletter_batch, 1–25), DELAY_MS between two, SECONDS
    // in all (below the host's max_execution_time), a new connection every PER_CONNECTION mails. A temporary failure is
    // tried again after RETRY_MINUTES, at most ATTEMPTS times. API_URL = where the unsubscribe links point.
    'NEWSLETTER_CRON_KEY' => '',
    // Team push (research/PWA-APPS.md §5): the drafts kind runs from the host's cron, GET|POST /api/push/cron?key=… every
    // 5 minutes (also the HTTP Basic password); empty = the URL answers 404 and no drafts are told about
    'PUSH_CRON_KEY' => '',
    'NEWSLETTER_PER_HOUR' => '200',
    'NEWSLETTER_BATCH' => '20',
    'NEWSLETTER_DELAY_MS' => '1000',
    'NEWSLETTER_SECONDS' => '25',
    'NEWSLETTER_PER_CONNECTION' => '50',
    'NEWSLETTER_ATTEMPTS' => '3',
    'NEWSLETTER_RETRY_MINUTES' => '15',
    // the `{signin}` button's personal login link (Domain\Newsletter\NewsletterSignin, #65): valid this many days, once
    'NEWSLETTER_SIGNIN_DAYS' => '14',
    'API_URL' => 'https://api.scutoons.com',

    // ── Web Push (agents/research/PWA.md §2b; Domain\Push) ─────────────────────────────────────────────────
    // The VAPID key pair (base64url: public = the 65-byte point, private = the 32-byte scalar) – `php bin/console
    // push:vapid-keys` prints a new pair. Never in git: GitHub secrets → the deploy's config.php. Empty = push is off
    // (GET /api/push/key answers null, the reader app shows no opt-in). SUBJECT = the contact push services may write to.
    'VAPID_PUBLIC_KEY' => '',
    'VAPID_PRIVATE_KEY' => '',
    'VAPID_SUBJECT' => 'mailto:hello@scutoons.com',

    // ── Embassy mail (agents/EMBASSY.md "Communication strategy"; docs/API.md "Embassy" → "Mail" / "Inbox") ───
    // People send one-to-one mails from the Embassy app (POST /api/embassy/orgs/{id}/mail) from this shared mailbox,
    // through MAIL_TRANSPORT and the SMTP server in SMTP_HOST / _PORT / _SECURE; Reply-To is the same address. The
    // name shows as the sender ("Top Toy · SCUTOONS").
    'EMBASSY_MAIL_FROM' => 'hello@scutoons.com',
    'EMBASSY_MAIL_FROM_NAME' => 'Top Toy · SCUTOONS',
    // The SMTP account of that mailbox; empty = SMTP_USER / SMTP_PASS
    'EMBASSY_SMTP_USER' => '',
    'EMBASSY_SMTP_PASS' => '',
    // The inbox sync (POST /api/embassy/inbox/sync, bin/console embassy:inbox): replies are read back by IMAP (implicit
    // TLS) and logged on the organisation they answer. Empty host = no sync. User / password empty = the Embassy's SMTP
    // account. FOLDER = where replies arrive; SENT = a folder that gets a copy of every mail sent from the app (e.g.
    // "Sent", so the shared mailbox shows it too); empty = no copy.
    'EMBASSY_IMAP_HOST' => '',
    'EMBASSY_IMAP_PORT' => '993',
    'EMBASSY_IMAP_USER' => '',
    'EMBASSY_IMAP_PASS' => '',
    'EMBASSY_IMAP_FOLDER' => 'INBOX',
    'EMBASSY_IMAP_SENT' => '',

    // ── Limits ─────────────────────────────────────────────────────────────────────────────────────────
    // GET/POST /access/ + /code/ lookups per IP and hour (every attempt counts – against code enumeration); 0 = no limit.
    // Default of the runtime setting lookup_limit_per_hour (admin app → Settings; a stored value wins)
    'ACCESS_MAX_PER_IP_HOUR' => '120',
    // QR scans per client IP and hour (GET /code/?scan=1 from the qr app, trusted proxy; own bucket). Events share one
    // IP, so keep it high; 0 = no limit. Default of the runtime setting scan_limit_per_hour (admin app → Settings)
    'SCAN_MAX_PER_IP_HOUR' => '1000',
    'LOGIN_TTL_MINUTES' => '30',
    'LOGIN_MAX_ATTEMPTS' => '5',
    'LOGIN_MAX_PER_EMAIL_HOUR' => '5',
    'LOGIN_MAX_PER_IP_HOUR' => '30',
    'SESSION_TTL_DAYS' => '365',
    // Single sign-on cookie `scutoons_sid` (Http\SessionCookie): `Domain=` this when the API host is it or a subdomain
    // (api.scutoons.com, qr.scutoons.com/api) – every app on *.scutoons.com shares the session; other hosts (localhost)
    // get a host-only cookie. Empty = always host-only.
    'SESSION_COOKIE_DOMAIN' => 'scutoons.com',
    'GRANT_MAX_PER_HOUR' => '120',
    'IMPORT_MAX_PER_HOUR' => '10',
    'IMPORT_MAX_QTY' => '99',
    // Ownership claims + gifting (agents/ITEMS.md "Ownership, claims, gifting"): claims a reader may file per 24 h, evidence
    // files per claim and their size (bytes); gifts a reader may send per 24 h, recipient lookups per 24 h (unknown
    // addresses count – against probing which emails have an account), the re-gift cooldown of a received item (hours)
    'CLAIM_MAX_PER_DAY' => '5',
    'CLAIM_FILES_MAX' => '5',
    'CLAIM_FILE_MAX_BYTES' => '5242880',
    'GIFT_MAX_PER_DAY' => '10',
    'GIFT_LOOKUPS_PER_DAY' => '30',
    'GIFT_COOLDOWN_HOURS' => '24',
    // Gift invites (agents/research/GIFT-INVITES.md): the key of GET /api/invites/cron (the host's daily cronjob returns
    // the invites that ran out); empty = no cron URL (they are returned on the next visit / admin view anyway)
    'CAMPAIGN_CRON_KEY' => '',
    'PROGRESS_MAX_BYTES' => '262144',
    'CONTENT_MAX_BYTES' => '2097152',
    // Admin lists: largest page_size
    'ADMIN_MAX_PAGE_SIZE' => '500',
    // GET /api/admin/versions: the version.json of every deployed app (app=url, comma separated)
    'VERSION_URLS' => 'viewer=https://scutoons.com/version.json,admin=https://admin.scutoons.com/version.json,'
        . 'creator=https://creator.scutoons.com/version.json,shop=https://shop.scutoons.com/version.json,'
        . 'qr=https://qr.scutoons.com/version.json',

    // ── Avatar bodies ──────────────────────────────────────────────────────────────────────────────────
    // Characters every member may wear as the reader's avatar body (each with its default body – the regular persona's
    // default look), without per-member rows (Domain\Characters\CharacterUnlocks). JSON ["<character>", …]. Owner
    // 2026-10-05: Cat Pope.
    'AVATAR_DEFAULT_UNLOCKS' => '["catpope"]',

    // ── Caches and private storage (the catalog is in the database: agents/CATALOG.md) ───────────────────
    // Composed answers of the library endpoints /api/library/* (agents/DATA.md): auto (APCu when enabled, else files
    // in CACHE_PATH) | apcu | file | none. The keys contain the version ids, so publishing needs no invalidation.
    'CACHE_DRIVER' => 'auto',
    'CACHE_PATH' => 'storage/cache',
    // The developer's migration runs (BACKLOG 10.61) back up the rows each UPDATE / DELETE changes here (JSON lines,
    // one file per run) – private, never overwritten by a deploy.
    'MIGRATION_BACKUP_PATH' => 'storage/migration-backups',
    // The migration files the developer's runs plan and apply (tests point it elsewhere)
    'MIGRATIONS_PATH' => 'migrations',
    // POST /api/deploy/migrate with the header `X-Migrate-Key: <this>` runs the pending migrations (the deploy calls it
    // right after the API upload – GitHub secret MIGRATE_KEY, >= 32 random chars). Empty = switched off (404).
    'MIGRATE_KEY' => '',
    // The venue relay (#201, Domain\Lan): all-inkl's KAS API sets <name>.lan.scutoons.com (KAS login w0… + its
    // password, GitHub secrets KAS_LOGIN / KAS_AUTH_DATA); empty = /api/admin/lan-names answers 503 not_configured.
    'KAS_LOGIN' => '',
    'KAS_AUTH_DATA' => '',
    // The *.lan.scutoons.com certificate the workflow lan-cert.yml stores (PUT /api/deploy/lan-cert) – private
    'LAN_CERT_PATH' => 'storage/lan-cert',
    // 1 = toons / chapters with the status `draft` are readable through the public library endpoints (local
    // development, like the viewer's dev mode); production 0. Creators read drafts with ?draft=1 either way.
    'CONTENT_SHOW_DRAFTS' => '0',
    // Batch manifests, QR SVGs, certificates, signatures, ZIPs (was generate/files/)
    'FILES_PATH' => 'storage/files',
    // The member's MCP server bundle (GET /api/mcp/download, /version – agents/MCP.md §3a): the api deploy builds it here
    'MCP_PATH' => 'mcp',
    // ML-DSA-65 keypair of the certificate signatures (was api/keys/keypair.json)
    'KEYPAIR_PATH' => 'storage/keys/keypair.json',
    // The storyboard's shared work storage (#120, agents/research/STUDIO-IN-CREATOR.md §2.2): attempts members share for
    // review, <toon>/<chapter>/<name> on the API host's own disk – never the readers' CDN, served to creators only;
    // emptied per chapter after its release (DELETE /api/admin/storyboard/work/<toon>/<chapter>)
    'STORYBOARD_WORK_PATH' => 'storage/storyboard-work',
    // Largest work file through the API in bytes (a raw PUT body: PHP's post_max_size doesn't apply; the host's
    // request body limit and the disk do)
    'STORYBOARD_WORK_MAX_BYTES' => '268435456',

    // ── Storyboard generators (#121, docs/API.md "Storyboard" → "Provider jobs"; Domain\Storyboard\Generators) ──
    // The paid providers' API keys – secrets: GitHub secrets → the deploy's config.php (deploy/targets.json), or
    // QR_<KEY> in the environment. Never the repository-root .env (it is not read for these), never in an answer: a
    // provider without its key is listed as `configured: false` with the key's name. _URL = the provider's API base
    // (the tests point it at a stand-in).
    'LUMA_API_KEY' => '',
    'LUMA_API_URL' => 'https://agents.lumalabs.ai',
    'RUNWAY_API_KEY' => '',
    'RUNWAY_API_URL' => 'https://api.dev.runwayml.com',
    'GEMINI_API_KEY' => '',
    // The default of the runtime setting generation_budget (admin → Settings): whole euros all paid generations may
    // cost together (spent + reserved + the next estimate ≤ budget, else 402 budget_exhausted). 0 = no budget = every
    // paid job is refused (402 no_budget) – the owner sets it in the admin.
    'GENERATION_BUDGET' => '0',
    // The ECB's daily euro reference rates (prices are shown in euro – DECISIONS 2026-10-20): fetched at most once a
    // day, cached; empty = never fetched (the fixed fallback rate of Domain\Storyboard\EuroRates applies)
    'ECB_RATES_URL' => 'https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml',
    // How long a signed reference link (a work-storage file handed to a provider) works, in minutes
    'STORYBOARD_REF_MINUTES' => '15',
    // A provider job's status is asked for at most once per this many seconds (GET /storyboard/generations[/{id}])
    'GENERATION_POLL_SECONDS' => '5',

    // ── Media assets (agents/ASSETS.md): the asset registry + its storage drivers ─────────────────────────
    // Which storage the asset layer works on: auto (the local folder ASSET_LOCAL_ROOT when it exists – every dev
    // machine –, else bunny when configured) | local | bunny
    'ASSET_STORAGE' => 'auto',
    // The local media folder (what scripts/dev/media-serve.mjs serves on the dev CDN port): dev storage
    'ASSET_LOCAL_ROOT' => '../media',
    // The public media base readers load (purge URLs, links in the asset manager); empty = CDN_URL
    'CDN_PUBLIC_URL' => '',
    // bunny.net (all optional; these are also read from plain env vars and the repository-root .env –
    // docs/CONFIGURATION.md). ENDPOINT = the zone's region hostname (default storage.bunnycdn.com); PASSWORD = the zone's
    // read/write password (uploads, replace, delete), READONLY_PASSWORD = its read-only password (every list / read);
    // PULLZONE_ID = the numeric pull zone id; BUNNY_API_KEY = the account API key (purge, statistics, request logs –
    // empty switches those off)
    'BUNNY_STORAGE_ZONE' => '',
    'BUNNY_STORAGE_ENDPOINT' => '',
    'BUNNY_STORAGE_PASSWORD' => '',
    'BUNNY_STORAGE_READONLY_PASSWORD' => '',
    'BUNNY_PULLZONE_ID' => '',
    // Protected files (agents/research/SCORES.md §5 – scores' MusicXML / PDF): their own bunny.net storage + pull zone
    // with token authentication. URL = that pull zone's base (https://<zone>.b-cdn.net or a custom host), KEY = its
    // token authentication key (secret). Empty KEY = signed URLs fall back to CDN_URL (dev: the local media server).
    'PROTECTED_CDN_URL' => '',
    // ... and its storage zone, where the catalog editor's score uploads go (PUT /api/assets/protected): the zone name
    // (variable) + its read/write password (secret). Unset on a dev machine = the local media folder; unset on
    // production = uploads answer 503 (never the public storage)
    'BUNNY_PROTECTED_STORAGE_ZONE' => '',
    'BUNNY_PROTECTED_STORAGE_PASSWORD' => '',
    'BUNNY_PROTECTED_CDN_KEY' => '',
    'BUNNY_API_KEY' => '',
    // Only keys below this prefix may be written / deleted on the zone (e.g. "_test/" while trying the driver); '' = all
    'BUNNY_WRITE_PREFIX' => '',
    // Largest upload through the API in bytes (the PHP limits apply too); bigger files: bin/console assets:upload
    'ASSET_UPLOAD_MAX_BYTES' => '209715200',
    // Optimization jobs (bin/console assets:jobs): the ffmpeg / ffprobe executables
    'FFMPEG_PATH' => 'ffmpeg',
    'FFPROBE_PATH' => 'ffprobe',

    // ── Logs ───────────────────────────────────────────────────────────────────────────────────────────
    // Error log (logs/ is never served); empty = the host's log
    'LOG_PATH' => 'logs/error.log',
    // the qr app's error log, read by the admin's "Server errors" page (qr/public/index.php writes it; api/ and qr/ are
    // siblings in production as in the repo); empty = not shown
    'QR_LOG_PATH' => '../qr/logs/error.log',
];
