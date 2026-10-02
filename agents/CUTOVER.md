# CUTOVER — the one runbook (beta 2026-10-04, side by side, new database)

Owner: Tilman. Guide: the cutover agent (updates the checkboxes, answers "next step", diagnoses errors read-only).
Strategy (owner, 2026-10-02): **bunny first, then the new platform side by side** in a new FTP subfolder with a **new
MySQL database**, tested on test hostnames, then the subdomains are switched one by one; the old system stays untouched
and reachable at `legacy.scutoons.com` / `qr-legacy.scutoons.com` (= backup + rollback). The new platform reads its
media from bunny's interim `https://scutoons.b-cdn.net` (folder relayout first); the legacy system keeps
`cdn.scutoons.com` (all-inkl) until the post-beta domain switch.

This file replaces the scattered checklists **by reference** (they stay as background, this file wins on order):
PLATFORM.md "Cutover checklist (all-inkl)" + "Emails" + "SSO — result", DATA.md §4, ITEMS.md merge/seed notes
(wardrobe, substances, players, record player repair, ownership), EMBASSY.md "To go live" + "Stores",
MERGE-media-quality.md, MAP.md §8.8, server/docs/DATABASE.md + DEPLOY.md, deploy/README.md, ASSETS.md (worktree
`C:\Development\scutoons-assets`, branch `assets`) "Connecting bunny.net".

**Legend** — `[ ]` open · `[x]` done · `[-]` skipped (reason) · **O** = owner (KAS, phpMyAdmin, FTP, GitHub, bunny
dashboard, DNS) · **A** = agent (local commands, read-only checks: GET/HEAD requests, logs, dry runs).
Agents never run production-changing commands, never see or print credentials.

**Where credentials go (never into chat, commits or this file):**

| Credential | Goes into | Used by |
|---|---|---|
| FTP user of the new subfolder | GitHub → `scutoons` → Settings → Environments → `production`: secrets `FTP_SERVER`, `FTP_USERNAME`, `FTP_PASSWORD` (replace the old values; keep the old ones in your password manager) | every deploy |
| New MySQL DB (name / user / password) | `production`: variables `DB_HOST`, `DB_NAME`; secrets `DB_USER`, `DB_PASS` | api `config.php` |
| `AUTH_SECRET`, `QR_PROXY_SECRET` (new random, ≥ 32 chars), `SMTP_*`, `GENERATE_PASSWORD` | `production` (list: root `.env.example`) | api, qr |
| bunny keys (`BUNNY_*`, `CDN_PUBLIC_URL`) | **the only thing in a local repo-root `.env`** (gitignored): today `C:\Development\scutoons-assets\.env`, after the assets merge `C:\Development\scutoons\.env` (move the file) | asset manager / the one-time `assets:sync` (local) |

**Owner rule (2026-10-02):** the new DB and the FTP subfolder credentials go **only** into the GitHub `production`
environment (the deploy writes them into the api's `config.php` on the server) – never into a local file. The local
repo-root `.env` holds the bunny keys and nothing else that is used.
| old CDN FTP (cdn.scutoons.com) | `cdn/.env` (gitignored) – unchanged | `cdn/sync.py` |
| old DB, old FTP | unchanged, your password manager | old system (legacy) |

---

## Current state (fill in during Phase 0 – no secrets, only names/paths/numbers)

| What | Value |
|---|---|
| KAS target dir `scutoons.com` / `www.` | `/scutoons.com/` (per its FTP user; confirm in KAS → Domains) |
| KAS target dir `qr.scutoons.com` (old qr folder) | `/qr.scutoons.com/` (per its FTP user; confirm) |
| KAS target dir `creator.scutoons.com` (in use?) | |
| KAS target dir `cdn.scutoons.com` (old CDN folder) | `/cdn.scutoons.com/` (per its FTP user; confirm) |
| Old DB tables (expected: `qr` only, maybe `schema_migrations`) | |
| Old DB `qr` at export T0: rows / registered (`email<>''`) / signed / `CHECKSUM TABLE qr` / time (UTC) | **469 / 35 / 381**; MAX(created_at) 2026-09-26 13:28:27, MAX(claimed_at) 2026-09-28 07:49:46; per edition: NULL 169, `3_catpope_vinyl` 300 (sum 469); `CHECKSUM TABLE qr` = **3162238151** |
| New root folder + FTP user | `/web/`, user "github actions next" (2.1) |
| Test hostnames (proposal `next.scutoons.com` = viewer, `qr-next.scutoons.com` = qr) | |
| DB labels: **DB-T** (test build) / **DB-P** (switch-day build, only if needed – Phase 7) | |

---

## Agent work this plan needs (flagged, not done yet)

| # | Task | Needed for | Estimate |
|---|---|---|---|
| T1 | **DONE (coordinator, commit 6fcb82c).** Was required. Add the viewer test origin `https://next.scutoons.com` to `ALLOWED_ORIGINS` in `deploy/targets.json` (api `config.php`; it is also the CSRF list) + deploy README note. Without it the viewer on the test host cannot sign in (CORS). Fallback without a code change: use `embassy.scutoons.com` as the viewer test host (already allowed; embassy is held back). | Phase 6 | 15 min + dry run |
| T2 | n/a since Phase 4 is dropped (kept for 10.3). `cdn/sync.py` skips `.json` / `.bin` → the camera model `models/mobilenet_v2_100_224/` never reaches cdn.scutoons.com. Add both to `SYNC_EXTENSIONS` – or upload that folder by hand (step 4.3). | Phase 4 | 10 min |
| T3 | **n/a** – KAS creates subfolder-rooted FTP users (2.1). Was: only if the owner cannot get an FTP user rooted in the subfolder: a remote prefix in `deploy/targets.json` / `deploy.mjs` (e.g. `${FTP_ROOT}`). | Phase 5 | 30–60 min |
| T4 | Only if the owner rejects "rebuild or keep" in Phase 7: a delta import (`bin/console legacy-delta --since=<T0> --sql` from an old-DB export: new codes INSERT IGNORE, new email registrations / re-signatures UPDATE where the new DB has no owner). Not recommended – the checksum gate in 7.1 makes it unnecessary. | Phase 7 | 2–3 h |

What already exists (no build needed): the 16 idempotent migrations (they turn a copy of the old `qr` table into the
new schema exactly like an in-place upgrade), `content-seed.mysql.sql`, `backfill-edition-numbers --sql` (no DB needed),
`tests/entities-compare.php <api> --report`, the old DB's own export as the import source. The fresh-DB schema file
`server/migrations/schema/qr.mysql.sql` is **incomplete** (no content/mail/ownership tables) – do not use it.

---

## Phase 0 — Safety (nothing changes for readers)

Decide before: nothing.

- [-] **0.0 O (optional, D7) – Promo codes first:** not done before 0.1. If D7 needs a promo code: make it in the
  new admin / the api's legacy `generate/` (5.5) on DB-T, or in the old `generate/` now – then 7.1's checksum differs
  and DB-P is rebuilt from a fresh export (it then carries the code).
- [x] **0.1 O – Export the OLD database.** Done 2026-10-02: export stored in the owner's Proton Drive backups folder
  (Scutoons/Backups); it contains **only the `qr` table** (confirmed – no accounts/inventories/progress to migrate).
  Numbers recorded in "Current state" (469 / 35 / 381, checksum 3162238151). phpMyAdmin → old DB → *Export* → *Custom* → all tables, structure + data,
  format SQL, compression gzip → save as `scutoons-db-old-2026-10-0X.sql.gz` in two places offline (disk + USB/cloud
  drive). Right after, in the old DB's *SQL* tab run and note the numbers in "Current state":
  ```sql
  SELECT COUNT(*) AS codes, SUM(email <> '') AS registered, SUM(signature IS NOT NULL) AS signed, MAX(created_at), MAX(claimed_at) FROM qr;
  SELECT edition_id, COUNT(*) FROM qr GROUP BY edition_id;
  CHECKSUM TABLE qr;
  SHOW TABLES;
  ```
  Verify: the .gz opens (7-Zip), contains `CREATE TABLE \`qr\`` and `INSERT INTO \`qr\``. Rollback: n/a (read-only).
  If `SHOW TABLES` lists more than `qr` (+ `schema_migrations`): stop and tell the guide.
- [x] **0.2 O – Back up the old qr folder** – done 2026-10-02 (hash / publicKey check result not reported yet). FTP/WinSCP, the main FTP account. Download into an offline folder:
  `api/keys/keypair.json` (**CRITICAL – the private signing key; never upload anywhere public, never send by mail/chat**),
  the whole `generate/files/` (`batch/`, `certificates/`, `svgs/`, `signatures/`, …), `generate/.htpasswd`, `config.php`.
  Verify: per subfolder of `generate/files/` file count + total size = the server's (WinSCP "Properties");
  `Get-FileHash keypair.json -Algorithm SHA256` noted (hash only, here in the log is fine); the `publicKey` inside the
  file equals the one served by `https://qr.scutoons.com/api/keys/keys.php` (you compare the first/last 20 characters –
  agents never open the keypair file).
- [x] **0.3 O – Back up the old viewer folder** – done 2026-10-02 (`/scutoons.com/` → Proton Drive backups). (FTP download of the folder from "Current state"). Optional – it stays
  untouched anyway.
- [x] **0.4 O – Freeze the old system's deploys:** *(done 2026-10-02)* GitHub → `mandymozart/qr.scutoons.com` → Settings → Actions →
  *Disable actions* (its workflow deploys production on a push to any branch). Also: **no new batches / regenerations
  in the old `generate/` from now on** (they would not reach the new DB – see 7.1).
- [x] **0.5 O – Record the KAS state** *(done 2026-10-02)* (KAS → Domains): the target directory of every (sub)domain → "Current state".
  Check that none of the new folder names below would collide with an existing folder.
- [ ] **0.6 A – Local copies checked:** `cdn/root` = 824 files / 855,854,341 bytes, identical copy in
  `C:\Development\scutoons-assets\cdn\root` (checked 2026-10-02) = the media backup; `server/db/local.sqlite` copied
  to `server/db/local.sqlite.bak-2026-10-0X-cutover`.
- [ ] **0.7 O (optional) – Old CDN listing:** WinSCP/FileZilla on the CDN FTP folder → a recursive listing saved to
  `cdn/logs/remote-listing-2026-10-0X.txt` (the sync never deletes, so this is documentation only).

Phase 1 waits for the assets merge (see its preconditions); Phase 0, 2 and 3 do not depend on it.

---

## Phase 1 — bunny.net initialisation in the NEW folder layout (owner decision 2026-10-02)

**Strategy change (owner):** the folder relayout happens **before** the bunny upload, and the new platform reads its
media from bunny's interim URL **`https://scutoons.b-cdn.net`** from its first deploy (`VITE_CDN_URL` / `CDN_URL` in
`deploy/targets.json`, comes with the assets merge). The legacy system keeps `cdn.scutoons.com` (all-inkl FTP CDN,
untouched). The switch of the name `cdn.scutoons.com` to bunny stays post-beta (10.3).

Preconditions (agent/coordinator): branch `assets` – relayout copy + reference rewrite (214 files, 552 references),
hand edits, full suite green – **merged into `platform`**; the bunny `.env` moved to `C:\Development\scutoons\.env`;
the seed SQL (`content-seed.mysql.sql`) regenerated with the new paths (Phase 3.3 imports that version); the new
migration `2026-10-02-assets` added to the Phase 3.2 list (A updates 3.2 at the merge).

Decide before: **D1** the zone's old-layout files (330 from the older snapshot + whatever the old layout leaves): the
new platform no longer references them, the legacy system uses all-inkl → recommendation: keep until the public
release, then prune via the admin's Unused list (never via a bulk dashboard delete).

All commands in `C:\Development\scutoons\server` (PowerShell), after the merge.
- [x] **1.1 O – Relayout locally:** *(done 2026-10-02 by the coordinator: assets merged into `platform` (b3be086, 67774da), 318 old files removed, `cdn/root` = 720 files / 806 MB, new layout only; full suite + browser checks green)* `php bin/console assets:relayout --delete-old` (exact options as the assets merge
  report states; `cdn/root` then holds only the new layout). Verify: A runs the validation's media checks /
  `assets:check-storage` comparison – 0 references to missing files.
- [x] **1.2 O – Write fence off:** *(n/a now: the main checkout's `server/config.local.php` has no `BUNNY_WRITE_PREFIX` line – the `.env` with the 7 keys moved to `C:\Development\scutoons`)* in `server/config.local.php` the `BUNNY_WRITE_PREFIX` line commented out.
- [x] **1.3 O – Read-only check:** *(done: 7 keys set, zone empty, pull zone 6731366 origin = storage zone #1599226, API key valid)* `php bin/console assets:check-storage` → all keys "set", zone `scutoons`, pull zone
  6731366 origin = the storage zone.
- [x] **1.4 O – Plan (dry run):** *(done: 720 files / 805.9 MB to upload, 0 same / different / only on target)* *(the zone is EMPTY now – the owner emptied it, D1 moot: expect ≈ 720 to upload / ≈ 806 MB, 0 different, 0 only on
  the target)* `php bin/console assets:sync --source-root=../cdn/root --hash` → paste the summary
  (to upload / different / only on target) to the guide before applying.
- [x] **1.5 O – Apply:** *(done 2026-10-02, log `cdn/logs/bunny-init-2026-10-02.json`)* `php bin/console assets:sync --source-root=../cdn/root --hash --apply --overwrite --purge-zone
  --log=../cdn/logs/bunny-init-2026-10-0X.json` (verified uploads, retries, never deletes; long-running). This includes
  the camera model (`models` → its new folder) – sync.py's `.json`/`.bin` gap does not apply here.
- [x] **1.6 O – CORS on the pull zone:** *(done: CORS on; extensions eot, ttf, woff, woff2, css, js, jpg, jpeg, png, webp, gif, mp3, mp4, mpeg, svg, webm, json, bin, zip, avif, pdf)* bunny dashboard → Pull Zones → `scutoons` → Headers → *Add CORS headers* ON,
  extension list incl. `mp4, mp3, json, bin, webp, png, jpg, jpeg, svg, zip` (the media-quality probe `fetch`es the 480p
  video with `Range`, the camera loads `model.json` + shards).
- [x] **1.7 O+A – Verify:** *(done: 720 paths on both, 0 only local / 0 only bunny / 0 different, 805.9 MB – catpope 305, characters 391, scutoons 22, toptoy 1, yellowmouse 1; samples model.json, trailer 720p, prologue 480p with Origin + Range → 206, right content-range/type, ACAO `*`)* `assets:check-storage` → **0 missing, 0 different**. A: HEAD/GET through
  `https://scutoons.b-cdn.net/…` on samples of the new layout (trailer 720p, prologue 480p, a chapter-2 slice, an item
  icon, a wear overlay, `model.json`, the album zip), size = local, `Access-Control-Allow-Origin` present with an
  `Origin: https://scutoons.com` request, `Range` → 206.
- [ ] **1.8 O – Write fence back on:** add `define('BUNNY_WRITE_PREFIX', '_test/');` to `C:\Development\scutoons\server\config.local.php`
  (the main checkout's local server must not write to the production zone).
- Rollback: none needed (no reader uses bunny yet; re-runnable; nothing deleted on the zone).

---

## Phase 2 — The new slot on all-inkl + GitHub (nothing public changes)

Decide before: **D2** subfolder name + test hostnames (proposal above) · **D3** `VITE_API_URL` straight to
`https://api.scutoons.com/api` (recommended – api. exists from day one; the old viewer keeps using qr.scutoons.com/api,
which the new qr app proxies after the switch).

- [x] **2.1 O – New root folder + its own FTP user.** *(done: user rooted in `/web/`, FTP_* secrets set)* KAS today has 4 FTP users – main `w020b50e` (`/`, root),
  `cdn-sync` `f017c2d3` (`/cdn.scutoons.com/`), github actions `f017b642` (`/scutoons.com/`), github actions `f017b651`
  (`/qr.scutoons.com/`). **Leave all four untouched** until after the switch (old deploys, legacy, CDN sync).
  KAS → FTP → *new FTP user* "github actions next", directory = a **new root folder `/web/`** (D2 decided 2026-10-02; created
  with it), rights read/write/list/view. Its credentials **only** into GitHub → `production` secrets `FTP_SERVER`,
  `FTP_USERNAME`, `FTP_PASSWORD` (replace whatever is there) – not into the local `.env`, never the root user's login.
  Verify: WinSCP with the new user → an empty folder (you must not see `scutoons.com/` or `qr.scutoons.com/`).
  No deploy change needed (T3 n/a): `deploy/targets.json` remotes are relative (`api/`, `qr/`, `viewer/dist/`, …) and
  `deploy.mjs` hands them to the FTP action as `server-dir` = relative to this user's login folder = `/web/`.
- [x] **2.2 O – Folder layout inside `/web/`** *(done)* (create the empty folders; the deploys fill them):

  | Folder (from the KAS root) | Filled by | Domain at the test phase (2.6) | Domain after the switch (Phase 8) |
  |---|---|---|---|
  | `/web/api/` (docroot `api/public/`; private `api/storage/keys/`, `api/storage/files/`, `api/logs/`) | target `api` (server/) + 2.3 | `api.scutoons.com` (new) | same |
  | `/web/qr/` (docroot `qr/public/`) | target `qr` | `qr-next.scutoons.com` (test) | **`qr.scutoons.com`** |
  | `/web/viewer/dist/` | target `viewer` | `next.scutoons.com` (test) | **`scutoons.com` + `www.scutoons.com`** |
  | `/web/admin/dist/` | target `admin` | `admin.scutoons.com` (new) | same |
  | `/web/creator/dist/` | target `creator` | `creator.scutoons.com` | same |
  | `/web/shop/dist/` | target `shop` | `shop.scutoons.com` (new) | same |
  | `/web/embassy/dist/` | held back (D5) | – | later (10.6) |
  | `/scutoons.com/` (old viewer, untouched) | – | `scutoons.com`, `www.` | **`legacy.scutoons.com`** |
  | `/qr.scutoons.com/` (old qr + generate/, untouched) | – | `qr.scutoons.com` | **`qr-legacy.scutoons.com`** |
  | `/cdn.scutoons.com/` (old FTP CDN, untouched) | – | `cdn.scutoons.com` | same (legacy only; new platform = `scutoons.b-cdn.net`) |

> **Keypair reference (0.2/2.3):** production `keypair.json` = backup ZIP `…\Scutoons\Backups\qr.scutoons.com.zip` →
> `qr.scutoons.com/api/keys/keypair.json`, SHA-256 `409c19834377d1d9a6e82bd542fcc58a116c1a0a728aae8a0e26624df1e77bbe`.
> **WARNING:** the local dev key `server/storage/keys/keypair.json` is a DIFFERENT key (SHA-256 `11934cea…`) – never upload it
> to production. After uploading, `/api/keys/keys.php` on api. must serve the same `publicKey` as the old qr site.

- [ ] **2.3 O – Private data → `/web/api/storage/`** *(partly: `generate/files` copied; keypair extracted to the backup folder, SHA-256 409c1983… ✓; upload to
`/web/api/storage/keys/` pending confirmation)* (before the first api deploy): backup's `api/keys/keypair.json`
  → `api/storage/keys/keypair.json`; backup's `generate/files/*` → `api/storage/files/` (same subfolders).
  Verify: file counts + sizes per subfolder = 0.2; keypair SHA-256 = 0.2. Rollback: delete the copies (old untouched).
- [x] **2.4 O – New database DB-T.** *(done; `DB_HOST`/`DB_NAME` stored as secrets – fine, the workflow passes secrets and variables)* KAS → Databases → new database (MariaDB, utf8mb4). Name/user/password → GitHub
  `production` (`DB_HOST`, `DB_NAME` variables; `DB_USER`, `DB_PASS` secrets).
- [x] **2.5 O – GitHub environment `production`** *(done: all keys `deploy/targets.json` references are present; `SMTP_HOST` = the main account's kasserver host, 465/ssl)* (restricted to branch `main`): every entry of the root
  `.env.example` – `AUTH_SECRET`, `QR_PROXY_SECRET` new random strings ≥ 32 chars; `MAIL_FROM=no-reply@scutoons.com`,
  `SMTP_*` of that mailbox (exists, SPF/DKIM set); `VITE_API_URL=https://api.scutoons.com/api` (D3),
  `VITE_CDN_URL=https://scutoons.b-cdn.net` (bunny interim URL – new platform only; the qr app's `CDN_URL` follows it), `VITE_WWW_URL=https://www.scutoons.com`, `VITE_CREATOR_URL`,
  `VITE_SHOP_URL`, `VITE_ADMIN_URL`; `GENERATE_PASSWORD` (old value, only for the legacy `generate/`).
  Verify: the summary at the end of `.env.example` (10 secrets, 12 variables) all present.
- [x] **2.6 O – Subdomains → `/web/`** *(done: admin./api./creator./next./qr-next./shop. → `/web/…` with SSL, PHP 8.5; `embassy.` → `/web/embassy/dist/` exists, stays undeployed; qr./cdn./scutoons.com untouched)* (KAS → Domains → Subdomain; SSL Let's Encrypt + "force HTTPS" on each):
  new `api.` → `/web/api/public/` · new `admin.` → `/web/admin/dist/` · new `shop.` → `/web/shop/dist/` ·
  `creator.` → `/web/creator/dist/` (if 0.5 shows it unused) · test `next.` → `/web/viewer/dist/` · test `qr-next.`
  → `/web/qr/public/`. **Do not touch `scutoons.com`, `www.`, `qr.`, `cdn.` yet.**
  Rollback: delete the new subdomains.

---

## Phase 3 — Build the new database DB-T (old DB untouched)

Note (0.1 numbers): production has **no Dago or beetles codes** (only NULL 169 + vinyl 300), so D4 concerns the
dev DB / the batch files only, not this import. Decide before: **D4** the duplicate code sets (37 Dago codes signed #1–37 and 20 beetles codes in no batch manifest,
MEMORY "qr.scutoons.com backend") – **not blocking**: the import copies every code as it is; the backfill numbers only
manifest codes. Decide before the public release which set was printed (revoke the other in the admin).

- [x] **3.1 O – Import the old data.** *(done 2026-10-02)* phpMyAdmin → DB-T → *Import* → the 0.1 file (`.sql.gz` is fine). Verify:
  `SELECT COUNT(*) FROM qr;` = 0.1's number.
- [x] **3.2 O – Migrations, in this order** *(done 2026-10-02: all 17 incl. `2026-10-02-assets`; 3 triggers present)* (`server/migrations/mysql/`, *Import* tab per file; all idempotent):
  1. `2026-09-29-accounts-inventory.mysql.sql`
  2. `2026-09-30-code-owner-guard.mysql.sql` – **SQL tab, not Import:** paste the 3 `CREATE TRIGGER` lines, set the
     *Delimiter* field to `$$`, and replace the trailing `;` of each line with `$$`
  3. `2026-09-30-content-versions.mysql.sql`
  4. `2026-09-30-entity-keys.mysql.sql` (after content-versions)
  5. `2026-09-30-settings.mysql.sql` (before member-characters – that file's header asks for it)
  6. `2026-09-30-member-characters.mysql.sql`
  7. `2026-09-30-revoke-events.mysql.sql`
  8. `2026-09-30-roles-signup.mysql.sql`
  9. `2026-09-30-sensitive-content.mysql.sql`
  10. `2026-10-01-embassy.mysql.sql` (embassy app held back, the tables are harmless)
  11. `2026-10-01-media-quality.mysql.sql`
  12. `2026-10-01-reader-state.mysql.sql`
  13. `2026-10-02-embassy-stores.mysql.sql` (after embassy)
  14. `2026-10-02-mail-log.mysql.sql`
  15. `2026-10-02-ownership.mysql.sql` (after code-owner-guard)
  16. `2026-10-02-ownership-signup.mysql.sql`
  17. `2026-10-02-assets.mysql.sql` (the asset registry; came with the assets merge – run after the 16)
  Hint: if phpMyAdmin's *Import* answers "Incorrect format parameter" (seen on content-versions), paste the file into
  the *SQL* tab with delimiter `;` instead.
  Not to run: `migrations/mysql/legacy/*` (already in the old data), `schema/qr.mysql.sql`.
- [x] **3.3 O – Content seed:** *(done 2026-10-02: entity_versions = 48)* *(unblocked 2026-10-02: the file is regenerated with the new media paths – import THIS version)* *Import* `shared/content/seed/content-seed.mysql.sql` (48 entities: catpope chapters
  0–5, scenes, season 1 with related items, the record-player-repair pickups, trailer + chapter-0 quality variants,
  credits, characters with Top Toy's tendencies, map, toptoy/yellowmouse). **Not needed:** `credits-seed`,
  `characters-seed` (subsets), `2026-10-01-media-quality-chapter0.mysql.sql` (only acts on an old chapter 0 – harmless
  if run).
- [x] **3.4 A+O – Edition numbers.** *(done 2026-10-02: imported, 300 numbered)* *(A part done 2026-10-02: `C:\Development\scutoons\cdn\logs\edition-numbers-2026-10-02.sql`
  (gitignored), 300 UPDATEs from the one production manifest `BATCH-2026-06-16T1431-3_catpope_vinyl.json` – all
  `3_catpope_vinyl`, only where `edition_number IS NULL`; after import expect `numbered` = 300. O: import it after 3.2.)* A: `cd server; php bin/console backfill-edition-numbers --sql
  --manifests=<backup>/generate/files/batch > ../logs/edition-numbers-2026-10-0X.sql` (no DB needed; A reports the
  statement count). O: *Import* that file into DB-T.
- [x] **3.5 O – Verify DB-T** *(done 2026-10-02: all checks as expected)* *(qr counts ✓ 2026-10-02: 469 codes / 300 numbered / 381 signed / 35 registered; still open:
  per edition, entity_versions after 3.3, roles, users = 0)* (SQL tab; paste the results to the guide):
  ```sql
  SELECT COUNT(*) AS codes, SUM(email <> '') AS registered, SUM(signature IS NOT NULL) AS signed, SUM(edition_number IS NOT NULL) AS numbered FROM qr;
  SELECT edition_id, COUNT(*) FROM qr GROUP BY edition_id;
  SELECT COUNT(*) FROM entity_versions;            -- 48
  SELECT id, name FROM roles ORDER BY id;          -- member, admin, creator, liaison, ambassador
  SHOW TRIGGERS;                                   -- qr_owner_immutable, inventory_code_owner_insert, inventory_code_owner_update
  SELECT COUNT(*) FROM users;                      -- 0
  ```
  codes / registered / signed / per edition = 0.1 exactly (**469 / 35 / 381**, per edition NULL 169 + `3_catpope_vinyl` 300); numbered = 3.4's statement count (minus codes not in the DB).
- Rollback: drop DB-T in KAS and start 3.1 again (the old DB is never touched).

---

## Phase 4 — Uploads to the old FTP CDN cdn.scutoons.com — n/a (owner decision 2026-10-02)

- [-] **4.x** Not needed: the new platform reads media from bunny (`https://scutoons.b-cdn.net`, Phase 1); the legacy
  system keeps `cdn.scutoons.com` with what it already has. Do **not** run `cdn/sync.py` against the new layout (it would
  upload the relayouted tree next to the old one). Note for later (10.3 backup target): `sync.py` skips `.json`/`.bin`,
  and answering `n` to a "DIFFERS" prompt downloads the server's file over the local one.

---
## Phase 5 — Deploy the new platform into `/web/` (nothing points at it except the new/test hostnames)

Decide before: **D5** embassy stays held back for the beta (default yes: not in `targets.json` `order`, workflow manual).
Preconditions: **2.1 done (FTP secrets = the subfolder user!)**, 0.4 done, T1 done. After the assets merge also: the 7 `BUNNY_*` / `CDN_PUBLIC_URL`
secrets in `production` (step 10.1b – needed from the first deploy, since the merge ships the asset manager).

- [ ] **5.1 A – Local dry run:** `npm run deploy:dry-run` (builds everything; plan: api → `api/`, qr → `qr/`, viewer →
  `viewer/dist/`, shop, creator, admin; protected `storage/**`, `db/**`, `logs/**`). Plus the full suite on the tree
  to release (PLAN.md "definition of done").
- [ ] **5.2 O (with the coordinator) – Merge `platform` → `main` and push.** Every push to `main` deploys the touched
  apps (environment `production`) – into `/web/` because of 2.1.
- [ ] **5.3 O – GitHub → Actions → "Deploy all" → Run workflow with dry run** → read the plan, then **without dry run**
  (order api → qr → frontends). Every run prints the migrations reminder – already done in Phase 3.
- [ ] **5.4 A – Verify** (PHP note: KAS runs PHP 8.5, CI builds/tests on 8.3 – if `/api/health/` or qr show errors or
  deprecation output, set the subdomains to PHP 8.3 in KAS): `https://api.scutoons.com/api/health/` → `status ok`, `db ok`, `version 1.0.0`, `git_sha` =
  main's head; `https://{admin,shop,creator,next}.scutoons.com/version.json`, `https://qr-next.scutoons.com/version.json`.
- [ ] **5.5 O – Legacy `generate/` on the api** (only if you still want it): upload the backup's `generate/.htpasswd` to
  `/web/api/public/generate/.htpasswd`, set `AuthUserFile` in `/web/api/public/generate/.htaccess` to its absolute
  path (KAS shows `/www/htdocs/<account>/web/api/public/generate/.htpasswd`).
- Rollback: Actions → the app's workflow on an earlier commit; nothing public points here yet.

---

## Phase 6 — Test the new system on the test/new hostnames (DB-T)

Decide before: **D6** admin notification recipients (who gets "new ownership claim" mails; Emails page or
`MAIL_ADMIN_TO`) · **D7** your sign-up code: a promo code created in the OLD `generate/` before 0.1 (it is copied
into the new DB) or one of your own product codes.

- [ ] **6.1 A+O – api.scutoons.com:** A checks `/api/items/` (27 items), `/api/keys/keys.php` (same `publicKey` as
  the old `qr.scutoons.com/api/keys/keys.php`), `/api/certificates/<CODE>` for 3 real codes from different batches,
  printed UPPER CASE → `valid: true`, `/api/stockists/` → empty list, `/?c=x` → 404, and
  `php server/tests/entities-compare.php https://api.scutoons.com --report` (library = seed). O: `/generate/` asks for
  the password (if 5.5).
- [ ] **6.2 O – qr-next.scutoons.com (printed URLs):** with real codes: membership `/?c=CODE` and `?c=CODE` (no slash)
  → item page with Continue + Add to my account; a registered one → Read + "This is mine? File a claim"; promo → Create
  account; `/?c=CODE&skip=1` → redirect; unknown → 404 page; `/verify.html?code=<UPPER CASE>` → Authentic + fingerprint
  = the printed one; `/api/validate/?c=CODE` (JSON, `Access-Control-Allow-Origin: *`); `/api/keys/keys.php`.
  (Continue / Add-to-account links still go to the OLD viewer at scutoons.com during this test – expected.)
- [ ] **6.3 O – Your account + roles.** `https://next.scutoons.com/account?code=<D7 code>` → sign up with your email →
  the login mail arrives (check spam; SPF/DKIM). Then phpMyAdmin → DB-T → SQL:
  ```sql
  INSERT IGNORE INTO user_roles (user_id, role_id, granted_at, granted_by)
    SELECT id, r.id, UTC_TIMESTAMP(), 'cli' FROM users JOIN roles r ON r.name IN ('admin','creator') WHERE users.email = '<your email>';
  ```
- [ ] **6.4 O – admin.scutoons.com:** sign in (SSO: already signed in from 6.3) → Dashboard, Codes (search a real
  code), Events (the 6.2 scans with different IP hashes – one hash for all = `QR_PROXY_SECRET` differs), Claims (empty),
  Catalog (27 items, Albums), **Emails → set the admin recipients (D6)**, templates render de/en, the log shows 6.3's
  `login_code` as sent; versions of every app.
- [ ] **6.5 O – next.scutoons.com (viewer):** home, `/catpope`, chapter 0 (video plays; account → Medienqualität
  works), chapter 1 de + `/en/catpope/chapter/1`, `?code=CODE` unlocks on the device, inventory (basic dress on the
  avatar, testo bar), the camera opens and loads the model, `/catpope/credits`, `/catpope/download`, `/shop` → shop,
  `/stores` → empty state, account page (My claims, media quality, hide sensitive content).
- [ ] **6.6 O – creator.scutoons.com:** sign in → catpope → chapter 2 → preview de/en (broken record player on the first
  slice, the coupon on the last). **Do not publish yet** if 7.1 might rebuild the DB (edits would be lost).
- [ ] **6.7 O – shop.scutoons.com:** `/de/` and `/en/` list the products, Bandcamp links work.
- [ ] **6.8 O – Ownership end to end (DB-T only):** on a test code: register it to account A, file a claim from
  account B (with a photo), admin → Claims → transfer → both mails in Emails log; gift an item A → B (`gift_received`).
- Rollback: none needed (test hosts only).

---

## Phase 7 — Switch-day database (gate: keep DB-T or rebuild DB-P)

Decide before: **D8** accept that test-phase data is thrown away if the old DB changed since T0 (recommended; the
alternative is T4).

- [ ] **7.1 O – Gate.** In the OLD DB: `CHECKSUM TABLE qr;` and the 0.1 count query (T0: 469 / 35 / 381, MAX(created_at)
  2026-09-26 13:28:27, MAX(claimed_at) 2026-09-28 07:49:46, checksum **3162238151**). **Identical to 0.1 → keep DB-T**
  (it is already the production DB; tick 7.2–7.5 as `[-]`). Different (a registration, a new code, a re-signature since
  T0) → 7.2.
- [ ] **7.2 O – Fresh export** of the old DB at a quiet hour (as 0.1, new T0 values in "Current state").
- [ ] **7.3 O – DB-P:** new database in KAS → 3.1–3.5 on it.
- [ ] **7.4 O – Point the api at DB-P:** GitHub `production` `DB_NAME` / `DB_USER` / `DB_PASS` → DB-P; Actions →
  `server.yml` (or the api's deploy) → Run. A: `/api/health/` → `db ok`; `SELECT COUNT(*) FROM users` on DB-P = 0.
- [ ] **7.5 O – Redo the small setup on DB-P:** 6.3 (account + roles), 6.4 (admin recipients).
- Rollback: `DB_*` back to DB-T and redeploy the api.

---

## Phase 8 — Switch the public hostnames (system by system)

Decide before: **D9** the beta runs on the public domains – chapter 2 stays **draft** (only creators/admins read it)
or is **published** now (everyone on scutoons.com sees it a week before the public release)? · **D10** a quiet hour.

- [ ] **8.1 O – Keep the old system reachable first:** new subdomains `legacy.scutoons.com` → the old viewer folder,
  `qr-legacy.scutoons.com` → the old qr folder (SSL each). Verify both load. (The old viewer's API calls go to
  qr.scutoons.com – after 8.2 that is the new system; its sign-in features may fail on CORS – expected.)
- [ ] **8.2 O – qr.scutoons.com → `/web/qr/public/`** (SSL stays; re-issue if KAS asks). Repeat 6.2 on
  `qr.scutoons.com` with real codes, incl. 3 printed certificates on `/verify.html`. **Rollback: target back to the old
  qr folder (instant).**
- [ ] **8.3 O – scutoons.com + www.scutoons.com → `/web/viewer/dist/`** (SSL both). Repeat 6.5 on the real domain;
  scan a sticker with a phone end to end (qr → Continue → viewer → Add to my account). **Rollback: both targets back
  to the old viewer folder.**
- [ ] **8.4 A – After-switch checks:** `/version.json` on every host, the printed-URL list of PLATFORM.md "RULE: printed
  / public URLs" (status + target), `entities-compare --report` against the api.
- [ ] **8.5 O – Late-delta check** (old DB, 30 min after 8.2 and again the next day):
  `SELECT code, email, claimed_at FROM qr WHERE claimed_at > '<T0>' OR created_at > '<T0>';` → expected empty (the
  old landing page is only reachable on qr-legacy now). Rows → tell the guide (re-apply by hand in the new DB).
- Rollback of the whole switch: 8.3 and 8.2 targets back. Data written in the new DB since the switch (sign-ups,
  registrations, claims) is not in the old DB.

---

## Phase 9 — Beta go (Sunday 2026-10-04)

Decide before: **D9** (chapter 2) · **D11** testers (who, how many) and their codes: promo codes from admin → Codes →
New (type promo) or a batch · **D12** where testers report problems · **D13** confirm the ownership defaults: dispute
sign-up (a reader without an account may sign up only to file a claim), gift limits (`GIFT_COOLDOWN_HOURS` 24,
`GIFT_MAX_PER_DAY` 10, `GIFT_LOOKUPS_PER_DAY` 30, `CLAIM_MAX_PER_DAY` 5) – changeable by config later.

- [ ] **9.1 O – Chapter 2** per D9 (creator → catpope → season 1 → chapter 2 status → Publish).
- [ ] **9.2 O – Testers get:** `https://scutoons.com` (+ `/catpope`), their promo code (`https://qr.scutoons.com/?c=<CODE>`
  or `https://scutoons.com/account?code=<CODE>`), `https://shop.scutoons.com`, the feedback channel (D12), a short
  list of what to try: sign up, read chapters 0–2 de/en, avatar + wardrobe, camera (cucumber), record player repair
  (broken player in chapter 2 scene 1 + coupon at the end → combine), gift an item to another tester, media quality.
- [ ] **9.3 O/A – Watch daily:** `/web/api/logs/error.log` (FTP; paste excerpts to the guide), admin → Emails (failed
  mails), Claims (open), Events (scans, sign-ups), Dashboard; KAS traffic of qr.; `/api/health/`.

---

## Phase 10 — After the beta (towards the public release, ~a week later)

- [ ] 10.1 Fix the beta's issues on `platform` → `main` (deploys automatically).
- [ ] 10.1b Right after the assets merge: the 7 bunny keys (`BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT`,
  `BUNNY_STORAGE_PASSWORD`, `BUNNY_STORAGE_READONLY_PASSWORD`, `BUNNY_PULLZONE_ID`, `BUNNY_API_KEY`, `CDN_PUBLIC_URL`)
  as GitHub `production` secrets for the live asset manager (the merge adds them to `targets.json`); verify in admin →
  Assets → Storage (set / missing per key, no values).
- [ ] 10.1c Fonts (IBM Plex Mono, Bangers + OFL licences) to `scutoons/fonts/` on the CDN; the apps keep their local
  copies for the beta.
- [ ] 10.2 Asset layer: merge `assets` (migration `2026-10-02-assets` in phpMyAdmin, `ASSET_STORAGE=bunny` + `BUNNY_*`
  into `production`/targets, `assets:scan`) – ASSETS.md.
- [ ] 10.3 cdn.scutoons.com → bunny: custom hostname on pull zone 6731366 + CNAME → `scutoons.b-cdn.net` + SSL;
  `CDN_PUBLIC_URL` and `VITE_CDN_URL` → `https://cdn.scutoons.com` + redeploy (the legacy system loses the old FTP CDN
  then – keep legacy hosts only until that point); a final `assets:sync` first (ASSETS.md "domain switch"). The old FTP CDN
  becomes the backup target.
- [ ] 10.4 Folder restructure: `assets:layout` re-run + owner review, batch relink (to build), the model path code change,
  then prune (ASSETS.md "Migration plan").
- [ ] 10.5 Remove the test hosts `next.` / `qr-next.` (and T1's origin); later `PROXY_FULL_API=0` when qr's log shows
  no `/api/*` traffic; remove `generate/` + `GENERATE_PASSWORD` after admin parity.
- [ ] 10.6 Embassy go-live (EMBASSY.md "To go live"): subdomain, `order`, workflow trigger, roles, import.
- [ ] 10.7 After one release without problems: delete the old folders (keep backups); the old DB stays as an archive.
- [ ] 10.9 KAS FTP cleanup (after the switch + the CDN move): delete the users `cdn-sync` (f017c2d3), github actions
  `f017b642` and `f017b651`; remove their old secrets from GitHub (old qr repo, repo-level FTP secrets of the old viewer
  workflow). The main user stays, used only by hand.
- [ ] 10.8 Public release.

---

## Decide-before list (all phases)

| # | Decision | Blocks | Default / recommendation |
|---|---|---|---|
| D1 | old-layout files on the bunny zone: keep or prune | Phase 1 (no) | keep until the public release, then prune via Unused |
| D2 | root folder name; test hostnames | 2.1, 2.6, T1 | **decided: `/web/`** ("the house of the applications"); hostnames still proposed: `next.` + `qr-next.` |
| D3 | `VITE_API_URL` directly api. | 2.5 | yes |
| D4 | duplicate Dago/beetles code sets: which was printed | public release (not the beta) | decide in the beta week |
| D5 | embassy stays held back | 5 | yes |
| D6 | admin mail recipients (`MAIL_ADMIN_TO` / Emails page) | 6.4 | owner's address |
| D7 | the code you sign up with | 6.3 | promo code from the old `generate/` before 0.1 |
| D8 | test data may be discarded on a DB rebuild | 7 | yes |
| D9 | chapter 2 during the beta: draft or published | 8/9 | owner |
| D10 | switch time window | 8 | quiet hour, Sat 2026-10-03 |
| D11 | testers + their codes | 9 | promo codes |
| D12 | feedback channel | 9 | owner |
| D13 | ownership defaults: dispute sign-up, gift + claim limits | 9 | as built |
| – | Not blocking the beta: record player coupon place once chapter 5 exists, tape/CD player sourcing (ITEMS.md); Nova Lux vs Nova Ludwig in the credits; stores' 7 open questions (EMBASSY.md); media-quality thresholds; trailer from the prologue film; the 11 unused CDN files | – | after the beta |

---

## Progress log

- 2026-10-02 — runbook written (side-by-side + new DB strategy). Next: 0.1.
- 2026-10-02 — 0.1 done (export in the owner's Proton Drive backups; only the `qr` table). Owner: credentials only in the
  GitHub `production` environment; relayout BEFORE the bunny upload, new platform on `https://scutoons.b-cdn.net`
  (Phase 1 rewritten, waits for the assets merge), Phase 4 n/a. Next: 0.2 (+ the 0.1 numbers).
- 2026-10-02 — 0.3 done (legacy `/scutoons.com/` backed up). KAS FTP users recorded; 2.1 = new user "github actions next"
  rooted in `/web/` (T3 n/a), 2.2 folder layout + domain map, 10.9 FTP cleanup.
- 2026-10-02 — D2 decided: the new root folder is `/web/` (was proposed `/scutoons-next/`); test hostnames still proposed.
- 2026-10-02 — 0.2 done. Open: the 0.1 numbers, the 0.2 hash/publicKey check. Next in parallel: 0.4, 0.5, 2.1, 2.2, 2.4, 2.5, 2.6.
- 2026-10-02 — 0.1 numbers: 469 codes / 35 registered / 381 signed, MAX(created_at) 2026-09-26 13:28:27, MAX(claimed_at) 2026-09-28 07:49:46; per-edition + CHECKSUM pending.
- 2026-10-02 — 0.1 per edition: NULL 169, 3_catpope_vinyl 300 (= 469). No dago/beetles codes in production. CHECKSUM pending.
- 2026-10-02 — 0.1 complete: CHECKSUM TABLE qr = 3162238151 (Phase 7 reference).
- 2026-10-02 — 0.4, 0.5, 2.1, 2.2, 2.4, 2.5, 2.6 done; 2.3 partly (keypair pending); T1 done (6fcb82c). Next: keypair → 2.3 verify, then 3.1/3.2 on DB-T.
- 2026-10-02 — keypair reference SHA-256 409c1983…7bbe recorded (dev key 11934cea… must never go to production); batch manifests for 3.4 are in the same ZIP (`qr.scutoons.com/generate/files/batch/`).
- 2026-10-02 — 3.4 SQL generated (300 statements, vinyl batch only) → cdn/logs/edition-numbers-2026-10-02.sql.
- 2026-10-02 — 3.1 + 3.2 done (16 migrations, 3 triggers). Next: 3.4 import + check 469 / 300 numbered / 381 signed / 35 registered; 2.3 keypair upload pending.
- 2026-10-02 — 3.4 done; DB-T qr counts match (469 / 300 / 381 / 35). 3.3 waits for the assets merge (regenerated seed).
- 2026-10-02 — assets merged (b3be086, 67774da), relayout done (1.1), .env in C:\Development\scutoons, bunny zone empty. Migration 17 `2026-10-02-assets` added to 3.2; 3.3 unblocked. Next: 3.3 seed, migration 17, then 1.3/1.4 from C:\Development\scutoons\server. Worktree scutoons-assets can be removed later.
- 2026-10-02 — 3.3 + 3.5 done (48 entities, roles, users 0, editions NULL 169 + vinyl 300); migration 17 awaiting the owner's confirmation; 1.3 + 1.4 done. Owner running 1.5 (log cdn/logs/bunny-init-2026-10-02.json), then 1.6, 1.7.
- 2026-10-02 — migration 17 ran on DB-T (3.2 complete). 1.5 bunny upload running.
- 2026-10-02 — 1.5, 1.6, 1.7 done (bunny = cdn/root, CORS + Range verified). Open in Phase 1: 1.8. Next: 1.8, then Phase 5.
