# CUTOVER — the one runbook (beta 2026-10-04, side by side, new database)

Owner: Tilman. Guide: the cutover agent (updates the checkboxes, answers "next step", diagnoses errors read-only).
Strategy (owner, 2026-10-02): **bunny first, then the new platform side by side** in a new FTP subfolder with a **new
MySQL database**, tested on test hostnames, then the subdomains are switched one by one; the old system stays untouched
and reachable at `legacy.scutoons.com` / `qr-legacy.scutoons.com` (= backup + rollback). Readers stay on
`cdn.scutoons.com` (all-inkl) until after the beta.

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
| bunny keys (`BUNNY_*`, `CDN_PUBLIC_URL`) | repo-root `.env` of `C:\Development\scutoons-assets` (gitignored) – already there | assets:sync / check-storage (local) |
| old CDN FTP (cdn.scutoons.com) | `cdn/.env` (gitignored) – unchanged | `cdn/sync.py` |
| old DB, old FTP | unchanged, your password manager | old system (legacy) |

---

## Current state (fill in during Phase 0 – no secrets, only names/paths/numbers)

| What | Value |
|---|---|
| KAS target dir `scutoons.com` / `www.` | |
| KAS target dir `qr.scutoons.com` (old qr folder) | |
| KAS target dir `creator.scutoons.com` (in use?) | |
| KAS target dir `cdn.scutoons.com` (old CDN folder) | |
| Old DB tables (expected: `qr` only, maybe `schema_migrations`) | |
| Old DB `qr` at export T0: rows / registered (`email<>''`) / signed / `CHECKSUM TABLE qr` / time (UTC) | |
| New subfolder name (proposal `scutoons-next/`) | |
| Test hostnames (proposal `next.scutoons.com` = viewer, `qr-next.scutoons.com` = qr) | |
| DB labels: **DB-T** (test build) / **DB-P** (switch-day build, only if needed – Phase 7) | |

---

## Agent work this plan needs (flagged, not done yet)

| # | Task | Needed for | Estimate |
|---|---|---|---|
| T1 | **Required.** Add the viewer test origin `https://next.scutoons.com` to `ALLOWED_ORIGINS` in `deploy/targets.json` (api `config.php`; it is also the CSRF list) + deploy README note. Without it the viewer on the test host cannot sign in (CORS). Fallback without a code change: use `embassy.scutoons.com` as the viewer test host (already allowed; embassy is held back). | Phase 6 | 15 min + dry run |
| T2 | Optional. `cdn/sync.py` skips `.json` / `.bin` → the camera model `models/mobilenet_v2_100_224/` never reaches cdn.scutoons.com. Add both to `SYNC_EXTENSIONS` – or upload that folder by hand (step 4.3). | Phase 4 | 10 min |
| T3 | Only if the owner cannot get an FTP user rooted in the subfolder: a remote prefix in `deploy/targets.json` / `deploy.mjs` (e.g. `${FTP_ROOT}`). | Phase 5 | 30–60 min |
| T4 | Only if the owner rejects "rebuild or keep" in Phase 7: a delta import (`bin/console legacy-delta --since=<T0> --sql` from an old-DB export: new codes INSERT IGNORE, new email registrations / re-signatures UPDATE where the new DB has no owner). Not recommended – the checksum gate in 7.1 makes it unnecessary. | Phase 7 | 2–3 h |

What already exists (no build needed): the 16 idempotent migrations (they turn a copy of the old `qr` table into the
new schema exactly like an in-place upgrade), `content-seed.mysql.sql`, `backfill-edition-numbers --sql` (no DB needed),
`tests/entities-compare.php <api> --report`, the old DB's own export as the import source. The fresh-DB schema file
`server/migrations/schema/qr.mysql.sql` is **incomplete** (no content/mail/ownership tables) – do not use it.

---

## Phase 0 — Safety (nothing changes for readers)

Decide before: nothing.

- [ ] **0.0 O (optional, D7) – Promo codes first:** if you want to sign up (6.3) and invite testers with promo codes
  made in the OLD `qr.scutoons.com/generate/`, create them now – the export in 0.1 carries them into the new DB.
- [ ] **0.1 O – Export the OLD database.** phpMyAdmin → old DB → *Export* → *Custom* → all tables, structure + data,
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
- [ ] **0.2 O – Back up the old qr folder (FTP/WinSCP, the main FTP account).** Download into an offline folder:
  `api/keys/keypair.json` (**CRITICAL – the private signing key; never upload anywhere public, never send by mail/chat**),
  the whole `generate/files/` (`batch/`, `certificates/`, `svgs/`, `signatures/`, …), `generate/.htpasswd`, `config.php`.
  Verify: per subfolder of `generate/files/` file count + total size = the server's (WinSCP "Properties");
  `Get-FileHash keypair.json -Algorithm SHA256` noted (hash only, here in the log is fine); the `publicKey` inside the
  file equals the one served by `https://qr.scutoons.com/api/keys/keys.php` (you compare the first/last 20 characters –
  agents never open the keypair file).
- [ ] **0.3 O – Back up the old viewer folder** (FTP download of the folder from "Current state"). Optional – it stays
  untouched anyway.
- [ ] **0.4 O – Freeze the old system's deploys:** GitHub → `mandymozart/qr.scutoons.com` → Settings → Actions →
  *Disable actions* (its workflow deploys production on a push to any branch). Also: **no new batches / regenerations
  in the old `generate/` from now on** (they would not reach the new DB – see 7.1).
- [ ] **0.5 O – Record the KAS state** (KAS → Domains): the target directory of every (sub)domain → "Current state".
  Check that none of the new folder names below would collide with an existing folder.
- [ ] **0.6 A – Local copies checked:** `cdn/root` = 824 files / 855,854,341 bytes, identical copy in
  `C:\Development\scutoons-assets\cdn\root` (checked 2026-10-02) = the media backup; `server/db/local.sqlite` copied
  to `server/db/local.sqlite.bak-2026-10-0X-cutover`.
- [ ] **0.7 O (optional) – Old CDN listing:** WinSCP/FileZilla on the CDN FTP folder → a recursive listing saved to
  `cdn/logs/remote-listing-2026-10-0X.txt` (the sync never deletes, so this is documentation only).

Phase 1 can start in parallel (it only touches bunny; the upload takes a while).

---

## Phase 1 — bunny.net initialisation (readers unaffected; they stay on cdn.scutoons.com)

Decide before: **D1** keep or delete the 134 files that exist only on bunny (old `images/1x|3x/**`, 6 old album mp3s,
`videos/output.mp4`; 10 of them referenced nowhere). Recommendation: **keep** (sync never deletes, deleting in the
dashboard is irreversible, they cost nothing). Emptying the zone is NOT needed.

All commands in `C:\Development\scutoons-assets\server` (PowerShell). Source = the main checkout's media.
- [ ] **1.1 O – Write fence off.** `server/config.local.php` line with `BUNNY_WRITE_PREFIX` must be commented out
  (`//define('BUNNY_WRITE_PREFIX', …`). State 2026-10-02: **already commented out** – just confirm.
- [ ] **1.2 O – Read-only check:** `php bin/console assets:check-storage` → all keys "set", zone `scutoons` listed,
  pull zone 6731366 origin = storage zone. Expect ≈ 628 to upload, 2 different, 134 only on bunny.
- [ ] **1.3 O – Plan (dry run):**
  `php bin/console assets:sync --source-root=C:/Development/scutoons/cdn/root --hash`
  Expect: ≈ 628 to upload (≈ 664 MB), 2 different (`catpope/audio/CatPope-Awakening-Music.zip`,
  `catpope/images/s1c2-thumbnail.jpg`), 134 untouched. Anything else → tell the guide first.
- [ ] **1.4 O – Apply:**
  `php bin/console assets:sync --source-root=C:/Development/scutoons/cdn/root --hash --apply --overwrite --purge-zone --log=../cdn/logs/bunny-init-2026-10-0X.json`
  (verified uploads, 3 retries, never deletes). Long-running – leave the window open.
- [ ] **1.5 O – Verify:** `php bin/console assets:check-storage` → **0 missing, 0 different** (134 only-on-bunny is
  fine). **A:** HEAD samples through the pull zone, size = local:
  `https://scutoons.b-cdn.net/catpope/videos/s1-trailer-loop.720p.mp4`,
  `…/catpope/videos/roar-little-kitty.480p.mp4`, `…/catpope/images/webtoon/s1c2/s1c2-slice-1.webp`,
  `…/scutoons/items/repair_coupon.png`, `…/characters/catpope/wear/scutoons_tshirt.webp`,
  `…/models/mobilenet_v2_100_224/model.json`, `…/catpope/audio/CatPope-Awakening-Music.zip`.
- [ ] **1.6 O – Write fence back on:** uncomment `define('BUNNY_WRITE_PREFIX', '_test/');` in the worktree's
  `server/config.local.php` (no dev server may write to the production zone).
- Rollback: none needed (no reader uses bunny; uploads can be re-run; nothing was deleted).

---

## Phase 2 — The new slot on all-inkl + GitHub (nothing public changes)

Decide before: **D2** subfolder name + test hostnames (proposal above) · **D3** `VITE_API_URL` straight to
`https://api.scutoons.com/api` (recommended – api. exists from day one; the old viewer keeps using qr.scutoons.com/api,
which the new qr app proxies after the switch).

- [ ] **2.1 O – Subfolder + FTP user.** KAS → FTP → new folder `<NEW>/` (e.g. `scutoons-next/`) next to the old
  folders, and a **new FTP user whose root is `<NEW>/`** (then `deploy/targets.json` needs no change). If KAS can't do
  that → T3. Credentials → GitHub `production` secrets `FTP_SERVER` / `FTP_USERNAME` / `FTP_PASSWORD` (see table).
  Verify: log in with WinSCP as the new user → you see an empty folder, not the old ones.
- [ ] **2.2 O – Folders in `<NEW>/`:** `viewer/dist/`, `admin/dist/`, `creator/dist/`, `shop/dist/`, `qr/public/`,
  `api/public/`, `api/storage/keys/`, `api/storage/files/`.
- [ ] **2.3 O – Private data → `<NEW>/api/storage/`** (before the first api deploy): backup's `api/keys/keypair.json`
  → `api/storage/keys/keypair.json`; backup's `generate/files/*` → `api/storage/files/` (same subfolders).
  Verify: file counts + sizes per subfolder = 0.2; keypair SHA-256 = 0.2. Rollback: delete the copies (old untouched).
- [ ] **2.4 O – New database DB-T.** KAS → Databases → new database (MariaDB, utf8mb4). Name/user/password → GitHub
  `production` (`DB_HOST`, `DB_NAME` variables; `DB_USER`, `DB_PASS` secrets).
- [ ] **2.5 O – GitHub environment `production`** (restricted to branch `main`): every entry of the root
  `.env.example` – `AUTH_SECRET`, `QR_PROXY_SECRET` new random strings ≥ 32 chars; `MAIL_FROM=no-reply@scutoons.com`,
  `SMTP_*` of that mailbox (exists, SPF/DKIM set); `VITE_API_URL=https://api.scutoons.com/api` (D3),
  `VITE_CDN_URL=https://cdn.scutoons.com`, `VITE_WWW_URL=https://www.scutoons.com`, `VITE_CREATOR_URL`,
  `VITE_SHOP_URL`, `VITE_ADMIN_URL`; `GENERATE_PASSWORD` (old value, only for the legacy `generate/`).
  Verify: the summary at the end of `.env.example` (10 secrets, 12 variables) all present.
- [ ] **2.6 O – Subdomains → `<NEW>/`** (KAS → Domains → Subdomain; SSL Let's Encrypt + "force HTTPS" on each):
  new `api.` → `<NEW>/api/public/` · new `admin.` → `<NEW>/admin/dist/` · new `shop.` → `<NEW>/shop/dist/` ·
  `creator.` → `<NEW>/creator/dist/` (if 0.5 shows it unused) · test `next.` → `<NEW>/viewer/dist/` · test `qr-next.`
  → `<NEW>/qr/public/`. **Do not touch `scutoons.com`, `www.`, `qr.`, `cdn.` yet.**
  Rollback: delete the new subdomains.

---

## Phase 3 — Build the new database DB-T (old DB untouched)

Decide before: **D4** the duplicate code sets (37 Dago codes signed #1–37 and 20 beetles codes in no batch manifest,
MEMORY "qr.scutoons.com backend") – **not blocking**: the import copies every code as it is; the backfill numbers only
manifest codes. Decide before the public release which set was printed (revoke the other in the admin).

- [ ] **3.1 O – Import the old data.** phpMyAdmin → DB-T → *Import* → the 0.1 file (`.sql.gz` is fine). Verify:
  `SELECT COUNT(*) FROM qr;` = 0.1's number.
- [ ] **3.2 O – Migrations, in this order** (`server/migrations/mysql/`, *Import* tab per file; all idempotent):
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
  Not to run: `migrations/mysql/legacy/*` (already in the old data), `schema/qr.mysql.sql`, the assets branch's
  `2026-10-02-assets` (post-beta).
- [ ] **3.3 O – Content seed:** *Import* `shared/content/seed/content-seed.mysql.sql` (48 entities: catpope chapters
  0–5, scenes, season 1 with related items, the record-player-repair pickups, trailer + chapter-0 quality variants,
  credits, characters with Top Toy's tendencies, map, toptoy/yellowmouse). **Not needed:** `credits-seed`,
  `characters-seed` (subsets), `2026-10-01-media-quality-chapter0.mysql.sql` (only acts on an old chapter 0 – harmless
  if run).
- [ ] **3.4 A+O – Edition numbers.** A: `cd server; php bin/console backfill-edition-numbers --sql
  --manifests=<backup>/generate/files/batch > ../logs/edition-numbers-2026-10-0X.sql` (no DB needed; A reports the
  statement count). O: *Import* that file into DB-T.
- [ ] **3.5 O – Verify DB-T** (SQL tab; paste the results to the guide):
  ```sql
  SELECT COUNT(*) AS codes, SUM(email <> '') AS registered, SUM(signature IS NOT NULL) AS signed, SUM(edition_number IS NOT NULL) AS numbered FROM qr;
  SELECT edition_id, COUNT(*) FROM qr GROUP BY edition_id;
  SELECT COUNT(*) FROM entity_versions;            -- 48
  SELECT id, name FROM roles ORDER BY id;          -- member, admin, creator, liaison, ambassador
  SHOW TRIGGERS;                                   -- qr_owner_immutable, inventory_code_owner_insert, inventory_code_owner_update
  SELECT COUNT(*) FROM users;                      -- 0
  ```
  codes / registered / signed / per edition = 0.1 exactly; numbered = 3.4's statement count (minus codes not in the DB).
- Rollback: drop DB-T in KAS and start 3.1 again (the old DB is never touched).

---

## Phase 4 — Media uploads to the LIVE CDN cdn.scutoons.com (additive; old readers unaffected)

Readers (old and new builds) load media from cdn.scutoons.com (all-inkl) during the beta.
Decide before: nothing.

- [ ] **4.1 O – Sync** (`cd C:\Development\scutoons\cdn`, `.venv\Scripts\activate`, `python sync.py`): selection
  `3` → toons `all` (catpope, characters, models, scutoons, toptoy, yellowmouse). On "Upload … [MISSING on remote]?"
  answer `a` (all). On "**DIFFERS** … Upload local version?" answer **`y`** – **never `n`: `n` downloads the server's
  file over your local one.** Expected uploads include: chapter 2 slices `catpope/images/webtoon/s1c2/*`, key visuals,
  `s1-cover-*-textless.webp`, trailer `catpope/videos/s1-trailer-loop.{480p,720p,1080p}.mp4`, prologue
  `roar-little-kitty.{480p,720p,1080p}.mp4`, music `.128/.320.mp3`, item icons `scutoons/items/*` (players,
  broken record player, repair coupon, wear items, substances, tuna can, weed), `characters/**` (WebPs, avatars,
  A-poses, `wear/` overlays), supporters SVGs. Log: `cdn/logs/sync_*`.
- [ ] **4.2 O – Root config:** `python sync.py` → `1` (`.htaccess`, CORS for `*.scutoons.com` incl. `Range`) – only if
  it differs.
- [ ] **4.3 O – Camera model by hand** (sync.py skips `.json`/`.bin`; or T2 first): upload
  `cdn/root/models/mobilenet_v2_100_224/` (model.json + 4 shards) to the CDN FTP folder `models/mobilenet_v2_100_224/`.
- [ ] **4.4 A – Verify** (HEAD/GET, size = local): the sample list of 1.5 on `https://cdn.scutoons.com/…`, plus
  `Origin: https://scutoons.com` → `Access-Control-Allow-Origin: https://scutoons.com` on `model.json` and a `Range:
  bytes=0-524287` GET of `roar-little-kitty.480p.mp4` → 206 (media quality "Automatic").
- Rollback: none needed (new names only; the two replaced files are the corrected versions).

---

## Phase 5 — Deploy the new platform into `<NEW>/` (nothing points at it except the new/test hostnames)

Decide before: **D5** embassy stays held back for the beta (default yes: not in `targets.json` `order`, workflow manual).
Preconditions: **2.1 done (FTP secrets = the subfolder user!)**, 0.4 done, T1 done.

- [ ] **5.1 A – Local dry run:** `npm run deploy:dry-run` (builds everything; plan: api → `api/`, qr → `qr/`, viewer →
  `viewer/dist/`, shop, creator, admin; protected `storage/**`, `db/**`, `logs/**`). Plus the full suite on the tree
  to release (PLAN.md "definition of done").
- [ ] **5.2 O (with the coordinator) – Merge `platform` → `main` and push.** Every push to `main` deploys the touched
  apps (environment `production`) – into `<NEW>/` because of 2.1.
- [ ] **5.3 O – GitHub → Actions → "Deploy all" → Run workflow with dry run** → read the plan, then **without dry run**
  (order api → qr → frontends). Every run prints the migrations reminder – already done in Phase 3.
- [ ] **5.4 A – Verify:** `https://api.scutoons.com/api/health/` → `status ok`, `db ok`, `version 1.0.0`, `git_sha` =
  main's head; `https://{admin,shop,creator,next}.scutoons.com/version.json`, `https://qr-next.scutoons.com/version.json`.
- [ ] **5.5 O – Legacy `generate/` on the api** (only if you still want it): upload the backup's `generate/.htpasswd` to
  `<NEW>/api/public/generate/.htpasswd`, set `AuthUserFile` in `<NEW>/api/public/generate/.htaccess` to its absolute
  path (KAS shows `/www/htdocs/<account>/<NEW>/api/public/generate/.htpasswd`).
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

- [ ] **7.1 O – Gate.** In the OLD DB: `CHECKSUM TABLE qr;` and the 0.1 count query. **Identical to 0.1 → keep DB-T**
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
- [ ] **8.2 O – qr.scutoons.com → `<NEW>/qr/public/`** (SSL stays; re-issue if KAS asks). Repeat 6.2 on
  `qr.scutoons.com` with real codes, incl. 3 printed certificates on `/verify.html`. **Rollback: target back to the old
  qr folder (instant).**
- [ ] **8.3 O – scutoons.com + www.scutoons.com → `<NEW>/viewer/dist/`** (SSL both). Repeat 6.5 on the real domain;
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
- [ ] **9.3 O/A – Watch daily:** `<NEW>/api/logs/error.log` (FTP; paste excerpts to the guide), admin → Emails (failed
  mails), Claims (open), Events (scans, sign-ups), Dashboard; KAS traffic of qr.; `/api/health/`.

---

## Phase 10 — After the beta (towards the public release, ~a week later)

- [ ] 10.1 Fix the beta's issues on `platform` → `main` (deploys automatically).
- [ ] 10.2 Asset layer: merge `assets` (migration `2026-10-02-assets` in phpMyAdmin, `ASSET_STORAGE=bunny` + `BUNNY_*`
  into `production`/targets, `assets:scan`) – ASSETS.md.
- [ ] 10.3 cdn.scutoons.com → bunny: custom hostname on pull zone 6731366 + CNAME → `scutoons.b-cdn.net` + SSL;
  `CDN_PUBLIC_URL=https://cdn.scutoons.com`; a final `assets:sync` first (ASSETS.md "domain switch"). The old FTP CDN
  becomes the backup target.
- [ ] 10.4 Folder restructure: `assets:layout` re-run + owner review, batch relink (to build), the model path code change,
  then prune (ASSETS.md "Migration plan").
- [ ] 10.5 Remove the test hosts `next.` / `qr-next.` (and T1's origin); later `PROXY_FULL_API=0` when qr's log shows
  no `/api/*` traffic; remove `generate/` + `GENERATE_PASSWORD` after admin parity.
- [ ] 10.6 Embassy go-live (EMBASSY.md "To go live"): subdomain, `order`, workflow trigger, roles, import.
- [ ] 10.7 After one release without problems: delete the old folders (keep backups); the old DB stays as an archive.
- [ ] 10.8 Public release.

---

## Decide-before list (all phases)

| # | Decision | Blocks | Default / recommendation |
|---|---|---|---|
| D1 | bunny-only files: keep or delete | Phase 1 (no) | keep |
| D2 | subfolder name, test hostnames | 2.1, 2.6, T1 | `scutoons-next/`, `next.` + `qr-next.` |
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
