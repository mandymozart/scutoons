# Changelog – creator (creator.scutoons.com)

Versions: `package.json` `version`; every build writes `version.json` (deploy/README.md "Versions").

## 0.1.0 – 2026-09-30

- Standalone app split out of the viewer (P1a): own login (role `creator`), editor, draft preview, bottom ActionBar.
- Reads `VITE_WWW_URL` for the reader link (old `VITE_VIEWER_URL` still works); build writes `version.json` (P3);
  deployed to `creator/dist/` on the host.
