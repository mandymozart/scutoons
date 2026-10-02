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
    // ── Database: SQLite when DB_SQLITE_PATH is set (local), else MySQL/MariaDB (production) ──────────────
    'DB_SQLITE_PATH' => '',
    'DB_HOST' => 'localhost',
    'DB_PORT' => '',
    'DB_NAME' => '',
    'DB_USER' => '',
    'DB_PASS' => '',

    // ── Secrets ────────────────────────────────────────────────────────────────────────────────────────
    // Long random string (>= 32 chars): login codes, IP/device hashes, access tokens. openssl rand -hex 32
    'AUTH_SECRET' => '',
    // Password of the legacy generate/ SPA (static admin token, until the admin app replaces it)
    'GENERATE_PASSWORD' => '',
    // Shared secret of the qr app (qr.scutoons.com, calls the API server-side): only requests carrying it in
    // X-Scutoons-Proxy-Secret may forward the client IP / device id (Http\Middleware\DeviceId). Empty = trust nobody.
    // Same value as QR_PROXY_SECRET in the qr app's config. openssl rand -hex 32
    'QR_PROXY_SECRET' => '',

    // ── URLs ───────────────────────────────────────────────────────────────────────────────────────────
    // The qr app (magic links of login requests without an Origin; it forwards /?login= to the viewer) and the viewer
    'QR_URL' => 'https://qr.scutoons.com',
    'APP_URL' => 'https://scutoons.com',
    // The admin app (links in the admin notification mails, e.g. a new ownership claim)
    'ADMIN_URL' => 'https://admin.scutoons.com',
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
    'PROGRESS_MAX_BYTES' => '262144',
    'CONTENT_MAX_BYTES' => '2097152',
    // Admin lists: largest page_size
    'ADMIN_MAX_PAGE_SIZE' => '500',
    // GET /api/admin/versions: the version.json of every deployed app (app=url, comma separated)
    'VERSION_URLS' => 'viewer=https://scutoons.com/version.json,admin=https://admin.scutoons.com/version.json,'
        . 'creator=https://creator.scutoons.com/version.json,shop=https://shop.scutoons.com/version.json,'
        . 'qr=https://qr.scutoons.com/version.json',

    // ── Avatar bodies ──────────────────────────────────────────────────────────────────────────────────
    // Characters (with their A-pose looks) every member may wear as the reader's avatar body, without per-member rows
    // (Domain\Characters\CharacterUnlocks). JSON { "<character>": ["<look>", …] }. Owner 2026-09-30: Cat Pope, swim + naked.
    'AVATAR_DEFAULT_UNLOCKS' => '{"catpope":["swim","naked"]}',

    // ── Data files (config/) and private storage ───────────────────────────────────────────────────────
    'ITEMS_PATH' => 'config/items.json',
    'EDITIONS_PATH' => 'config/editions.json',
    'TOONS_PATH' => 'config/toons.json',
    // Shop catalog + homepage featured products (GET /api/products/, /api/products/featured/)
    'PRODUCTS_PATH' => 'config/products.json',
    'FEATURED_PRODUCTS_PATH' => 'config/featured-products.json',
    // Composed answers of the library endpoints /api/library/* (agents/DATA.md): auto (APCu when enabled, else files
    // in CACHE_PATH) | apcu | file | none. The keys contain the version ids, so publishing needs no invalidation.
    'CACHE_DRIVER' => 'auto',
    'CACHE_PATH' => 'storage/cache',
    // 1 = toons / chapters with the status `draft` are readable through the public library endpoints (local
    // development, like the viewer's dev mode); production 0. Creators read drafts with ?draft=1 either way.
    'CONTENT_SHOW_DRAFTS' => '0',
    // What each code type grants (server-only, never served)
    'PERMISSIONS_PATH' => 'config/permissions.json',
    // Batch manifests, QR SVGs, certificates, signatures, ZIPs (was generate/files/)
    'FILES_PATH' => 'storage/files',
    // ML-DSA-65 keypair of the certificate signatures (was api/keys/keypair.json)
    'KEYPAIR_PATH' => 'storage/keys/keypair.json',

    // ── Media assets (agents/ASSETS.md): the asset registry + its storage drivers ─────────────────────────
    // Which storage the asset layer works on: auto (the local folder ASSET_LOCAL_ROOT when it exists – every dev
    // machine –, else bunny when configured) | local | bunny
    'ASSET_STORAGE' => 'auto',
    // The local CDN folder (what `python cdn/server.py` serves and `cdn/sync.py` uploads): dev storage
    'ASSET_LOCAL_ROOT' => '../cdn/root',
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
];
