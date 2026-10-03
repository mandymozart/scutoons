# SCUTOONS API (server/)

The backend of every app – **api.scutoons.com** (agents/PLATFORM.md "Hosting layout & deployment"): accounts, access,
items, inventory, avatar, progress, content store, products, admin API (`/api/*`), `/api/health/` – plus, until the
admin app retires it, the legacy `generate/` admin SPA.

The public pages of **qr.scutoons.com** (QR landing page `/?c=CODE`, `/verify.html`) are the separate qr app
([`../qr`](../qr/README.md)); it calls this API server-side and proxies the printed API URLs `/api/validate/` and
`/api/keys/keys.php` (+ the verify page's `generate/get-signature.php` / signature files, + the whole `/api/*` during the
transition) here byte for byte. It is a **trusted proxy**: with `QR_PROXY_SECRET` the API takes the client IP and
device id it forwards.

PHP 8.3 · [Slim 4](https://www.slimframework.com/) + PHP-DI · Composer (PSR-4 `Scutoons\` → `src/`) · MySQL/MariaDB in
production, SQLite locally · PHPUnit · PHPStan (level 8). Shared hosting friendly: no long-running process, no shell
needed in production (`vendor/` is built by CI and uploaded).

The HTTP contract of the public API is `agents/ITEMS.md` in the monorepo (it must not change); the admin API is its
section "Admin API". **Printed URLs never change** (agents/PLATFORM.md "RULE") – `tests/public-urls.php` guards them.

## Local setup

```bash
composer install
cp config.local.php.example config.local.php     # set AUTH_SECRET: php -r "echo bin2hex(random_bytes(32));"
php bin/console migrate                           # creates/upgrades db/local.sqlite
composer serve                                    # http://localhost:9000  (bin/dev-serve.mjs: php -S workers + OPcache)
```

- Landing page + verify page: the qr app on http://localhost:9001 (`cd ../qr && composer serve`, same `QR_PROXY_SECRET`)
- API: http://localhost:9000/api/ (frontends: `VITE_API_URL=http://localhost:9000/api`). `composer serve` runs
  `bin/dev-serve.mjs` (Node): `php -S` answers one request at a time and cannot fork on Windows, so it starts 4 workers
  (`[::1]:9200-9203`, OPcache on) behind a small proxy on :9000 (::1 + 127.0.0.1). `composer serve:php` = the plain
  single-process `php -S` on the same port. Numbers: agents/PERFORMANCE.md.
- Legacy admin SPA: http://localhost:9000/generate/ (password = `GENERATE_PASSWORD`)
- Login mails land in `logs/mail.log` (`MAIL_TRANSPORT = log`).
- Database: SQLite (`db/local.sqlite`) by default; `npm run dev:db` (root) switches the local stack to a MariaDB
  container – the production engine – and `npm run dev:db sqlite` back (docs/DATABASE.md "Local MariaDB").
- Local-only files (gitignored): `config.local.php`, `db/local.sqlite`, `storage/files/` (batch output),
  `storage/keys/keypair.json` (signing keys), `logs/`.
- Dev ports: `../shared/dev-ports.json` (the single source; viewer 8000, CDN 8001, API 9000, qr 9001, creator 10000,
  admin 11000, shop 12000, embassy 13000, MariaDB container 13306, Adminer 13380). On the dev server (`php -S`) CORS allows every port of that file
  automatically (`Settings::allowedOrigins`); `ALLOWED_ORIGINS` in `config.local.php` only needs extra origins.

## Everyday commands

| Command | What |
|---|---|
| `composer serve` | dev server on :9000 |
| `composer test` | PHPUnit **and** every HTTP contract suite against freshly started servers on a copy of the DB |
| `composer test:unit` / `composer test:contract` | only one of the two |
| `composer analyse` | PHPStan |
| `npm run dev:db test` (root) · `php -d extension=pdo_mysql vendor/bin/phpunit -c phpunit.mysql.xml` | PHPUnit on the local MariaDB container (docs/DATABASE.md "Local MariaDB") |
| `php bin/console migrate [--status]` | apply pending migrations |
| `php bin/console roles grant\|revoke\|list <email> [role]` | account roles (member, admin, creator) |
| `php bin/console catalog:seed [--replace]` · `catalog:export-sql [--write]` | empty catalog tables ← `config/*.json` · the seed as the INSERTs of the `2026-10-04-catalog` migrations (agents/CATALOG.md) |
| `php bin/console import-content shared/content/seed [--sql]` · `export-content <dir>` | content in/out of the store in the interchange format (`--sql` = production seed; agents/DATA.md) |
| `php bin/console backfill-edition-numbers [--sql]` · `report-orphan-codes` | edition numbers from batch manifests |

## Structure

```
public/            web root: index.php (front controller), router.php (php -S), .htaccess,
                   assets/, generate/ (legacy admin SPA files)
src/Http/          Controllers/ (thin), Middleware/, Json, Download, CurrentSession
src/Domain/        services, no HTTP: Accounts, Auth, Codes, Access, Catalog, Inventory, Avatar, Progress,
                   Content, Events, RateLimit, Roles, Admin, Mail (the email module); ApiError, RequestContext, Secrets
src/Infrastructure/ Config (Settings), Database (PDO, MigrationRunner), Mail (Transport, Smtp), Signing (KeyStore),
                   Storage (FileStore), Templates, Clock
src/Console/       bin/console commands
src/Kernel.php     builds the container and the Slim app (middleware order)
config/            settings.php (defaults of every setting), container.php (DI), routes.php (every URL),
                   the catalog seed: items.json, products.json, featured-products.json, editions.json, toons.json,
                   permissions.json (the catalog is in the database – catalog:seed, agents/CATALOG.md)
templates/         login mails
migrations/        sqlite/, mysql/ (dated, recorded in schema_migrations), schema/ (base schema), mysql/legacy/
bin/console        command line
tests/             Unit/ (PHPUnit) + contract suites: api-smoke, content-smoke, claim-race, admin-smoke,
                   and ../qr/tests/public-urls.php; run-contract.php starts the API + the qr app and runs them all
storage/           private files (batch output, keys) – not in git, never served
tools/keys/        one-time key generation pages (never deployed)
.htaccess          only for the old host layout (document root = this folder): forwards into public/
```

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) – request flow, middleware, conventions, **how to add an endpoint**, tests
- [docs/API.md](docs/API.md) – endpoints and behaviour (access model, signup rule, roles, tokens, revoke/statistics,
  privacy, content store); the contract is agents/ITEMS.md
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) – every setting
- [docs/DATABASE.md](docs/DATABASE.md) – migrations, console, production database steps (phpMyAdmin)
- [docs/DEPLOY.md](docs/DEPLOY.md) – CI workflow, secrets, FTP, document root, **cutover from the old layout**
- Signing / certificates: [docs/KEY-MANAGEMENT.md](docs/KEY-MANAGEMENT.md), [docs/SECURITY-MODEL.md](docs/SECURITY-MODEL.md),
  [docs/VERIFICATION-GUIDE.md](docs/VERIFICATION-GUIDE.md) and the other signature notes
