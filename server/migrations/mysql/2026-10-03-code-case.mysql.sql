-- Codes are case-sensitive (owner 2026-10-03: "make it case sensitive"): every column that holds a code gets the
-- binary collation utf8mb4_bin, so `code = ?` matches exactly – as on SQLite (BINARY) and as the generator makes them
-- (nanoid: A-Z a-z 0-9 _ -). Until now these columns were utf8mb4_general_ci: lookups ignored case.
-- All code columns change together (joins on code must not mix collations: run the whole file at once).
-- The unique key on qr.code was case-insensitive, so no codes that differ only in case exist: nothing can collide.
-- Idempotent (MODIFY to the same definition again changes nothing). phpMyAdmin: SQL tab, run the file as it is.
-- Safe in either order with the deploy: the API only uses `code = ?` and never relies on the collation.
ALTER TABLE `qr` MODIFY `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL COMMENT 'qr code';
ALTER TABLE `code_items` MODIFY `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL;
ALTER TABLE `inventory` MODIFY `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL;
ALTER TABLE `login_requests` MODIFY `claim_code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL;
ALTER TABLE `code_events` MODIFY `code` VARCHAR(8) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL;
ALTER TABLE `users` MODIFY `signup_code` VARCHAR(32) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL COMMENT 'code used to create the account';
ALTER TABLE `ownership_claims` MODIFY `code` VARCHAR(32) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL;
ALTER TABLE `item_transfers` MODIFY `code` VARCHAR(32) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NULL DEFAULT NULL COMMENT 'the code that moved (NULL = a virtual item gift)';
