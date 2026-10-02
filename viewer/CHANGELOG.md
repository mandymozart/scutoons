# Changelog – viewer (scutoons.com)

Versions: `package.json` `version`; every build writes `version.json` (deploy/README.md "Versions").

## Unreleased

- "Where to buy" (2026-10-02, agents/EMBASSY.md "Stores"): new page `/stores` (`/en/stores`) – the Embassy's listed
  stores from `/api/stockists/`, independent shops first, then the others, grouped by country and city; filters product
  + country in the URL (`?product=`, `?country=`); empty states with "Buy online". "Läden vor Ort" / "Local stores" next
  to "Kaufen" in the toon page's related row and under the landing page's editions when a store carries the product;
  "Wo kaufen" in the menu and the landing footer. The related row's buttons wrap (up to three). `npm run check:stores`.

- Ownership + gifting (2026-10-02, agents/ITEMS.md "Ownership, claims, gifting"): claim sheet "Register it now" warning
  and "This is mine" (statement + optional proof files, sign-up for it without an account; `?code=…&claim=1`),
  "Give away" in the item detail, toasts for received gifts / codes moved here, account page "My claims".

## 0.1.0 – 2026-09-30

- Platform baseline (branch `platform`): consumes the shared packages (`@scutoons/tokens`, `content`, `reader`,
  `api`); creator code moved out (P1a); `/shop` redirects to the shop app, products from the API (P2b).
- Build writes `version.json` (P3); deployed to `viewer/dist/` on the host (deploy/targets.json).
