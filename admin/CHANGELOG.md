# Changelog – admin (admin.scutoons.com)

Versions: `package.json` `version`; every build writes `version.json` (deploy/README.md "Versions").

## 0.1.0 – 2026-09-30

- New app (P2a): login + role `admin`, dashboard, codes, batches (port of `generate/`), users & roles, events,
  catalog, content.
- Build writes `version.json` (P3); deployed to `admin/dist/` on the host. `GET /api/admin/versions` lists the
  deployed versions of every app (dashboard widget: admin app).
