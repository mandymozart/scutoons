# Changelog – qr (qr.scutoons.com)

## Unreleased

- verify.html (2026-10-02): checks through the API's `/api/certificates/*` (always proxied, also with
  `PROXY_FULL_API=0`) – authentic only when signed with the published SCUTOONS key AND matching the record of the code
  (before: any self-signed file passed). Case-insensitive code input, pasted QR/verify URLs, `?code=` checks right away,
  de/en, registration status (registered / not yet + register link), revoked state, escaped output. URL unchanged.

## 1.0.0 – 2026-09-30

- Self-hosted fonts (2026-09-30): landing page and verify.html use `assets/tokens/scutoons-tokens.css`, a generated
  copy of `@scutoons/tokens` with the IBM Plex Mono woff2 (`bin/sync-tokens.mjs`). The landing page no longer loads
  Google Fonts; verify.html no longer links the old site's `https://scutoons.com/assets/styles.css` (gone after the
  cutover) and was rendered in Times/Arial. URLs, statuses and page content unchanged.

- New app (platform P3): the QR landing page, verify page and legacy API URLs of qr.scutoons.com move out of the server
  into this plain PHP app. Pages are byte-identical to the server's; the printed-URL suite passes unchanged.
- Server-side lookups against the API (`/api/code/?scan=1`, `/api/items/`) with a short cache (APCu or files).
- New states: API down → "Scanning works, try again in a minute" (503, Continue when cached); too many lookups → 429.
- Proxies `/api/validate/`, `/api/keys/keys.php`, the verify page's signature endpoints and (`PROXY_FULL_API`) the whole
  `/api/*`, byte for byte, forwarding the client IP / device id as a trusted proxy (`QR_PROXY_SECRET`).
- `/version.json`.
