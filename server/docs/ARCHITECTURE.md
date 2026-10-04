# Architecture

## Request flow

`public/index.php` (Apache: `public/.htaccess` sends every non-static request there; `php -S`: `public/router.php`)
→ `Scutoons\Kernel::app()` → middleware (outside in) → route → controller → domain service → JSON response.
This server is the API only (api.scutoons.com): the QR landing page and the verify page are the qr app (`../qr`,
qr.scutoons.com), which calls this API server-side and proxies the printed API URLs here. Non-API paths answer a
plain 404 (except the legacy `generate/` SPA until the admin app retires it).

| # | Middleware (`src/Http/Middleware/`) | What it does |
|---|---|---|
| 1 | `NormalizePath` | `/api/me/` = `/api/me` (routes are declared without the trailing slash) |
| 2 | `Cors` | `/api/*` for `ALLOWED_ORIGINS` (Bearer or the session cookie: `Allow-Credentials`); `OPTIONS` ends here with 204; `/api/validate/` is open to `*`; `/api/toons/` and `/api/keys/keys.php` send none (as always) |
| 3 | `ErrorHandler` | `ApiError` → `{ status:"error", error, message, …extra }`; unknown route/method → 404 `not_found` "Unknown endpoint."; anything else → logged, 500 `server_error` |
| 4 | Slim routing | `config/routes.php` |
| 5 | `DeviceId` | fills the domain's `RequestContext`: client IP, valid `X-Device-Id`, `Origin`; from the qr app (trusted proxy: `X-Scutoons-Proxy-Secret` = `QR_PROXY_SECRET`) the forwarded `X-Scutoons-Client-Ip` / `X-Scutoons-Device-Id` instead |
| 6 | `JsonBody` | keeps the raw body; `JsonBody::of($request)` parses lazily (`400 invalid_json`), `JsonBody::object()` keeps `{}` |
| 7 | `Session` | `Authorization: Bearer …` or the cookie `scutoons_sid` (single sign-on, `Http\SessionCookie`; Bearer wins) → `CurrentSession` attribute; a cookie on POST/PUT/DELETE counts only with an `ALLOWED_ORIGINS` Origin (CSRF); the user is looked up only when asked for |
| – | route middleware | `RequireRole(admin)` on `/api/admin/*` (401/403 `forbidden { role }`), `RateLimit` on `/api/access/` + `/api/code/` (per IP; qr-app scans in their own bucket; limits = runtime settings, docs/API.md "Rate limits"), `GenerateToken` on the legacy `generate/*.php` |

CORS runs outside the error handler, so error responses carry the CORS headers too.

## Layers

- **`src/Http/Controllers/*`** – thin: read the request (query, `JsonBody`, `CurrentSession`), call one service,
  answer with `Json::response()` (JSON with unescaped slashes/unicode, `Cache-Control: no-store`) or `Download`.
- **`src/Domain/*`** – the rules. Services get their dependencies by constructor (PHP-DI autowiring), talk to the
  database through `Infrastructure\Database\Database`, read the time from `Clock`, throw `Domain\ApiError(status,
  code, message, extra)` for contract errors. No superglobals, no headers, no `exit`. The request data they need
  (IP, device id, origin) comes from `RequestContext`.
- **`src/Infrastructure/*`** – `Settings` (env `QR_<KEY>` → constant `<KEY>` → `config/settings.php`), `Database`
  (lazy PDO, MySQL/SQLite, `transaction()` with `BEGIN IMMEDIATE` on SQLite / `FOR UPDATE` on MySQL),
  `MigrationRunner`, `Mail\Transport` + `Smtp` (delivery only – mails are sent through `Domain\Mail\Mailer`, see API.md "Emails"), `KeyStore` (ML-DSA-65 keypair), `FileStore` (batch output), `Templates`
  (plain PHP templates in `templates/`), `Clock` (`SystemClock`, `FrozenClock` for tests).
- **`config/`** – `settings.php` (every setting with its default), `container.php` (the few non-autowired services),
  `routes.php` (every URL), the catalog seed (`items.json`, `products.json`, `featured-products.json`, `editions.json`,
  `toons.json`, `permissions.json`) – the catalog itself is in database tables since 2026-10-04 (`Domain\Catalog\CatalogStore`,
  agents/CATALOG.md).

### Time formats (kept from the plain-PHP server)

Account tables (`users`, `sessions`, `inventory`, `code_events`, `entity_versions`, …) store UTC `Y-m-d H:i:s`
(`Clock::utc()`, answers use `Support\Time::iso()`); the `qr` table stores the server's local time (`Clock::local()`,
answers `Time::localIso()`).

## How to add an endpoint

1. **Domain:** put the rule in a service under `src/Domain/<Area>/` (constructor-injected, returns arrays, throws
   `ApiError` with a stable error code). Database access via `Database` with positional parameters; times from `Clock`.
2. **Controller:** add a method to a controller in `src/Http/Controllers/` (or a new invokable class). Read the body
   with `JsonBody::of($request)` (before the session check – that is the order clients rely on), the user with
   `CurrentSession::of($request)->requireUser()` (401) or `->user()` (optional); answer with `Json::response()`.
3. **Route:** register it in `config/routes.php` without a trailing slash (`/api/things` answers `/api/things/` too).
   Role-protected: inside the `/api/admin` group, or `->add(new RequireRole($c->get(Roles::class), Roles::CREATOR))`.
   Per-IP limit: `->add($lookupLimit)` or a new `RateLimit`.
4. **Contract:** document request/response/errors in `agents/ITEMS.md` (public API) or its "Admin API" section, and
   in `docs/API.md`.
5. **Tests:** a unit test for the service in `tests/Unit/Domain/` (fresh SQLite per test, `FrozenClock`), and HTTP
   checks in a contract suite (`tests/api-smoke.php`, `tests/admin-smoke.php`, …). `composer test` and
   `composer analyse` must pass.
6. **Database changes:** a new dated file per driver in `migrations/sqlite/` and `migrations/mysql/` (idempotent:
   `IF NOT EXISTS`; `ADD COLUMN` is skipped when the column exists) – see [DATABASE.md](DATABASE.md).

## Conventions

- **Errors:** `{ status:"error", error:"<code>", message, …extra }` – never a stack trace. `401 unauthorized`,
  `403 forbidden { role }`, `404 not_found`, `429 rate_limited { retry_after }`, `400 invalid_json`.
- **Admin lists:** `?page=&page_size=&sort=&q=&<filter>=` → `{ status, items, pagination: { total, page, pageSize } }`
  (`Domain\Admin\ListQuery`: page from 1, page size clamped to `ADMIN_MAX_PAGE_SIZE`, `sort=-field` descending,
  unknown sort field → `400 invalid_sort`).
- **Privacy:** code views and statistics never contain emails or user ids (only `claimed` / `account` booleans and a
  10-character device hash prefix); the users views are the place for emails.
- **Legacy:** `src/Http/Controllers/Legacy/GenerateController.php` + `GenerateToken` keep the `generate/*.php` URLs
  and answers of the old admin SPA (static password token) until the admin app replaces `public/generate/`.

## Tests

- `tests/Unit/` (PHPUnit, `composer test:unit`): services, the Slim app in-process (`AppTest`), infrastructure.
  `TestCase` gives each test a fresh copy of a migrated SQLite database, a `FrozenClock` and isolated settings
  (`Settings::fixed()` – no `config.local.php`, no environment).
- Contract suites (`composer test:contract` = `tests/run-contract.php`: copies the local DB, starts the API with
  `php -S` on :9180 (+ :9185 with a lookup limit of 1) and the qr app on :9181–9184, runs everything, stops them):
  `api-smoke.php` (the public API, 187 checks), `content-smoke.php`, `claim-race.php` (concurrent claims and signups),
  `admin-smoke.php` (admin API) – both take the qr app's URL as second argument for the landing-page scans – and
  `../qr/tests/public-urls.php` (printed URLs through the qr app, identical to the old server; proxies byte-identical;
  API down; rate limit; `PROXY_FULL_API=0`). They boot through `tests/support/legacy.php`, which gives
  them the static calls of the old server (`Db::`, `Codes::`, …) as facades over the new services.
