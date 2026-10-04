# SCUTOONS admin

The administration app for `admin.scutoons.com`, a standalone SvelteKit app (static SPA build). Access: accounts with
the role `admin` (the server checks it on every `/api/admin/*` call). Replaces the legacy `generate/` SPA of the server.

- **Sign-in**: single sign-on (`@scutoons/auth` `AuthGate`) – signed in on any SCUTOONS app = signed in here; else
  email → one-time code (or the magic link `<admin origin>/?login=<token>`). Role `admin`.
- **Sections** (navigation as data in `src/lib/navigation.ts`, also the command palette: Ctrl+K / `/`):

| Route | What |
|---|---|
| `/` | Dashboard: KPIs, daily scans/unlocks and claims/signups (SVG charts), claimed/total per edition (`?days=&edition_id=`) |
| `/codes` | Codes table: search, filters (edition, type, status), sort, pager, bulk revoke/unrevoke, CSV export, detail drawer (`?open=<code>`) |
| `/calendar` | Events on scutoons.com/events (content entity `events`): edit the list, "Save & publish" stores a new version and publishes it |
| `/codes/<code>` | Code page: facts, revoke/unrevoke with a reason, activity + event timeline, certificate (rebuilt from the stored signature: QR SVG, certificate PDF, printable QR, signature file) |
| `/codes/new` | Generate one code (types membership, event, xr, promo, page; toon, redirect, expiry, edition + certificate fields), signed with ML-DSA-65 |
| `/batches` | Batch history (resume, ZIP, print sheet) and "New batch" (`?new=1`) |
| `/batches/<file>` | The batch pipeline per code: sign → db → svg → pdf → sig, pause/abort, resumable after a reload |
| `/users`, `/users/<id>` | Accounts (search, role and opt-in filters), detail with owned codes and role toggles |
| `/events` | Activity log with filters (event types, code, edition, toon, date range) |
| `/catalog/<kind>` | The catalog in the database (agents/CATALOG.md part 3): items, recipes, albums, products, editions, toons, permissions, code types, meta – list in order, form, history with undo; items: bulk "Import images …" (`/catalog` = items) |
| `/content` | The content store per toon/file/locale: newest and published version, links into the creator – read-only |

Lists keep their state in the URL (`?page=&page_size=&sort=&q=&<filter>=`, admin API convention).

## QR codes, certificates, batches (port of `generate/`)

`src/lib/qr/`: the same libraries and formats as the legacy SPA – `qrcode-svg.js` (datalog/qrcode-svg, vendored
unchanged because it is not on npm; 256px, pad 4, ECL H), `jspdf` 3.0.3, `@noble/post-quantum` (ML-DSA-65, signatures
verify with the verify page's bundled copy), the templates `templates/cos.md` + `qr.md`. File names:
`SCUTOONS-QR-<TYPE>-<code>.svg`, `SCUTOONS-COA-<code>.pdf`, `SCUTOONS-QR-PRINT-<code>.pdf`, `SIGNATURE-<code>.json`,
batch manifests `BATCH-<UTC timestamp>-<edition>.json`, print sheet `…-PRINT.pdf`. The regenerate downloads on the
code page keep the legacy regenerate names (`qr-<code>.svg`, `certificate-<code>.pdf`, `qr-print-<code>.pdf`,
`signature-<code>.json`).

**Printed URLs never change** (agents/PLATFORM.md): the QR URL `https://qr.scutoons.com?c=<CODE>` and the certificate
line "Verify at https://qr.scutoons.com/verify.html with code: <CODE>" are constants in `src/lib/qr/config.ts`, not env.

## Local development

```sh
npm install              # at the repo root
npm run dev:admin        # http://localhost:11000
npm run check -w admin
npm run build -w admin   # → admin/build (SPA, index.html fallback + .htaccess rewrite)
```

Env (`admin/.env.development`, see `.env.example`): `VITE_API_URL` (dev `http://localhost:9000/api`, prod fallback
`https://qr.scutoons.com/api`), `VITE_CREATOR_URL` (dev `http://localhost:10000`), `VITE_VIEWER_URL` (dev
`http://localhost:8000`). The API must allow the origin (`ALLOWED_ORIGINS`: `http://localhost:11000`,
`https://admin.scutoons.com`). Login codes are in the server's `logs/mail.log` locally.

Deploy: `.github/workflows/admin.yml` (push to `main` touching `admin/**` or `shared/**`) → FTP secrets
`ADMIN_FTP_SERVER`, `ADMIN_FTP_USERNAME`, `ADMIN_FTP_PASSWORD`; optional repo variables `VITE_API_URL`,
`VITE_CREATOR_URL`, `VITE_VIEWER_URL`.
