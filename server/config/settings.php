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
    // Item icons: <CDN_URL><icon path>. Local CDN: http://localhost:8001/
    'CDN_URL' => 'https://cdn.scutoons.com/',
    // Browser origins that may call the API with a Bearer token (comma separated): viewer, creator, admin + dev ports
    'ALLOWED_ORIGINS' => 'https://scutoons.com,https://www.scutoons.com,https://creator.scutoons.com,https://admin.scutoons.com,https://shop.scutoons.com,https://embassy.scutoons.com,'
        . 'http://localhost:8000,http://localhost:10000,http://localhost:11000,'
        . 'http://localhost:5173,http://localhost:5174,http://localhost:5175,http://localhost:4173',

    // ── Mail ───────────────────────────────────────────────────────────────────────────────────────────
    // smtp (production) | log (development: mails appended to MAIL_LOG_PATH) | mail (PHP mail())
    'MAIL_TRANSPORT' => 'smtp',
    'MAIL_FROM' => 'no-reply@scutoons.com',
    'MAIL_FROM_NAME' => 'SCUTOONS',
    'MAIL_LOG_PATH' => 'logs/mail.log',
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
    'GRANT_MAX_PER_HOUR' => '120',
    'IMPORT_MAX_PER_HOUR' => '10',
    'IMPORT_MAX_QTY' => '99',
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

    // ── Logs ───────────────────────────────────────────────────────────────────────────────────────────
    // Error log (logs/ is never served); empty = the host's log
    'LOG_PATH' => 'logs/error.log',
];
