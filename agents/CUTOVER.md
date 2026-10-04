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
| old CDN FTP (cdn.scutoons.com) | the owner's password store (the repo copy was deleted 2026-10-04) | nothing in the repo (`cdn/` removed 2026-10-04) |
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
| T2 | **n/a** – Phase 4 dropped, and `cdn/` removed 2026-10-04 (no sync.py). Was: `cdn/sync.py` skips `.json` / `.bin` → the camera model `models/mobilenet_v2_100_224/` never reaches cdn.scutoons.com. Add both to `SYNC_EXTENSIONS` – or upload that folder by hand (step 4.3). | Phase 4 | 10 min |
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
- [ ] **0.6 A – Local copies checked:** `media/` (then `cdn/root`) = 824 files / 855,854,341 bytes, identical copy in
  `C:\Development\scutoons-assets\cdn\root` (checked 2026-10-02) = the media backup; `server/db/local.sqlite` copied
  to `server/db/local.sqlite.bak-2026-10-0X-cutover`.
- [ ] **0.7 O (optional) – Old CDN listing:** WinSCP/FileZilla on the CDN FTP folder → a recursive listing saved to
  `logs/cutover/remote-listing-2026-10-0X.txt` (the sync never deletes, so this is documentation only).

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
- [x] **1.5 O – Apply:** *(done 2026-10-02, log now `logs/cutover/bunny-init-2026-10-02.json`)* `php bin/console assets:sync --source-root=../cdn/root --hash --apply --overwrite --purge-zone
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
  | `/web/embassy/dist/` | target `embassy` | `embassy.scutoons.com` (new, owner sets it up) | same |
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
  system keeps `cdn.scutoons.com` with what it already has. The repo's FTP tooling (`cdn/sync.py`) was removed
  2026-10-04; a later backup to the old FTP CDN (10.3) = the `FtpStorage` driver (ASSETS.md "Backup").

---
## Phase 5 — Deploy the new platform into `/web/` (nothing points at it except the new/test hostnames)

Decided: **D5** embassy ships with the release (owner 2026-10-04; in `targets.json` `order`).
Preconditions: **2.1 done (FTP secrets = the subfolder user!)**, 0.4 done, T1 done. After the assets merge also: the 7 `BUNNY_*` / `CDN_PUBLIC_URL`
secrets in `production` (step 10.1b – needed from the first deploy, since the merge ships the asset manager).

- [ ] **5.1 A – Local dry run:** `npm run deploy:dry-run` (builds everything; plan: api → `api/`, qr → `qr/`, viewer →
  `viewer/dist/`, shop, creator, admin; protected `storage/**`, `db/**`, `logs/**`). Plus the full suite on the tree
  to release (PLAN.md "definition of done").
- [ ] **5.2 O (with the coordinator) – Merge `platform` → `main` and push.** Every push to `main` deploys the touched
  apps (environment `production`) – into `/web/` because of 2.1.
- [ ] **5.3 O – GitHub → Actions → "Deploy all" → Run workflow with dry run** → read the plan, then **without dry run**
  (order api → qr → frontends). Every run prints the migrations reminder – already done in Phase 3.
- [x] **5.4 A – Verify** *(done 2026-10-02: all six on `88e4d88`, `npm run deploy:status`)* (PHP note: KAS runs PHP 8.5, CI builds/tests on 8.3 – if `/api/health/` or qr show errors or
  deprecation output, set the subdomains to PHP 8.3 in KAS): `https://api.scutoons.com/api/health/` → `status ok`, `db ok`, `version 1.0.0`, `git_sha` =
  main's head; `https://{admin,shop,creator,next}.scutoons.com/version.json`, `https://qr-next.scutoons.com/version.json`.
- [x] **5.5 – n/a (owner 2026-10-02: no legacy `generate/` on the new platform).** The old generate UI is excluded from the api deploy (`public/generate/**`); the verify page's `generate/get-signature.php` + signature-file routes are API routes and keep working. Former text: upload the backup's `generate/.htpasswd` to
  `/web/api/public/generate/.htpasswd`, set `AuthUserFile` in `/web/api/public/generate/.htaccess` to its absolute
  path (KAS shows `/www/htdocs/<account>/web/api/public/generate/.htpasswd`).
- [x] **5.6 – One deploy pipeline** *(2026-10-02, after the ECONNRESET failures of parallel uploads)*: pushes to `main` run
  only `deploy.yml` – it deploys the targets whose live `git_sha` is behind (per `watch` paths), one at a time, with one
  FTP retry; per-app workflows and "Deploy all" are manual buttons in the same queue. Check with `npm run deploy:status`
  (deploy/README.md "Workflows").
- Rollback: Actions → the app's workflow on an earlier commit; nothing public points here yet.

---

## Phase 6 — Test the new system on the test/new hostnames (DB-T)

Decide before: **D6** admin notification recipients (who gets "new ownership claim" mails; Emails page or
`MAIL_ADMIN_TO`) · **D7** your sign-up code: a promo code created in the OLD `generate/` before 0.1 (it is copied
into the new DB) or one of your own product codes.

- [x] **6.1 A+O – api.scutoons.com:** *(done 2026-10-02: relayout-paths SQL run, compare re-run – only the expected differences left; `/generate/` → 404 as 5.5)* *(A done 2026-10-02: items 27; certificates valid for real membership / promo /
  page codes (the keys match); stockists empty; `/?c=x` 404. entities-compare (locally: `php -d curl.cainfo=<ca-bundle>`):
  the expected differences are chapter 2 (draft, D9), the draft toons toptoy / yellowmouse and the dev-only sandbox; the real
  one: `map`, `characters`, `asset-requests` still had the pre-relayout paths (map + character images 404) → **O: run
  `shared/content/seed/2026-10-02-relayout-paths.mysql.sql` in phpMyAdmin (SQL tab), then A re-runs the compare.** Not
  cutover-related, open since before: `boy_full.svg`, `crows_portrait.svg` exist on no CDN.)* A checks `/api/items/` (27 items), `/api/keys/keys.php` (same `publicKey` as
  the old `qr.scutoons.com/api/keys/keys.php`), `/api/certificates/<CODE>` for 3 real codes from different batches,
  printed UPPER CASE → `valid: true`, `/api/stockists/` → empty list, `/?c=x` → 404, and
  `php server/tests/entities-compare.php https://api.scutoons.com --report` (library = seed). O: `/generate/` asks for
  the password (if 5.5).
- [x] **6.2 O – qr-next.scutoons.com (printed URLs):** *(done 2026-10-02: all 9 cases; A: validate CORS + keys checked)* with real codes: membership `/?c=CODE` and `?c=CODE` (no slash)
  → item page with Continue + Add to my account; a registered one → Read + "This is mine? File a claim"; promo → Create
  account; `/?c=CODE&skip=1` → redirect; unknown → 404 page; `/verify.html?code=<UPPER CASE>` → Authentic + fingerprint
  = the printed one; `/api/validate/?c=CODE` (JSON, `Access-Control-Allow-Origin: *`); `/api/keys/keys.php`.
  (Continue / Add-to-account links still go to the OLD viewer at scutoons.com during this test – expected.)
- [x] **6.3 O – Your account + roles.** *(done 2026-10-02: signed up with promo KbYfB3qp; SMTP_PASS secret was wrong → fixed, checked with `server/bin/smtp-login-test.php`)* `https://next.scutoons.com/account?code=<D7 code>` → sign up with your email →
  the login mail arrives (check spam; SPF/DKIM). Then phpMyAdmin → DB-T → SQL:
  ```sql
  INSERT IGNORE INTO user_roles (user_id, role_id, granted_at, granted_by)
    SELECT users.id, r.id, UTC_TIMESTAMP(), 'cli' FROM users JOIN roles r ON r.name IN ('admin','creator') WHERE users.email = '<your email>';
  ```
- [x] **6.4 O – admin.scutoons.com:** *(done 2026-10-02: dashboard 500 = `REVOKE` reserved in MariaDB → fixed 93946ef; `settings` table was missing on DB-T although 3.2 lists it → created by hand; recipients set (D6): tilman@porschuetz.de, admin@scutoons.com)* sign in (SSO: already signed in from 6.3) → Dashboard, Codes (search a real
  code), Events (the 6.2 scans with different IP hashes – one hash for all = `QR_PROXY_SECRET` differs), Claims (empty),
  Catalog (27 items, Albums), **Emails → set the admin recipients (D6)**, templates render de/en, the log shows 6.3's
  `login_code` as sent; versions of every app.
- [x] **6.5 O – next.scutoons.com (viewer):** *(done 2026-10-02: all but the QR camera on the owner's webcam – never worked there (blurry webcam), test on the phone in Phase 9; credits/download pages work but nothing links to them → 10.12)* home, `/catpope`, chapter 0 (video plays; account → Medienqualität
  works), chapter 1 de + `/en/catpope/chapter/1`, `?code=CODE` unlocks on the device, inventory (basic dress on the
  avatar, testo bar), the camera opens and loads the model, `/catpope/credits`, `/catpope/download`, `/shop` → shop,
  `/stores` → empty state, account page (My claims, media quality, hide sensitive content).
- [x] **6.6 O – creator.scutoons.com:** *(done 2026-10-02: sign-in, chapter 2 preview de/en. The preview does not play reader events, so the record player / coupon pickups are tested in the viewer once chapter 2 is released on 2026-10-04 – Phase 9.)* sign in → catpope → chapter 2 → preview de/en (broken record player on the first
  slice, the coupon on the last). **Do not publish yet** if 7.1 might rebuild the DB (edits would be lost).
- [x] **6.7 O – shop.scutoons.com:** *(done 2026-10-02)* `/de/` and `/en/` list the products, Bandcamp links work.
- [ ] **6.8 O – Ownership end to end (DB-T only):** *(2026-10-02, owner's vinyl #127 `LyBWEkur`: register to A, claim from B (dispute sign-up), admin transfer + the old-owner mail work; open: gift B → A, transfer back to A)* on a test code: register it to account A, file a claim from
  account B (with a photo), admin → Claims → transfer → both mails in Emails log; gift an item A → B (`gift_received`).
- [x] **6.9 O – Assets on production:** *(done 2026-10-03: first rescan failed – MariaDB insert id after UPDATE, fixed 50b0782; then 720 files, 659 uses, 10 missing; the creator's trailer picker lists the videos. Open: the 10 missing paths; the trailer tile shows no poster)* admin → Assets → **Rescan** (registers the bunny files as assets + the usage index;
  until then the creator's asset picker says "Nothing found"). Check the summary's `missing`, then pick a video in the
  creator (season → trailer). *(found 2026-10-02 by the owner in the creator)*
- More owner tests before the switch: to be listed (owner 2026-10-02: "we still need to test some things").
- Rollback: none needed (test hosts only).

---

## Phase 7 — Switch-day database (gate: keep DB-T or rebuild DB-P)

Decide before: **D8** accept that test-phase data is thrown away if the old DB changed since T0 (recommended; the
alternative is T4).

- [ ] **7.1 O – Gate.** *(owner 2026-10-02: DB-T stays the production DB – no rebuild; 7.2–7.5 drop. A late delta is handled by 8.5.)* In the OLD DB: `CHECKSUM TABLE qr;` and the 0.1 count query (T0: 469 / 35 / 381, MAX(created_at)
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
- [ ] **8.4 A – After-switch checks:** first set `status` in `deploy/targets.json` to the real hosts (qr →
  `https://qr.scutoons.com/version.json`, viewer → `https://scutoons.com/version.json`) so `deploy.yml` and
  `npm run deploy:status` read the live commits there; `/version.json` on every host, the printed-URL list of PLATFORM.md "RULE: printed
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
- [x] 10.1b *(done 2026-10-02. Hint: setting secrets with `gh` gave 404 until gh was re-authorised – the fine-grained
  token did not cover the `scutoons` repo.)* Right after the assets merge: the 7 bunny keys (`BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT`,
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
- [ ] 10.6 Embassy go-live (EMBASSY.md "To go live"): subdomain + SSL, roles, import (`order` done 2026-10-04).
- [ ] 10.7 After one release without problems: delete the old folders (keep backups); the old DB stays as an archive.
- [ ] 10.9 KAS FTP cleanup (after the switch + the CDN move): delete the users `cdn-sync` (f017c2d3; its repo tooling is gone since 2026-10-04), github actions
  `f017b642` and `f017b651`; remove their old secrets from GitHub (old qr repo, repo-level FTP secrets of the old viewer
  workflow). The main user stays, used only by hand.
- [x] 10.10 Local MariaDB in Docker instead of SQLite for dev + tests (owner 2026-10-02, after the admin dashboard 500 on
  DB-T that SQLite never showed): same server version as all-inkl, the mysql migrations, CI runs the suite on it too.
  *(done 2026-10-03, branch `dev-mariadb`; server/docs/DATABASE.md "Local MariaDB")* `docker/compose.yml` (MariaDB
  10.11, port 13306, volume, healthcheck; Adminer 13380 behind a profile); `npm run dev:db` = container + databases +
  migrate + the `*.mysql.sql` content seed + promo `devpromo` + switch + API restart + test user; `npm run dev:db sqlite`
  back (SQLite stays the default); `npm run dev:db test` / `phpunit -c phpunit.mysql.xml` = the unit suite on MariaDB;
  CI job `test-mariadb` in server.yml. Found on MariaDB: two signups at the same moment deadlocked in the basic dress
  (fixed, test); `qr.code` was case-insensitive in production (utf8mb4_general_ci) → 10.24; a flaky test fixture.
  Production's values (owner, phpMyAdmin 2026-10-03: 10.11.14, sql_mode NO_ENGINE_SUBSTITUTION, latin1 server
  defaults) are the container's defaults since branch `db-case`.
- [ ] 10.11 Server errors in admin (owner 2026-10-02, after reading `/web/api/logs/error.log` by FTP to find the dashboard
  500): an admin-only API endpoint that reads the newest lines of `LOG_PATH` (newest first, size-capped, no secrets –
  messages only), and an admin page "Server errors" showing them (filter by day / text). Same for the qr app's log.
- [ ] 10.12 Viewer: `/catpope/credits` and `/catpope/download` work but no page links to them (owner 2026-10-02) –
  **owner decision: link them (toon page / season slot / menu) or drop the routes.** Also: the QR camera never decoded on the owner's webcam (blurry
  image) – check focus / resolution constraints and a sharper decode path once the phone test (Phase 9) is in.
- [ ] 10.13 Visibility rule (owner 2026-10-02): `draft` is seen by creators only, `coming-soon` is the only non-published
  status the public sees. Production's public API follows it (season 1 lists chapters 0, 1, 2 = coming soon). Check
  the signed-in creator's viewer list: it showed chapters 4 and 5 as "coming soon" while the creator marks them draft.
  Later: drafts only for creators who are authors of that toon.
- [ ] 10.14 Admin claim page rework (owner 2026-10-02, during 6.8: "the ui in admin for claim needs rework") – details
  from the owner; first guess: the two side-by-side note fields + the decision buttons hidden in the bottom bar.
  Also: the viewer's claim form promises the current owner never reads the statement – true for owners, but an owner who
  is also an admin recipient gets it in `ownership_claim_admin`; reword if the owner wants.
- [ ] 10.15 Content source of truth is the DB now (owner 2026-10-02): local dev from `export-content` of production
  instead of the stale `shared/content/seed`; the seed becomes a starter set for empty databases (or goes).
- [x] 10.16 Mail Reply-To (owner 2026-10-02: a mistake, fixed before the beta): `MAIL_REPLY_TO` = hello@scutoons.com
  (Reply-To header; the texts say "reply to this email", mail comes from no-reply@).
- [ ] 10.16b Admin claim page: no raw data (owner 2026-10-02) – part of 10.14.
- [ ] 10.17 Deploys are not atomic: FTP replaces files one by one, so for a few seconds the API runs half old / half new
  code (2026-10-02 23:49: "No entry or class found for Transport" during a deploy). Upload to a fresh folder and switch
  (rename / symlink), or a maintenance flag during the upload.
- [ ] 10.18 Batches + code generation on real data (owner 2026-10-03): admin → Batches (and the generate side) still
  read the static lists (editions / items / batch manifests from files); populate them from the database (catalog,
  editions, codes) instead.
- [ ] 10.19 Setup / installer for a fresh environment (owner 2026-10-03): one step that runs the migrations, the content
  seed and the asset scan in order and checks each (the cutover did them by hand – 6.9 found the empty asset registry
  only through the creator's picker). Content keeps plain CDN paths; the registry is the index for the tools.
- [ ] 10.20 Asset picker: the selected tile turns black but its kind icon stays black (blank white box) – invert the icon
  too (owner screenshot 2026-10-03). The tile's size sums every file incl. the master (roar-little-kitty 491 MB) – show the
  default rendition's size or label the total.
- [ ] 10.21 Asset picker rework (owner 2026-10-03: "we do have to fix the ui of the picker and its relation to the page
  where we pick media – there is redundance and confusion"): drafts first (2–3 layouts, owner picks). Known: the choice box
  opens above the grid, the upload is a heading-looking fold-out, the page's own path fields repeat what the picker shows;
  includes 10.20.
- [ ] 10.22 Creator chapter tree: "Add group" + "Add separate scene" (cut off, owner 2026-10-03: "weird") – the second is a
  storage detail (a ref group = its own scene entity, from the old file layout). One "Add group"; the editor picks the
  storage. Scene keys (`chapter2/scene-14`) are internal IDs only, never shown as names (owner 2026-10-03: "legacy from
  files – in the db we don't care about ids that way"); the label is the name (done for the tree / crumbs: 3109fd8).
- [ ] 10.23 Asset jobs on production (owner 2026-10-03: "Optimize" stays queued): `assets:jobs` reads the queue from its
  own DB – locally the dev DB, and production's MariaDB is not reachable from outside. The runner on the owner's machine
  (ffmpeg) should take jobs through the API (admin token): list queued → download the source from bunny → transcode →
  upload → mark done. Job #1 (video-renditions, 2026-10-02 22:45) can be dropped.
- [ ] 10.24 Codes case-sensitive + strict session sql_mode (owner 2026-10-03, branch `db-case`): **O – run
  `server/migrations/mysql/2026-10-03-code-case.mysql.sql` in phpMyAdmin** (whole file at once; before or after the API
  deploy, both orders are safe; idempotent). Every code lookup is exact, also `/api/certificates/` + the verify page
  (no case-insensitive fallback; certificates printed before show UPPER CASE – typed that way they are unknown now, the
  QR / signature file still verify); new certificates print the exact code. The API sets a strict session `sql_mode`
  (`DB_SQL_MODE`, production's server mode is not strict). Optional check afterwards: the information_schema SQL in
  server/docs/DATABASE.md "Matching production" (all utf8mb4; the code columns utf8mb4_bin).
- [ ] 10.25 Map places ↔ story: where a place lives (owner 2026-10-03, "lets investigate … isn't group the better place?" –
  **decide first, after the open UI fixes**). Findings 2026-10-03:
  - **Unlock today:** on the place (world map `place.unlock` = toon, chapter, `slice` = the slice's image path, `at`).
    The viewer unlocks it when the reader *sees* that slice (`world.svelte.ts` `sliceSeen` → `placesForSlice`); a place
    whose chapter has no slices yet unlocks from the chapter position `at` / a finished chapter (`unlocksFromProgress`).
    The position never unlocks a slice place (VN choices skip groups). The slice inspector's "Unlocks place" box is the same
    setting from the slice's side – its wording confuses ("the slice represents the place").
  - **Jump today:** VN `jumpTo` = chapter + optional group label (`/catpope/chapter/2?label=scene-15`), the next bar too –
    groups are the jump unit. The map card links only `place.chapter` (opens the chapter at its start, not at the place).
  - **Idea:** the place on the **group** ("this scene is: Cathedral"): unlocks when the reader enters the group, the card
    jumps to `chapter/N?label=<group>`, one setting shown from both sides (map builder: "Chapter 2 · scene-15"); chapter +
    position stays the fallback for chapters without groups (3–5).
  - **Doesn't line up yet:** chapter 1 is one group of 95 slices with two places (apartment slice 2, apartment-blocks
    slice 72); chapter 2 `scene-20` holds church-square (slice 20) + cathedral (21); `scene-17` has none. Group labels are
    not the screenplay scene numbers (places say scene 13–17, groups are scene-14 … scene-20).
  - **Blocker:** splitting groups breaks the reader's infinite scroll (owner 2026-10-03) – fix that first, or keep groups
    as they are and let a place point at a group + an optional slice (unlock precision) instead of splitting.
  - **Open decisions:** split vs. group + slice; rename groups to their places (labels are in jump links – only with OK).
- [x] 10.27 VN story logic – **built 2026-10-03 for the release** (owner: "we have to build it now. it is part of the release").
  Concept: jumpTo = navigation (groups are pages, by design); the story's own flow lives in the group's script. Done:
  one runner (`viewer/src/lib/visual-novel/runner.ts`, group script + slice / item triggers); `label` (named point) +
  `divert` (event, choice action, a slice event diverting into its group's script); `if` ("Only if") on any event;
  `setVariable` (set / add); choice actions as a list; `jumpTo.target.anyOf` (a random group, never the current one);
  `{var:clock}`; validation (points unique, diverts hit a point, VN flow per group); creator forms. Chapter 2 stroll:
  `scripts/dev/stroll-chapter2.mjs` (local), `npm run check:stroll` (headless play-through, 15 checks). Still open from the
  research: `once` choices + fallback, visit counts. Original notes:
- [ ] (history) 10.27 VN story logic, ink's ideas in our JSON events (owner 2026-10-03, **after 10.25**; research into inkle's ink).
  **Owner decision 2026-10-03: adopt some of the concepts below – which ones is still open (pick before starting).**
  Not ink itself: ink is a text script (our authors use the creator's forms), its localisation is one file per language
  or line-ID tooling (breaks our per-field de source / en override), and knots would be a second address system next to
  group labels. Today: variables are written (`set`/`add`, `{var}` in text, item grants) but nothing reads them – no
  conditions; a choice has one action; the only branch is leaving the group (a choice without jump continues with the same
  lines). Steps, all shared (structural) fields so translation and 10.25 stay untouched:
  1. One runner: `toon/interactions/visual-novel.svelte` and `visual-novel/event-processor.ts` both run events (the
     README calls the second deprecated, slice triggers + placed items still use it) – one shared step function first.
  2. `addChoice.params.if: { variable, op: '=='|'!='|'>='|'<', value }` – schema, `checkEvent` in validate.ts, an
     "Only if" fold under Advanced in `vn-events.svelte`, the runner skips the choice when false. Then on `showMessage`.
  3. `once` choices (+ a fallback choice) and `onSelect` as a list (set a flag *and* jump).
  4. In-group labels + `jumpTo {type:'label'}` (branch for a few lines, rejoin – ink's gathers; guard against loops).
  5. Visit counts per group / label (`seen:scene-15`) next to the variables.
- [ ] 10.26 Delete a chapter safely (owner 2026-10-03: "how could i delete a chapter? … super dangerous"). Today nothing
  can be deleted: the API has no delete for entities (versions are append-only), so no chapter is lost. Numbers are given
  automatically (next free, never reused) and are the chapter's identity (entity `chapter:<toon>:<n>`, URL, release order;
  shown big in the chapter's Meta since f345743). Proposed rules, enforced by the **server**, the creator only mirrors them:
  only a chapter that was **never published** in any language (status draft) and is the **last of its season**; a typed
  confirmation (the chapter number, in-page dialog); delete = removed from the season's sequence + the entity marked deleted,
  its versions kept (admin can restore; purging is a separate admin step). A published chapter is never deleted – at most
  hidden (draft / coming soon), so links and reading progress keep working.
- [ ] 10.29 **Catalog into the database** (owner 2026-10-04: "lets just do it properly … we have to do them all now"): items, recipes,
  albums, products, editions, featured, permissions / code types, toons (admin-created) → tables + history; slots / stats → code;
  basic admin UI with the asset picker and bulk imports. Plan and status: agents/CATALOG.md. Part 1 (tables, loaders,
  admin API) built 2026-10-04 on branch `catalog/db`: **O – run `server/migrations/mysql/2026-10-04-catalog.mysql.sql` in
  phpMyAdmin** (whole file at once; before or after the deploy; idempotent) – steps + check query in CATALOG.md "Status".
- [ ] 10.28 **SCUTOONS MCP server** – an agent with the rights of the signed-in creator (owner 2026-10-03: "very important
  … i want to also connect other services that help me create more sophisticated assets"; first step right after the
  release, branch `mcp`). Why MCP and not only an "Ask" panel: the API can do much more than the UI, any MCP client can use
  it (Claude Code / Desktop today, claude.ai connector + the creator's own panel later), and other MCP servers (asset
  generation, image / audio tools) combine with it in the same client – the agent makes an asset there and puts it into a
  slice here.
  - **Package** `tools/mcp` (`@scutoons/mcp`, TypeScript, official MCP TypeScript SDK; versions checked when building).
    Calls the existing PHP API with the creator's token (`Authorization: Bearer`, like the creator) → the API's role
    checks apply unchanged: the agent can do exactly what that creator can.
  - **Tools** (step 1): `list_toons`, `list_chapters` / groups / points (`scriptLabels`), `read_entity` (draft or published,
    one or all languages, with version ids), `save_draft` (content + base version → 409 on a newer edit; runs
    `@scutoons/content` `validateFile` first and returns the problems), `validate`, `list_versions` / `restore_version`,
    `search_assets` (+ upload later), characters / cast / moods, map places. **Publish** is a separate tool, off by default
    (env flag), one explicit confirmation per call. Saves only ever create new draft versions (append-only, restorable).
  - **Resources**: the format rules the agent reads first – VN events (label / divert / if / setVariable / choice actions /
    anyOf, `vn-script.ts`), text presets, slice sizes and media variants, entity keys, the one-click and flow rules.
  - **Transport**: stdio first (local: Claude Code / Desktop against local or production API, token from `.env` /
    `npm run dev:test-user` locally). Remote (Streamable HTTP + sign-in, claude.ai connector, the creator's "Ask" panel via
    the API's MCP connector) is step 2 – the shared hosting (FTP deploy) can't run a long-lived Node server: a small Node
    host or a PHP implementation, owner decision then.
  - **Checks**: a script like `check:stroll` that drives the tools against the local stack (read → patch → validate → save
    draft → restore), never against production.
  - **Example task**: "set up the chapter 2 stroll" (today `scripts/dev/stroll-chapter2.mjs`) as one request.
  - **Step 1 built (2026-10-03/04, branch `mcp`)**: `tools/mcp` (`@scutoons/mcp`, MCP TypeScript SDK v2
    `@modelcontextprotocol/server` 2.3 – the split packages that replace `@modelcontextprotocol/sdk`; zod 4). Runs from
    the repo with Node's type stripping (`bin/scutoons-mcp.mjs` → `src/main.ts`, nothing built). Config: `SCUTOONS_API`
    (default local), `SCUTOONS_TOKEN` (locally falls back to `logs/dev/test-user.json`, never for a remote API),
    `SCUTOONS_MCP_PUBLISH=1` (publish tool, off by default). Tools: `list_toons`, `list_chapters` (groups: label, scene
    entity, slices, `scriptLabels` points), `list_entities` (every stored document of a toon / the platform),
    `search_content` (full text in a toon's documents, the screenplay per line), `read_entity` (draft / published /
    version id, one or every locale, `path` / `lines` for a part), `read_characters` (cast season 1 like the creator,
    looks, moods), `read_map_places`, `search_assets`, `upload_asset` (a new file from a URL or local file → the asset
    manager, 409 when the path is taken), `validate` (validateFile with the chapter editor's context + validateLocales;
    validateCast for cast), `save_draft` (`ops` set / insert / remove on the `base` version, or whole `content`; errors
    refuse the save unless `allow_errors`; message gets "(agent)"; 409 explained), `list_versions`, `restore_version`,
    `submit_for_review`, `publish_entity` (only with the env flag, `confirm: true` per call). Resources
    `scutoons://rules/{workflow,entity-keys,vn-events,flow,text-presets,slices}` generated at start from the source.
    `npm run check:mcp` (`scripts/dev/mcp-check.mjs`, 31 checks, local only, restores what it changes). README: Claude
    Code / Desktop, local + production. Not validated yet: season status of a chapter and placed items against the
    catalog (the creator passes both).
  - **Step 2 decisions (owner 2026-10-04)**:
    - **Order:** the creator's "Ask" panel is the way for creators; the API token (step 1, stdio) stays for Claude Code /
      Desktop; no claude.ai connector for now (no OAuth needed – revisit only if creators want claude.ai).
    - **Host: bunny.net Magic Containers** (EU, the same bunny account as the CDN): the Ask service in a Docker container.
      Billed by usage (~$0.02 / core-hour + RAM, account minimum $1/month) – check the real idle cost in the 14-day trial.
      Rejected: Cloudflare Workers (US; free plan 10 ms CPU per request is tight for validating a big chapter; needs
      bundling), bunny Edge Scripting (Deno, a port), Koyeb / Render / Hetzner / Scaleway (a second provider).
      Also considered and set aside (owner 2026-10-04: "we stick to the node server on bunny"): the agent loop in the PHP
      API (tools in the browser, or a PHP port of the story rules). Separate idea, not decided: the API checking every
      save against the generated JSON Schemas (`shared/content/schema`).
    - **Auth: long-lived personal access tokens** for Claude Code / Desktop (owner's choice; OAuth was recommended as
      safer). To build: a creator page "Tokens" – create (shown once, only the hash stored), name, last used, revoke; never
      the browser session; the API accepts them as Bearer like a session. Expiry optional.
    - **"Ask" panel: the owner pays** (SCUTOONS, personal business); model Claude Sonnet 5.5. Every AI call is logged:
      creator, service, model, tokens, cost → an admin overview by creator and service per day / month. **No limits in
      SCUTOONS** (creators need to get the job done; the owner controls spending on the Anthropic account side).
    - **Asset upload: yes** (`upload_asset`; remote: from a URL only).
    - **Publish: keep the tool** (drafts get reviewed; the chat history shows what an agent did). Today off by default with
      `confirm: true` per call, and the API lets every creator publish. Review levels (who may publish without a review)
      are a separate task in the API, for agents and the UI alike.
    - **Chat history: yes** (owner 2026-10-04: "I need to see what I did before if something stops or breaks").
  - **Step 2 built (2026-10-04, branch `mcp`)**:
    - **Ask service** `tools/mcp/src/ask.ts` (`npm run dev:ask`, port 9002 in dev-ports; `npm run dev` starts it when the
      repo-root `.env` has `ANTHROPIC_API_KEY`): `POST /api/ask`, Server-Sent Events. The creator's session (Bearer or the
      `scutoons_sid` cookie) checked against `/me/` (role creator); the MCP tools in-process (InMemoryTransport) with that
      session; Claude Sonnet 5.5 (server-side fallback `default`, effort medium, cached system prompt with the format
      rules); context per message: page, the header's text language, the app language. Every API call → JSON line in
      `logs/ask/usage.jsonl` (creator, model, tokens, cost in USD). A loop guard of 60 rounds (not a spending limit).
    - **Chat history** in the API: table `agent_chats` (migration `2026-10-04-agent-chats`, **to run on production**),
      `/api/agent-chats` (list / get / put / delete, role creator, own chats only, 503 without the table). The service
      saves after every round and loads a chat to go on with it.
    - **Creator panel** (`creator/src/lib/components/ask/ask-panel.svelte`): round header button; docked right, the page
      makes room, drags wider; full screen on phones. Streams the answer, each tool call a step, saved drafts as links;
      stop, new chat, earlier chats (clock button), the open chat comes back after a reload. `VITE_ASK_URL` for
      production (empty = "not available").
    - **Deploy prepared (2026-10-04, branch `ask-deploy`)**: `tools/mcp/Dockerfile` (+ `Dockerfile.dockerignore`, build
      from the repo root; node:22-alpine, runs the TS sources, port 8080, health `/api/ask/health`, SIGTERM; built and run
      locally); `.github/workflows/ask-image.yml` pushes `ghcr.io/mandymozart/scutoons-ask:latest` on changes to main.
      Usage log in the API: table `agent_usage` (migration `2026-10-04-agent-usage`), `POST /api/agent-usage` (the
      creator's session + the shared `AGENT_SERVICE_KEY` / `SCUTOONS_SERVICE_KEY`; user from the session), admin
      `GET /api/admin/agent-usage?from&to&group=day|month` (per creator / service / model); the service falls back to its
      file. Deploy: `AGENT_SERVICE_KEY` into the API's config.php (targets.json), `VITE_ASK_URL` into the creator build.
      Verified locally: a question → 2 calls, $0.04, one row per call in the database, the overview sums them.
      **Owner steps** (tools/mcp/README.md "Production"): the two migrations in phpMyAdmin; the secret `AGENT_SERVICE_KEY`;
      bunny Magic Containers app (image from ghcr.io, port 8080, env `ANTHROPIC_API_KEY`, `SCUTOONS_API`, `ASK_ORIGINS`,
      `SCUTOONS_SERVICE_KEY`); DNS `ask.scutoons.com`; the variable `VITE_ASK_URL`.
    - **Open:** an admin page for the usage overview (the API is there); check that bunny's proxy streams the answer
      (Server-Sent Events) and doesn't buffer it; personal access tokens.
  - **Review levels (owner 2026-10-04; follows "Publish: keep the tool")** – who may publish without a review:
    - **Decided:** not a role – a relation per toon, set by an admin, like GitHub reviewers. `reviewer` (per toon or `*`)
      gets a mail on every submit and publishes or sends back; `needs_review` (per account per toon or `*`) = may save
      drafts and submit, not publish / unpublish. No rule = publishes directly (as before; no backfill). Reviewed levels:
      every toon-level entity (toon page + settings, season, chapter + scenes, cast, credits, **script** too); platform-wide
      ones (characters, moods, people, roles, events, map, asset-requests) publish directly for everyone. A submit is
      pinned to its version (the reviewer publishes what they read; newer drafts stay drafts); a newer submit of the same
      file annuls the open one. No reviewer assigned → `409 no_reviewer` "ask an admin to add a reviewer". Mails: review
      requested (reviewers), published / sent back (author) – one per person per action, listing the files.
    - **Built (API, branch `claude/vigilant-chandrasekhar-67b979`):** migration `2026-10-04-reviews` (**production: run it**;
      two new tables `content_review_rules`, `content_reviews`), `Domain\Content\Reviews`, `GET|POST /api/reviews`,
      `POST /api/reviews/publish|return`, `403 review_required` on `/api/entities/<key>/publish|unpublish`, admin
      `/api/admin/review-rules` + `/api/admin/users/{id}/review-rules/{toon}/{rule}`, mails `review_requested` /
      `review_published` / `review_returned` (de + en), setting `CREATOR_URL`. server/docs/API.md "Reviews", PHPUnit `ReviewsTest`.
    - **Built (UI, drafts approved by the owner 2026-10-04):** admin user page → section "Reviews" (the account's rules,
      add toon / All toons + Reviewer / Needs review, remove). Creator: for `needs_review` accounts every page's Publish
      button is "Submit for review" (the publish dialog + an optional note; `lib/editor/reviews.svelte.ts`), the history
      panel hides publish / unpublish for them and marks versions "in review"; the dashboard shows "To review" (one row per
      submit: author, time, languages, note, "newer drafts" hint, Send back / Publish with a note to the author) and "Your
      submissions" (in review / sent back with the reviewer's note) – both only when not empty. Mails name the toon
      ("Cat Pope", not the id) and link to the dashboard (owner: no further action needed than the list).
    - **Agents (2026-10-04, branch `ask-deploy`):** the MCP tool `submit_for_review` (`POST /api/reviews`, items pinned to a
      version + note); `publish_entity` answers `403 review_required` for `needs_review` accounts – the agent submits instead.
- [ ] 10.8 Public release. Set one version everywhere (root + every app + api/qr → `1.0.0`; today root 0.0.0, frontends 0.1.0, api/qr 1.0.0) and tag the commit `v1.0.0`; optional tag `v1.0.0-beta.1` on the beta commit (owner 2026-10-03).

---

## Decide-before list (all phases)

| # | Decision | Blocks | Default / recommendation |
|---|---|---|---|
| D1 | old-layout files on the bunny zone: keep or prune | Phase 1 (no) | keep until the public release, then prune via Unused |
| D2 | root folder name; test hostnames | 2.1, 2.6, T1 | **decided: `/web/`** ("the house of the applications"); hostnames still proposed: `next.` + `qr-next.` |
| D3 | `VITE_API_URL` directly api. | 2.5 | yes |
| D4 | duplicate Dago/beetles code sets: which was printed | public release (not the beta) | decide in the beta week |
| D5 | embassy stays held back | 5 | no – ships with the release (owner 2026-10-04) |
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
- 2026-10-02 — 3.4 SQL generated (300 statements, vinyl batch only) → cdn/logs/edition-numbers-2026-10-02.sql (now logs/cutover/).
- 2026-10-02 — 3.1 + 3.2 done (16 migrations, 3 triggers). Next: 3.4 import + check 469 / 300 numbered / 381 signed / 35 registered; 2.3 keypair upload pending.
- 2026-10-02 — 3.4 done; DB-T qr counts match (469 / 300 / 381 / 35). 3.3 waits for the assets merge (regenerated seed).
- 2026-10-02 — assets merged (b3be086, 67774da), relayout done (1.1), .env in C:\Development\scutoons, bunny zone empty. Migration 17 `2026-10-02-assets` added to 3.2; 3.3 unblocked. Next: 3.3 seed, migration 17, then 1.3/1.4 from C:\Development\scutoons\server. Worktree scutoons-assets can be removed later.
- 2026-10-02 — 3.3 + 3.5 done (48 entities, roles, users 0, editions NULL 169 + vinyl 300); migration 17 awaiting the owner's confirmation; 1.3 + 1.4 done. Owner running 1.5 (log cdn/logs/bunny-init-2026-10-02.json, now logs/cutover/), then 1.6, 1.7.
- 2026-10-02 — migration 17 ran on DB-T (3.2 complete). 1.5 bunny upload running.
- 2026-10-02 — 1.5, 1.6, 1.7 done (bunny = cdn/root, CORS + Range verified). Open in Phase 1: 1.8. Next: 1.8, then Phase 5.
- 2026-10-02 — 10.1b done (7 bunny secrets in production). Awaiting confirmation: 2.3 keypair upload, 1.8 write prefix.
- 2026-10-03 — 10.10 done on branch `dev-mariadb` (local MariaDB in Docker, tests + CI on it). Open: production's version / sql_mode / collation from phpMyAdmin → `docker/compose.yml`.
- 2026-10-04 — repo `cdn/` removed (branch media-folder): media/ + scripts/dev/media-serve.mjs, logs → logs/cutover/.
