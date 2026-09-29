# SCUTOONS server

One PHP codebase for two subdomains (agents/PLATFORM.md "Domain architecture"):

- **qr.scutoons.com** – the public pages printed on physical items: QR landing page `/?c=CODE`, the certificate
  verify page `/verify.html`, the legacy `/api/validate/` and the public key `/api/keys/keys.php` (plus, for now, the
  legacy `generate/` admin SPA and – transition – the full API).
- **api.scutoons.com** – the backend of every app: accounts, access, items, inventory, avatar, progress, content
  store, admin API (`/api/*`).

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
composer serve                                    # http://localhost:9000  (php -S … -t public public/router.php)
```

- Landing page: http://localhost:9000/?c=CODE · verify page: http://localhost:9000/verify.html
- API: http://localhost:9000/api/ (frontends: `VITE_API_URL=http://localhost:9000/api`)
- Legacy admin SPA: http://localhost:9000/generate/ (password = `GENERATE_PASSWORD`)
- Login mails land in `logs/mail.log` (`MAIL_TRANSPORT = log`).
- Local-only files (gitignored): `config.local.php`, `db/local.sqlite`, `storage/files/` (batch output),
  `storage/keys/keypair.json` (signing keys), `logs/`.
- Dev ports (final): viewer 8000, CDN 8001, server 9000, creator 10000, admin 11000. `composer serve:9100` runs a
  second copy on :9100 (e.g. next to the old plain-PHP server during the transition).

## Everyday commands

| Command | What |
|---|---|
| `composer serve` | dev server on :9000 |
| `composer test` | PHPUnit **and** every HTTP contract suite against freshly started servers on a copy of the DB |
| `composer test:unit` / `composer test:contract` | only one of the two |
| `composer analyse` | PHPStan |
| `php bin/console migrate [--status]` | apply pending migrations |
| `php bin/console roles grant\|revoke\|list <email> [role]` | account roles (member, admin, creator) |
| `php bin/console import-content <viewer/static/api> [--sql]` | seed the content store (`--sql` = production seed) |
| `php bin/console backfill-edition-numbers [--sql]` · `report-orphan-codes` | edition numbers from batch manifests |

## Structure

```
public/            web root: index.php (front controller), router.php (php -S), .htaccess,
                   verify.html, assets/, generate/ (legacy admin SPA files)
src/Http/          Controllers/ (thin), Middleware/, Json, Download, CurrentSession
src/Domain/        services, no HTTP: Accounts, Auth, Codes, Access, Catalog, Inventory, Avatar, Progress,
                   Content, Events, RateLimit, Roles, Admin; ApiError, RequestContext, Secrets
src/Infrastructure/ Config (Settings), Database (PDO, MigrationRunner), Mail (Mailer, Smtp), Signing (KeyStore),
                   Storage (FileStore), Templates, Clock
src/Console/       bin/console commands
src/Kernel.php     builds the container and the Slim app (middleware order)
config/            settings.php (defaults of every setting), container.php (DI), routes.php (every URL),
                   items.json, editions.json, toons.json, permissions.json (server-only)
templates/         landing page, login mails
migrations/        sqlite/, mysql/ (dated, recorded in schema_migrations), schema/ (base schema), mysql/legacy/
bin/console        command line
tests/             Unit/ (PHPUnit) + contract suites: api-smoke, content-smoke, claim-race, admin-smoke,
                   public-urls; run-contract.php starts servers and runs them all
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
