# SCUTOONS shop

The limited-editions shop, a standalone SvelteKit app (static build) for `shop.scutoons.com`. Split out of the
viewer in platform phase P2b; `scutoons.com/shop` (+ `/de/shop`, `/en/shop`, `/shop/…`) stays a redirect to this
app (printed-URL rule, agents/PLATFORM.md).

- One tile per colourway / test press; sold-out pieces stay visible with a badge and a disabled button.
- "Buy" links to the product on Bandcamp; edition info ("Limited edition of N · N left"), sizes (sold-out sizes
  struck through), variant prices.
- Data: `GET <VITE_API_URL>/products/` (server `config/products.json`) via `@scutoons/api` `createProductsApi`,
  with a header-free client (simple GET, no CORS preflight). Types + helpers: `@scutoons/content` (`ProductCatalog`,
  `pickVariant`, `formatPrice`, …). Images from the CDN (`VITE_CDN_URL`).
- Locale: URL prefix `/de/`, `/en/` first (the viewer's redirect sends the reader's locale), then the cookie, then
  the browser language (paraglide, `messages/{de,en}.json`).
- Header: the SCUTOONS logo (same place and size as the viewer's menu logo) links back to `VITE_WWW_URL`; de/en
  switch (`@scutoons/ui` Button).

## Local development

```sh
npm install            # at the repo root
npm run dev:shop       # http://localhost:12000
npm run check -w shop
npm run build -w shop  # → shop/build (SPA, index.html fallback + .htaccess rewrite)
```

Env (`shop/.env.development`, see `.env.example`): `VITE_API_URL` (dev `http://localhost:9000/api`, prod fallback
`https://qr.scutoons.com/api`), `VITE_CDN_URL` (dev `http://localhost:8001`), `VITE_WWW_URL` (dev
`http://localhost:8000`, prod `https://scutoons.com`). The API needs the platform server (`scutoons-server/`,
`/api/products/`); the old `qr.scutoons.com` repo does not have that endpoint.

Deploy: `.github/workflows/shop.yml` (push to `main` touching `shop/**` or `shared/**`) → FTP secrets
`SHOP_FTP_SERVER`, `SHOP_FTP_USERNAME`, `SHOP_FTP_PASSWORD`; optional repo variables `VITE_*`.
