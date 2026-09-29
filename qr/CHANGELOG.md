# Changelog – qr (qr.scutoons.com)

## 1.0.0 – 2026-09-30

- New app (platform P3): the QR landing page, verify page and legacy API URLs of qr.scutoons.com move out of the server
  into this plain PHP app. Pages are byte-identical to the server's; the printed-URL suite passes unchanged.
- Server-side lookups against the API (`/api/code/?scan=1`, `/api/items/`) with a short cache (APCu or files).
- New states: API down → "Scanning works, try again in a minute" (503, Continue when cached); too many lookups → 429.
- Proxies `/api/validate/`, `/api/keys/keys.php`, the verify page's signature endpoints and (`PROXY_FULL_API`) the whole
  `/api/*`, byte for byte, forwarding the client IP / device id as a trusted proxy (`QR_PROXY_SECRET`).
- `/version.json`.
