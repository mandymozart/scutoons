# SCUTOONS creator

The editor for toon / season / chapter configs and VN scene files, a standalone SvelteKit app (static build) for
`creator.scutoons.com`. Access: accounts with the role `creator` (the server checks it on every content call).

- **Sign-in** on this origin: email → one-time code (or the magic link `<creator origin>/?login=<token>` from the mail).
  Sessions are per origin (localStorage `scutoons_session`), so signing in to the viewer doesn't sign you in here.
- **Editor**: `/` toons · `/<toon>` season and chapters · `/<toon>/chapter/<n>` chapter editor (meta, groups, slices,
  WYSIWYG text boxes, audio, VN events, validation, history, undo/redo, autosave, save draft / publish).
- **Draft preview**: `/<toon>/chapter/<n>/preview?locale=de|en[&label=<group>]` renders the newest saved versions
  (chapter + scene files) with `@scutoons/reader`, like the reader does. VN events are not played; horizontal groups
  are shown one below the other.

Built on the shared packages: `@scutoons/content` (types, validation, paths), `@scutoons/reader` (Dialogs, Slice,
Onomatopoeia), `@scutoons/api` (session, content store), `@scutoons/ui` (login screen, preview controls),
`@scutoons/tokens`.

## Local development

```sh
npm install            # at the repo root
npm run dev:creator    # http://localhost:10000
npm run check -w creator
npm run build -w creator   # → creator/build (SPA, index.html fallback + .htaccess rewrite)
```

Env (`creator/.env.development`, see `.env.example`): `VITE_API_URL` (dev `http://localhost:9000/api`, prod fallback
`https://qr.scutoons.com/api`), `VITE_CDN_URL` (dev `http://localhost:8001`), `VITE_VIEWER_URL` (dev
`http://localhost:8000`; "Open the reader" and the viewer's static `/api` files: `toons.json`, `characters.json`, the
store's fallback), optional `VITE_STATIC_API`.

The API must allow this origin (`ALLOWED_ORIGINS`: `http://localhost:10000`, `https://creator.scutoons.com`). In
production the viewer host must also send CORS headers for `/api/*.json` to the creator origin (static fallback files).

Deploy: `.github/workflows/creator.yml` (push to `main` touching `creator/**` or `shared/**`) → FTP secrets
`CREATOR_FTP_SERVER`, `CREATOR_FTP_USERNAME`, `CREATOR_FTP_PASSWORD`; optional repo variables `VITE_*`.
