Status 2026-10-12: concept, waits for the owner's answers Q1–Q9 (FEATURES 10.79). The map-only reset it mentions as "being built" is built (1ac465ca).

# ACCOUNT-RESTART — play SCUTOONS again as a new identity, keep the physical items (concept, 2026-10-11)

Research only – nothing built. Owner (2026-10-11): "einen Agenten starten, der ein Research macht, wie man ein Account
komplett neu starten könnte, ohne dass physikalische Items verloren gehen. Quasi als eine neue Identität das Ganze
nochmal durchspielen. Und physische Items sharen? Keine Ahnung, da brauchen wir noch ein paar Konzepte."

Not this file's topic: the map-only reset (`resetAt` in the map state, `shared/content/src/map.ts` `resetMapState`,
`ProgressService::mergeMapState`) – being built in parallel. A full restart can reuse its idea (a marker that beats the
union merge) for the whole save game.

## 1. Where an account's state lives today

| State | Where | Keyed by | Multi-device |
|---|---|---|---|
| Save game (variables incl. `picked:` keys, checkpoints, history, map, `created_at`) | `progress` row `toon_id='scutoons'` (`ProgressService`), localStorage `scutoons_game_scutoons` (`viewer/src/lib/contexts/world.svelte.ts`), shape `shared/content/src/save-game.ts` | user | UNION (map, checkpoints, history), variables newer-wins – a delete on one device comes back from another |
| Inventory | `inventory` (`InventoryService`), one row per piece / per stack; `source`, `code`, `edition_number` | user | server only (guest: localStorage, imported on login) |
| Story grant refs | `rate_limits` bucket `grantref:<user>:<item>`, ref = place (+ respawn period) – never pruned | user + item | server |
| Take refs | `rate_limits` bucket `takeref:<user>:<item>` | user + item | server |
| Scan cooldowns | `rate_limits` `scan:` / `scanday:` / `grant:` | user | server, time-windowed |
| Avatar | `avatar_slots` (entry per slot), `avatar_body`, `member_characters` / `member_character_looks` (`CharacterUnlocks`, source `default|code|item|story|admin`) | user | server |
| Reader stats / effects | `member_stats`, `member_effects` (`Reader/ReaderState`) | user | server |
| Profile | `users`: email (UNIQUE, the login), `display_name` (= `{reader.name}`), locale, `hide_sensitive`, `show_hints`, `media_quality`, `marketing_opt_in`, `signup_code` | user | server |
| FX settings | localStorage `scutoons_fx_volume`, `scutoons_fx_haptics` (`viewer/src/lib/fx-settings.svelte.ts`) | device | none |
| Owned codes (physical pieces, memberships) | `qr.claimed_by` (+ `qr.email`), guarded by `qr_owner_immutable` | user | server |
| Ownership history | `item_transfers`, `ownership_claims` (+ files) (`Ownership/*`) | code / user | server |
| Roles, sessions, API tokens (MCP), AI usage | `user_roles`, `sessions`, `api_tokens`, `agent_usage` | user | server |
| Newsletter | `mail_contacts` / `mail_suppressions` – by **email**, not user | email | server |

Precedent: `scripts/dev/test-user.mjs --reset` (lines ~156–190) is already a raw-SQL "fresh signup" for the dev test
user: deletes non-starter inventory, resets starters to qty 1 (re-inserts consumed ones), deletes `progress`,
`avatar_slots`, `avatar_body`, `member_stats`, `member_effects`, `grantref:<user>:%` – but it also **releases the
codes** (`UPDATE qr SET claimed_by = NULL`), i.e. exactly what a reader restart must NOT do.

## 2. Item sources (inventory `source`, verified in code)

| source | Written by | What it is | Physical? | On restart |
|---|---|---|---|---|
| `qr` | `CodeService::claimInTransaction` (line ~253), `OwnershipTransfers::moveCode` (claim / admin move) | the twin of a bought piece; row has `code` (+ `edition_number`); unique (one code = one owner) | **yes** (real object, money) | **keep** |
| `gift` with `code` | `OwnershipTransfers::moveCode` kind `gift` (via `Gifts`) | a physical twin given by another reader | **yes** | **keep** |
| `gift` without `code` | `Gifts::gift` (line ~105) → `add(…, 'gift')` | a virtual item another reader gave | no – but a social act, not this playthrough's story | keep (owner question Q3) |
| `story` | `InventoryService::grant` (source `story`, ref = place) | a pickup in a toon | no | **reset** (+ its refs) |
| `scan` | `InventoryService::grant` (source `scan`, needs a scan tool) | the camera found a real cucumber etc. | half: a real-world find, but not owned / not unique | reset (owner question Q4) |
| `craft` | `InventoryService::craft` | output of a recipe (inputs consumed) | no | reset |
| `starter` | `Users` signup (line ~58), `grantStarterWearables` | camera, default wearables | no, everyone has them | reset to a fresh signup's set (qty 1, consumed ones back) |
| guest import | `InventoryService::import` → `source = grantable[0]` (`story`/`scan`) | a guest's device inventory merged on login | no | reset; the device's guest copy must not re-import (§4) |

Not inventory sources, but owned: **purchases** arrive only as codes (`qr`) – there is no `purchase` source; the shop
app has no orders. **Memberships** = claimed `qr` rows of a membership type; access is derived in
`AccountService::me` / `Access/Permissions` (a membership code may add no item at all) → keep. **Admin** has no
inventory grant; admins move codes (`kind admin`, lands as `qr`) and unlock characters (`source admin`).
**Character unlocks** (`CharacterUnlocks::SOURCES`): `code` / `item` / `admin` / `default` keep, `story` reset
(today only the admin writes unlocks, so in practice: keep all).

Rule of thumb: **physical = the row has a `code`** (and `kind: physical` in the catalog; `take` already refuses
physical items). Everything without a code is gameplay, except the virtual gift (Q3).

Edge to check before building: `InventoryService::remove` consumes virtual pieces first but falls back to
code-bound pieces (`ORDER BY (code IS NOT NULL)`) – a recipe whose input is a physical item would delete a physical
twin's row while `qr.claimed_by` still points at the account. Today probably no recipe does that; a restart must not
"restore" it either. Worth a guard (recipes never consume a coded piece).
**Fixed on branch `claude/quizzical-goldstine-44c6a4` (2026-10-12, owner: "physische Items, kombiniert, verschwinden
nicht"):** `remove` never touches a code-bound row; `craft` skips `kind: physical` inputs and code-bound pieces (they
only have to be there), the output comes in addition; `use` / `take` count only consumable pieces
(`InventoryService::consumable`). Test: `InventoryServiceTest::testCraftingNeverConsumesAPhysicalPiece`.

## 3. Reset vs keep (a "new identity")

| | Reset | Keep | Note |
|---|---|---|---|
| Save game (map, checkpoints, history, variables, `picked:` / `took:`, `created_at`) | ✓ | | `created_at` restarts `reader.daysSinceStart` and the shuffle seed |
| Story grants + `grantref:` refs, takes + `takeref:` refs | ✓ | | refs must reset with the items or the places stay empty |
| Crafted, scanned, story, guest-imported items | ✓ | | |
| Starter items | ✓ (back to qty 1) | | as `test-user --reset` |
| Physical twins (`code` rows), owned codes, memberships | | ✓ | never release `qr.claimed_by` |
| Virtual gifts received | ? | ? | Q3 |
| Avatar: worn slots | ✓ (unequip all, re-wear basic dress) | | coded wearables stay in the inventory, just taken off |
| Avatar body / character unlocks | `story` ones | `code`/`item`/`admin`/`default` | Q5: re-pick the body? |
| Reader stats / effects | ✓ | | baselines |
| Reader name (`display_name`) | ✓ (asked again by `askText`) | | "new identity" – Q2 |
| `hide_sensitive`, `show_hints`, `media_quality`, locale, FX | | ✓ | settings of the person, not the character |
| Certificates, `item_transfers`, `ownership_claims` | | ✓ | proof of ownership is outside the game |
| Newsletter, marketing opt-in, roles, sessions, API tokens, AI usage | | ✓ | account / person level |
| Rate-limit windows (`grant:`, `scan:`, `scanday:`, `import:`) | | ✓ | abuse limits must not reset (else reset = farm) |

## 4. Concepts

### A. Playthroughs (recommended)
The account owns 1..n playthroughs; one is current. Everything story-made belongs to a playthrough, everything
physical / personal to the account.
- **Data:** `playthroughs (id, user_id, number, name NULL, started_at, ended_at NULL)`; `users.playthrough_id`.
  `progress` gets a `playthrough_id` column in the key (or, minimal: `toon_id = 'scutoons'` stays and the row is
  per playthrough). `inventory.playthrough_id` NULL = account-level (`code` rows, Q3 gifts); set = story-made.
  Refs move to `grantref:<user>:<pt>:<item>` / `takeref:<user>:<pt>:<item>`. `avatar_slots` per playthrough (or
  reset on switch). `member_stats` / `member_effects` per playthrough.
- **Reads/writes:** every inventory query becomes "account-level OR current playthrough"; `count`, `room`,
  `worldFull` (`max_world` counts accounts – a unique item in an archived playthrough still blocks it: decide, Q6).
- **Restart:** `POST /me/playthroughs/` → new row, current = it; nothing is deleted, the old one is archived
  (could be switched back to, or shown as "your first journey").
- **Devices:** the save game carries `playthrough` (id). `ProgressService::put` refuses / ignores a write for a
  playthrough that is not current (409 `playthrough_changed` with the current state); the viewer replaces its local
  copy instead of union-merging. Guest localStorage inventory is never imported into a playthrough other than the
  one the device started in.
- **Migration:** one idempotent SQL: create table, one playthrough #1 per user, backfill `progress`, `inventory`
  (NULL for `code IS NOT NULL`), rename existing `grantref:`/`takeref:` buckets (`UPDATE rate_limits SET bucket = …`).
- **Risks:** touches every inventory query and the grant/take paths; `max_world` semantics; more rows over time.
  Best history and reversibility.

### B. A second persona under the same login
Like A, but the persona is a full identity: own `display_name`, avatar body, unlocks, maybe own settings, switchable
in the account menu (Netflix profiles). Physical items are account-level and visible to all personas (= "sharing with
myself" for free).
- **Data:** `personas` replaces `playthroughs`; every user-keyed gameplay table gets `persona_id`.
- **Devices:** the device remembers the active persona; the save game key becomes `scutoons_game_<persona>`.
- **Risks:** the biggest change (avatar, character unlocks, reader stats, progress, inventory, refs); a household
  could use personas as family members – then it is really a household feature (§5) and needs consent rules for
  minors. Only worth it if several people / characters per login is a goal.

### C. Move the physical items to a fresh account
The reader signs up again with another email; the physical codes move from the old account to the new one.
- **How:** today's transfer core already does it: `OwnershipTransfers::moveCode` (kind `admin`, or a new kind
  `self`) per owned code; the inventory rows follow the code. Memberships are codes too.
- **Data:** none, or a kind `self` in `item_transfers`. Optional: a "move everything to my other account" flow with a
  confirmation mail to both addresses.
- **Devices:** a different login – no merge problem at all; the old account's devices keep their state.
- **Risks:** needs a second email (`users.email` UNIQUE); loses name, roles, tokens, newsletter link; **deleting the
  old account first releases its codes to anyone** (trigger `users_release_codes`, migration
  `2026-10-07-account-deletion-frees-codes`) – the move must come before any deletion. Works almost today (an admin
  can move codes one by one). Good fallback / support tool, poor product feature.

### D. In-place reset with an epoch (tombstone)
One button "Start over": the server deletes the story-made rows (as §3) and raises `users.game_epoch`.
- **Data:** `users.game_epoch INT`; the save game carries `epoch`. Refs either deleted (`grantref:<user>:%`,
  `takeref:<user>:%`) or include the epoch.
- **Devices:** `ProgressService::put` / `mergeSaveGame`: a state with a lower epoch never merges into a higher one –
  it is dropped and the device gets the new empty state (the same trick as the map's `resetAt`, for the whole save
  game). The viewer on load: server epoch > local epoch → replace localStorage, clear the guest inventory copy.
- **Migration:** one ADD COLUMN; existing save games have epoch 0.
- **Risks:** destructive, no way back (a mis-tap loses a long journey → strong confirmation, maybe a 7-day undo
  copy in a `progress_archive` row); otherwise the smallest change. It is A with only one playthrough kept.

## 5. Sharing physical items (owner's open question)

What exists: **gifting** (`Ownership/Gifts`, `OwnershipTransfers::moveCode` kind `gift`) moves the code for good;
claims move it by admin decision; **reading with someone else's code is allowed** (access ≠ ownership). The
certificate never names the owner (signed by hand, "Original buyer" empty) – the platform's transfer log is the
record. So sharing never touches the certificate; it touches `qr.claimed_by`, the `inventory` row and `avatar_slots`.

| Meaning | Ownership | Who may use / wear | Data | Notes |
|---|---|---|---|---|
| Between my own identities (A / B) | account | every playthrough / persona | none (account-level rows) | comes free with A/B |
| **Lend** to a person | stays with the owner | the borrower, until returned / a date | `item_loans (code, owner_id, borrower_id, until, returned_at)`; the borrower sees the piece (flag `lent`), may wear it; not giftable / not craftable by the borrower | owner can recall; ends on owner's account deletion |
| **Gift** | moves | the new owner | exists | certificate passed on with the object |
| **Household / crew** | each piece keeps one owner | all members use (read, wear, scan tools) | `households (id, name)`, `household_members (household_id, user_id, role, joined_at)`; inventory reads add "pieces of my household" as shared, never counted for `max`/`max_world` | invites + leave; minors (consent); one household per account? |

Use, not ownership, is what is shared: `max_world` keeps counting owners; claims and gifts still need the owner;
a lent / shared piece is never moved by the borrower.

## 6. Recommendation

1. **Build D first, shaped as A** – an epoch number in the save game and on the user, server-side "start over" that
   deletes story-made rows + refs and keeps every `code` row, memberships, settings, tokens. Small, solves the
   multi-device resurrection the union merge causes, and the epoch is exactly A's playthrough number later.
2. **Grow into A (playthroughs)** if the owner wants old journeys kept or switchable: then archive instead of delete
   (`playthrough_id` on progress / inventory / refs).
3. **C as a support tool** (admin "move all codes to account X") – nearly there, needed anyway for people with two
   emails.
4. **B only together with households** – it is the same question ("several people / characters on one account").
5. Sharing: start with **lending** (one table, no ownership change, clear UX); households later.

## 7. Open owner questions

- Q1. Restart = delete the old journey (D) or keep it as an archive you can return to (A)?
- Q2. New identity = a new reader name (`{reader.name}` asked again) and a new avatar body / outfit choice?
- Q3. Virtual gifts from other readers: keep (account) or reset (they belonged to the old journey)?
- Q4. Camera finds (`scan`, real cucumbers): gameplay (reset) or "my real-world finds" (keep)?
- Q5. Crafted items made FROM a physical item (if a recipe ever allows it) – what survives?
- Q6. A unique (`max_world`) story item in an old journey: freed for others on restart, or still blocked?
- Q7. How often may a reader restart (abuse: farming unique / limited items)? Rate limits stay account-level.
- Q8. Sharing: is lending enough, or do you want households (family, crew) – and do they include minors?
- Q9. Guests (no account): is "clear local data" enough, or a visible "start over" in the viewer too?

Files: `server/src/Domain/Inventory/InventoryService.php`, `server/src/Domain/Progress/ProgressService.php`,
`shared/content/src/save-game.ts`, `shared/content/src/map.ts`, `viewer/src/lib/contexts/{world,inventory}.svelte.ts`,
`server/src/Domain/Ownership/{Gifts,OwnershipTransfers,OwnershipClaims}.php`, `server/src/Domain/Codes/CodeService.php`,
`server/src/Domain/Characters/CharacterUnlocks.php`, `server/src/Domain/Reader/ReaderState.php`,
`scripts/dev/test-user.mjs`, `agents/ITEMS.md` ("Ownership, claims, gifting", "Certificates", "Item limits"),
`agents/research/second-hand-market.md`.
