# SCUTOONS creator

The editor for toon / season / chapter configs and VN scene files, a standalone SvelteKit app (static build) for
`creator.scutoons.com`. Access: accounts with the role `creator` (the server checks it on every content call).

- **Sign-in** on this origin: email → one-time code (or the magic link `<creator origin>/?login=<token>` from the mail).
  Sessions are per origin (localStorage `scutoons_session`), so signing in to the viewer doesn't sign you in here.
- **Languages are per toon**: the toon page's "Languages" section sets the source language and the translations
  (`Language` enum). Stored as the toon's index entry (`sourceLanguage`, `languages`) in the content store file
  `<toon>/_/index.json` (draft/publish like the other files; before the first save: the viewer's `toons.json`). Every
  editor takes its locales + source from it (`src/lib/editor/languages.ts`); a newly added language offers
  "Create <locale> from <source>" for toon.json, season, chapters and scene files.
- **Editor**: `/` toons · `/<toon>` languages, season and chapters · `/<toon>/season/<n>` season meta · `/<toon>/toon` toon.json · `/<toon>/chapter/<n>` chapter editor (meta, groups, slices,
  WYSIWYG text boxes, audio, VN events, validation, history, undo/redo, autosave, save draft / publish).
- **Chapter status = the season**: the chapter's visibility (published / draft / coming-soon) is set per language in
  the season (`season1.json` → `sequence[].status`), in the chapter meta panel or the toon page; it is saved with the
  chapter, and the chapter file's deprecated `status` is mirrored on save. A chapter always belongs to a season: "New
  chapter" creates it inside the season (a toon without one gets "Create season 1", which also adopts existing chapter
  files as drafts).
- **Season and toon meta**: `/<toon>/season/<n>` edits `season<n>.json` (title / plot per language; number, release,
  authors, covers with the CDN picker, credits lists once for all languages, differing languages shown as overrides,
  "per language" / "same for all"; plus the chapter status table), `/<toon>/toon` edits `toon.json` (title,
  description, id, status, release, chapters, authors, thumbnail, covers, map, downloads). Both validate into the
  problem list and save / publish through the ActionBar; the season page also mirrors chapter statuses on save.
- **Draft preview**: `/<toon>/chapter/<n>/preview?locale=<one of the toon's languages>[&label=<group>]` renders the newest saved versions
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
