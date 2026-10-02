# Changelog – admin (admin.scutoons.com)

Versions: `package.json` `version`; every build writes `version.json` (deploy/README.md "Versions").

## Unreleased

- Claims (2026-10-02, agents/ITEMS.md "Ownership, claims, gifting"): `/claims` list (open count in the sidebar) and
  `/claims/[id]` (statement, claimant, owner at filing / now, certificate check, evidence, history; transfer / reject with
  a required note); the code page shows the ownership history and "Transfer ownership".

- Emails (nav group System): `/emails` sent-mail log (filters, pager, URL-driven), `/emails/templates` every email
  rendered de/en with sample values + the admin notification recipients.

- Certificate panel (2026-10-02): "signature valid" now verifies against the server's public key and the code, not
  the key embedded in the stored signature (`verifySignature` is async).

## 0.1.0 – 2026-09-30

- New app (P2a): login + role `admin`, dashboard, codes, batches (port of `generate/`), users & roles, events,
  catalog, content.
- Build writes `version.json` (P3); deployed to `admin/dist/` on the host. `GET /api/admin/versions` lists the
  deployed versions of every app (dashboard widget: admin app).
