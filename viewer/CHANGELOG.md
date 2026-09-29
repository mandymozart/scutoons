# Changelog – viewer (scutoons.com)

Versions: `package.json` `version`; every build writes `version.json` (deploy/README.md "Versions").

## 0.1.0 – 2026-09-30

- Platform baseline (branch `platform`): consumes the shared packages (`@scutoons/tokens`, `content`, `reader`,
  `api`); creator code moved out (P1a); `/shop` redirects to the shop app, products from the API (P2b).
- Build writes `version.json` (P3); deployed to `viewer/dist/` on the host (deploy/targets.json).
