-- A deleted account gives its codes back (owner 2026-10-07, BACKLOG 10.39: "What happens to a unique item (max_world)
-- when its owner's account is deleted? – It is back up for grabs. The code still applies."; ITEMS.md "Item limits").
-- How an account is deleted: there is no deletion feature in the API / viewer / admin – an account goes by a plain
-- `DELETE FROM users` (phpMyAdmin). The foreign keys (ON DELETE CASCADE) then remove its inventory, avatar, sessions,
-- save game, roles, … – so its items stop counting toward `max_world` by themselves (InventoryService::worldFull counts
-- the owners in `inventory`). What stayed behind: `qr.claimed_by` (no foreign key) kept pointing at the gone account and
-- `qr.email` kept its address, so the physical product's code was dead for good (409 already_claimed) and held the
-- deleted person's email.
-- From now on the trigger below releases every code the account owned when its users row is deleted (claimed_by NULL,
-- email '' = "not registered", as an admin release; the guard qr_owner_immutable allows A → NULL): the next person
-- claims the code as usual and gets what it contains. Open ownership claims the account had filed are withdrawn (the
-- claimant is gone); claims others filed for its codes stay open (an admin's "transfer" then claims the free code for
-- the claimant). item_transfers / code_events keep their rows (no foreign keys on purpose; the admin shows "deleted
-- account"). The same rule repairs accounts deleted before this migration (the two UPDATEs).
-- Analysis 2026-10-07 on the dev DB: the prod seed carries no accounts or codes (1 local test user, 1 promo code, 0
-- claimed codes, 0 codes of a missing account, 0 open claims of a missing claimant) – production's numbers come from
-- the SELECTs below. Idempotent (the UPDATEs match nothing a second time; CREATE TRIGGER IF NOT EXISTS).
-- phpMyAdmin: select the database (d0487143), SQL tab. Check first – codes still bound to an account that no longer
-- exists, and open claims filed by one (both answer 0 afterwards):
--   SELECT q.`code`, q.`type`, q.`claimed_by` FROM `qr` q LEFT JOIN `users` u ON u.`id` = q.`claimed_by` WHERE q.`claimed_by` IS NOT NULL AND u.`id` IS NULL;
--   SELECT c.`id`, c.`code` FROM `ownership_claims` c LEFT JOIN `users` u ON u.`id` = c.`claimant_id` WHERE c.`status` = 'open' AND u.`id` IS NULL;
-- then run the two UPDATEs; the trigger: set "Delimiter" to $$ and replace the trailing ";" of the CREATE TRIGGER line
-- with $$ (or run: php bin/console migrate).
UPDATE `qr` q LEFT JOIN `users` u ON u.`id` = q.`claimed_by` SET q.`claimed_by` = NULL, q.`email` = '' WHERE q.`claimed_by` IS NOT NULL AND u.`id` IS NULL;
UPDATE `ownership_claims` c LEFT JOIN `users` u ON u.`id` = c.`claimant_id` SET c.`status` = 'withdrawn', c.`decided_at` = UTC_TIMESTAMP() WHERE c.`status` = 'open' AND u.`id` IS NULL;
CREATE TRIGGER IF NOT EXISTS `users_release_codes` BEFORE DELETE ON `users` FOR EACH ROW BEGIN UPDATE `qr` SET `claimed_by` = NULL, `email` = '' WHERE `claimed_by` = OLD.`id`; UPDATE `ownership_claims` SET `status` = 'withdrawn', `decided_at` = UTC_TIMESTAMP() WHERE `claimant_id` = OLD.`id` AND `status` = 'open'; END;
