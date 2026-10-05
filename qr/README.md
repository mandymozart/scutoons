# qr — qr.scutoons.com

The public pages printed on physical items (agents/PLATFORM.md "RULE: printed / public URLs never change"), as a small
**plain PHP** app: server-side rendered, no JS framework, no Composer dependencies. It calls the API
(`api.scutoons.com`, the `server/` folder) from the server side – an instant page on scan, works without JavaScript, no
CORS, lookups are cached and a friendly page shows when the API is briefly down.

| URL | What |
|---|---|
| `/?c=CODE`, `?c=CODE` | QR landing page (every printed QR code). Claimable codes: item preview + **Continue** (`redirect_to?code=`) + **Add to my account** (`<viewer>/account?code=`); promo codes: Continue + **Create account**; page / event / xr codes redirect right away. Unknown 404, expired / revoked 410, too many lookups 429, API down 503 "Scanning works, try again in a minute" (+ Continue when the code is cached) |
| `/?c=CODE&skip=1` | redirect to `redirect_to?code=` right away (a fresh cached lookup is enough, no scan counted) |
| `/?login=TOKEN` | old magic links → `WWW_URL/?login=TOKEN` |
| `/` | enter a code by hand (form parameter `c`) |
| `/verify.html` | certificate verify page (static; loads `assets/js/*` and, via the proxy, `generate/get-signature.php` + `generate/files/signatures/SIGNATURE-<code>.json`) |
| `/api/validate/`, `/api/keys/keys.php` | **proxied** to the API byte for byte (status, body, content type, caching, CORS headers) – printed / linked URLs |
| `/api/*` | proxied too while `PROXY_FULL_API` is on (default; released viewers call `https://qr.scutoons.com/api/…`), else 404 JSON |
| `/generate/…` | the legacy admin SPA lives on the API: 302 to `API_URL/generate/…` (until the admin app retires it) |
| `/version.json` | `{ app: "qr", version, git_sha, built_at }` (written by the deploy; computed locally) |

Every other path shows the landing page (as the old catch-all rewrite did).

## How it talks to the API

- `GET /api/code/?c=CODE&scan=1` – the code's public info; `scan=1` makes the API log the `scan` event (it did that
  itself when the landing page lived in the server). `GET /api/items/` – names, descriptions, icons.
- **Trusted proxy:** every call carries `X-Scutoons-Proxy-Secret` (= `QR_PROXY_SECRET`), the visitor's IP in
  `X-Scutoons-Client-Ip` and its `X-Device-Id` in `X-Scutoons-Device-Id`. The API honours the forwarded values only when
  the secret matches its own `QR_PROXY_SECRET` (server `Http\Middleware\DeviceId`), so per-IP rate limits and the
  statistics count the visitor, not this server. Without the secret the API sees this server's IP (still works).
- **Cache** (APCu when available, else JSON files in `cache/`): successful code lookups are used for `skip=1` for
  `CODE_CACHE_TTL` (60 s) and kept `CODE_CACHE_KEEP` (1 day) only for the fallback page's Continue link; the catalog for
  `ITEMS_CACHE_TTL` (5 min), older copies while the API is down. A scan always asks the API (counted + rate-limited).
- Timeouts: connect 2 s, page lookups 5 s, proxied calls 30 s (`API_CONNECT_TIMEOUT`, `API_TIMEOUT`, `PROXY_TIMEOUT`).

## Structure

```
public/            docroot: index.php (front controller), router.php (php -S), .htaccess, verify.html, assets/js/,
                   assets/tokens/ (GENERATED copy of @scutoons/tokens: tokens CSS + IBM Plex Mono woff2)
bin/               sync-tokens.mjs – writes public/assets/tokens from shared/tokens (--check: in sync?)
src/               App (routing), Landing (the page), Api (lookups + cache), Proxy, Http (curl), Cache, Config,
                   View (templates), Response, Client
templates/landing/ page templates + texts (de/en) – moved from the server unchanged, + unavailable.php
tests/             public-urls.php – the printed-URL regression suite (run by server/ `composer test`)
.htaccess          only when the docroot is qr/ instead of qr/public/: forwards into public/, denies the rest
config.php         production, written by the deploy (deploy/targets.json → "qr"); never in git
config.local.php   local (copy config.local.php.example); never in git
```

## Local development

```sh
cp config.local.php.example config.local.php   # API_URL http://localhost:9000, same QR_PROXY_SECRET as server/config.local.php
composer serve                                  # http://localhost:9001  (php -S on the qr port of ../shared/dev-ports.json)
```

The API must run on :9000 (`cd server && composer serve`). Ports: agents/PLATFORM.md "Local dev ports".
Settings (defaults in `src/Config.php`): `API_URL`, `QR_PROXY_SECRET`, `PROXY_FULL_API`, `WWW_URL`, `CDN_URL`, the
timeouts and cache TTLs, `CACHE_DIR`. An environment variable `QRAPP_<KEY>` wins over the config files.

## Styles and fonts

The landing page and verify.html load `assets/tokens/scutoons-tokens.css` – tokens, the IBM Plex Mono `@font-face`
rules and the base styles of `@scutoons/tokens` – with the woff2 files beside it (relative `url()`s, no font CDN).
The landing layout also inlines `templates/landing/fonts-cdn.css` (generated too) with `{{CDN}}` = `CDN_URL`: the same
Plex faces with the CDN copy (`scutoons/fonts/`) first and the local file as the fallback; the later `@font-face` wins, so
the page shares the cached fonts with the viewer (agents/ASSETS.md "Fonts"). verify.html (static) keeps the local fonts.
It is generated, never edited by hand: after a change in `shared/tokens` run `node qr/bin/sync-tokens.mjs` and commit
the result. `npm run check` (the tokens package) and `qr.yml` fail when the copy is stale; `bin/` is not deployed.

## Tests

`composer test` in `server/` starts the API and this app on a copy of the database and runs
`tests/public-urls.php` against it: every printed URL (status, type, redirect target, `?c=` with and without the
slash), the proxies byte for byte against the API, `PROXY_FULL_API=0`, the fallback page with the API down, the
rate-limited page. By hand against running servers:

```sh
php tests/public-urls.php http://localhost:9001 --api=http://localhost:9000 [--out=results.txt]
```

## Deploy

`.github/workflows/qr.yml` (lint + offline smoke test; deploy from `main`) → host folder `qr/`, docroot `qr/public/`
(deploy/README.md). The generated `config.php` sets `API_URL=https://api.scutoons.com`, `QR_PROXY_SECRET` and
`CDN_URL`. Switch the transition off by setting `PROXY_FULL_API` to `0` in `deploy/targets.json` once the stats show no
more `/api/*` traffic on qr – `/api/validate/` and `/api/keys/keys.php` stay proxied forever.
